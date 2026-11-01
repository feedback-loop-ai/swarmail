# Implementation record — artifact roots through root-owned symlinked ancestors

Implements [spec.md](spec.md) on `repair/artifact-root-symlinks` from `a4f552d`.

## What changed

- `evals/src/artifacts.rs`: `Ownership`, `admit_ancestor` (pure predicate),
  `resolve_root` (read-only checked walk) and its injectable `resolve_with`.
  `ensure_root`, `safe_root`, `read_file`, `publish`, `create_private_dir`
  and `Admission::at` have no changed line.
- Five call sites, each resolving once and using only the returned path:
  `runner::finalize` (`trial --artifacts-dir`), `MatrixEvidence::start`
  (`matrix --artifacts-dir`, baseline retention), `check --artifacts-dir`,
  `check --matrix`, `baseline_leaf` (`baseline --output` parent).
- Docs: `evals/README.md` states the rule; `evals/run-plan.md` and the
  `evals/tests/common/mod.rs` and `artifact_root` comments reworded.

## Deviations and findings

- `resolve_root` refuses a `..` anywhere in the supplied path before judging
  any link, so `<link>/../x` reports `parent path refused` (spec AC3) rather
  than the link's ownership. `..` inside a link target refuses when spliced.
- `resolve_with` takes the admission predicate as a parameter. Unprivileged
  tests cannot own a root-owned link, so the splice semantics (relative and
  absolute targets, a link inside a target, `..` in a target, the 40-expansion
  bound, 40 accepted / 41 refused) are proved with a permissive predicate, and
  the real predicate is proved by the table test.
- `tempfile::tempdir()` follows the umask (0775 here), which made a link held
  in it refuse for the holder's mode first. The inline `artifact_root` helper
  and the portable CLI test now chmod the tempdir 0700.
- `check` prints only `operation_failed` on every refusal (`fail()` hides the
  message), so the CLI test proves the `check` boundary by contrast: a trial
  retained under the real path replays (exit 0); the same trial through a
  same-uid link exits 2.
- No accepted decision states the old rule (`git grep -i symlink docs/decisions`
  names none for artifact roots), so no addendum is drafted and nothing is
  accepted. `benches/mail_compare.rs`'s `--output` stays unvalidated
  (pre-existing).

## Gates (all `--locked`, this Linux host)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` (root, evals) | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings`; evals clippy | exit 0 |
| `cargo test --workspace --locked` | all ok |
| `cargo build --locked --bin swarmail` | ok; `SWARMAIL_BIN=target/debug/swarmail` |
| `cd evals && cargo test --locked` | all ok (lib 58, bin 20, runner 38, …) |
| `bash scripts/coverage-gate.sh` / `selftest` | pass, exact 100/100/100 / pass |
| `bash scripts/publish-guard-test.sh` | OK |
| `bash scripts/seal-branch-test.sh` | 7 cases and 10 mutants passed |
| `cargo deny check licenses` | ok |
| `brokkr compile --bundle .brokkr/recipes/self` | **not run**: no `brokkr` binary on this host (the pinned artifact is not provisioned and nothing may be installed). Nothing under `.brokkr/` changed; the verify seat and CI's `brokkr bundle (pinned)` job run it. |

`git diff a4f552d` shows no change to `Cargo.toml`, `Cargo.lock`,
`evals/Cargo.toml`, `evals/Cargo.lock` or `deny.toml`, and no line touching
`O_NOFOLLOW`, `create_new`, `DirBuilder`, `ARTIFACT_*` in `evals/src`.

## Only CI proves

The macOS-gated tests (`macos_tmp_resolves_through_its_root_owned_link_and_retains_there`
in `artifacts.rs`, `macos_tmp_artifact_roots_are_accepted_and_resolve_under_private_tmp`
in `evals/tests/runner.rs`) are cfg-gated out on Linux; CI's `test (macos-latest)`
job is their proof. On this host no symlinked ancestor exists among the probed
roots, and the portable test printed that it relies on the predicate table and
the macOS tests for acceptance.
