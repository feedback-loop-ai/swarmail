# Implementation plan — gate the eval harness in CI and the verify seat

Implements [spec.md](spec.md). Compatible work under accepted 0045 §8 and
accepted 0055: every accepted ruling is preserved; nothing waits on
acceptance and nothing is accepted. The chief commits spec.md, plan.md and
tasks.md only.

## Technical context

One Rust crate at the root plus the standalone `evals/` workspace (its own
`Cargo.lock`, a root-lock subset plus the two 0055 named dev-dependency
exceptions). Gates are a GitHub Actions workflow, the pinned bash verify seat
in `.brokkr/recipes/self/scripts/`, and Markdown contracts proven by root
tests. No production Rust changes, no new dependency, no new file outside
`specs/eval-ci-verify-gates/`. `jq` is the only non-cargo tool the resolution
uses; the seat already requires it and both GitHub runners ship it.
`.specify/templates/` holds no realm template; this plan supplies context,
constitution check, structure, research, contracts, quickstart and risks.

## Constitution check

- 0001–0005: untouched; nothing under `src/` changes.
- 0020: no new test harness file; the delivery-constitution fixture and the
  docs tests are extended in place; no sleep as synchronization.
- 0045 §8: compatible work under an accepted decision; 0055 changes by dated
  addendum only; no body line edited.
- 0050 §1: the verify seat is pinned-bundle bytes; editing it re-pins its
  digest in `tests/delivery_constitution.rs` and needs `brokkr compile`.
  The public shims (`scripts/*.sh`) are not touched, so `shims.sha256` is
  unchanged.
- 0056: commit unsigned, never push.
- 0057/0099: no route, model or recipe change; the nine required contexts
  (2026-10-24 observation) are unchanged because no job is added.
- 0076: trials stay sequential; the suite's admission contract is unchanged.
- Dependencies: `deny.toml`, root `Cargo.toml`/`Cargo.lock`,
  `evals/Cargo.toml`/`Cargo.lock` unchanged except one comment line.

## Research and chosen approach

**Binary resolution.** Cargo's JSON message stream is the one authoritative
record of what it built; the harness's own `build_swarmail` reads the same
`compiler-artifact` records. The gate uses it from the repository root:

```bash
SWARMAIL_BIN="$(cargo build --locked --bin swarmail --message-format=json \
  | jq -r 'select(.reason == "compiler-artifact" and .target.name == "swarmail"
                  and (.target.kind | index("bin"))) | .executable')"
test -x "$SWARMAIL_BIN"
```

After `cargo test --workspace --locked` this is a no-op or a relink of the
same source under the same lock. Setting the variable explicitly also
overrides any inherited value, so the harness's fallback build never runs in
a gate. In the seat the `jq` filter is lifted into a shell variable to keep
the `run` line readable; the implementer may do the same in CI.

**Where the steps sit.** Inside the `test` matrix job so both OSes run all
three gates and lint both `cfg(target_os)` arms, after the root suite (so the
build is warm) and before the port-80 steps (macOS's runs as root and must
stay last). Named steps, because GitHub's per-step durations are the per-OS
wall-time measurement the task requires.

**The seat.** Three `run` lines after root `cargo test`, before `brokkr
compile`; each `run` already exits the seat with verbatim evidence on a
nonzero step. Offline is already exported. The header comment names
`evals/Cargo.toml` as the second manifest the host's `cargo fetch --locked`
must have covered.

## Project structure (files touched)

