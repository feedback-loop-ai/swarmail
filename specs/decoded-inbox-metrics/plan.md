# Implementation plan: decoded inbox metrics

Implements [spec.md](spec.md); the operator accepted the exact contract in
[0065](../../docs/decisions/0065-decoded-inbox-metrics.md) on 2026-10-24.
Runtime code and behavioral proofs remain outstanding.
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
and no push. This design commit contains exactly four files; index and
historical-documentation changes belong to the later handoff.

## Project structure and files touched

| File | Planned responsibility |
|---|---|
| `src/api.rs` | Decode inside the existing helper's matching arm; correct its comment and extend its existing inline fixtures. |
| `tests/api.rs` | Two parked awaits, full parsed scrape, completion and malformed-UTF-8 wire compatibility proofs; reuse existing parser/text HTTP helpers. |
| `docs/decisions/0065-decoded-inbox-metrics.md` | Accepted scoped amendment; runtime behavior and proofs remain outstanding. |
| `specs/decoded-inbox-metrics/{spec,plan,tasks}.md` | Owning design and later evidence/dated clarifications. |
| `docs/decisions/README.md` | Index the accepted 0065 record with its matching date and scope. |
| `specs/http-plane-edges/{spec,plan}.md` | Dated accepted-0065 pointers scope supersession of raw-key preservation; preserve historical text. |
| `README.md` | Later metric-selector migration note and invalid-UTF-8 accounting limitation beside metrics documentation. |

No production changes to guard callers, registries, admission, store, SMTP,
MCP or renderer. The acceptance handoff adds only a dated marker to 0043 §6;
gates and bundle policy remain unchanged.

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
