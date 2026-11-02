# Plan — MailSlurper service-API shim

Spec: [spec.md](spec.md). Decision:
[0118](../../docs/decisions/0118-mailslurper-shim.md) (proposed).

## Technical context

- A single Rust crate on axum 0.8, tokio, serde and chrono (all already
  dependencies). No new crate, no new feature, and both lockfiles and
  `deny.toml` stay untouched.
- The store's global, newest-first read already exists:
  `Store::list_all` → `compat::scoped(store, None)`. The global wipe exists
  too: `Store::clear_all_async`, used by `compat::delete_all` with 0014's
  `503` mapping.
- Bounded page work exists: `compat::page_work`, `serialize_capped` and
  `MAX_COMPAT_PAGE_BYTES`. Single-mail renders go through
  `compat::render_by_id`.
- The Host/Origin guard is `api::origin_guard`, keyed by
  `AppState::http_origin_guard: Option<u16>`.
- Commit failure is injected in wire tests with `common::refusing_trigger`
  (real SQLite, see `tests/compat.rs`
  `a_blocked_delete_refuses_the_shim_deletes_with_503`).

## Constitution check

- **0001 accept == stored.** The shim only reads committed inboxes, and
  AC13 reads immediately after each `250`. Pass.
- **0002 await subscribes first.** The wipe goes through `clear_all_async`,
  so emptied inboxes are released to watchers as with `delete-all`, and no
  new path touches the store. Pass.
- **0003 oldest-only.** Age codes are refused by name, and the cap is
  untouched. Pass.
- **0005 ingest extraction.** The projection reads stored fields only, and
  there is no `mail_parser` call in the new code. Pass.
- **0011.** Amended narrowly by proposed 0118 (a second spelling of the one
  wipe, on an opt-in listener). The PR is held for acceptance (0045 §8).
- **0020.** Unit tests stay inline in `src/compat.rs`. Wire shapes go in
  `tests/mailslurper.rs`. No new cold path, so no `_tests.rs` harness is
  required.
- **Coverage.** Every new arm is executed by a unit or wire test, listed
  in tasks.md. No `coverage(off)`.
- **Held paths.** None touched apart from the decision itself, which stays
  `proposed`.

## Project structure (files touched)

| File | Change |
|---|---|
| `src/config.rs` | `ServeArgs::mailslurper_listen: Option<String>` (`--mailslurper-listen`, env `SWARMAIL_MAILSLURPER_LISTEN`, value name `ADDR`, a doc comment stating off-by-default and the global mailbox); `Config::mailslurper_listen: Option<String>`; `From<&ServeArgs>` |
| `src/lib.rs` | After the POP3 bind, bind the listener when the address is set, then read its `local_addr`. Spawn its serve task only when `Some`, with an `AppState` clone whose `http_origin_guard` is the MailSlurper port when the bind is loopback. Add `RunningServer::mailslurper_addr: Option<SocketAddr>`. Update the inline test's `Config` literal. |
| `src/api.rs` | Extract `serve`'s graceful-plus-deadline race into `pub(crate) async fn serve_app(listener, app: Router, stop_rx, shutdown)`; `serve` calls it with `router(state)`. Make `origin_guard` `pub(crate)`. No behaviour change on the HTTP plane. |
| `src/compat.rs` | A MailSlurper section (detail below) with inline unit tests. The module doc names the third shim. |
| `Config` literals: `tests/common/mod.rs`, `tests/listen.rs`, `tests/persistence.rs`, `tests/stdio.rs`, `tests/starttls.rs`, `benches/read_stall.rs`, `src/compat/page_work_tests.rs`, `src/deliverability/concurrency_tests.rs`, `src/store/persist_tests.rs`, `src/webhook/ca_construction_tests.rs`, `src/lifecycle_tests.rs` | Add `mailslurper_listen: None`. `common` gains a helper that starts a server with the listener on `127.0.0.1:0`. |
| `tests/mailslurper.rs` (new) | The wire suite for AC1–AC11 and AC13–AC14, over a raw HTTP/1.1 client in the style of `tests/compat.rs`'s `http_raw`, so request headers are sent exactly as recorded |
| `tests/config_cli.rs`, `tests/binary.rs` | AC3: the flag and env parse, the binary binds the address, and an unbindable address fails startup |
| `docs/guide/compat-shims.md` | The title and intro cover three shims. A MailSlurper section (0118 §§1–6): the route table, the global-view sentence in bold, the ingress-prefix note, local, Docker and cluster recipes (chart `config` env plus a deployer Service on 4437, with the NetworkPolicy caveat), the omissions, the known differences (including "not counted in `/metrics`") and the *recalled* list. |
| `docs/guide/quickstart.md`, `docs/guide/network-posture.md` | Name `SWARMAIL_MAILSLURPER_LISTEN` (no default, off) wherever the listeners and env knobs are listed |
| `docs/decisions/README.md` | The 0118 row, `proposed` |
| docs-contract tests (`tests/store_contract_docs.rs` or `tests/compat.rs`) | Pin the MailSlurper section (AC15) |

