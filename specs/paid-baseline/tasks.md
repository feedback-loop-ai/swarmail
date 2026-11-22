# Implement and prove the paid baseline

- [x] Add the proposed 0137 index entry and implement deterministic product planning, freshness-margin parsing, dry-run and explicit authorization in the baseline command. Extend the existing dry-run test with default and deliverability counts, ordered declarations/coordinates, estimate, per-family rule, stable ID across output paths and changed ID for changed product. Prove zero reads, creations and launches. Give missing authorization and wrong ID separate tests, including resume.
- [x] Implement the closed paid selection and private contract preflight for all three families, with exact profile/version, measured pinned digest and strict auth shapes. Extend wrapper tests with independent missing-family, wrong-version, digest mismatch, wrong auth shape, unsafe file, opaque launcher, timeout and bounded malformed-banner refusals. Assert family/reason, exit 2 and zero paid launches even when only the last family fails. Exercise v1 direct callers.
- [x] Implement clock-injected freshness validation and scheduling. Give too-close expiry its own negative test. Cover equality, one unit below, overflow, stale Codex access expiry despite recent refresh, malformed claims, each dsh route and rotation, missing attestation, renewal, earliest eligible chunk across tasks/families, all-deferred reasons and interruptible wait. Use a controlled clock/wakeup and independent launch log.
- [x] Carry the verified binding to the direct wrapper and all admitted paid rows, through failures, refusals, retention/replay and report inputs. Replace a CLI between preflight and exec and on resume to prove refusal. Check actual fake executable hashes and versions separately from harness hashes, earlier supported rows and missing required paid bindings. Place synthetic canaries in every credential and probe output, inspect argv, diagnostics and retained/public evidence for leaks, and check only the intended projection enters each CLI.
- [x] Introduce the exclusive run writer, immutable manifest, slot/attempt assignments and private atomic publication using existing artifact and journal helpers. Test unsafe roots, a second writer, drifted product/harness/target/contracts, duplicate assignments with terminal scores, tampering and partial initialization. Assert evidence bytes survive refusals and a competing process starts no slot.
- [x] Connect existing assigned-cell execution and cleanup to the scheduler and retained control gate. Run one actual baseline command with fake direct CLIs over the deliverability tier. Assert the two expected control outcomes precede all six five-trial chunks, maximum concurrency one, exact 30 paid coordinates, canonical row order, report status and version/hash on every paid row. Reuse real control and loopback fixture helpers. Inject a bad control and corrupt retained control separately and assert zero paid launches.
- [x] Implement resume reconciliation, cancellation and the ambiguous-attempt confirmation path. Kill at bounded observable events between chunks and mid-chunk after a trial marker but before coordinator acknowledgement. Compare pre/post score bytes, exact coordinate sets and independent per-coordinate launch counts. Include a measured failure and require one launch per committed slot. Kill before publication, prove surviving ownership blocks retry, then prove confirmed cleanup allows a fresh attempt only for the unscored coordinate. Preserve cancellation evidence, operational error precedence and existing matrix signal tests.
- [x] Publish or verify canonical streams and report from retained records. Stop after each final publication, resume and compare bytes, exit status and zero additional trial launches. Test conflicting output, completed runs and terminal harness errors. Round-trip rows with the existing report/replay readers.
- [x] Update the two eval READMEs, wrapper guide and run plan with exact authorized start/resume examples, margin, private installation/credential requirements, operator smoke limits and unmeasured accounting. Exercise documented flags against CLI help and existing documentation-boundary checks. Date the status in docs/evals/README.md.
- [x] Run targeted tests and every local gate in docs/house-rules.md, including coverage selftest, publish-guard proof and the locked standalone suite against the artifact-resolved candidate. Use artifacts::resolve_root before safe_root, direct item-level Linux cfgs for Linux helpers and the prescribed non-Linux ignore on tests launching binaries or reading /proc. Bound every test wait. Record targeted filters, measured added seconds (budget a few seconds), gates and CI-owned proofs in the implementation ledger using “Part of #393”. Commit unsigned as work progresses. Use stand-ins and loopback throughout.

## Implementation ledger (2026-11-22)

Part of #393. Every change is in the standalone `evals/` workspace, its docs
and `docs/`. No root `src/`, `scripts/`, `.github/`, `.brokkr/`, lock or
`deny.toml` change, and no new package.

**Built.** `baseline` prints the plan first: `product_id`, control slots,
task-major chunks, the six declarations, the estimate, and the
per-credential freshness rules and margin. `--dry-run` stops there. Spending
needs `--authorized-product` (`authorization_missing` /
`authorization_mismatch`). `wrappers::paid_contracts` verifies the version-2
selection for every planned family: profile, exact version, a pinned digest
that is measured, an executable that is a regular file and not a symlink, a
bounded `--version` probe, and the auth shape. `wrappers::valid_until` reads
Claude `expiresAt`, the integer `exp` of the Codex access token (after
`last_refresh` parses) and the dsh per-route attested `valid_through`. The
run has an exclusive `run.lock` and writes `manifest.json` before any slot.
Controls are reused only when the run's single complete control matrix
validates. Chunks start while `fresh_enough` holds, and an all-deferred wait
prints its reason and is interruptible. `attempts/<id>.json` is published
before a chunk's matrix starts and `.closed` after its owned cleanup. The
scored-slot ledger is rebuilt from retained trial evidence. A missing closure
needs `--confirm-cleanup <id>`, and `admission::idle()` must hold. Derived
output is published or byte-verified. The wrapper rechecks the pin and expiry
before exec. `provenance.agent.cli` is optional and validated; it is present
on every paid baseline row, refusal rows included.

