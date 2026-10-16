# Spec — the cost of the workers and the posture of the wire (issues #49, #48)

- **Run:** run 10 (chief architect, synthesizing the simplicity and
  robustness positions) · **Feature slug:** `off-worker-parses-loopback-posture`
- **Issues:** [#49](https://github.com/feedback-loop-ai/swarmail/issues/49)
  (parsing and raw-byte copies off the async workers, result sizes
  bounded) — delivered first, the larger code surface — then
  [#48](https://github.com/feedback-loop-ai/swarmail/issues/48) (the
  network posture of the unauthenticated control plane), per the framing.
- **Decision:** [0030](../../docs/decisions/0030-loopback-plane-bounded-results.md)
  (proposed) — the posture ruling and the result-size bound defer to it
  for the *why*.
- **Base:** `66a45bb` (run 9 landed + the state-home repoint). Both
  positions were grounded at `4daa332`; every line they cite is re-verified
  against this tree and is unchanged by the two intervening commits
  (the migration moved no `src/` line).
- **Frozen guarantees:** 0001/0002/0003 untouched. `src/store.rs` ends at
  a one-word diff (the `offload` seam's visibility); `src/persist.rs` at
  zero diff; the mutation boundary's logic untouched.

## The problem

1. **The workers pay for a parse that no await owed**
   (`src/smtp.rs:1002` → `build_email` at `:1199-1289`). MIME parse,
   HTML↔text conversion, extraction regexes and a full `raw.to_vec()`
   run on the session's async worker *before* `insert_async`; a few
   concurrent large messages stall banners, `await` handlers and every
   other session's turn. The seed path runs the same builder
   (`build_seed_email` ends in `build_email`, `src/smtp.rs:1159-1199`).
2. **Three handlers deep-copy bytes serde provably skips.** `Email.raw`
   is `#[serde(skip)]` (`src/model.rs:52-53`) and the `rc` feature is on
   (`Cargo.toml:41`); `EmailList` already serializes `Vec<Arc<Email>>`
   byte-identically (`src/api.rs:446-456`, pin at `:1020-1043`). The
   remaining deep clones are exactly three: the await `Found` arm
   (`src/api.rs:692`), `get_message` (`:773`) and `seed` (`:903`) —
   `await?count=100` over 1 MiB mails copies ~100 MiB per request for
   nothing.
3. **`contains` allocates the mail twice per search**
   (`src/compat.rs:365-383`): `String::from_utf8_lossy(raw)` **and** a
   `.to_lowercase()` of it, per mail, per query — two full-size
   allocations per 50 MiB mail per request, on the unauthenticated read
   plane.
4. **Pages are unbounded on every plane.** `limit`/`count` above any cap
   are honored everywhere: REST `list`/`await` (`src/api.rs:638, 691`),
   the four compat pages (`src/compat.rs:92-93, 108-109, 212-213,
   241-242`), MCP `get_latest` count, search limit and wait count
   (`src/mcp.rs:235, 268, 307`). Upstream MailHog refuses above 250.
5. **The SSE wake rebuilds the view per subscriber** (`src/feed.rs:64-100`
   → `threads::view`, newest `VIEW_CAP = 200`, `src/threads.rs:38`) with
   no coalescing — bounded per wake, but multiplied by subscribers.
6. **The plane is wider than its README.** All three listeners default to
   `0.0.0.0` (`src/config.rs:33-42`) while README:80 says `swarmail
   serve` runs "the same ports on localhost"; the HTTP surface is
   unauthenticated (read every OTP, wipe inboxes, set chaos, register
   webhooks on arbitrary hosts — a blind SSRF whose `/metrics` ledger
   rows report whether something answered); and `/mcp` validates no
   `Origin`, so a DNS-rebinding page could drive it from a browser
   (the MCP Streamable HTTP transport's MUST).

## The positions, ruled

The panel wrote two contrarian positions (`.brokkr/state/design/
positions/{simplicity,robustness}.md`). Ruled here, not averaged:

**Adopted (both agree):** parse and seed-construction move to the
blocking pool — but never into the mutation boundary (robustness §1,
which simplicity's "one visibility word + two call-site wraps" implements);
the three deep clones become `Arc` serializations; page caps are
**refused, not clamped**, per 0018 §3/0025, at 250; `/mcp` — ruled as the
whole HTTP plane — validates `Origin`; webhooks are documented, not
restricted; no auth layer; redirects stay refused (`Policy::none`); the
bench is `writer_stall`-shaped (record verbatim, no wall-clock validity).

**Rejected, with the cost named:**

| Rejected | Whose | Cost of losing it |
|---|---|---|
| A `451` arm for a panicking parse (robustness §2) | robustness | A parser panic over hostile bytes remains a dead session — today's behavior. The arm is a cold branch over an unproven path (`mail-parser` is no-panic-on-input by design) needing a panicking-parser seam and a 0020-rule-2 harness: permanent machinery, no acceptance line. Re-openable as its own decision on first evidence. |
| SSE wake coalescing on a drain tick (robustness §5) | robustness | A many-subscriber, high-rate inbox pays ≤ 200 mails serialized per wake per subscriber. Correctness is never at stake (0002 re-applied per iteration; the store is the truth). The bench records the wake cost; the dirty-bit shape is named for whoever has evidence. |
| Per-plane bind split: HTTP loopback, SMTP `0.0.0.0` (robustness §1.5) | robustness | Co-tenant SMTP sharing needs one loud env. One rule — loopback, remote by explicit env — is three identical literals and one README sentence; the split costs a permanent asymmetry every future seat re-derives. |
| `Host`-header validation on the plane (robustness §1.5) | robustness | A rebinding page's `Host` is exactly what it believes; a static Host allowlist cannot serve explicit remote binds at all. The `Origin` check keyed to the bind posture closes the class with no new vocabulary. |
| A `validation.rs` module, a limit newtype, an operator knob for the cap | simplicity (vs. both positions' temptations) | 0025 §1 refused the newtype once; the shared check is one fn beside the constant. |

**Corrected (position vs. code, code wins):**

- Robustness's honest sentence "there is no per-target concurrency cap"
  is **stale**: 0029 §5 landed `WEBHOOK_DELIVERY_CAP = 32` per
  normalized-URL sink key (`src/webhook.rs:51, 314`). The remaining
  exposure is the blind dial + the coarse per-instance ledger, not an
  unbounded fan-out.
- Robustness's "slow-reader `Arc` tail" is **not new**: today's deep
  clone holds the identical liveness tail (the response owns a full copy
  until written). Simplicity's correction is right — the swap strictly
  removes a transient duplicate; the bench records it as a measured win,
  not a sizing warning.
- Simplicity's "`contains` loses its per-mail allocations" is adopted
  **with the fold divergence named**: `(?i)` regex is simple case
  folding, `str::to_lowercase` is the full mapping — ASCII-identical,
  exotic-Unicode margins differ, and the decision names it rather than
  pretending the swap is fold-invisible.

## The design

### Issue #49 — the workers, the copies, the caps

- **Parse and seed construction offload via the proven seam.** `Store::offload`
  (`src/store.rs:677-686`) gains `pub(crate)`; the DATA arm wraps its
  `build_email` call and the seed path wraps `build_seed_email` in it.
  0023's argument carries: the parse is an independent CPU leg, the reply
  is written after `insert_async` resolves regardless of which thread
  parsed, and 0001's ordering is untouched. The parse runs **before** the
  mutation boundary, never inside it — a hostile 50 MiB body cannot
  serialize every accept behind the boundary lock. The in-flight park
  (`park_delivery`) moves up one statement so
  `swarmail_deliveries_in_flight` stays live from the DATA terminator
  through the parse and the commit, exactly as its comment promises.
  Panic symmetry holds (0023 §5): a panicking parse unwinds into the
  awaiting session — no new arm, no `451` (ruled above).
- **Three `Arc` type-swaps.** `AwaitResponse.emails: Vec<Arc<Email>>`;
  `get_message` and `seed` serialize the stored `Arc` directly. The
  `EmailList` indistinguishability pin is the pattern; the wire is
  byte-identical and `raw` never appears. `/raw` (0022) and POP3 octets
  are untouched.
- **`MAX_PAGE_SIZE: usize = 250` with one shared check** beside
  `DEFAULT_LIST_LIMIT` (`src/api.rs:86-97` shape): a
  `checked_page_size(field, n)` returning the 0025 sentence shape
  (`{field} must be at most 250`), applied at the nine staging sites.
  Refusals ride each plane's existing envelope (REST/compat `400` JSON,
  MCP `-32602` staging refusal / the wait's `isError` family). The wait
  surfaces' `count` is capped on both planes; `assert`'s exact-count
  `count` and inert `limit` are exempt and pinned; `offset`/`start` are
  not page sizes.
- **`contains` folds by regex.** The needle is escaped into one
  `(?i)` regex per request; fields and lossy raw are matched without
  per-mail allocation. `raw` stays searchable (0022's byte-honesty, the
  "raw source" pin), links/codes stay non-candidates, every candidate is
  still collected (no short-circuit) so a miss looks like a hit.
- **One bench, three lenses:** `benches/read_stall.rs` (`harness = false`,
  `writer_stall`-shaped) — (a) a large-mail await: parse inline vs
  offloaded, per-mail worker occupancy + ticker overshoot; (b) a large
  compat search: deep-clone page (reconstructed inline) vs `Arc` page;
  (c) the `contains` fold: lowercase-alloc shape vs regex shape, plus the
  per-wake view cost recorded. Numbers verbatim; the operator rules.

### Issue #48 — the posture

- **Loopback default, explicit binds elsewhere** (decision 0030 §1):
  `src/config.rs:33-42` → `127.0.0.1:<port>` ×3; Dockerfile `ENV` keeps
  the container on `0.0.0.0`; chart `values.yaml` passes the same three
  binds through the generic `config:` mapping; README states the truth
  for both shapes and README:80 becomes true.
- **The Origin middleware** (decision 0030 §2): a present `Origin` on any
  HTTP route is `403` unless its authority is a loopback spelling of the
  bound HTTP port — enforced only when the listener is loopback-bound
  (the bind is the vocabulary; explicit remote binds are the operator's
  recorded posture). No `Origin` → pass. `AppState` carries the bound
  port; `serve`'s callers construct it. The UI is unaffected (same-origin
  loopback; no `/mcp` calls).
- **Webhook SSRF documented** (decision 0030 §5): the README names the
  blind-dial exposure beside the registration example — redirects already
  refused, the ledger coarse per instance, the fan-out already capped per
  target by 0029 §5.
- **No auth layer, no webhook allowlist, no redirect policy change** —
  refused in the decision with their costs.

## Acceptance Criteria

### AC1 — the parse leg leaves the workers

1. `Store::offload` is `pub(crate)`; the DATA arm and the seed path
   offload their `build_email`/`build_seed_email` work onto the blocking
   pool; `src/store.rs` shows no other diff and `src/persist.rs` none.
2. The parse is not fused into `Store::insert`: the mutation boundary's
   body is unchanged, and nothing parses while the boundary lock is held.
3. `park_delivery` is registered before the offloaded parse and released
   after the insert resolves: `swarmail_deliveries_in_flight` is live
   from the DATA terminator to the insert's resolution.
4. A cap-sized (50 MiB) SMTP DATA accept over the real wire still ends in
   `250 2.0.0 OK: stored as …` with the mail queryable immediately after
   (0001 re-proved on the moved path), and a concurrent `await` completes
   without a stall that the bench's lens (a) shows the old shape paying.

### AC2 — the wire carries no copied `raw`

1. `await?...`'s `Found` page, `GET /api/v1/inboxes/{inbox}/messages/{id}`
   and `POST .../seed` serialize the stored `Arc<Email>`; no response
   body contains a `raw` key; each is byte-identical to the whole-clone
   shape it replaced (the `EmailList` pin pattern, extended).
2. `/raw` and POP3 still serve the stored bytes (0022/0028 unchanged).

### AC3 — pages are refused above 250, on every plane

1. One shared check owns the sentence; a supplied `limit`/`count` above
   250 is refused, naming the field and the accepted range, on: REST
   `list` and `await`; Mailpit list/search; MailHog v2 list/search; MCP
   `get_latest`, `swarmail_search_emails`, and both wait surfaces' `count`.
2. Exactly 250 is accepted (the boundary value), on every plane; the
   defaults (100 / 50) are unchanged.
3. The wait surfaces' `count` cap applies on both planes; 0010's
   `count=0` semantics are unchanged; a refused wait registers no waiter
   (0025 §5).
4. `assert`'s `count` (exact, any magnitude) and `limit` (inert) are not
   capped — pinned by test.
5. `openapi.json` publishes `maximum: 250` beside every `limit`;
   `llms.txt` states the cap.

### AC4 — the search fold allocates nothing per mail

1. `contains` matches case-insensitively across text, html, subject, to,
   cc, from and raw without a per-mail `to_lowercase` or
   `from_utf8_lossy().to_string()` allocation; every candidate is still
   collected (a miss is reported as a hit in the envelope's counts).
2. Wire behavior is pinned: case-insensitive hits across all candidate
   fields, non-candidates (links, codes) never match, and the existing
   `contains` unit pins (`src/compat.rs:758-772`) stay green.

### AC5 — the loopback posture is the truth

1. The binary's three defaults are `127.0.0.1:1025/1110/8025`
   (`tests/config_cli.rs` pins updated); `SWARMAIL_*_LISTEN` still
   overrides each.
2. The image keeps its contract: the Dockerfile passes the three
   explicit `0.0.0.0` binds; the chart passes the same three through
   `config:`; `scripts/chart-verify.sh` stays green.
3. README:80 and the env-knob section state the real defaults for both
   shapes; a redirect is still refused (`Policy::none` untouched).

### AC6 — a present `Origin` cannot drive the plane from a rebinding page

1. With the HTTP listener loopback-bound (the default), a request
   carrying `Origin: http://attacker.com:8025` — or any non-loopback or
   wrong-port spelling — is answered `403` before the router, on every
   method; `/mcp` included (the MCP transport's MUST).
2. `Origin` absent → pass (bridge, agents, curl, test suites); a loopback
   `Origin` matching the bound port → pass (the UI's own fetches).
3. With an explicit off-loopback bind, the check is inert — the operator's
   recorded posture, documented in 0030 §2 and the README.
4. The UI never regresses: same-origin loopback requests pass the guard.

### AC7 — the bench records the before/after, honestly

1. `benches/read_stall.rs` (new, `harness = false`) records, in one
   process: large-mail await occupancy (parse inline vs offloaded), large
   compat search (deep-clone page vs `Arc` page), and the `contains`
   fold — plus the per-wake view cost. No wall-clock validity assertions
   (0020's discipline binds benches); numbers are recorded verbatim and
   ruled as measured (0023's discipline).

### AC8 — the gates and the boundaries

1. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
   warnings`, `cargo test --workspace --locked`, `brokkr compile --bundle
   bundles/self`, `scripts/coverage-gate.sh` (exact 100/100/100, missed ==
   0) and `cargo deny check licenses` are clean.
2. `src/persist.rs` is at zero diff; `src/store.rs` at a one-word diff;
   no new dependency; no `coverage(off)`; no new `src/` module.
3. Decision 0030 is committed with this spec, status `proposed`; 0018's
   `Amended by` marker is a landing task upon acceptance, not now.