Not touched: `Cargo.toml`, `Cargo.lock`, `evals/`, `deny.toml`, `charts/`,
`Dockerfile`, `.github/`, `scripts/`, `.brokkr/`, and `openapi.json` and
`llms.txt` (both describe the HTTP plane, which does not change).

## The `src/compat.rs` section

- `pub fn mailslurper_router(state: AppState) -> Router`: the routes
  `/mail` (GET list, DELETE wipe), `/mailcount`, `/mail/{id}` and
  `/mail/{id}/message`, layered with
  `from_fn_with_state(state.clone(), api::origin_guard)`, then
  `.with_state(state)`. There is no `count_in_flight` layer: `/metrics`
  reports the HTTP plane.
- `const SLURPER_PAGE_SIZE: usize = 50;` (25 before the review return; see
  § Review return)
- A closed list query: `pageNumber` only (`deny_unknown_fields`, read as a
  string and parsed by hand, so that `0`, `-1` and `x` share one named
  refusal).
- `struct SlurperItem<'a>(&'a Email)`, with a hand-written `Serialize` in
  0118 §4's key order.
- The page envelope: `mailItems` (skipped when empty), `totalPages`,
  `totalRecords`. `fn slurper_page_body(page, total, budget)` uses
  `serialize_capped`.
- `fn slurper_date(email)` uses `DateTime::from_timestamp_millis` and
  `%Y-%m-%d %H:%M:%S`, falling back to `received_at` verbatim on `None`.
- `fn slurper_body(email)` returns the HTML, else the text, else `""`.
- `PruneBody` holds the `pruneCode` value, with `deny_unknown_fields`.
  `fn prune_verdict(code: &Value) -> Result<(), ApiError>`.

## Approach details

1. **List.** Stage the query first, so a refusal costs no scan. Then
   `page_work`: `scoped(store, None)`, `total = all.len()`, and
   `start = (page - 1) * SLURPER_PAGE_SIZE` with `checked_mul` (an overflow is past the
   end). `page_of` cuts the page, and `slurper_page_body` serializes it
   under `MAX_COMPAT_PAGE_BYTES`. Bytes go to `json_body`. Over budget is
   a `400`:
   `mail page exceeds byte budget {MAX}; fetch each mail by GET /mail/{id}`.
2. **Empty-page omission.** Pin the exact bytes
   `{"totalPages":0,"totalRecords":0}`.
3. **Count.** `mailbox_size(store, None)` inline (a sum of lengths with no
   render), answered as `{"mailCount": n}`. The empty query goes through
   `parse_query::<ViewQuery>` on `RawQuery`, as `delete_all` does.
4. **One item, its message.** Both go through `render_by_id`. The item is
   `json_body(serde_json::to_vec(&SlurperItem(e)))`. The message is
   `text/html; charset=utf-8` with `slurper_body(e)`. Both have closed,
   empty queries.
5. **Delete.** The checks run in this order:
   - the query (`ViewQuery`): a `400` naming the key;
   - the body (`Result<Json<PruneBody>, JsonRejection>`): a rejection is a
     `400` reading `deletion requires the JSON body {"pruneCode":"all"}`;
   - `prune_verdict`;
   - `clear_all_async`, which maps `Ok(n)` to
     `200 text/plain; charset=utf-8` with `n`, and `Err` to
     `503 storage commit failed`.

   The verdicts:
   - `"all"`: proceed.
   - `"60plus" | "30plus" | "2wksplus"`:
     `prune code "<c>" is not supported: swarmail evicts only the oldest mail past a per-inbox cap (decision 0003); send "all"`.
   - any other string: `unknown prune code "<c>"; send "all"`.
   - a non-string or `null`: `pruneCode must be the string "all"`.
6. **Projection.** `toAddresses` is `email.recipients` in order when that
   list is non-empty, otherwise the `email.to` addresses. `ccAddresses` is
   the `email.cc` addresses. `fromAddress` is
   `compat_sender(email).unwrap_or("")`: reuse it, never re-derive it.
7. **Startup wiring.** The bind sits with the other binds, before any
   `tasks.spawn`. The serve task uses the same `shutdown_rx` and drain
   deadline through `api::serve_app`, so `RunningServer::stop` joins it.

## Research notes

