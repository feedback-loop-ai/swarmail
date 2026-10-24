# Implementation plan: extraction and compatibility proof closure

Implements [spec.md](spec.md). Design handoff only; source inspection is not
execution evidence.

## Technical context

One Rust 2024 crate. Existing Tokio current-thread tests, Store::offload,
Axum responses, SQLite/rusqlite fixtures and real HTTP/SMTP helpers cover
the required boundaries. Python 3 is already the offline generator's
maintenance interpreter; introduce no Cargo integration or new dependency.

There is no .specify/templates/plan-template.md in this checkout. This plan
supplies the requested technical context, constitution check, project
structure, research, data model, contracts, quickstart and risks inline.
The chief commits exactly spec.md, plan.md and tasks.md in this directory.

## Constitution check

No frozen guarantee, accepted ruling or public behavior changes. Preserve
extraction at ingest, verbatim restore, awaited full response construction,
source-only search and ordered per-id commits. No new ADR is needed for
tests proving existing contracts. If source evidence exposes a production
defect, record the exact defect and regression before a narrowly compatible
repair; a semantic amendment needs a revised design/proposed decision and,
where it contradicts an accepted ruling, operator acceptance first.

Follow 0020 placement: cold concurrency diagnostics remain in the already
declared src/compat/page_work_tests.rs harness; public protocol tests remain
in tests/compat.rs; branch-stable inline tests stay measured. Every new
cfg(test) observation must be exercised. Preserve exact 100% line, branch
and function coverage and the complete house inventory. No exclusions,
gate/policy edits, new crate, allowlist changes or test feature flags.

## Project structure and explicit ownership

These permissions are for subsequent implementation in this run, not a
request for a speculative edit. Historical lane restrictions do not prevent
these specifically assigned proof operations.

| Path | Authorized final edit / execution / temporary mutation |
|---|---|
| specs/extraction-compat-proofs/{spec,plan,tasks}.md | This design; later dated clarification and evidence index/task completion with actual receipts. |
| src/compat/page_work_tests.rs | Execute existing held-route and callback tests; narrowly strengthen actual-work observation/diagnostics only if controls demonstrate a gap. Keep existing route coverage and observations. |
| src/compat.rs | Inspect owners and callers; necessary cfg(test)-only invocation observations. Temporarily mutate render_one/build paths for serialization-only proof and haystacks/contains for search proof, restoring exact bytes. No planned production change. |
| tests/compat.rs | Execute existing search and prefix regressions; tighten only a demonstrated missing assertion/fixture control. Keep real servers and SQLite. |
| src/store.rs | Execute emptied-entry regression. Optional precise test name/comment correction only; production mutation logic and seams remain unchanged. |
| src/extract/entities.rs | Execute independent mapping test; temporary same-cardinality scalar mutant, restored exactly. Test repair only on demonstrated gap; no planned table update. |
| scripts/gen-entities.py | Execute unchanged. A proven generator defect may receive a narrowly scoped repair preserving the pinned format and checks, with evidence and revalidation. |
| scripts/data/html-entities.json | Read-only pinned input; tampered bytes exist only in memory or private scratch. |
| src/api.rs, src/persist.rs, src/smtp.rs and other source | Read-only context for response callers, persistence and acceptance ordering. No unrelated changes. |
| .brokkr/state/ under this run | Uncommitted receipts, saved source and mutant diffs; preserve both position files and old evidence. Use a private subdirectory for new proof logs. |

A required edit outside this table is a concrete scope finding to report,
not permission to silently widen the task. No source-wide refactor is planned.

## Research and baseline

Read both run-local positions, intake framing, charter and relevant accepted
decisions plus proposed 0061. Source baseline is
238372dc18997fe704cb1fb4ebd2cfa2a64a61fd.

Inspection confirms:

- The generator validates raw SHA before parsing/emitting. NAMED has an
  independent JSON mapping test; count alone is not its oracle.
- render_one invokes find and build inside one awaited Store::offload,
  observes invocation threads and runs lookup/built gates. Native callers
  share this owner. The harness has eleven held real routes and a separate
  callback-internal thread witness.
- haystacks contains source text/html/subject/addresses/raw and excludes
  arrays. The wire fixture deliberately restores historical-style arrays
  absent from those candidates.
- delete_messages awaits one delete_async per input id and refuses on the
  first error. The existing fixture selectively refuses the middle id and
  checks live/reopened suffix and retry.
