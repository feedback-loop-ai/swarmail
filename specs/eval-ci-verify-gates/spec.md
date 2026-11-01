# Gate the eval harness in CI and the verify seat

Date: 2026-10-31. Status: design draft. Epic #192 (close every open review
residual), issue #201 (P2; evals, ci-release, decisions). Zero-Codex
`claude-careful` lane (decisions 0098, 0099; review residuals rework under
accepted 0102). Base: `main` at `8dfdf6b`. Operator direction 2026-10-31:
PR #273 (`epic192/issue201-eval-pr`, head `3d4686c`) is closed as
unmergeable; this run delivers the issue fresh. #273 is reference material
only — nothing from it is merged or cherry-picked.

## Feature and rationale

`evals/` is its own `[workspace]`: the root `cargo fmt`, `clippy`, `test` and
`deny` never reach it. Accepted [0055](../../docs/decisions/0055-agent-eval-harness.md)
deferred the CI and verify wiring "until epic #120 batch 5's gate changes
land". Batch 5 (`1d45e57`) and batch 6 (`40767cb`) landed; nothing wired
evals. So the oracle/null/isolation proofs, the lock-subset boundary test and
the evals crate's own fmt and clippy are unenforced, and a change to the MCP
surface the oracle hard-codes (`evals/src/mcp.rs`: the `protocolVersion`
check and the `content[0].text` JSON requirement) can break the harness, or
silently change what it measures, with CI green.

The change is wiring and paperwork. Every proof the issue wants enforced
already exists in `evals/tests/` and `evals/src/`; the defect is that no gate
runs them. This design therefore adds **no new file, CI job, status context,
script, dependency, eval test or decision**. It adds the three evals gates to
the three places that together are the one inventory — CI, the pinned verify
seat, and `docs/house-rules.md` § Gates — and brings 0055 and the eval
documents current by dated addendum and dated clarification.

## Authority and scope

0055 is **accepted** (operator, 2026-10-19). Its deferral was conditional and
the condition is met, so wiring the gates is compatible work under accepted
0045 §8 and contradicts no accepted ruling. No decision is declared by this
spec; no decision is accepted by this run. 0055 changes only by a dated
addendum — its body, including the historical "## Proposed ruling" heading and
"This proposed decision…" sentence, stays byte-for-byte (operator framing
2026-10-31). 0076 (sequential trials) is accepted (operator, 2026-10-26) and
trials stay sequential. The bundle lives at `.brokkr/recipes/self/` (0100).

Preserved untouched: 0001 synchronous accept/store, 0002 subscribe-before-
check, 0003 oldest-only pruning, 0004 AUTH inbox routing, 0005 ingest
extraction. Nothing under `src/` changes; the root coverage denominator and
the exact 100% floor are untouched. No dependency is added; `deny.toml` is
unchanged; the nine required status contexts of 2026-10-24 are unchanged.
No paid trial runs: the eval suite's trials are oracle and null agents on
loopback fixtures.

## Panel synthesis — rulings

The simplicity and robustness positions agreed on the shape (steps inside
the existing `test (ubuntu-latest)` and `test (macos-latest)` jobs; three
`run` lines in `verify-seat.sh`; addendum-only 0055; keep the `#201` tokens
the boundary test asserts; no Python artifact selector; no deny widening).
Where they differed, the rulings and what the losing side costs:

| Question | Ruling | Cost accepted |
|---|---|---|
| Where fmt/clippy run | In the `test` matrix on both OSes, not the ubuntu-only `quality` job (intake's C1 is overridden here). Evals has `cfg(target_os = "linux")` / `not(linux)` arms; Linux clippy never lints the macOS arm, and the scope says "on ubuntu and macOS" for all three. | fmt runs twice (seconds). |
| One combined step or named steps | **Robustness wins**: one named step per gate (resolve binary, fmt, clippy, test). The task requires the PR to say what each gate adds to wall time on each OS; GitHub's per-step durations are that measurement. A single combined step would need hand timing. | Four steps instead of one in the workflow. |
| How `SWARMAIL_BIN` is found | **Simplicity wins**: `cargo build --locked --bin swarmail --message-format=json` piped through `jq` to Cargo's own `compiler-artifact` record for the `swarmail` bin, then `test -x`. Honors `CARGO_TARGET_DIR`; same mechanism as the harness's own resolver; `jq` is already a hard dependency of the seat and preinstalled on both runners. Robustness's `test -x`, path echo and SHA-256 print are kept (cheap, and they feed the PR's provenance sentence). #273's ~50-line Python selector, duplicated in CI and the seat, is rejected: a second owner of a job the harness already does. | The binary may be a relink of the test build's artifact rather than the same inode; both are the candidate's source under the candidate's lock, which is what the issue asks. |
| Harness fallback build in CI/verify | Never. `SWARMAIL_BIN` is set explicitly and asserted executable before the suite, so the harness's "build when unset" path cannot run in either gate. | None. |
| Host lock around the seat's eval tests | **Not now.** Admission is `/tmp/swarmail-evals-<uid>`; whether concurrent hermetic exec boxes share `/tmp` is unproven, and a `flock` in the pinned seat is more code, a re-pin and more fixture work for a race nobody has observed. A sibling's refusal surfaces as the named `concurrent_trials_refused` diagnostic in the verify notes, and the remedy is a rerun. Revisit with evidence. | A concurrent sibling run can red a correct candidate with a named, non-silent diagnostic. |
| Roster/vacuity assertion | **Not now.** `cargo test` exits 0 on zero tests, but the root suite has the same property and evals has no `#[ignore]` today; a program that counts tests is new code the issue did not ask for. The implement seat records the test counts in the PR instead. | A future PR that ignores every eval test would pass green; review owns that. |
| rust-cache for `evals/target` | Adopt robustness's one-line `workspaces` input on the existing cache action. No new action; directly cuts the macOS cold cost. | Larger cache. |
| Job timeout | Keep 30 minutes. Raise to 45, with the measurement in the comment, only if the first CI run shows the macOS `test` job above ~21 minutes. This run cannot observe CI, so that is an operator follow-up, named in the PR. Never split into a new job (a new, non-required context). | A first-run timeout is a red, not a false green. |
| `cargo deny` on `evals/Cargo.lock` | Not added (not in the issue's gates). Named honestly: root deny does not read the evals lock; the boundary test pins every foreign package to an exact (name, version, source) triple, so a license cannot change without a version change that turns the suite red. | The evals license check stays by-manifest-path and seat-run. |
| Finding 3 (stdout channel) | **Already closed on main; cite, do not change.** `drain()` in `evals/src/runner.rs` discards every byte after the one-shot typed announcement; the only `unbounded_channel`s are test seams. `readiness_continues_draining_after_the_first_valid_announcement` (`evals/tests/runner.rs`) already writes 1 MiB after the announcement and asserts a pass with bounded output — that is the flood proof robustness asked for. No #151 comment: posting to GitHub is outward-facing and not this run's to do. | None; the PR cites the test by name. |
| Decision for this change | None declared. 0055 is accepted; its addendum records the wiring. Specs are not decisions and are corrected directly as dated clarifications. | None. |

## Requirements

**R1 — CI gates on both OSes.** `.github/workflows/ci.yml`'s `test` matrix
job, after `cargo test --workspace --locked` and before the port-80 steps
(macOS's runs as root and stays last), gains four named steps: resolve the
candidate's `swarmail` binary from Cargo's artifact record into
`SWARMAIL_BIN` via `$GITHUB_ENV` (asserting `-x`, printing its path and
SHA-256); `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked
-- -D warnings` and `cargo test --locked`, each with `working-directory:
evals`. The toolchain step of that job lists `rustfmt, clippy`; the cache
step's `workspaces` covers `. -> target` and `evals -> target`. No
`continue-on-error`, no `|| true`, no OS condition on any eval step. Job
names and the nine required contexts are unchanged.

**R2 — Verify seat gates.** `.brokkr/recipes/self/scripts/verify-seat.sh`
runs, after the root `cargo test` and before `brokkr compile`, three `run`
lines: evals fmt, evals clippy, and evals test with `SWARMAIL_BIN` resolved
from the same `cargo build --locked --bin swarmail --message-format=json | jq`
record and asserted executable. Offline stays `CARGO_NET_OFFLINE=true`; the
seat installs nothing. The header's "cargo fetch --locked ran on the host"
sentence names `evals/Cargo.toml` as a second manifest the host must have
fetched (the 0055 named exceptions resolve from the local cache). The final
pass note names the evals gates. The seat's digest pin in
`tests/delivery_constitution.rs` moves with it; the fake-cargo fixture gains a
`build` branch that prints one artifact record naming an executable fixture
file, and the gate-order expectation gains the four new cargo calls.

**R3 — One inventory.** `docs/house-rules.md` § Gates lists the evals
commands in its gate block and adds one sentence: `evals/` is a standalone
unpublished workspace outside the root coverage denominator (0055), so CI
and the verify seat run its fmt, clippy and tests explicitly, `--locked`,
against the candidate's root `swarmail` binary, on ubuntu and macOS; the
root `cargo deny` does not read its lock (the boundary test pins it). The
inventory test in `tests/store_contract_docs.rs` gains the three lines as
needles. CI wiring gains needles in `tests/delivery_constitution.rs`.

**R4 — 0055 by addendum only.** Append `## Dated addendum — 2026-10-31, CI
and verify wiring (#201)` recording: acceptance on 2026-10-19 (so the body's
"Proposed ruling" and "This proposed decision" are historical draft text);
the batch-5 deferral condition is met and the deferral text in the body and
in the 2026-10-30 amendment's last sentences is superseded; the exact gates,
their CI placement and the seat placement; `SWARMAIL_BIN` from the
candidate's build on both OSes; macOS eval proof is now CI-owned; 0076 is
accepted (operator, 2026-10-26), correcting the 2026-10-25 addendum's
"remains proposed"; the evals license check remains by manifest path; the
stdout residual's disposition (closed by `drain()` and its flood test); and
that the addendum accepts nothing, adds no dependency and changes no
guarantee. `git diff` of the file shows added lines only. The
`docs/decisions/README.md` row for 0055 gains "; amended 2026-10-31: CI and
verify wiring (#201)" with status unchanged.

**R5 — Specs and README current.** `specs/agent-eval-harness/spec.md` (AC11,
AC12 and the 2026-10-24 ownership paragraph) and
`specs/eval-measurement-repairs/spec.md` ("still proposed", "proposed 0055")
say 0055 is accepted and drop the deferral, as dated 2026-10-31
clarifications that preserve earlier dated entries. `evals/README.md`
replaces "Until #201 adds evals CI gating…" with the statement that CI and
the verify seat run these gates (#201), keeps the commands as the local
reproduction, moves the "outside CI … until #201" / "#201 owns" sentences to
the past tense, and corrects #149's path: the fixture app is the `fixture`
subcommand in `evals/src/fixture.rs`; `evals/fixture-app` never existed.
`evals/Cargo.toml`'s "Until issue #201 wires CI" comment is updated. Every
one of `evals/README.md`, `evals/run-plan.md`,
`specs/agent-eval-harness/spec.md` and 0055 still contains the literal
`#201` (asserted by `sequential_contracts_and_decision_links_are_honest`).
References to still-proposed 0073/0075 are left alone.

**R6 — Red proofs, not committed.** The implement seat demonstrates both
reds the issue names in scratch copies and records the output in
`specs/eval-ci-verify-gates/implementation.md`: (a) a root `Cargo.lock`
entry evals shares bumped → `every_evals_package_is_already_in_the_root_lock`
fails under `cargo test --locked`; (b) the root binary's MCP text-block
emission changed → the oracle's `content[0].text` parse fails and the proof
suite reds. Both mutations are reverted; neither is committed.

**R7 — Wall time, honestly.** The implement seat measures each evals gate on
Linux (cold and warm `evals/target`) with the exact commands and records
them. The PR body carries a per-OS, per-gate table; macOS figures and the
ubuntu CI figures come from the PR's own CI run's step durations (this run
cannot push or observe CI), with the local Linux numbers labelled as the
pre-push estimate. No number is invented.

**R8 — All gates locked and green.** Root fmt, clippy, test (`--locked`);
`brokkr compile --bundle .brokkr/recipes/self` (re-pins the seat); the
coverage gate and its selftest at exact 100%; publish-guard, seal and
delivery-proof tests; `cargo deny check licenses`; shims unchanged
(`sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256`);
and the three evals gates by hand against `cargo build --locked --bin
swarmail`'s artifact. This run's own pinned verify seat predates the change
and will not run the new lines; the gate-order test proves the new seat's
sequence by execution and the implement seat records the hand run.

## Acceptance Criteria

- **AC1 — CI wiring.** `grep -c evals .github/workflows/ci.yml` is nonzero;
  the `test` job contains named steps running, with `working-directory:
  evals`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked
  -- -D warnings` and `cargo test --locked`, preceded by a step that sets
  `SWARMAIL_BIN` from `cargo build --locked --bin swarmail
  --message-format=json` and asserts it executable. No eval step carries
  `if:`, `continue-on-error` or `|| true`. Job names are unchanged; the
  nine required contexts are unchanged. Both-OS green is CI-only proof.
- **AC2 — Verify seat.** `verify-seat.sh` contains the three `run` lines
  between root `cargo test` and `brokkr compile`;
  `verify_seat_runs_gates_without_login_and_preserves_failure_evidence`
  asserts the gate order `fmt, clippy, test, (evals) fmt, clippy, build,
  test, brokkr, coverage-gate.sh` by execution against the fake cargo;
  `pinned_bundle_shims_and_proof_dispatch_are_digest_bound` carries the
  seat's new digest; `brokkr compile --bundle .brokkr/recipes/self` succeeds;
  the shims manifest check stays green (the seat is not a shim).
- **AC3 — One inventory.** `the_gate_inventory_has_one_home_and_everyone_points_at_it`
  requires the three evals command lines in house rules' Gates section and
  passes; `grep -c evals docs/house-rules.md` is nonzero.
- **AC4 — Lock drift turns red.** Recorded scratch proof: with a shared root
  lock entry bumped, `cd evals && cargo test --locked` fails in
  `tests/boundary.rs::every_evals_package_is_already_in_the_root_lock`.
- **AC5 — MCP shape change turns red.** Recorded scratch proof: with the
  root binary's tools/call text block changed, `cd evals && SWARMAIL_BIN=…
  cargo test --locked --test proof` fails.
- **AC6 — 0055 addendum.** `git diff 8dfdf6b -- docs/decisions/0055-agent-eval-harness.md`
  shows added lines only, all below the 2026-10-30 amendment; the addendum
  is dated 2026-10-31, records acceptance 2026-10-19, the wiring, 0076's
  acceptance, the superseded deferral text and the stdout disposition.
  `every_index_row_matches_its_records_status` and
  `every_decision_file_has_exactly_one_resolving_index_row` pass.
- **AC7 — Specs and README.** `grep -n 'still proposed\|proposed 0055'
  specs/eval-measurement-repairs/spec.md` is empty; `grep -n 'batch 5\|Until
  epic\|until #120\|Until #201' evals/README.md specs/agent-eval-harness/spec.md`
  is empty; `grep -rn fixture-app evals/README.md specs/` is empty and the
  README names the `fixture` subcommand (`evals/src/fixture.rs`);
  `sequential_contracts_and_decision_links_are_honest` passes.
- **AC8 — Stdout residual.** The PR body cites
  `readiness_continues_draining_after_the_first_valid_announcement` and
  `drain()` as the fix and its proof; no eval lifecycle code changes.
- **AC9 — Wall time.** `specs/eval-ci-verify-gates/implementation.md` holds
  the Linux measurements per gate (cold and warm) with the commands used;
  the PR body's table has one row per gate and one column per OS, with CI
  cells sourced from the PR's run and labelled so.
- **AC10 — Gates.** Every house gate in § Gates, locked, plus the three
  evals gates, is green locally and recorded with exit codes; coverage is
  exact 100% with the selftest; no new dependency, no deny change, no push,
  no merge, no decision accepted, commits unsigned.

## Out of scope and honest limits

- No `cargo deny` for `evals/Cargo.lock`; the gap is named, not closed.
- No host lock and no roster assertion in the seat (rulings above).
- No change to evals lifecycle code, admission, scoring or result schema.
- CI job timings on macOS are observable only after the operator pushes;
  this run reports what it can measure and names what it cannot.