- **Verified, binding:** the consumer's two calls and headers, the fields
  `mailItems`, `totalRecords`, `subject`, `body` and `toAddresses`, and the
  omission of `mailItems` when empty (the operator's description).
- **Recalled:** everything listed in 0118 §7 as still recalled. No design
  seat has web access, so no claim is presented as read from upstream
  source by this seat.
- **Reported from upstream source by the review panel (2026-11-02):** the
  page size of 50 and the other items 0118 §7 lists under that heading. They
  are cited, with file paths and a libmailslurper commit, by the guarantees
  member. The independent member recalled 50 independently.

## Data model

Unchanged: no new stored field, no schema change, no persistence change.

## Contracts

0118 §§3–5 and spec R3–R10. There is no OpenAPI entry: the listener is a
separate plane, and its contract is the guide's MailSlurper section.

## Quickstart

```bash
swarmail serve --mailslurper-listen 127.0.0.1:4437
MAIL_SLURPER_ENDPOINT=http://localhost:4437/mail npm test     # the consumer, unchanged
# container: -e SWARMAIL_MAILSLURPER_LISTEN=0.0.0.0:4437 -p 4437:4437
```

## Risks

| Risk | Mitigation |
|---|---|
| **Recalled upstream shapes are wrong** (as the page size was: 25 against upstream's 50) | The consumer-read fields are pinned from the operator's description. The page size now follows the review's source citation, and 0118 §7 makes it, like everything else that decides `mailItems`' contents, acceptance-blocking. What is still recalled is labelled in 0118 §7 and the guide. AC8 asserts the literal 50 once, so a drift is a red test and not a silent window. |
| **Page 1 misses the mail the suite just triggered** (more than 50 mails between the consumer's cleanups) | Upstream has the same limit. The receive-time order puts the newest delivery first whatever the sender's `Date` header says. The guide says that the consumer reads one page of 50. |
| **A list page over 64 MiB is a `400` the consumer cannot handle** | Refusing whole is 0044's rule (never truncate). Fifty items under 64 MiB is the normal case. The guide states the cost. |
| **Concurrent list pages each hold up to 64 MiB on the blocking pool** | Accepted for a single-tenant test sink (one Swarmail per suite, 0118 §2). A per-listener concurrency limit is a follow-up if the listener ever fronts a shared deployment. |
| **A future refactor gives the listener its own `axum::serve` and loses the shared timeouts and drain** | The listener goes through `api::serve_app` (R1). The drain leg of the existing lifecycle tests and review guard it. No extra test: a structural pin would be a test of code layout, not behaviour. |
| **`DRAIN_RECEIPT` is consumed by whichever listener drains first** (correctness, info) | No current test enables both listeners with a drain receipt. A future in-crate drain test that does must take the receipt per listener. No seam is added now for a test that does not exist. |
| **A global wipe on a shared Swarmail destroys parallel sessions' mail** | The listener is off by default, so only an operator enables it. Only the literal `{"pruneCode":"all"}` wipes. Every AC10 refusal carries a before/after count. The guide states the global mailbox in bold. |
| **DNS rebinding reaches the wipe on a loopback bind** | The same `origin_guard`, keyed to the MailSlurper port (AC4) |
| **The cluster cannot reach 4437 through the chart** | The guide gives the deployer-Service recipe and the NetworkPolicy caveat. Chart support is a follow-up named in 0118. The nightly needs only the env and one Service. |
| **A large page stalls the async worker or exhausts memory** | The build and serialization run on the blocking pool, under a byte cap that refuses the whole page by name (AC12) |
| **A refused filter breaks a client that sends one** | A named `400` beats a silent wrong match. Alkemio sends none, and the guide lists the refused keys. |
| **Every `Config` literal must change** | The compiler finds every literal, and the plan lists the known ones. `clippy --all-targets` catches the benches. |
| **Coverage regresses on the optional-listener arms** | Every existing test runs the `None` arm, and `tests/mailslurper.rs` runs `Some`. The date fallback has an `i64::MIN` unit test. Each prune-verdict arm has a unit test as well as its wire refusal. |
| **The PR cannot merge before 0118 is accepted** | Stated in 0118's consequences and §9. The mechanical hold list of 0056's addendum does not catch this case, so the ship notes below carry it to the merger. Held for the operator: expected, not a failure. |
| **Content-type strictness refuses a client that omits `application/json`** | Alkemio's client sends JSON with the JSON content type. Refusing is 0011's rule for a destructive route, and the guide documents it. |

## Review return (2026-11-02)

Run `issue-314-a-mailslurper-compatib-0acb1d84`. Review of head `439eba5`
(tree `f20a7b3`) returned `residual` with `spec_defect: true`. The frozen
guarantees were found intact by every member, and there was no security
residual. The design panel argued the return. Simplicity and robustness
agree on the spine and on the defect. Where they differed, the ruling is
in the spec's rulings table. Each finding and the change that answers it:

| Finding (member, severity) | Answer | Owner |
|---|---|---|
| Page size 25 against upstream's 50 (guarantees MEDIUM, spec defect; independent MEDIUM; security and correctness noted it) | Spec R5 and AC8 (51-mail fixture, the literal 50 asserted once, pages summing to the snapshot); 0118 §3 and §9; `SLURPER_PAGE_SIZE = 50`; the guide's paging line and fixed-paging difference | T15, T16 |
| The page size listed as non-blocking while it decides `mailItems` (independent) | 0118 §7: the page size, and everything that decides `mailItems`, is acceptance-blocking; the order difference is declared for the operator to rule on | this commit (0118) |
| 0118's Amends names only 0011, not 0004's "only cross-inbox destructive endpoint" (guarantees LOW) | 0118's Amends line names 0004 and narrows it explicitly; the decision index row says it amends 0004 and 0011 | this commit (0118), T17 |
| `llms.txt` still says `delete-all` "is the only cross-inbox wipe" (guarantees LOW) | Spec R11 and AC15: scope the claim to the HTTP plane and name the MailSlurper route | T17 |
| Guide title and intro, README and `repo-layout.md` name two shims (guarantees INFO) | Spec R11 and AC15: three shims everywhere (plan line 56 already promised this; the build missed it) | T17 |
| `dateSent` source and zone, and list order, not documented (guarantees INFO) | 0118 §§2 and 4 state them; AC15 lists them in the guide | T17 |
| Past-the-end `200` against a recalled `404` (independent INFO) | Kept, documented, listed as still recalled in 0118 §7 | T17 |
| Unknown id `404` against upstream `500`, `ccAddresses` extra, empty `mailItems` `[]` against the operator's observation (guarantees INFO) | 0118 §7 records them; AC15 lists them in the guide | T17 |
| The recalled shapes were never checked against source (correctness LOW) | 0118 §7 now separates source-reported items from recalled ones. The operator's check stays an acceptance condition, and no seat can do it. | this commit (0118) |
| F1: off-worker build at the listener's site proven only transitively (tests INFO) | AC17: the existing held-page-build harness test also holds `GET /mail` on the listener | T18 |
| F2: "not in `/metrics`" is prose only (tests INFO) | Prose scoped to the exact gauge name (AC15). A behavioural test is refused: the in-flight total is process-global, and every concurrent test in the binary moves it. No seam holds a request inside the listener's router, so an assertion would be vacuous at rest or flaky while held. | T17 |
| F3: the raw `req()` in `tests/mailslurper.rs` has no deadline (tests INFO) | AC18: named connect and read deadlines | T19 |
| Hold routing: 0056's mechanical hold list does not catch this PR (correctness and independent INFO) | 0118 §9, and the ship notes below | ship |
| `DRAIN_RECEIPT` taken in `serve_app` (correctness INFO) | Recorded in Risks; no change | — |
| Robustness: no test that a wipe persists across restart | AC14's second leg | T20 |

No acceptance criterion is weakened. AC8 is tightened: it moves to a
51-mail fixture and the page-sum invariant. Its old 26-mail fixture
would test the wrong boundary with a 50-item page. AC14 and AC15 gain
legs, and AC17 and AC18 are new. No new route, config field, dependency,
file outside the test and docs set, or decision number.

### Files the revision touches

| File | Change |
|---|---|
| `src/compat.rs` | `SLURPER_PAGE_SIZE = 50`; the `slurper_list` doc comment says 50; inline tests that hard-code 25 or 26 follow the constant |
| `src/compat/page_work_tests.rs` | `held_compat_page_builds_leave_smtp_and_health_serving` enables the listener and holds `GET /mail` on it as a fifth build (AC17). The build site records its observation through `render_site`, if the gate requires one. |
| `tests/mailslurper.rs` | AC8 rewritten for 51 mails; `req()` under named deadlines (AC18); AC14's wipe-restart leg; the docs-contract assertions of AC15 |
| `docs/guide/compat-shims.md` | Title and intro for three shims; paging 50; the known differences of AC15; the recalled list matches 0118 §7 |
| `src/static/llms.txt` | The scoped wipe sentence |
| `README.md`, `docs/guide/repo-layout.md` | MailSlurper named among the shims |
| `docs/decisions/README.md` | The 0118 row says it amends 0004 and 0011 |

## Ship notes

The PR body's first line must read: **HOLD: contradicts accepted 0004 and
0011 under proposed 0118; do not merge on green; the operator accepts 0118
(and checks its §7 items against upstream) first.** The 0056 addendum's
mechanical hold list does not name this case (0045 §8 and 0118 §9 do).