- The emptied-entry regression already names the retained-guard/same-shard
  deadlock. It recreates its deleted inbox before reopen.

The intake's 60,746-byte generator equality and tamper exit are historical
spot-check evidence only. This chief has not rerun Rust proofs or mutants.
Fresh final-candidate execution is mandatory.

Baseline SHA-256 identities recorded by chief:

| File | SHA-256 |
|---|---|
| scripts/gen-entities.py | 80541b78d56ec6ac6318adefc5bb8033f2c487fec375ad9118339b3856fb36d8 |
| scripts/data/html-entities.json | d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6 |
| src/extract/entities.rs | 4334e2e8f0af1ba8a6c0a65fa0b716f51a7ee70867018e350ff0b4d154cc6335 |
| src/compat.rs | ae67feaa8cc8c67fd42d7f312c51563d8dbf1b9cf9c1ef11c05ad970b55af0f5 |
| src/compat/page_work_tests.rs | d6cc899fe8d6fdd0cae10ffe9e21df966588bfc511d29184abb59e714908f5cb |
| tests/compat.rs | 1a8c35b6bfae5a208f45f0099227264729a187e194b41e6d1cf4bfb2b6d1e873 |
| src/store.rs | cf0abb5be160b0acdb2c427521ca0a71def73db69f667735cec38c28727acbc2 |

## Data model

No public type, persisted schema, table, migration or API shape changes.
Reuse per-store gates, test-local channels and invocation metadata. Keep
private fixtures, ephemeral ports, temporary SQLite paths and distinct
inboxes. Historical-style arrays are a fixture, never query-time extraction.

Each run-local proof receipt records candidate commit and source hashes,
command, test count, exit, relevant diagnostic, saved mutant diff and restored
source identity. Use separate files for baseline, negative and restored runs;
never overwrite a failed gate or historical receipt. Put a concise evidence
index in this plan during implementation, linking the actual run-local logs.

## Contracts and implementation approach

### Entity reproduction and independent control

Capture the generator subprocess's complete stdout/stderr and exit in memory.
Locate exactly one complete static NAMED declaration in entities.rs and compare
all declaration bytes including the final newline. Assert a unique complete
match and nonempty output, so a suffix or partial-table comparison cannot pass.
Pass snapshot bytes plus one newline through /dev/stdin to the same unchanged
generator; assert digest refusal, nonzero exit and empty stdout. Keep pin,
snapshot, table and attribution unchanged in the final candidate.

Run the independent Rust mapping test. Temporarily replace one mapping's value
with a different valid scalar without changing the name, sort order or entry
count. Require successful compilation and an actual mapping assertion failure,
then restore byte-identical source and rerun green. No permanent generator
wrapper or mutation framework is warranted.

### Held response proof

Run both existing tests first. Inspect a minimal serialization-only mutant:
actual find and projection/render/copy execute on the worker; the serializer
and old hold still execute off-worker. Where Build combines projection and
serialization, split only the temporary mutant's call path as necessary;
a whole-response move followed by an offloaded identity is not a
serialization-only control. Keep test observations attached to the work they
claim to observe; do not delete the seam to force red.

Run the held-route test separately and record its intended assertion or
hold-expiry/progress diagnostic. Run the callback witness separately. If an invocation-site assertion aborts
before its callback can record a thread, use a second temporary diagnostic
variant that bypasses only that adjacent assertion for the companion test;
retain the callback-internal observation and the same misplaced real work.
This variant cannot substitute for the held-route red above. Retain actual
callback thread observations rather than treating a nearby gate's thread
as evidence.
If the held evaluator misses the defect, minimally strengthen the existing
harness/actual invocation observation and repeat the experiment. Keep normal
build output and release code unchanged.

Use the existing explicit release and finite diagnostic gate escape, with
owned request handles and cleanup on new failure paths. A timeout is a failure
diagnostic, never a latency SLA or permission to serve only after release.
No sleeps as synchronization, global observation registry or new Store hook.

### Wire proofs and deadlock explanation

Execute the source-only search fixture on both dialects, preserving stored
array readback and source-positive controls. Verify fixture needles do not
occur in source/raw. Temporarily chain arrays into the existing candidate
iterator; require the miss assertion to fail, restore and rerun.