| File | Change |
|---|---|
| `.github/workflows/ci.yml` | `test` job: toolchain `components: rustfmt, clippy`; cache `workspaces: . -> target` and `evals -> target`; four named steps (resolve `SWARMAIL_BIN` → `$GITHUB_ENV` with `test -x` and `sha256sum`; evals fmt; evals clippy; evals test), each eval step `working-directory: evals`; a comment naming #201 and the no-fallback rule. Timeout stays 30. |
| `.brokkr/recipes/self/scripts/verify-seat.sh` | Header sentence on the evals fetch; three `run` lines; final pass note names the evals gates. |
| `tests/delivery_constitution.rs` | Re-pin the seat digest; fake `cargo` gains a `build` branch emitting one artifact record naming an executable fixture file (the fixture creates `candidate/evals/`); gate-order expectation gains `cargo:fmt --all -- --check`, `cargo:clippy --all-targets --locked -- -D warnings`, `cargo:build --locked --bin swarmail --message-format=json`, `cargo:test --locked`; `verify_runs_the_pinned_gates_and_checks_the_public_shims` and the CI-shape test gain the evals needles. |
| `tests/store_contract_docs.rs` | `the_gate_inventory_has_one_home_and_everyone_points_at_it` gains the three evals command needles. |
| `docs/house-rules.md` | § Gates: evals lines in the gate block plus one sentence (standalone workspace, both OSes, candidate binary, deny gap named). |
| `docs/decisions/0055-agent-eval-harness.md` | Appended dated addendum 2026-10-31 only. |
| `docs/decisions/README.md` | 0055 row gains the amendment clause; status unchanged. |
| `specs/agent-eval-harness/spec.md` | Dated 2026-10-31 clarification: AC11/AC12 deferral and the 2026-10-24 ownership paragraph brought current; `#201` kept. |
| `specs/eval-measurement-repairs/spec.md` | "still proposed" / "proposed 0055" → accepted, as a dated note. |
| `evals/README.md` | Gates paragraph rewritten; ownership sentences to past tense; #149 path corrected; `#201` kept. |
| `evals/run-plan.md` | Unchanged unless a sentence reads as future-tense deferral; `#201` kept. |
| `evals/Cargo.toml` | One comment line: the gates now run in CI and verify. |
| `specs/eval-ci-verify-gates/implementation.md` | Measurements, red-proof transcripts, gate receipts, PR text draft (new, implement seat). |

## Contracts

- CI step names: `Resolve the candidate Swarmail binary`, `evals fmt`,
  `evals clippy`, `evals test`.
- Seat labels: `evals cargo fmt`, `evals cargo clippy`, `evals cargo test`.
- House-rules gate block additions (the inventory test's needles):
  `cd evals && cargo fmt --all -- --check`,
  `cd evals && cargo clippy --all-targets --locked -- -D warnings`,
  `cd evals && SWARMAIL_BIN=<the candidate's swarmail> cargo test --locked`,
  preceded by `cargo build --locked --bin swarmail`.
- 0055 addendum heading: `## Dated addendum — 2026-10-31, CI and verify
  wiring (#201)`.

## Quickstart (local reproduction)

```bash
cargo test --workspace --locked
bin="$(cargo build --locked --bin swarmail --message-format=json \
  | jq -r 'select(.reason=="compiler-artifact" and .target.name=="swarmail" and (.target.kind|index("bin"))) | .executable')"
test -x "$bin"
(cd evals && cargo fmt --all -- --check)
(cd evals && cargo clippy --all-targets --locked -- -D warnings)
(cd evals && SWARMAIL_BIN="$bin" cargo test --locked)
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh && bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh && bash scripts/seal-branch-test.sh
cargo deny check licenses
sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256
```

Measure with `time` around each evals command twice: once after `rm -rf
evals/target` (cold), once warm.

## Risks

| Risk | Mitigation |
|---|---|
| macOS `test` job approaches the 30-minute cap with a cold `evals/target`. | Cache `evals -> target`; named steps expose the delta; the PR names the operator follow-up (raise to 45 with the measurement) if the first run shows the job above ~21 minutes. Never a new job. |
| The offline verify seat cannot resolve `evals/Cargo.lock` (host cache lacks `pulldown-cmark 0.13.6`/`unicase 2.10.0`). | Implement seat runs `cargo fetch --locked --offline --manifest-path evals/Cargo.toml` and records the result; the seat header names the second manifest; a miss is a one-line "no matching package", not a silent pass. |
| Concurrent delivery runs share `/tmp/swarmail-evals-<uid>` admission and red a sibling's seat. | Ruled out of scope without evidence; the refusal is the named `concurrent_trials_refused`, visible verbatim in verify notes; rerun is the remedy. Recorded in the addendum. |
| `evals/build.rs` Git facts under a shallow, detached CI checkout make a provenance test refuse. | Implement seat reproduces with a detached, depth-1 clone locally before declaring done; if it refuses, the fix is `fetch-depth` on the test job's checkout, never a skip. |
| `jq` absent on a runner or filter matches zero or many records. | `test -x "$SWARMAIL_BIN"` fails closed on empty or multi-line output; both GitHub runner images ship `jq`; the seat already refuses without it. |
| Re-pinning the seat digest and the gate-order fixture drift. | The gate-order test proves the sequence by execution; the digest test names the file that changed. |
| Doc edits break `sequential_contracts_and_decision_links_are_honest` (`#201` token, links) or the inventory test. | Every edited file keeps a literal `#201`; links are checked by the test; run the docs tests before commit. |
| Rewriting 0055's body by accident. | AC6 requires `git diff` to show added lines only; the reviewer checks it. |
| This run's pinned seat cannot run the new lines. | Recorded as a limit; the hand run and the gate-order test are the evidence until the next run's seat carries the pin. |
