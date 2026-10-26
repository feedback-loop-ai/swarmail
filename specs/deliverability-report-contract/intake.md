# Intake — epic #251 issue #252: deliverability report contract

Run: `epic-251-issue-252-define-the-de-e2e822d8`; phase: intake; 2026-10-26.

## Goal

Define and implement a deterministic, versioned per-message deliverability report, computed lazily from stored bytes and cached without stale reuse, exposed through REST, MCP and the web UI and usable by existing assert/wait surfaces, with only trivial demonstration rules.

## Authority and checked evidence

The supplied issue packet's README.md and operator-rulings.md both passed
`sha256sum -c SHA256SUMS` from their directory. The README has an empty prior
runs section. The operator's #251 ruling of 2026-10-26 governs: analysis is
lazy and cached, outside SMTP; later authentication work uses an existing
permissive crate with a justified allowlist widening; spam rules are built-in
and deterministic, with no external scorer. This issue implements the report
contract and plumbing only. #253 owns DNS policy; authentication, content and
spam rules belong to later stories. No dependency or allowlist widening is
justified by this contract-only slice.

Read the house rules and decision index, founding decisions 0001–0003/0005,
0010's assert/wait semantics, 0029's validation contract, 0030's worker and
result bounds, and 0031's distinction between stored extraction and derived
views. Existing accepted guarantees remain requirements. Record decision
0085 as **proposed**, with an index entry, during design/implementation;
0085 is absent in this candidate and is assigned explicitly by the task.
Do not accept a decision or amend accepted extraction semantics by stealth.
The lazy report is a new analysis product; it must not move or overwrite
stored links, codes, bodies or other ingest extraction.

The worktree began clean at `ebc7848`. Local history for the affected owners
includes `f49d122` (test-helper cleanup), `d2f7148` (bounded CA execution),
`ca76e24` (CA work off workers) and `3ed2fbb` (closed seed admission).
Spot-checks: `Email.raw` in `src/model.rs` retains bytes and skips JSON;
`Store::insert` in `src/store.rs` is synchronous;
`Store::wait_for` subscribes before its first scan and again after a closed
watcher; `src/api.rs` owns `get_message`, `assert_messages` and
`await_messages`; `src/mcp.rs` owns shared tool dispatch/result shaping;
`src/ui.rs::message_view` owns the message page. Existing tests live in
`tests/api.rs`, `tests/mcp.rs`, `tests/stdio.rs`, `tests/ui.rs`,
`tests/burst.rs`, `tests/p2.rs`, `tests/rate.rs` and owner-local modules.
The framing leaves module layout, exact endpoint/tool/parameter names and
cache ownership to design, with their proofs required below.

### Explicit local dependency: newer MCP contract

