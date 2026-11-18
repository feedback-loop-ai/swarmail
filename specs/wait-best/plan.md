# Plan — wait-best

## Technical context

Rust comparison bench, tokio I/O and existing locked ring/base64; no new
package. Base `ed17b7f` contains story A (`9ac77db`), the bounded transport
from `fc47a90` (20 tests), and T1's 35 captured wait documents and integrity
tests. T1/T3/T4 are built; preserve and rerun them, not reimplement them. `Observation` has Native/Poll;
`validate_budgets` requires the common interval, v4 refuses nonempty waits,
`Chunk` has only Cost, and `OWED_SECTIONS` includes api/waits/testexec.
`Owned` owns complete HTTP scans, `FindScan` adapts them, and `observe`
currently accepts repeated expected tokens without recording that error.
Add v4 semantics around these existing owners. The shared stream client exists; story B cannot be assumed merged.

The repository exposes a spec template but no plan template. This plan
includes technical context, constitution check, structure, research, data
model, contracts, quickstart and risks inline.

## Constitution check

[0132](../../docs/decisions/0132-mail-comparison-protocol-v4.md) and its
acceptance settle the feature; no new ruling is sought. The panel synthesis
is explicit in [spec.md](spec.md). Keep accepted guarantees and historical
protocols unchanged, keep evals standalone, and use test placement rules of
0020. There is no server change or coverage exclusion. Preserve the 150,000
cap placeholders, one launch owner and no-retry measurement policy. All
house gates remain required. No collection or publication is authorized.
Necessary scripts wiring is within the user's stated approval of 0132;
avoid unrelated wrapper changes.

## Research and evidence

Use the nine-row evidence table in docs/evals/README.md and the documents
captured in ed17b7f. T1 is complete; do not recapture or infer upstream bytes.
All native_wait cells are closed. Of the four push channels, MailHog's
WebSocket is selected by its T10b proof (2026-11-18); the other three are
cited polls naming their readiness or registration gap.
Resolve only the remaining readiness questions under the spec's continuation
rulings. MailHog uses WebSocket, not an additional SSE path. Mailpit and
Inbucket remain cited polls unless the captured record proves readiness.

For smtp4dev add the minimal JSON handshake, record-separator framing and
distinguishable OnMessagesChanged probe/broadcast barrier in targets.rs.
Establish the required envelope from captured evidence before enabling it;
missing framework evidence is a named gap. Bound split/coalesced records,
reject unsupported forms and separate handshake, ping, completion, close,
probe and delivery. Test that probe traffic cannot satisfy the SMTP proof.
No generic SignalR package or additional module is needed.

Absent binary bytes mean missing proof, not absence of stream support.
An established source-level readiness gap permits an honest fallback.
Do not invent either to finish. Implement a small documented registration
envelope when necessary for a qualified channel; a full SDK, replay engine,
TLS bypass or readiness delay is outside scope. An unsupported required
transport is a named capability gap. No selected stream is already proven
by this design.

## Project structure and files

| Files | Implementation |
|---|---|
| `docs/evals/draft/sources.lock.v5.json`, `docs/evals/sources/<target>/` | Preserve T1 evidence; update only justified readiness/qualification facts |
| `docs/evals/draft/protocol.v4.json` | Closed native_wait cells, citations, three profiles and operation budget; keep B/D/E debt |
| `benches/mail_compare/protocol.rs` | Stream/evidence validation and version-specific profile/budget rules |
| `benches/mail_compare/clients/stream.rs` (existing) | Reuse bounded WebSocket/SSE transport; change only for demonstrated integration gaps |
| `benches/mail_compare.rs`, `tests/mail_compare.rs` | Path module declarations and waits chunk dispatch/test wiring |
| `benches/mail_compare/targets.rs` | Minimal target registration and existing authoritative scan interface; no copied routes |
| `benches/mail_compare/runner.rs` | Observer state, multiplicity, waits chunk/windows and bounded sends/QUIT |
| `benches/mail_compare/records.rs`, `report.rs`, `stats.rs` as needed | V4 facts, validation, report/admission recomputation and signed timing |
| `tests/mail_compare/v4.rs`, existing `tests/mail_compare/waits.rs` | Profile/parser/lifecycle/accounting/admission tests and ignored proofs |
| `scripts/compare-mail.sh` only if needed | Extend derived chunk dispatch while preserving bounds/signals/cleanup |
| `docs/evals/README.md` | Evidence matrix, limitations, proof commands and remaining story debt |

Do not change active `docs/evals/protocol.json` or `sources.lock.json`,
history, `src/`, dependency locks, adapters or accepted decision bodies.
The 0132 index row already exists; verify it rather than rewriting it.
Stories B/D may later share this transport by path without API-family
modules being required for story C.

## Data model and contracts

Extend typed observation with Stream and explicit backstop and transport/
evidence references. Reject mismatched intervals and unresolved references. Add a narrow
version-aware document rule: v4 refuses v5-only wait documents even with
literal caps restored; valid old reviewed documents remain legal. Test a
version-only downgrade and injection into historical v4 separately.
Reuse document identity instead of a second registry. Configuration is
source-cited and target-specific; connections use the owned target's declared
endpoint, not dynamic URL discovery.

V4 rows gain optional version-qualified scan cause and exact answer counts.
Omit additions for old rows and refuse them for old manifests. Preserve
multiplicity before judging answers; retain unrelated counts independently.
Report per-answer duplicate when expected occurrences exceed one, unrelated
when unrelated count exceeds zero. A mixed answer increments both and is
never an exact success. Missed means expected mail had no timely return;
ordinary empty expiry is expected. Transport errors and late answers retain
their reasons and censorship. Preserve legacy outcome vocabulary where
possible without silently changing historical classification.

