# Current-main implementation — issue #200

<<<<<<< HEAD
The port integrates shipped patches 03–06 into current #194/#196 owners,
preserving accepted #198/0071 business behavior. Framing/design patches 01–02
supply historical context; this run's committed design governs implementation.
All 53 evidence-packet checksums and the candidate engine digest match.
Decision 0063 has exactly one proposed index row. The original accepted 0055
bytes remain a prefix; a dated addendum records the historical scoped direction
and its supersession by accepted 0071. No proposal is accepted.

## Implementation and proof map

- T1–T4: strict required provenance on result v2, with no v1 decoder or invented
  provenance. Report refuses v1/mixed and pre-provenance v2 input before output;
  arithmetic remains unchanged. Separate artifact/projection envelopes stay v1.
  Python hashlib independently generated the compact current-v2 catalogue and
  definition vectors. Catalogue hash is
  `6068b687475db4be0000fa325de9f2b610b98470526085010ebd648e04862a94`;
  `business-score-v2` binds the accepted trap-disclosing definitions.
- T5–T7: build-time harness identity, measured executable digests, private owned
  source fingerprints/rechecks, bounded public worker CLI context and original
  native argv. Identity tests exercise worktree/packed refs/archive/rebuild,
  dirty-to-dirty edits, opaque/identical artifacts and replacement. Existing
  closed Git/Cargo/worker/runtime maps and process ownership remain intact.
- T6/T12: shipped cleanup acknowledgement and source-observation signal latch
  remain beside the existing owners. Both signals, retained-child conflict,
  Pending/release/bypass, envelope expiry and source-error/no-Cargo controls
  pass. Immutable child evidence is never rewritten; conflict prevents index
  completion, and index errors cannot rewrite committed rows.
- T8: semantic framed SHA-256 choices and one fixture UTC time reproduce public
  difficulty. Independent vectors, fresh-process projections, controlled private
  entropy, Date/DTSTAMP/MIME/calendar equality and real invite/OTP exchanges
  retain separate CSPRNG secrets and real expiry/deadlines.
- T9: canonical provisioned claimant or null at shared recording, with writer-only
  membership guards before admin/artifact payloads. Known Decoy/null-flow trap
  attribution and historical v2 claim scoring remain. The live OTP canary test
  fails with raw unknown-claim recording restored (exit 101), then passes after
  restoring the port; mutation logs stay private under .forge/.
- T10–T13: shared 0700 leaf/0600 temporary no-clobber publication, metadata last,
  post-cleanup finalization, assigned matrix contexts/seeds, eight-cell retention
  and bounded offline replay through the existing tasks::score. Tests retain
  64 KiB capture/drain, original-byte bindings, every boundary, wrong/rehashed
  associations, absent state, bad-last-cell zero output and no saved execution.
- T14–T16: all 136 baseline tests remain in their original files; the private
  preservation receipt inventories their names. Completed naive controls retain
  zero exits, healthy SMTP/MCP/HTTP, true recovered goals and exact trap failures,
  now also checked through the retained CLI replay. Oracle/null/isolation,
  expiry-only/reset/non-target recovery, privacy and lifecycle assertions remain.

## Narrow adaptations

Four textual conflicts retain both main's trap tests and the shipped privacy
controls. Main's v2 scorer, catalogue, snapshot and result versions remain.
Old result/catalogue acceptance expectations become strict refusal and actual
v2 vectors. Readiness-flood tests retain the original 2048-byte execution-field
bound after projecting out new provenance, plus the full 64 KiB v2 bound.
Requested artifact failure is infrastructure failure; preparation failure keeps
bound absent-state evidence. Private-value, native-argv, environment, mode,
negative-control and protocol assertions are preserved.

The shipped stdout Pending seam observes pipe pressure, and cleanup bypass
waits for owner entry. Both preserve the existing bounds and prove their bypass
controls return with owned work alive. Source-task failures still traverse
owned cleanup, and cancellation there prevents subsequent Cargo admission.

## Verification and limits

Every house gate and the plan's supplementary checks passed locally with the
candidate-local pinned engine: format, locked Clippy/tests, warning-free bundle
compile, exact coverage/selftest, publish/seal proofs, license check, fail-closed
native-tls absence, design/delivery selftests and standalone eval format/Clippy/
tests. Exact coverage measured 14009/14009 lines, 788/788 branches and 1527/1527
functions. The complete eval suite passed 181 tests. The README's eight-cell
matrix and individual/bare-state checks ran successfully; offline ordered replay
returned the original rows with the original binary unavailable.

A full eval run exposed a readiness race in the existing cleanup-fault fixture:
the parent published a child PID before that child installed its TERM disposition.
The test-only script now acknowledges that setup before publishing readiness.
The original live-survivor, direct-reap, refusal and rescue assertions remain.
Three focused runs and the subsequent full suite pass without altered bounds.

Commands, exits and commit identifiers are recorded under
`.forge/issue200-port-gates/`, separately from the immutable historical packet;
pre-final receipts are archived there. All gates are rerun against the commit
containing this record before the mandatory phase result is written.

No root production, manifest/lock, dependency allowlist, bundle, routing or gate
changes. CI owns macOS eval proof through #201, privileged port-80, chart/history,
Docker, network RustSec and tag-only rate proof. Operator owns live settings and
sealing; #221 owns namespaces. No journal, GitHub or sibling-worktree read.
Public declarations must not contain secrets. Provenance/hashes show measured,
declared or explicitly unavailable facts and consistency, not authenticity,
compilation reproducibility, exact model replay or historical privacy. Incomplete
and conflicting retained evidence is deliberately not repaired.
=======
The port restores the shipped #200 public contracts inside the existing #194
and #196 owners. Decision 0063 remains proposed. The only accepted-record edit
is the exact scoped 0055 addendum from verified patch 0003; its prior bytes and
status are unchanged. The decision index adds one proposed 0063 row. The local
packet's eight checksums and candidate engine digest were verified. No journal,
GitHub, sibling worktree, decision acceptance or routing change was used.