**Deviations from the plan, for review.**
- The `--version` banners are fixed per profile
  (`2.1.310 (Claude Code)`, `codex-cli 0.164.0`, `0.1.5-rc.1`). The dsh
  banner grammar has not been observed; the operator's smoke confirms all
  three.
- A worker orphaned by a coordinator SIGKILL can finish its trial and retain
  its own hash-bound score. The resume counts that slot as scored and does
  not re-run it, and the proof asserts exactly that.
- An interrupt that lands during the controls leaves a complete control
  matrix with harness-error rows; a resume then refuses
  `baseline_controls_failed` and the operator starts a fresh `--output`
  (low, tracked; nothing paid can be lost there).

**Review return 1 (2026-11-22), what changed.**
- Nominal coordinates: a paid attempt's matrix plan carries `nominal` (run
  task and model index, planned repetitions), every cell's coordinates and
  seed are its planned slot's, and the seed derives from a fixture master
  recorded in the manifest (schema 2). The ledger cross-checks the plan's
  nominal and master against the assignment, so an edited `repetitions`
  refuses. Closures must name `owned_cleanup` or `operator_confirmed`.
- Recovery: a resume binds the manifest's recorded contracts and master and
  rechecks the live contracts and credentials only before a chunk starts; a
  complete run regenerates `report.json` with the selection, every CLI and
  every credential deleted.
- Launch binding: the coordinator hands each paid trial's wrapper the
  manifest digest and `valid_through` = chunk admission + margin through the
  worker context (`runner::PaidLaunch`). The wrapper refuses a drifted pin
  (`wrapper_contract_drift`), a login shorter than that
  (`private_credential_margin`) or a half binding, before exec.
- Unscored refusals: a paid agent gets a close-on-exec launch descriptor
  (fd 3). The wrapper writes `ready` just before exec and `unstarted` on any
  return; anything but exactly `ready` makes the trial a harness error
  (`agent_unstarted`) that consumes no slot. The model never holds the fd.
- The version probe gets a private TMPDIR and the four XDG directories
  (0075 §1). Codex validity is the earlier of the access token's `exp` and,
  when the id token is a JWT, its `exp`.
- Docs: wrappers.md drift wording, README's probe and resume wording,
  run-plan.md's filtering and incomplete-report sentences. The dry-run test
  carries the non-Linux ignore.

**Not proved here.** There is no probe-timeout test, because it would wait
out the real 10 s bound. Final-publication interruption is covered by
deleting and regenerating `report.json` and by refusing conflicting
`paid.jsonl` bytes, not by a kill during the write itself. A forged duplicate
score across two attempts would refuse with `slot_scored_twice`; the
integration test covers stray and edited evidence, not that forgery. No
test reaches `controls_ambiguous`, `confirm_cleanup_unknown`,
`paid_evidence_unexpected`, `closure_without_assignment`,
`matrix_plan_missing` or the reopen-side `baseline_output_unsafe`; the
launch-pipe setup failure arm and a wrapper killed before it reports are
also unexercised (both fall to the same harness-error path).

**Tests (targeted filters).** `--test paid_baseline` has five tests. The
full deliverability run with fake direct CLIs covers the controls gate,
Claude deferral, a SIGKILL mid-chunk, `run_writer_busy`, the live-owner fence,
`--confirm-cleanup`, a SIGTERM between chunks, the renewal wait, exact
per-model launch counts, unchanged committed score bytes, the CLI identity on
all 30 rows, canary absence, reopening with zero launches, `output_conflict`,
stray and edited evidence (agent, repetitions, closure cause), the nominal
coordinates and slot seeds of all 30 rows, a mid-chunk CLI rotation whose
three later trials are refused before exec and stay unscored (exactly two
glm-spark launches, `scored=2/5`, the resume runs `[2, 3, 4]`),
`contract_drift` and `digest_mismatch` between chunks, `manifest_drift` for a
changed margin, and recovery with no private input present. The other
four tests are the dry run, authorization, every contract and credential
refusal, and freshness/next-chunk. `--test baseline` keeps the hard and
expert real controls, now through the all-deferred wait and an interrupt,
plus a corrupt retained control. `--test wrappers paid_selection`
(launch-time pin, dsh route projection, expiry, missing contract, the run
binding's drift, margin and half-binding refusals, a bound launch, and an
expired Codex id token). `--lib
paid_tests cli_identity` (token-claim grammar, banners, CLI binding).

**Added test time** on this host (load 17–39): `paid_baseline` 87–91 s
(was 80 s; the mid-chunk rotation adds three unstarted trials and seven
short resume invocations); `baseline` 107 s → 37 s, because the 32-control
default run folded into the 2-control deliverability run; `wrappers
paid_selection` 0.5 s. The net is about +20 s of wall time on this host.

**Gates (local, final tree after review return 1).** Root: fmt, clippy
`--locked`, `cargo test --workspace --locked` (green), `cargo build --locked
--bin swarmail`, the coverage gate (exact 100/100/100) and its selftest,
publish-guard-test, seal-branch-test and `cargo deny check licenses`. Evals:
fmt, clippy `--all-targets --locked`, and `cargo test --locked` with
`SWARMAIL_BIN` set to the candidate's built binary (all green). No root
`src/`, `scripts/`, `.github/`, `.brokkr/`, lock or `deny.toml` change.

**Not run here.** `brokkr compile` for every recipe: no pinned brokkr binary
is reachable from this seat, and no `.brokkr/` file changed. Only CI runs the
macOS test job, Docker smoke, Helm, RustSec and the pinned bundle check.
