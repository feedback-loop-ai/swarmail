# Implementation plan: decoded inbox metrics

Implements [spec.md](spec.md); the operator accepted the exact contract in
[0065](../../docs/decisions/0065-decoded-inbox-metrics.md) on 2026-10-24.
At the design baseline, runtime code and behavioral proofs were outstanding;
the dated continuation receipt below records their later implementation.
The checkout has no `.specify/templates/plan-template.md`; the required
plan sections are supplied here directly.

## Technical context

One Rust 2024 crate using Tokio and Axum 0.8. `src/api.rs` owns
`inbox_of_path`, outer `count_in_flight`, RAII guards, static metric
registries and `park_await`. `Path<String>` supplies the decoded handler
identity. `percent-encoding = "2.3.2"` is already a direct dependency.
`tests/api.rs` has real-server fixtures and an independent whole-scrape
Prometheus parser. No package, lockfile, storage schema or config change.

## Constitution check

Preserve 0001/0002/0003/0005 and all delivery ordering. The only contract
amendment is 0043 §6's HTTP label identity; 0047's overflow amendment stays
in force. 0045 §8 bars conflicting implementation before acceptance.
Use real wire tests under 0004 and deterministic inline helper tests under
0020; cold polling diagnostics belong in `tests/api.rs`, not measured inline
code. Do not move existing inline modules or add coverage exclusions.
Follow the authoritative house-rules gates, unsigned commits under 0056,
and no push. Design-phase scope: that design commit contained exactly four
files; index and historical-documentation changes belonged to the later
handoff.

## Project structure and files touched

| File | Planned responsibility |
|---|---|
| `src/api.rs` | Decode inside the existing helper's matching arm; correct its comment and extend its existing inline fixtures. |
| `tests/api.rs` | Two parked awaits, full parsed scrape, completion and malformed-UTF-8 wire compatibility proofs; reuse existing parser/text HTTP helpers. |
| `docs/decisions/0065-decoded-inbox-metrics.md` | Accepted scoped amendment; current runtime status is recorded in the continuation receipt below. |
| `specs/decoded-inbox-metrics/{spec,plan,tasks}.md` | Owning design and later evidence/dated clarifications. |
| `docs/decisions/README.md` | Index the accepted 0065 record with its matching date and scope. |
| `specs/http-plane-edges/{spec,plan}.md` | Dated accepted-0065 pointers scope supersession of raw-key preservation; preserve historical text. |
| `README.md` | Later metric-selector migration note and invalid-UTF-8 accounting limitation beside metrics documentation. |

At the acceptance handoff, no production changes to guard callers, registries,
admission, store, SMTP, MCP or renderer were in scope. That handoff added only
a dated marker to 0043 §6; gates and bundle policy remain unchanged.

## Research and resolved choices

Both panel files and the run intake agree on the mismatch and authority
conflict. Inspection confirms `count_in_flight` calls the helper before
`next.run`; `park_await` raises cumulative registration then active awaits.
The existing hostile-name regression tests decoded await labels but never
compares the HTTP label. `tests/common/mod.rs`'s whitespace-split metric
reader cannot safely read space-bearing labels; reuse `parse_prometheus`.

Live invalid-UTF-8 refusal has not been reproduced by the design panel.
Capture its actual status/body before implementation; do not guess a JSON
envelope or add a new refusal. Audit the API executable's number of distinct
registered names before choosing in-process fixtures. Dedicated names, each
used once, make the expected cumulative value exactly 1. If cap headroom
cannot be proved, isolate the server process while retaining the same wire
assertions and parser; do not relax AC2.

## Data model and implementation approach

No persistent model change. Keep `Option<String>` and current segment
selection. In the recognized arm call
`percent_encoding::percent_decode_str(inbox).decode_utf8()`: own the decoded
string on success, own the complete original segment on error. Leave the
unrecognized arm untouched. One key is selected before the existing guard
is acquired and remains the guard's key through release. This preserves
all scoped routes and refusal accounting without sharing locks or adding
validation. The decoding allocation/scan is linear in the selected segment;
no new input-size guarantee is asserted.

Extend the existing branch-stable inline helper test with the AC1 matrix.
Use literal independent expected strings. Preserve the existing path
selection cases, including empty components; add a slash-data fixture whose
suffix confirms decoding happens after selection.

## Contracts and proof design