Attach cause when starting a scan and preserve it through the future-send/
observer row merge. Signed ack-to-return derives from client return and SMTP
ack timestamps. Admission checks mode, phase, timestamps, counts, windows
and receipt pairing and recomputes aggregates. It cannot independently prove
the physical event; that is the real proof's job.

## Observer and transport design

Use one owner with persistent buffered transport state, selecting among
cutoff, backstop, pending connect/readiness, notifications and one in-flight
complete scan. No detached task or unlimited queue. If an owned reader task
is needed for framing, bound its queue, propagate failures and cancel/join
on every return. Only qualified delivery events set the dirty flag.

Before first readiness, no scan starts. After readiness, run initial scan.
Established loss permits reconnect while backstop remains live. Successful
re-registration schedules a reconnect scan. Malformed transport is an error;
recoverable EOF/reset schedules reconnect, never resetting cutoff. Space
reconnect attempts by 100 ms, with at most one in flight, each capped by the
existing request bound and remaining operation budget.

Anchor the 1,000 ms backstop at initial readiness; traffic and scan completion
do not reset it. A tick during scanning retains one due flag; run that scan
as soon as the current bounded scan ends. Preserve a dirty flag for events
during scans. For simultaneous pending causes use initial_scan, reconnect,
backstop, event in that order. A complete scan may discharge already-pending
causes together, but causes arriving after it starts remain pending for the
next scan. Cutoff wins over starting work. Existing request-bound grace may
finish a scan as late/censored, never timely. Test event floods for timer
progress and bounded pending state.

Use named limits initially: 32 KiB handshake headers, 8 KiB header/SSE line,
1 MiB frame and assembled message/event, 2 MiB total transport buffering.
Check lengths with checked arithmetic before allocation. If pinned evidence
needs larger limits, document and test finite replacements before selecting
that stream; never remove bounds. Readiness/control payloads do not emit
delivery notifications. Reject unsupported extensions/subprotocols; implement
required application negotiation only from evidence. SSE respects HTTP body
framing and event boundaries; retry/id fields imply neither replay nor
readiness. Transport failure never silently downgrades the method.

## Quickstart and validation

Use `cargo test --locked --test mail_compare`,
`cargo test --locked --test store_contract_docs` and
`cargo bench --locked --bench mail_compare --no-run` during implementation.
Run the complete house inventory on the final candidate, resolving
`SWARMAIL_BIN` from its build artifact record. Root tests do not replace
evals checks, and default tests do not execute ignored proofs.

For each implemented candidate, document a separate dispatcher-host command
of this form after evidence fixes its target and test name:

```bash
cat /proc/loadavg
timeout 600 taskset -c 0-31 cargo test --locked --test mail_compare <exact-proof-name> -- --ignored --exact --test-threads=1
cat /proc/loadavg
```

Set documented pinned binary/runtime variables first. Capture the test exit
status independently of the final load read. Use existing launch/cleanup
ownership and enforced v4 resources when measuring a window; missing
resources refuse rather than yielding an unenforced measurement. Run proofs
sequentially and preserve results/cleanup receipts. T10 execution remains OPEN on the dispatcher host and is not a sandbox
implementation failure. Host success is still required before selection;
failure leaves the candidate polled. Preserve the six Linux-only cfg_attr
ignore patterns; new host proofs are Linux-only and explicitly ignored.
README must specify exact qualified test names, MAIL_COMPARE_* variables,
individual commands and cleanup expectations before handoff. No collection is needed for
design validation.

## Continuation changes and proof ownership

This is a continuation, not a review return; there are no panel defect IDs.
Correct stale baseline and task status from fc47a90/ed17b7f, add T2's explicit
0132 §1.2 lock boundary, and assign T10 execution to the dispatcher as the
user instructs. These changes relax no readiness or stream qualification
criterion. Local code/proof readiness, host execution pending and qualified
stream selection are three distinct states. Until host success, retain the
candidate's cited 100 ms poll and unresolved qualification in draft metadata;
the ignored proof can exercise its candidate path without changing selection.
Update T1's candidate-state assertions only when real qualification evidence
or a precisely cited final gap warrants it; retain all integrity regressions.

## Risks

- **Missing readiness evidence.** Inspect source and label precise gaps;
  the cost is polling latency for that row. Missing binaries cannot be
  relabelled as evidence of absence.
- **Hand-written protocol reader.** Bound bytes/time, reject unsupported
  negotiation and test fragmented/control/error paths plus real endpoints.
- **Cancellation loses bytes or a wake.** Persistent parser state, dirty
  flags and scan-generation boundaries; deterministic mid-read/scan tests.
- **Reconnect/event starvation.** Independent tick/cutoff and bounded
  attempts; test floods and stalled reconnects.
- **False latency wins.** Exact counts, signed timing, censored late answers
  and admission mutations expose deduplication or timestamp mistakes.
- **Lifecycle bypass.** Extend story A's chunk/sampler owner, retain OOM,
  exit and cleanup rules, and test wait sends/QUIT at chunk cutoff.
- **Concurrent B/D work.** Reuse today's scanner; integrate later shared
  clients without duplicate routes.
- **Historical regression.** Check serialization and admission as well as
  frozen hashes; version guards apply to reports and validators alike.
- **Missing bytes/cgroups.** Keep executable ignored proofs, refuse missing
  prerequisites and report the gap. Design completion asserts no
  implementation proof completion.
