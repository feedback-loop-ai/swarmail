# Current-main implementation — issue #200

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