Use two test-exclusive names such as `issue202 space` and `issue202-café`,
with encoded `%20` and `%C3%A9` segments. Confirm they are unused in that
executable and below the cap. Start both empty-inbox waits in a local
`JoinSet` or equivalent abort-on-drop owner. Bound the whole scenario and
HTTP operations; use a parking deadline shorter than their 60000 ms server
await timeout. Observe readiness from both decoded active-await rows, then
assert all six named samples and absence of encoded aliases in that same
fully parsed scrape. Do not wait for the HTTP assertion to become true:
the old helper must fail once both awaits are observably parked.

Separate families are not atomically sampled. Keeping both inboxes empty
and waits outstanding establishes the stable interval; do not add registry
locking to manufacture a snapshot. Registration may precede watcher
subscription; 0002's subscribe-before-scan already makes subsequent seeding
safe. Do not move registration or watcher setup for test convenience.

Seed, join successful replies, then observe both live rows disappear with
cumulative rows retained. Keep the `RunningServer` owner in the test scope; bound `stop()` on success
and retain abort-on-drop client ownership on failure/timeout. Its trackers
are owned, but detached Axum connections may outlive them; the per-test Tokio
runtime teardown owns remaining server futures/sockets. Do not claim that
`stop()` joins every connection.
For invalid UTF-8 use a distinct raw segment containing a valid escape and
`%FF`; pin the pre-change text response and assert no await rows for either
raw or would-be lossy alias. Helper fixtures prove the transient raw-label
fallback. Preserve existing lifecycle tests rather than claiming a client
abort necessarily cancels its server handler immediately.

Run the new regression against unchanged production first, or temporarily
restore only the old helper after implementation. Record the failed metric
assertion, not a timeout/compile error. Restore the final implementation and
record the passing assertion before gates.

## Quickstart and validation