Execute the existing selective-middle-id deletion fixture with real SQLite
and HTTP. Preserve exact error, prefix/suffix/unrelated checks across reopen,
and exact retry responses. This fixture is the negative control; additional
rollback/storage mutants are not mandatory without an identified blind spot.
No production mutation hook or atomic deletion redesign.

Keep the emptied-entry explanatory comment unless an accurate wording/name
improves discoverability. Run its existing synchronous regression in a process
with an outer terminating deadline, preferably after compiling the test binary
so compilation time cannot masquerade as a hang. A generous watchdog expiring
is a failed proof, not a passing deadlock detection. Preserve listing, watcher
and recreation checks and the actual scope of reopen.

### Restoration and evidence

Run one mutant at a time. Save exact candidate bytes/hashes before each edit,
retain a diff and failure log, restore in a finally/trap path, compare hashes,
then rerun the affected proof green. Do not use a broad reset that could erase
other work. A compile error or zero selected tests never counts as red.
Final source identity must match the restored candidate used by full gates.
After any real repair, refresh affected receipts against the new identity.

## Quickstart and verification

From the repository root, candidate design preflight is:

    bash bundles/self/scripts/design-check.sh check .

The final sequence check runs its dispatched pinned copy; that check alone
owns design advancement. A local preflight is not the dispatched verdict.

Implementation first runs:

    cargo test --locked --lib all_whatwg_entities_match_snapshot
    cargo test --locked --lib held_single_message_builds_leave_smtp_and_health_serving
    cargo test --locked --lib actual_lookup_and_build_leave_the_async_worker
    cargo test --locked --test compat extracted_only_fields_miss_both_wire_searches
    cargo test --locked --test compat shim_delete_preserves_committed_prefix_across_reopen_and_retry
    cargo test --locked --lib a_delete_that_empties_an_inbox_removes_the_entry_live_and_after_reopen

Wrap the last execution in a process watchdog; run the compiled selected
binary directly if needed. Verify each filter selected the intended test.
Then perform the controls above and restored-green runs.

Run every command in the authoritative
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
on the restored final candidate, with locked Cargo commands and exact coverage
plus selftest, publish-guard and keyless seal proofs, licenses and pinned
brokkr compilation. Check the executable against bundles/self/tooling.json;
an absent or mismatched tool is outstanding work, never success. Verify uses
pinned scripts and validates public shim identities, plus fail-closed
native-tls absence. No offline verification installation or fallback.

Record CI-only chart/history/selftest, real-Docker smoke, advisory audit,
tag-only rate and port-80 bridge proof separately; do not trigger release
actions or substitute local claims. The engine selected the full lane and
owns subsequent verify/review dispatch: Qwen Max correctness, Opus security,
official DeepSeek Flash guarantees, DashScope GLM 5.3 tests. No fallback,
DeepSeek Pro, skipped review or waived security hold. This chief launches
none of those seats and reports drafted only.

## Risks

| Risk | Mitigation and accepted limit |
|---|---|
| Existing green tests hide serialization-only work placement | Inspect a realistic split mutant; require held evaluator red and separate callback witness, then restored green. This proves named failures, not every future rewrite. |
| Gate thread is mistaken for work thread | Retain actual invocation observations and callback-internal thread recording. Reject disconnected-seam controls. |
| Generator/table share an incorrect assumption | Full stdout byte equality plus independent every-mapping JSON test and same-cardinality scalar control. Tamper proves raw digest refusal, not all hypothetical authorized snapshots. |
| A synchronous deadlock freezes its own timeout | External process watchdog with kill/cleanup; compilation is separated where practical. |
| Test timeout masquerades as concurrent progress | Require no gate expiry before release and full route completion; retain 250/count evidence. No throughput or pool-capacity promise. |
| Extracted-only fixture accidentally matches raw | Verify source absence and positive controls on both real HTTP dialects. |
| Prefix test proves only an empty prefix or transient memory | Selective middle fault, A/B/C and unrelated control, reopen and retry. Graceful restart does not replace hard-kill suites. |
| Proof edits shrink coverage or become shipping mechanisms | Existing 0020 harness, cfg(test)-only observations, exercised seams, unchanged exact gate and dependency policy. |
| Mutation leaks into final gates | Saved bytes, finally/trap restoration, hashes, clean diff and separate red/restored logs. |
| Historical receipts or proposed 0061 are treated as completion | Fresh source-bound receipts, explicit pending operator questions and downstream gate/review obligations. |
