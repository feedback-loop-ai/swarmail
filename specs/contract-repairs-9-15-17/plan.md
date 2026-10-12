# Plan — contract repairs #9/#15/#17 (append-stable thread keys, a loud shim DELETE, one chaos delay rule)

Implementation plan for `specs/contract-repairs-9-15-17/spec.md`. All
anchors re-verified against the working tree on
`feat/epic13-09-15-17-contracts` this session.

## Technical context

- Single Rust crate; production under `src/`, real-protocol suites under
  `tests/` (`tests/compat.rs` HTTP-shim suite with `seed()` at :84,
  `tests/smtp_edges.rs` protocol/chaos suite, `tests/ui.rs` thread
  endpoints over real SMTP, shared helpers in `tests/common/mod.rs`:
  `start()`, `http_json`, `http_raw`, `smtp_send`, `SmtpConn` with
  `connect_raw`/`reply`/`data`). No mocks anywhere in the loop.
- The relevant production modules: `src/threads.rs` (grouping, pass 2 at
  :218-234 — the key at :227), `src/compat.rs` (`delete_messages` at
  :257-272, `IdsBody` at :71-76), `src/smtp.rs` (chaos call sites at
  :202-210, :374-383, :388-396, :439-450; the 250 sequence at :452-456),
  `src/api.rs` (routes at :60-90; `ApiError` at :41).
- Coverage is measured by `scripts/coverage-gate.sh` — literal integer
  100/100/100 lines/branches/functions, missed == 0, `coverage(off)`
  forbidden. The wave's production diff is **net-negative**: one changed
  line in `threads.rs`, a smaller `delete_messages`, a flattened DATA arm.
  Every surviving/new branch needs a wire-driven test or the gate refuses;
  that refusal is the gate working.
- No new dependencies: tokio, serde, serde_json and std suffice; the test
  harness uses `probability: 100` rules and bounded `tokio::time::timeout`,
  both already in the tree. Model/policy unchanged (`glm-flash` pinned).

## Constitution check

| Guarantee | Effect of this wave |
|---|---|
| 0001 accept == stored | Untouched. #15's 400 and #17's 451 both fire strictly before any insert and store nothing; #9 adds no state. The 250 sequence (`src/smtp.rs:452-456`) is byte-identical. |
| 0002 await subscribes before it checks | Untouched — and *protected*: #15 keeps shim deletes flowing through `Store::delete` synchronously, so removals wake watchers; a shim-side side-channel would hang long-polls. The rewritten shim test keeps that honest. |
| 0003 oldest-only pruning | Untouched. Named interaction: with #9's anchor, cap-pruning a thread's oldest mail re-keys that thread once — recorded in the migration note as the accepted residual, not changed behavior of 0003. |
| 0004 per-test inboxes | **Restored.** `clear_all` becomes the only cross-inbox destructive endpoint again; the shim's wipe-on-garbage fallback is deleted. Decision 0011 (proposed) records the ruling. |
| 0005 extraction at ingest | Untouched. No extraction or header parsing changes; #9 consumes only fields already extracted. |

**Zero-diff demand (binding for review):** `src/store.rs`,
`src/persist.rs`, `src/model.rs`, `src/extract.rs`, `src/mcp.rs` and the
SMTP accept path — the `250` sequence at `src/smtp.rs:452-456` — carry
zero diff. `src/chaos.rs` changes a doc comment only (no executable line).
If an implementation of this wave touches any of those files, it has left
the spec and goes back.

## Project structure — files touched