With operator acceptance of 0065 recorded, run `cargo test --locked --test api` and
`cargo test --locked --lib api` for wire/helper/lifecycle evidence. Execute
every gate in [house rules](../../docs/house-rules.md#gates--part-of-every-change),
using its pinned tools and fail-closed native-tls proof, offline with no
installs. Keep 100% lines/branches/functions exact. CI-only proofs remain
explicitly CI-owned; unavailable gates must be reported, never called green.

The design draft was committed as `29f8737`, and its deterministic check
passed before this implementation phase. This phase records the supplied
acceptance and scoped historical pointers; it does not complete runtime tasks.
Run the full house-rule gates after these documentation edits and preserve the
original design validation receipt below.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Accepted contract silently overridden. | Accepted 0065 and dated scope pointers constrain runtime work; the runtime implementation and proof remain separate. |
| Existing consumers poll encoded labels. | Document selector migration and intentional coalescing; no duplicate compatibility rows. |
| Decode changes path structure or inbox identity. | Segment selection first, strict UTF-8, one pass; slash/percent/plus/padding/Unicode fixtures. |
| Malformed bytes become a valid lossy name. | Complete raw fallback, unchanged extractor refusal and no-registration wire proof. Accept the documented literal-percent collision. |
| Parallel tests share static registries or exhaust cap. | Exclusive names, headroom audit, no resets/global deltas; isolated process only if needed. |
| Scrapes race completion or deadlines leak work. | Empty inboxes during assertion, bounded operations, owned clients/server and joined successful completion. |
| Overengineering expands coverage obligations. | One existing helper and test file; no new abstraction/dependency; every new production branch measured. |
| Wider delivery hazards are mistaken for repaired behavior. | Preserve existing storage/webhook/chaos/TLS guarantees and proofs; no new durability or cancellation claims. |

## Design validation receipt — 2026-10-24

The chief ran the house-rule commands with `CARGO_NET_OFFLINE=true`.
Formatting, Clippy, coverage selftest, publish-guard proof, seal-branch
proof, license check and fail-closed native-tls absence passed.
`cargo test --workspace --locked` failed two `store_contract_docs` tests:
`every_decision_file_has_exactly_one_resolving_index_row` and
`the_inventory_refuses_missing_dangling_duplicate_and_malformed_entries`,
both because `0062-decoded-inbox-metrics.md` has no index row. The coverage
run reached the same failures and refused; no exact-coverage pass is claimed.
This is a known consequence of the explicit four-artifact design-commit
scope, not a pre-existing test failure. T1 must add the index row and rerun
these tests; T8 must rerun all delivery gates. The initial unqualified
`brokkr compile` attempt failed because brokkr was absent from PATH.
The retry with `.forge/bin/brokkr compile --bundle bundles/self` passed
(policy sweep: 0 unruled). Its SHA-256 matches the recorded pinned digest
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Run-local logs are under `.brokkr/state/design/chief-checks/`.


## Acceptance-recording implementation receipt — 2026-10-24

The task-supplied operator chat ruling (“accept that”) is recorded as accepted
0065 and indexed with the same status/date. The former 0062 numbering was
corrected while preserving reservations #180=0062, #181=0063 and #174=0064.
The five ruling clauses and the original design validation receipt above were
compared byte-for-byte and are unchanged. Relative links in the eight changed
documents resolve; 0043 §6 and both HTTP-plane historical bodies remain intact
apart from their dated scope markers. No runtime or test source changed.

Targeted proof: `CARGO_NET_OFFLINE=true cargo test --locked --test
store_contract_docs` passed (19 passed, 0 failed), including the decision-file
inventory, matching status and missing/dangling/duplicate entry controls.

The complete local house-rule inventory passed on this documentation state:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- `.forge/bin/brokkr compile --bundle bundles/self` (pinned binary SHA-256
  `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`)
- `bash scripts/coverage-gate.sh`: exact 100% — 12,953/12,953 lines,
  762/762 branches and 1,437/1,437 functions
- `bash scripts/coverage-gate.sh selftest`
- `bash scripts/publish-guard-test.sh` (51 mutation-red/restored-green receipts)
- `bash scripts/seal-branch-test.sh` (7 cases, 10 mutants)
- `cargo deny check licenses`
- fail-closed `cargo tree -i native-tls` absence proof

The CI-owned chart/history, Docker, network advisory, tag release-rate and
real port-80 bridge proofs were not run here; house rules assign them to CI.
This receipt records documentation acceptance only. Runtime behavior, API
regression work and independent review remain for the subsequent engine phases.

## Recovery proof design — refused control request metrics (2026-10-24; state at `62e6496`)

This section records the documentation-only recovery state. The continuation
implementation and current proof status are recorded in the receipt below.

The security residual concerns the future decoded request key before handler
admission. A selected `%0D%00` segment with an exclusive safe prefix decodes
strictly to CR/NUL for request accounting, then the existing handler refuses
it. This is a transient request row under the accepted escaping rules, not an
await row, admission receipt or demonstrated injection. Do not alter escaping,
refusal, accounting order or accepted identity to hide it.

Before changing `inbox_of_path`, issue the real HTTP request and capture the
current refusal's exact status and raw/text body. Pin the observed result; the
current contract is status 400 and body `inbox must not contain control
characters`. Do not infer a JSON envelope. The future changed-path test must
hold execution after request guards are acquired and until a complete scrape
has been read. If no public mechanism can provide that deterministic interval,
resolve a test-only seam at the middleware owner under decision 0020 and keep
the real router and HTTP request in the proof. Branch-stable tests remain
inline; cold failure diagnostics belong in the owner's `*_tests.rs` harness.
No seam or source change is part of this recovery.

Use the existing independent `tests/api.rs` parser for the full scrape and
add an independent valid CR/NUL quoted-label fixture plus malformed-input
controls. During the hold, require exactly one request sample of value 1 for
the exact decoded CR/NUL label, no encoded alias and no additional injected
sample. Confirm no active-await row or cumulative registration increment for
the identity before, during or after the refusal. Audit exclusive names and
headroom for the whole API test executable; use process isolation if named
headroom cannot be proved, and never substitute the overflow row for the named
sample. Release the hold, join the same refusal, then parse a later full scrape
to prove the request row disappears while await registration remains absent.
Bound readiness, requests, scrape, release, join and cleanup; use neither sleep
as synchronization nor client disconnect/unread-response tricks, and clean
up clients/server after failures. Keep AC4's invalid-UTF-8 fallback/refusal
proof separate because it exercises another contract path. All of these remain
future obligations; no live refusal-window scrape is claimed here.


## Continuation implementation receipt — 2026-10-24

This receipt updates the recovery-state statements above for the continuation
from `62e6496`. It preserves the recorded design, accepted ruling, earlier
recovery receipts and historical decision text. Direct review confirms 0065
still amends only 0043 §6's inbox-scoped HTTP request-label identity; the
finite `store_contract_docs` pointer inventory still does not prove that
semantic pointer, which remains a disclosed manual-review limitation.
Reservations #180=0062, #181=0063, #174=0064 and #202=0065 remain unchanged.
No decision, index, gate, policy, dependency, storage, SMTP, MCP or extraction
contract was changed.

The pre-change real-HTTP refusal capture pinned these exact body bytes:

- selected `issue202-control-%0D%00`: HTTP 400,
  `{"error":"inbox must not contain control characters"}`
- selected `issue202-invalid-%FF`: HTTP 400,
  "Invalid URL: Invalid UTF-8 in `inbox`"
- selected `issue202-mixed-%20%FF`: HTTP 400,
  "Invalid URL: Invalid UTF-8 in `inbox`"

The refusal-byte assertions pass with the final helper. The invalid and mixed
cases preserve the raw and would-be lossy identities without any active-await
or cumulative-registration row. The strict helper matrix covers selected
segment routing, empty components, ASCII, space, Unicode, padding, literal
plus/percent, malformed escapes, encoded slash, double encoding, `%FF`, and
whole-segment fallback after mixed valid escapes and invalid UTF-8.

The API integration executable was audited for process-global registration
headroom: it has 28 literal `/await` path spellings; bounded route fixtures
expand to fewer than 64 possible successful inbox identities, including the
three direct-router names and the two new exclusive names `issue202 space` and
`issue202-café`. Malformed-query, blank/control, and invalid-UTF-8 refusals do
not reach `park_await`. In the separate library-test executable, the only
admitted wait fixture is the 16-name MCP batch test; other MCP wait calls
reject before parking and the new refused-control harness registers none.
Thus the new API fixtures use two of the 4096 cumulative-name slots, the
library batch proof uses at most 16, and neither executable approaches the
cap. No process isolation, registry reset or suite serialization was needed.

The two-await regression first failed on the original helper after both
active-await rows appeared: the same full scrape contained only encoded HTTP
request labels, so the decoded request-row assertion failed directly (not by
timeout or build error). After exactly-once strict segment decoding, one
stable complete scrape had all six value-1 rows under the two decoded names,
with no encoded aliases. Encoded seeds released both joined waits with
`matched == 1`; the next scrape had no request or active-await rows and kept
both cumulative rows at 1.

The refused-control proof uses the real production router on an ephemeral
TCP listener. A request-owned one-shot in the outer `cfg(test)` layer is
consumed by `count_in_flight` and holds only the exact encoded request after
the real handler has created its 400 response. The independent strict parser
accepted the exact decoded CR/NUL label during the hold, found one value-1
request sample, no raw alias or additional control/injected request sample,
and no active or cumulative await registration before, during or after. The
unchanged refusal bytes were joined after release, and the later scrape showed
the request row removed. Scrapes passed while the hold was active. Separate
bounded tests prove observer cancellation, dropped release and cancelled
owner all free the hold. The old-helper experiment failed on the explicit
wrong-label assertion; restored code passed. The parser's independent fixtures
accept literal CR/NUL and continue to reject illegal escapes, raw LF and
malformed syntax. No injection was demonstrated, and the residual awaits the
protected security review rather than being declared cleared here.

Targeted regressions passed:

- `CARGO_NET_OFFLINE=true cargo test --locked --test api` — 66 passed
- `CARGO_NET_OFFLINE=true cargo test --locked --lib api` — 33 passed
- `CARGO_NET_OFFLINE=true cargo test --locked --test store_contract_docs` — 19 passed
- `CARGO_NET_OFFLINE=true cargo test --locked --lib api::refusal_tests` — 2 passed

The local house-rule inventory passed on this implementation:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked`
- `.forge/bin/brokkr compile --bundle bundles/self` — pinned binary SHA-256
  `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`
- `bash scripts/coverage-gate.sh` — exact 100%: 12,981/12,981 lines,
  764/764 branches and 1,437/1,437 functions
- `bash scripts/coverage-gate.sh selftest`
- `bash scripts/publish-guard-test.sh` — 51 mutation-red/restored-green receipts
- `bash scripts/seal-branch-test.sh` — 7 cases and 10 mutants
- `cargo deny check licenses`
- `sha256sum -c bundles/self/scripts/shims.sha256` — all seven fixed shims match
- fail-closed native-TLS absence proof — Cargo reported exactly
  “package ID specification ‘native-tls’ did not match any packages”

CI retains the chart/history and selftests, Docker lifecycle/context smoke,
network RustSec audit, tag-only release-rate proof and real port-80 bridge.
Those CI-owned proofs are not claimed locally. The engine's protected
independent review and security disposition remain pending on this final
candidate.