This candidate's `src/mcp.rs::PROTOCOL_VERSION` is only `2025-03-26`.
Neither newer revision negotiation nor `structuredContent`/`outputSchema`
is present. Decision records 0079/0080 (#152) and 0082 (#153), and their
implementation/test artifacts, are absent from this workspace. The supplied
rulings say those records were accepted and are recorded when #152/#153 land;
that statement does not supply their exact contract or code. The tracked
rulings copy also predates these later packet additions.

This is sufficient to frame intake, but exact compatibility with #153 on
2025-06-18 and 2025-11-25 cannot be proven from this candidate. Design must
name this dependency and obtain the operator-provided local #152/#153
contract/artifacts (including accepted 0082) before asserting exact newer
revision compatibility. Do not read sibling worktrees, GitHub, CI or a
journal, fabricate accepted text, silently omit the newer-revision criterion,
or expand this story into a replacement for #152/#153. The 2025-03-26
plain-text surface can be designed from the current code. This missing input
is not an intake finding that the feature contradicts a frozen guarantee.

## Acceptance criteria for the implementation

1. A shared typed report/finding contract serializes a versioned rule-set id
   and findings with a stable rule id, severity exactly `info`, `warning` or
   `error`, an explicit header/MIME-part/link location, and a fix hint.
   Decision 0085 specifies location identity (including repeated headers and
   part/link addressing), ordering and the initial demonstration rule ids.
   Round-trip/shape tests cover every severity and location variant, an
   empty report and nonempty reports; fixtures pin the emitted ids and hints.
2. The demonstration rules produce identical ordered report content for the
   same stored bytes, effective configuration and rule-set version, including
   across independent computations and cold/warm cache reads. Tests change
   arrival metadata while retaining bytes and pin the analysis content;
   report envelope identifiers may identify the selected message. No result
   depends on wall clock, random order, live DNS, network or an external
   scorer. Only trivial rules needed to demonstrate the contract ship.
3. SMTP/seed ingestion and restore never invoke the report evaluator. A
   test-only counter or deterministic owner-local seam proves zero analysis
   before the first report/predicate request and reuse after a repeated
   request. After an SMTP DATA `250`, mail and its exact raw bytes are
   immediately queryable even when analysis has not run. Reading a report
   does not mutate raw bytes, stored extraction fields or persisted rows.
4. Cache identity covers every input affecting analysis: the stored byte
   identity, effective analysis configuration and rule-set version. Tests
   warm a cache, vary each input independently, and prove an old report is
   not served. Test concurrent requests with deterministic synchronization;
   deletion, clear, oldest pruning and restore/restart cannot expose a cached
   report as a live message that no longer exists or reuse it for different
   bytes. Decision 0085 defines cache ownership/lifetime, concurrent misses
   and failure behavior, and tests exercise that chosen behavior. Analysis
   must run outside store mutation locks and the SMTP acceptance path.
5. A real REST request retrieves the selected stored message's report with
   the documented schema; missing/deleted/pruned messages return the
   documented not-found response. Invalid supplied parameters refuse before
   analysis/effects, following native closed-input rules. OpenAPI documents
   the route, finding/report shapes and predicate parameters; tests compare
   the documentation and actual handler responses.
6. An MCP tool and/or resource exposes the same report content as REST.
   For 2025-03-26, tests verify the plain text content response and existing
   error conventions. For negotiated 2025-06-18 and 2025-11-25, tests verify
   the report follows #153's supplied structured-output contract, including
   schema/content agreement and error behavior. Retain both HTTP and stdio
   coverage and existing revision behavior. The dependency above must be
   resolved before this criterion is reported proven.
7. The message web page exposes the report's rule-set id, severities, rule
   ids, locations and fix hints, and distinguishes an empty report from
   nonempty findings. Real-server UI tests prove these states and show that
   message-derived values are escaped; a report request does not change the
   stored message or its existing presentation fields.
8. Existing REST assert/await and MCP wait surfaces can evaluate a shared
   deliverability predicate such as “no findings at or above warning.” Tests
   cover the inclusive severity boundaries, empty/nonempty findings, success,
   mismatch, timeout, shutdown and malformed predicate input. An empty inbox
   must not satisfy a wait for a qualifying message; existing minimum-one
   wait and exact-count assert (including zero) semantics remain pinned.
   A deterministic interleaving proves delivery between subscription and
   the first predicate scan is observed; watcher closure/re-subscription
   still subscribes before scanning. No sleep-based synchronization.
9. A documented rule-provider/input seam allows #253's DNS-policy work to
   contribute typed findings and identify all additional analysis inputs for
   cache freshness without changing the report wire model. A deterministic
   test provider/fixture proves another rule's findings use the same ordering,
   severity predicate and report serialization. No DNS-policy implementation,
   network resolution, authentication/content/spam rules or external scorer
   is included in #252. Future mutable external inputs must be represented
   in cache identity rather than relying only on message id.
10. Proposed decision 0085 and current user/API/MCP documentation specify
    the final contract, lazy execution, cache identity, demonstration-only
    scope, assert/wait meaning and #253 seam. Existing specs remain historical.
    Every new production line/branch/function is executed under the exact
    coverage gate. All local house gates and verifier additions below pass
    on the committed implementation; fresh logs and commit/tree identities
    substantiate the result. Earlier intake gates do not prove implementation.

## Frozen guarantees and boundaries

Nearest are **0001** (lazy reporting cannot become SMTP work or delay stored
visibility) and **0002** (predicate waits keep subscribe-before-check on every
scan path). **0003** is adjacent through message/cache lifetime: report access
must not change oldest-only pruning or resurrect evicted mail. Preserve
0005's ingest extraction and raw-byte truth. This is compatible additive
analysis under the operator ruling, not permission to weaken an accepted
ruling. A design that requires such an amendment must identify it and wait
for the operator; a proposed 0085 alone is not acceptance.

Use the recipe routing in the candidate's bundles unchanged. No engine
re-pin, push, merge, tag, publication, issue closure or decision acceptance.
CI owns macOS/platform runs, Helm/chart history and bump proofs, Docker
smoke, network-backed RustSec, privileged port-80 bridge exchange and tag-only
release-rate proof. Live GitHub settings, namespaces, sealing/merging and
publication remain CI/operator owned, not additional local acceptance claims.

## Verify gate on the committed implementation

Focused real-server suites and owner-local deterministic tests:

```bash
cargo test --locked --test api
cargo test --locked --test mcp
cargo test --locked --test stdio
cargo test --locked --test ui
cargo test --locked --test burst
cargo test --locked --test p2
cargo test --locked --test rate
cargo test --locked --lib
```

Design must assign concrete test names to criteria 1–9. Keep tests inline
unless decision 0020 requires a cold-path harness; public protocol tests use
real servers, not mocked SMTP/HTTP. No new throughput claim or benchmark is
required for this contract-only story; existing real-server rate/burst tests
prove the acceptance path's count behavior.

Every house gate, using the requested engine and canonical pinned copies
after fixed public-shim integrity checks (0050):

```bash
export CARGO_NET_OFFLINE=true
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf '%s\n' "$out" >&2; exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2; exit 1
fi
```

Require exact zero missed lines, branches and functions, plus the complete
coverage selftest; neither a rounded percentage nor a selftest-only run is
proof. Verify runs offline and installs nothing. Final verify uses its pinned
bundle resources, never an unchecked candidate shim.

## Intake outcome

Resolved means this framing is recorded with falsifiable criteria and gate
commands. This phase implements no feature and creates no decision document.
The explicit task-wide gate request also requires an intake baseline attempt;
its receipt will record actual outcomes separately. The missing newer MCP
contract is explicitly carried to design, not claimed implemented or proven.


## Intake baseline verification

All 14 commands in [baseline-gates.json](baseline-gates.json) exited zero on
committed intake baseline `cb5bde0b72acac279cbfec2f6096c849cd115629`
(tree `bf7ef9189df8508bda98f2069371090fe0f3b53c`). This includes every
house gate, engine/shim integrity, design-validator selftest, delivery-boundary
proof and fail-closed native-tls absence. Cargo ran offline, without installs;
compile reported 76 valuations over 14 groups with zero unruled.

Coverage was exact: **14,677/14,677 lines, 822/822 branches and 1,597/1,597
functions**; the complete coverage selftest passed. Evidence is under
`target/coverage/run.fLg9UQ` and `.forge/evidence/issue252-intake/`.
The receipt records command exits, durations, log hashes and commit/tree
identity; all 14 log hashes were checked. The workspace client's 300-second
wait timed out while the runner continued; the completed persisted receipt
and logs prove the outcomes, not the interrupted client response.

This final receipt/documentation commit changes no tested source, gate or
bundle bytes. No report implementation or newer MCP conformance is claimed.
Design still needs the explicitly missing local #152/#153 contract artifacts,
and verify must rerun every gate on the committed implementation candidate.