| File | Change |
|---|---|
| `src/threads.rs` | One line: `key: thread_key(&thread_node(emails[0]))` at :227 (the bucket is already sorted oldest-first at :221-225). Reword the module doc (:9-14), `Thread::key` (:31) and `thread_key` (:162-163) — the anchor is the conversation's first message, not the lexicographic representative. Add the AC1 unit regression. |
| `src/compat.rs` | `delete_messages` (:257-272) loses the `clear_all` fallback: `Err(JsonRejection)` → `400` via `ApiError(StatusCode::BAD_REQUEST, …)`; `Ok` with an empty `ids` → the same 400; otherwise delete exactly `body.ids` through `state.store.delete` and reply `{"removed": n}`. Return type `ApiResult<Json<Value>>`. Reword the `IdsBody` (:71) and handler (:257-259) doc comments, which currently endorse the wipe. |
| `src/smtp.rs` | DATA arm (:439-450): remove the `if rule.error.is_some()` nesting — a fired DATA rule always sleeps, answers `error.unwrap_or("451 4.3.0 chaos")` and `continue`s (never stores). Zero diff elsewhere in the file. |
| `src/chaos.rs` | Doc comment only: one sentence on `ChaosRule::error` — `None` means the stage's default failure (`451 4.3.0 chaos`, or the close at connect), not *no failure*. |
| `README.md` | Three sections: the API-table row for `DELETE /api/v1/messages` (:293); *Threads and the live UI* (:308-316) gains the derivation + migration note; *Chaos — break SMTP on purpose* (:336-354) gains the four-bullet delay-only table and the two pinned semantics. |
| `src/static/openapi.json` | The `delete` operation on `/api/v1/messages` (:141-145): explicit selector required; absent/empty/unparseable/empty-`ids` → `400`; `{"removed": n}` otherwise. |
| `src/static/llms.txt` | The shim-delete line (:92-94) states the new contract; the threads line (:71-74) states the first-message anchor. |
| `tests/compat.rs` | Rewrite `shim_deletes_go_through_the_store` (:606): keep the selective-delete section verbatim; convert no-body/empty/unparseable/empty-`ids` to `400` + before/after exact counts over **both** seeded inboxes; keep the `delete-all` section as the pinned sole wipe. |
| `tests/smtp_edges.rs` | In `chaos_on_connect_mailfrom_and_data_with_and_without_error` (:324): rewrite the delay-only DATA section (:359-374) to assert `451 4.3.0 chaos` + exact count unchanged (it currently endorses the bug by asserting the mail **is** stored); add delay-only MAIL FROM and RCPT sections asserting the same default line verbatim. Rewrite `connect_chaos_with_delay_only_still_drops_the_session` (:399) to assert the reply is exactly `""` under a bounded wait. |
| `tests/ui.rs` | Add the AC1 regression beside `thread_endpoints_group_a_references_chain_over_real_smtp` (:313): send `<z@x.io>`, record the key from `GET …/threads`, send `<a@x.io>` with `In-Reply-To: <z@x.io>`, assert the key is unchanged and `count == 2`. |
| `docs/decisions/0011-shim-delete-contract.md` | New, status `proposed` — the shim DELETE contract (see the spec's ruling; the decision names the previously-destructive inputs and the rejected alternatives). |

Not touched: `src/store.rs`, `src/persist.rs`, `src/model.rs`,
`src/extract.rs`, `src/mcp.rs`, `src/api.rs`, `deny.toml`,
`scripts/coverage-gate.sh`, `bundle.json`, `policy.json`, `docs/decisions/README.md`
(the index row for 0011 lands at operator acceptance, matching how the
doubled 0010s sit today).

## Data model & contracts

- **No new types, no persisted-shape change.** `Email` is untouched; the
  store and the data file carry zero diff. Threads remain a pure function
  of the inbox contents, recomputed per request — "no thread state exists
  anywhere, so it can never disagree with the store" stays true and stays
  the module's first sentence.
- **Changed wire contract #1 (decision 0011, proposed):**
  `DELETE /api/v1/messages` — 200 `{"removed": n}` iff the body is a
  parseable object with a non-empty `ids` array (deletes exactly those
  ids through the store); every other body — absent, empty, non-JSON,
  wrong-typed, `{"ids": null}`, `{"ids": []}` — answers `400` with a JSON
  reason and deletes nothing. `DELETE /api/v1/delete-all` unchanged.
- **Changed wire contract #2 (repair toward documented intent):** a
  delay-only DATA chaos rule now answers `451 4.3.0 chaos` and stores
  nothing (was: store + `250`). `ChaosEvent::Data`'s doc becomes true.
- **Changed derived value (repair toward documented intent):** the thread
  `key` is FNV-1a of the conversation's oldest member's node. Same
  alphabet (16 lowercase hex chars), same lookup routes; values differ
  from today's for any thread whose representative was not the oldest
  member's node, and for pre-existing data after upgrade (the migration
  note).