## Implementation and proof mapping

- T1/AC11: exact historical-prefix/addendum comparison and baseline test-name
  inventory passed. Root `store_contract_docs` is included in final house tests.
- T2–T4/AC1–AC3: separate strict V1/V2 result types, original-byte version
  dispatch, immutable v1 result/report/catalogue goldens, typed availability,
  semantic seed/time keys and independently fixed framing/argv vectors.
  Report formulas are unchanged. Native OsString execution is separate from
  explicitly public UTF-8 command hashing before path resolution.
- T5/AC1/AC8: build-time harness Git facts with worktree/packed-ref/archive
  proofs; measured/rechecked executables; owned source comparisons including
  private dirty-to-dirty content fingerprints. Runtime Git and Cargo use the
  existing Groups, retained wait/drain identities and private runtime owners
  after signal subscription. Git's complete map and private-path exclusion
  have independent probes. Opaque target source remains unavailable.
- T6/AC3/AC10: fixture/mail public choices use fixture-sha256-v1; independent
  private entropy supplies every capability class. Fresh real fixture processes
  prove deterministic projections, calendar/MIME equality and OTP transitions.
- T7–T8/AC6–AC8: shared fixture recording copies only provisioned canonical
  claims; Admin and retained writers enforce membership before payload. Bare
  and strict retained historical readers keep unknown claims scoreable without
  reflection or mutation. Reader errors use bounded categories. Real live OTP
  and independent canary tests cover all action families and whole public trees.
- T9–T10/AC4–AC5: every new artifact uses the existing exclusive 0700 leaf,
  descriptor-private 0600 temporary, complete write and no-clobber publication.
  Metadata is last. All handled trial paths finalize after cleanup. Requested
  retention failure changes the trial outcome to infrastructure error.
- T11–T13/AC4–AC6/AC9/AC13: current main.rs remains the matrix owner. Assigned
  public facts travel through bounded strict CLI context before native argv;
  the complete worker environment map is unchanged. Exact assigned v2 rows,
  exits, 64 KiB drainage and ordered refusal accounting remain enforced. The
  index binds rows only after cancellation arbitration and stdout commitment.
  Immutable child conflicts leave files untouched and prevent completion.
  Replay validates the whole matrix before emitting any row, then rescores
  existing snapshots without binaries, services or command execution.
- T14–T15/AC7–AC10: all baseline test names remain in their original owner.
  Full suites and final gates are recorded under the ignored run-local
  `.forge/epic192/issue200/` logs. Current commands, not historical receipts,
  determine this phase's result.

## Baseline adaptations

Every main/runner/protocol test listed in plan.md remains. The only public
behavior adaptations to old assertions are required by #200: new worker rows
are v2 with generated assigned IDs; readiness-flood tests retain the original
2048-byte execution projection bound and add the 64 KiB v2 bound; requested
artifact failure is now infrastructure rather than best effort; preparation
failure retains bound absent-state evidence instead of leaving no artifacts.
All old private-value assertions, literal native argv, complete environment
maps, private permissions, negative controls and real protocol checks remain.
Historical matrix helper controls now use explicit wrapper argv, rather than
relying on caller environment inheritance that #196 correctly removed.

The stdout Pending acknowledgement seam now observes actual pipe pressure via
poll, rather than merely an unscheduled writer thread. It does not change the
production writer, five-second bound, unread-pipe test or signal assertions.
This prevents the larger v2 representation from acknowledging a transient
scheduling gap as the blocked write the regression needs to exercise. The
cleanup bypass seam also waits for the existing hold-entry acknowledgement
before dropping its waiter. Otherwise a faster cached finalizer could abort
that waiter before it initialized its cleanup owner. The negative control
still returns with live owned work and fails the same completion predicate;
production cleanup registration, waits and budgets are unchanged.

New composition tests cover both signals after real child retention and before
commitment: full stdout accounting, unchanged child bytes, exit 2 and absent
matrix index. Envelope expiry is a cancellation request through the same
observed Pending cleanup fence, with repeated signals and a bypass negative
control. No parent kill timer or second completion handshake was imported.
Index-write failure preserves the committed stdout verdict while exiting 2.
Execution wall time continues to exclude target build time. Both signals also
remain latched when source observation has finished and owned cleanup precedes
Cargo admission; task-join errors still pass through that cleanup fence.
The duplicate-boolean wire vector uses explicit fixture data instead of the
checkout's build-time dirty flag, so a clean commit exercises the same refusal.

## Discriminating evidence and limits

Restoring the original unknown-claim copying in an isolated mutation makes the
real private-claim publication regression fail; restoring the exact source
makes it pass. The receipt records only verdicts, without canary values. The
existing cleanup bypass controls and the new envelope bypass return while
owned work is alive; their ordinary counterparts cannot return until release.

Public provenance is measured, declared or explicitly unavailable. It is not
attestation, reproducible compilation, exact model replay, credential
sanitization of public declarations or a secrecy certificate for historical
input. Hashes prove consistency, not authenticity against a local writer.
The timeout threshold cannot promise a total completion latency that bypasses
owned cleanup. Incomplete/conflicting evidence is deliberately not repaired.

T16 is complete only when the committed candidate's full gate receipts pass.
The phase result records that actual verdict. House CI retains chart/history,
Docker, network audit, release and privileged port-80 proofs. macOS eval proof
belongs to #201; namespaces to #221. Current bundles/0058 own protected review
and security hold; this implementer supplies no independent review result.
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
