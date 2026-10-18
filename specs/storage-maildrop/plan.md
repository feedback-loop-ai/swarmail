# Implementation plan: storage and maildrop repairs

Implements [spec.md](spec.md) and proposed
[0037](../../docs/decisions/0037-pop3-size-accounting.md) /
[0038](../../docs/decisions/0038-store-memory-and-timestamps.md).
This is a design handoff, not evidence that the repairs have been implemented.

## Technical context

Single Rust crate using Tokio, DashMap and rusqlite; dependencies are already
present. Both protocol owners share `smtp::bound_io`. POP3 snapshots are
`Vec<MaildropMsg>` containing IDs, sizes and gone bits; the store lists Arc
handles. `Store::offload` already awaits blocking-pool work and propagates
panics. SQLite restores full Email rows in `seq, rowid` order before bind.
There is no applicable repository plan-template file; this plan follows the
house-requested technical-context, constitution, project-structure, research,
data-model, contracts, quickstart and risks structure used by adjacent specs.

## Constitution check

No asynchronous gap is added to `Store::insert`, and SMTP's final 250 still
follows the awaited synchronous commit. No peer timer surrounds insert or
QUIT deletion. Subscriber registration, the Closed-arm subscribe-before-scan
rule and oldest-only pruning are unchanged. Links/codes stay extracted at
ingest. New semantic statements live in proposed 0037/0038; only the operator
accepts them, and accepted decision files are not rewritten.

Tests use real protocol servers and per-test stores/inboxes. Inline tests
remain the default under 0020. Only an actual cold failure path justifies
an owner-adjacent `*_tests.rs`, declared under `#[cfg(test)]`; access to a
private seam alone does not. Every added seam is instance-local, test-only
and exercised. No feature flags, exclusions, gate edits, dependency additions
or allowlist changes. Coverage remains exact in lines, branches and functions
under 0035, with no TMPDIR override. Keep mawk compatibility, existing ANSI
discipline, chaos at 0/100, and the 30-second helper deadline.

## Project structure and files touched

| Path | Planned implementation |
|---|---|
| `src/pop3.rs` | Bound LIST/UIDL; complete-buffer single-line replies; one serializer with buffer/count sinks; offload PASS; one RETR buffer; live DELE check; use `has_inbox`; focused inline tests. |
| `src/smtp.rs` | Complete-buffer `write_line`, keeping bound and flush; helper structural test. No DATA, acceptance, AUTH, listener or timestamp rewrite. |
| `src/store.rs` | Add `has_inbox`; correct module memory/timestamp documentation; existing residue/prune tests plus present/absent lookup coverage. Add a test-only PASS rendezvous only if needed at this owner to keep it per instance. |
| `src/persist.rs` | Drop retired index on open; share full restore SQL construction with its plan test; legacy/fresh/idempotent schema checks. |
| `tests/pop3.rs` | Backpressure for both commands; corrected size oracles and DELE parity; stable numbering, UIDLs, RSET and live retrieval checks. |
| `tests/starttls.rs`, `tests/smtp_edges.rs` | Retain and, if needed, extend real STARTTLS flush-order and sequential command coverage. |
| `tests/persistence.rs`, `src/store/persist_tests.rs` | Real restart with legacy index and exact data/counters; deterministic acceptance-order versus timestamp fixtures where the existing harness owns the needed seam. |
| `src/pop3/maildrop_tests.rs` (conditional) | Only if the PASS/backpressure interleaving needs cold timeout cleanup and private hooks; document that concrete cold path beside the cfg(test) declaration. Real sockets remain mandatory. |
| `tests/store_contract_docs.rs` | Focused assertions for README, 0038, store module and chart comments; use the existing `listener_contract_docs.rs` pattern without adding a framework. |
| `benches/read_stall.rs` | Extend with real sequential SMTP latency mode and explicit profile/sample reporting; optionally report PASS scaling/RETR allocation costs. |
| `README.md`, `charts/swarmail/values.yaml` | Correct current size, memory, restore and timestamp claims, link proposed amendments; chart comments only, no resource/configuration default change. |
| `docs/decisions/README.md` | Add 0037/0038 as proposed during implementation. |
| `specs/storage-maildrop/{spec,plan,tasks}.md`, the two proposed decisions | Exactly the five design artifacts; later evidence and dated clarifications update their owning artifacts. |