## Test plan — the wave-1 settlement pattern

Real servers via `tests/common`; probability-100 rules make firing
deterministic; `delay_ms` in the *rule* exercises the delay line over a
real socket while the *test* sleeps never; exact counts over the count
endpoints prove what landed; bounded `tokio::time::timeout` only where a
close must be observed. No injected clocks (ruled — spec). Every new or
surviving branch is executed by a named test (AC18); the gate run is the
arbiter, and a measured arm means the arm is dead or under-tested — the
fix is a test.

## Risks

| Risk | Mitigation |
|---|---|
| **A client relied on garbage = wipe** (the shim DELETE flip). | That "contract" was the bug: one typo destroying parallel runs, and the #6 false-pass shape. The safe failure direction is documented (README, openapi, llms.txt, ADR 0011); `delete-all` remains for the legitimate bulk wipe, and the ADR names the migration for wipe-on-garbage clients. |
| **`{"ids": []}` → 400 breaks an empty-batch client.** | Accepted cost of the ruling (spec): the client must skip the call when its set is empty. One wire rule is easier to document, test and defend than a 200-no-op success shape on a destructive endpoint. |
| **Thread keys change once for pre-existing data** and whenever the oldest mail leaves the window / is pruned. | The migration note (README + module doc) states it up front; the failure class is the same as today's scheme (the representative's mail vanishing re-keys too); the persisted-anchor ADR is the pre-authorized follow-up if the realm ever needs more than append-stability. |
| **Same-millisecond arrivals tie-break on id** and can flip the anchor once. | Requires equal `received_ms` *and* adversarial ids; the issue's scenario has distinct arrival times. Documented as the residual; no guard branch is minted for it (each would be 100%-gated forever). |
| **Someone relied on delay-only DATA = "slow but accepted".** | It was undocumented divergence that made a chaos knob that cannot fail and silently defeated retry suites; `ChaosEvent::Data`'s doc already promised the new behavior. The per-stage tests are the receipt; retry suites now get a real 451 to retry against. |
| **The connect delay-only test could hang** if a regression makes the server neither write nor close. | The assert sits under a bounded `tokio::time::timeout`, converting a hang into a failure — the pattern the existing test already uses. |
| **Coverage gate: a new 400 arm measured uncovered**, or the flattened DATA arm loses an executing test. | Each arm is executed by a named rewritten test (AC5-AC8, AC16); the deleted branch shrinks the denominator, so the exact gate stays green by equality. `coverage(off)` remains forbidden. |
| **The 400's JSON body shape differs from upstream Mailpit's error shape.** | The status code is what the contract pins; the body matches the neighboring Mailpit-side search error style in the same file (`ApiError`). Cost of the reason string: one line. |
| **Review re-litigates the refused designs** (persisted anchors, inbox scope, injected clocks, delay cap). | The spec rules on each with grounds and names the losing side's cost; decision 0011 records the rejected alternatives so the next panel starts from the ruling, not the argument. |

## Quickstart

```bash
cargo test --lib threads::            # AC1/AC2/AC3
cargo test --test ui thread           # AC1/AC5 over real SMTP
cargo test --test compat shim_deletes # AC5-AC12
cargo test --test smtp_edges chaos    # AC14-AC17
bash scripts/coverage-gate.sh         # exact 100/100/100
```

By hand: seed two inboxes; `curl -X DELETE …/api/v1/messages -d 'not json'`
→ `400`, both inboxes still list; `curl -X DELETE …/api/v1/messages -d
'{"ids": ["<id>"]}'` → `{"removed": 1}`; put a delay-only data rule →
send mail → `451 4.3.0 chaos`, count unchanged; send a chain (root
`<z@x.io>`, reply `<a@x.io>`) → the thread key is the same before and
after.