`src/model.rs`, configuration surfaces, Cargo manifests/lock, gate scripts,
delivery bundle and accepted decision files need no change. Audit all callers
and misleading comments with `rg`; do not introduce another production module.

## Research and resolved choices

At `25ad4df`, LIST/UIDL call `write_all` without `bound_io`; both line helpers
split CRLF into a second write. `wire_len` constructs a fresh stuffed Vec.
RETR builds that body then copies it to another Vec. PASS runs its snapshot
and counting synchronously in the async handler. DELE checks only the
snapshot. USER/PASS enumerate and sort every inbox name for existence.
The store's watcher paths, parse-start stamp and full restore already have
the semantics selected for documentation; they are not implementation gaps
to fill with the losing panel proposals. The schema creates `emails_inbox`,
and its plan test uses `SELECT id` rather than the actual projection.

The spec's ruling table resolves all panel conflicts. In particular, the
counter shares the serializer, but does not cache at ingest; PASS moves off
the worker; RETR has one buffer; TCP_NODELAY, global admission accounting,
receiver guards and acceptance-time clocks are out of scope. If measurements
disprove a required acceptance criterion, revise the owning spec/plan/tasks
with a dated clarification rather than silently claiming a different design.

## Data model and algorithms

### Reply construction

Build each single-line helper's text plus CRLF into one owned buffer, then
call `write_all` once inside its existing bound. Keep SMTP `flush` in that
same future. POP3 needs no new flush or transport module. Wrap each existing
LIST/UIDL multiline write in `bound_io(..., idle)`; `?` propagates errors
and silently ends a timed-out write. Existing read-timeout goodbye handling
is separate. Do not touch listener tracking, recovery or default idle values.

### One serializer, two uses

Keep one local generic traversal behind `write_stuffed` and
`write_stuffed_limited`. It emits normalized payload slices and CRLF, and
marks an inserted transparency dot separately. A small emission callback
can express the sinks without a public trait: the Vec sink appends all
emissions; the counter sums only non-transparency emissions in a `u64`.
Preserve the current LF/terminal-CR algorithm and limited-line TOP behavior.
Name the replacement for `wire_len` to describe unstuffed payload octets.
No counting sink materializes bytes, and no independent newline parser is
added for counting. Tests use independent literal expected bytes.

For example, `b".123456\r\n.123456\r\n.123456789\r\n"` is 30 normalized
payload octets and 33 transmitted payload octets, excluding the terminator.
Exercise pure helper edges inline; protocol-valid SMTP fixtures retain
0034's complete DATA boundary. Missing-final-newline inputs can use direct
store insertion before a real POP3 retrieval, rather than weakening SMTP.

### Snapshot, retrieval and deletion

After the existing PASS existence recheck, clone the store Arc and inbox
name into one `Store::offload` closure. Run `list`, reversal and metadata
collection there, calculating each size from raw bytes with the counter.
Return only `Vec<MaildropMsg>` and await before entering TRANSACTION. Retain
the existing honest empty snapshot if clear races the non-atomic recheck.
Do not introduce a size threshold, worker service or timeout around the job.

RETR still resolves the snapshot number and fetches the live Email Arc.
Initialize its one Vec with the header using the snapshot's size, append
stuffed raw directly, append terminator and perform one bounded write.
Email content under a given stored ID is immutable in these protocol paths.
No intermediate body, `.raw.clone()` or deep Email clone is allowed. Vec
capacity growth and allocator transient costs are not an RSS promise.

DELE resolves the slot and checks `store.get(id)` before inserting its mark.
Absence returns the existing missing-message error; presence marks normally.
The check does not lock the maildrop or reserve mail against external removal.
QUIT retains its ordered, idempotent deletion and committed-prefix behavior.
Implement `Store::has_inbox` as `self.inboxes.contains_key(name)` and use it
for both USER and PASS, removing the enumeration helper.

### Store documentation and persistence

No runtime watcher/timestamp change is planned. Replace current memory
claims in README/module/chart comments with AC9–AC11's exact posture. Link
0038 as the proposed amendment to accepted 0021/0026 without editing those
files. Keep timestamps and stored raw unchanged, including during restore.

The existing schema/open path executes `DROP INDEX IF EXISTS emails_inbox`.
Keep `emails_seq`, column migration and `next_seq` handling intact. Define
one full restore-query builder (using existing `COLUMNS`) used by `load`
and by `EXPLAIN QUERY PLAN`; a duplicated narrowed projection is refused.
Collect all plan descriptions so a separate temp-sort row cannot hide behind
an index-using first row. Verify legacy open/reopen and preserved values.

## Contracts and proof design

**Backpressure (#71):** test LIST and UIDL separately against real
`pop3::serve`, one slot, a short test-only idle argument and a small TCP
receive buffer. Authenticate, pipeline enough replies to exceed buffering,
then keep the receiver open without reading. Establish that an actual
reply write is pending, rather than merely exhausting commands and observing
a read timeout. Where public socket observations cannot prove that boundary,
an exercised instance-local test hook may observe the real write's Pending
state and session completion, without replacing I/O or forcing expiry.
Observe re-admission within the helper deadline; a retry here may observe
expiry, but must not serve as a replacement for the existing #111
first-admission regression. Preserve the genuine-full refusal and shutdown
tests. No sleep supplies the ordering.

**One write (#83):** an inline recording AsyncWrite that accepts the whole
offered slice checks one complete line write in each helper and SMTP flush
ordering. This is a helper structural test, not a mock protocol server.
Retain real TCP and STARTTLS tests. Extend `read_stall` with sequential real
SMTP commands on loopback and a fixed small mail; warm up, then measure at
least 100 transactions. Report MAIL/RCPT/DATA-prompt/final-250 latency
separately, sample count, means/p50/p99, build profile and host. Run baseline
and repair on the same host/profile. The primary acceptance profile is debug,
matching the reproduction; release results may be additional evidence.

**Payload and memory (#84/#86):** verify all advertised values against
client-unstuffed byte lengths and literal normalized fixtures; also verify
wire transparency dots and terminator. Correct existing DELE skew test,
do not duplicate its false assertion. A focused structural regression checks
that the actual RETR arm passes its final output to the serializer and makes
one write, and that the count sink creates no Vec. Use an allocation bench
if source-shape inspection cannot reliably prove an implementation's bound;
do not install a global allocator into parallel tests. Include an old-shape
negative control so the structural test cannot pass the reviewed defect.

**HTTP progress (#86):** gate actual snapshot work within the offloaded
closure on a per-instance rendezvous, with real POP3 PASS and HTTP sockets
sharing a current-thread server runtime. After the gate acknowledges entry,
HTTP must respond before the gate is released. Arrange timeout cleanup from
an independent driver thread so a deliberately reintroduced inline snapshot
cannot deadlock the test runtime forever. That cold failure/cleanup path
belongs in the justified sibling harness; branch-stable helpers remain inline.
On the negative control, preserve the observational gate but remove offload:
the assertion must fail for lack of HTTP progress before release, not because
the seam failed to compile. No shared global gate or elapsed-ms threshold.

**Store claims (#87):** keep existing watcher count/reuse/clear/Closed/lag
tests. New doc assertions require the complete current contract and reject
old unqualified memory claims in current docs; historical accepted decisions
are intentionally not searched as though they were current replacements.
Use synthetic stamps opposite insertion order to prove pruning and restore
without clock assumptions. For a production red pin, create a legacy data
file with the retired index and real delivered mail, restart the real server,
read identical mail through POP3/REST, stop it, and assert index absence and
counter preservation in SQLite. Baseline keeps that index and fails the
assertion even though its wire behavior remains good.

**Evidence reconciliation:** the intake's blanket real-protocol red request
does not turn #87's allowed documentation branches into runtime changes or
override #83's structural/bench proof choice. AC14 explicitly rules that
distinction. Keep real-server preservation tests and the #87 index restart;
label doc-only reds honestly. For every child, record test/bench command,
baseline SHA, failing assertion, repaired SHA and result in an ignored
run-local ledger. Transplant tests to an isolated baseline worktree; if a
new seam is necessary, carry only observation scaffolding or use a minimal
old-behavior mutant on the repaired tree and name that limitation. A build
error, missing new decision file alone, or unchanged-runtime green is not
behavioral negative evidence. For documentation reds, compare old normative
claims against the corrected contract rather than just checking file presence.

## Quickstart and validation

Implement tasks in order on a feature branch. Run targeted tests as each
change lands; run all six gates before implementation delivery:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --locked --test pop3 --test starttls --test persistence
cargo test --workspace --locked --lib
cargo bench --profile dev --bench read_stall -- --smtp-roundtrips 100
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
cargo deny check licenses
```

`--smtp-roundtrips` is the bench mode to add, not an existing command claimed
to work today. Include the new documentation-test target once authored.
Coverage is exact 100/100/100 with no TMPDIR override. Save failures honestly;
an environmental refusal is not a passing gate. No baseline pass demonstrates
these new acceptance criteria.

Validate design artifacts for nonempty content, the Acceptance Criteria and
Risks headings, task checkboxes, relative links and proposed decision status.
Stage exactly the five design files and make a signed imperative commit with
a rationale body. Never stage panel/intake evidence or push. If Git metadata
is read-only, leave reviewable artifacts and disclose the missing commit in
the result file; do not create alternate Git metadata to bypass that boundary.
Later implementation owns README/index/chart edits and the framed PR/review/
17-check/ship procedure; issues remain for the operator to close.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Backpressure test passes because a command read expires. | Observe the actual blocked reply write and release; use real socket buffers plus a test-only observation hook if needed. Old unbounded writer must fail. |
| Shared serializer changes malformed-newline or TOP behavior. | Literal edge fixtures and real unstuffed-wire checks; share emission, preserve existing splitting rules exactly. |
| Counting on every PASS amplifies CPU or blocking-pool occupancy. | Offload the entire snapshot, allocate no payload; report scaling honestly. Cached ingest size is explicitly deferred. |
| RETR still exhausts memory under concurrent large fetches. | One additional representation, bounded write duration, quantified 12.5-GiB exposure before overhead; no zero-copy or RSS-ceiling claim. |
| Watcher names, inboxes or restore cause unbounded memory/OOM loops. | Current README/module/chart contracts expose the cost and full-restore behavior; preserve all release paths. No invented total-memory guarantee. |
| Timestamp sort disagrees with deletion or restore order. | Document parse-start wall time and test acceptance-based order with synthetic reversed stamps. Do not move pruning to timestamps. |
| A private gate deadlocks the old-behavior negative test. | Independent timeout cleanup thread and a concrete cold-path harness; assert HTTP progress while held. |
| Commit order or watcher correctness is weakened by adjacent refactoring. | Keep mutation/lifecycle algorithms unchanged; retain persistence, burst/rate, Closed/lag, DATA and shutdown suites. |
| An unused index survives upgrades or an EXPLAIN test omits a sort row. | Drop on every open; legacy/fresh/reopen tests; share actual SQL and inspect every plan row. |
| The single-write latency bench still misses 5 ms. | Record failure and revise the design with evidence; do not use a green structural test to waive the performance criterion. |
| Documentation alternative is misreported as a runtime repair. | Separate doc-contract reds, unchanged-runtime preservation and index-removal production proof in the ledger. |
| Environment refuses Git writes or socket-based gates. | Preserve artifacts/logs, report exact blockage and unmet checks; never claim committed or fully verified delivery. |
