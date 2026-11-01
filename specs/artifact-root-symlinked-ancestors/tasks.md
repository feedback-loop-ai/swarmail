# Ordered tasks — accept artifact roots named through root-owned symlinked ancestors

Each task is checked only with its paired test. Nothing is `complete` while a
box is open. No push, no merge, no decision drafted or accepted, commits
unsigned. `evals/` is outside the root coverage denominator (accepted 0055),
so these tests are a spec obligation, not a gate side-effect: every new
reason string must be produced by a test.

- [ ] **T1 — Baseline the untouched tree.** From `a4f552d`, run the
  quickstart's gate list once and record exit codes, test counts and the
  resolved `SWARMAIL_BIN` path in `implementation.md`; confirm
  `exclusive_destinations_and_symlinked_roots_are_never_reused` passes and
  that `resolve_root` does not yet exist (`git grep resolve_root` is empty).
  **Test:** every gate exits 0 on the baseline; the grep is empty.
- [ ] **T2 — The pure predicate (AC1, AC9).** Add `Ownership` and
  `admit_ancestor` to `evals/src/artifacts.rs` with the check order holder
  writable → target writable → link uid → holder uid → target uid, sticky
  exempting the write-bit test on root-owned directories.
  **Test:** `ancestor_admission_is_a_pure_rule_over_owner_and_mode` (inline)
  — a table over (link, holder, target) producing each of the five reason
  strings exactly once, accepting (0 0755, 0 0755, 0 0755) and
  (0 0777, 0 0755, 0 1777 sticky), refusing (0 0777, 0 0755, 0 0777).
- [ ] **T3 — The resolver (AC1, AC3, AC13).** Add `resolve_root` as the
  read-only checked walk in the plan: `..` first on every component, absent
  components appended, leaf symlink refused, ancestor links admitted by
  `admit_ancestor` then spliced from `read_link`, 40-hop bound, existing
  texts for non-directory and unavailable components.
  **Test:** `symlink_free_roots_resolve_to_themselves_and_create_nothing`
  (inline) — a fresh tempdir path plus two absent components resolves to its
  own components, twice, equal both times, with nothing created; a relative
  `../x` and `<dir>/../x` refuse with `parent path refused`.
- [ ] **T4 — Portable real-filesystem refusals and the root-owned probe
  (AC10, AC11).** In one inline test inside a fresh 0700 tempdir: a same-uid
  link held there → `symlinked ancestor not owned by root` and nothing under
  the target; a link held in a 0770 subdirectory → holder-writable reason; a
  same-uid link to a 0770 directory → target-writable reason; a dangling
  link → `symlinked ancestor target unavailable`; a link to a regular file →
  `symlinked ancestor target not a directory`; a two-hop same-uid chain →
  refuses at the first link; a leaf link → `symlink refused`. Then probe
  `/tmp`, `/var`, `/etc`, `/bin`, `/sbin`, `/lib`, `/lib64` for the first
  symlink and run the AC11 arm, printing which arm ran.
  **Test:** `symlinked_ancestors_resolve_only_when_root_owned_and_unswappable`
  (inline) passes on this Linux host (refusal arms; the probe reports no
  root-owned link or a named refusal) and on CI ubuntu (merged-usr accept arm
  when `/` is root-owned 0755).
- [ ] **T5 — Update the existing test (AC2, AC12).** Extend
  `exclusive_destinations_and_symlinked_roots_are_never_reused`: assert
  `resolve_root(link/must-not-create)` fails with
  `symlinked ancestor not owned by root`; keep the `ensure_root` failure,
  the `must-not-create` absence and the `../metadata.json` refusal.
  Rewrite the `artifact_root` helper comment (keep `canonicalize`).
  **Test:** the test passes; `every_publication_stage_refuses_without_a_completion_marker`
  and `artifact_leaf_refuses_existing_directories_files_and_symlinks` pass
  unchanged.
- [ ] **T6 — Site 1, `trial --artifacts-dir` (AC5, AC6).** In
  `runner::finalize`, resolve before `retain_trial` and pass the resolved
  path; keep the harness-error row and `artifact_write_failed`.
  **Test:** `evals/tests/runner.rs::artifact_write_failures_use_safe_categories`
  passes unchanged; new portable CLI test
  `a_same_uid_symlinked_artifact_ancestor_is_a_safe_harness_error` — `trial
  --task signup --agent oracle --artifacts-dir <tempdir>/link/out` where
  `link` is a same-uid symlink: outcome `harness_error`, stderr contains
  `artifact_write_failed` and not the path, nothing created under the
  target.
- [ ] **T7 — Sites 3 and 4, `check --artifacts-dir` / `--matrix` (AC5,
  AC6).** Resolve before `artifacts::trial` / `artifacts::matrix`; keep
  `artifact_evidence_invalid`.
  **Test:** existing `check` replay tests pass unchanged; the macOS CLI test
  of T10 exercises site 3 on CI; a portable CLI assertion in T6's test runs
  `check --artifacts-dir <tempdir>/link/<id>` and gets exit 2 with
  `artifact_evidence_invalid`.
- [ ] **T8 — Site 2, `MatrixEvidence::start` (AC5, AC6).** Bind
  `resolve_root(root)?` before `ensure_root`; keep
  `artifact_publication_failed`.
  **Test:** `main.rs` inline test — `MatrixEvidence::start` on
  `<tempdir>/link/root` (same-uid link) returns `Err` and creates nothing
  under the target; on the real tempdir path it succeeds as before.
- [ ] **T9 — Site 5, `baseline_leaf` (AC5, AC6).** Resolve the parent
  before `safe_root`, create the leaf under the resolved parent, return the
  real leaf; keep `baseline refused`, the exclusive 0700 create and the
  `baseline_output_*` reasons.
  **Test:** `main.rs` inline test — `baseline_leaf(<tempdir>/link/out)` with
  a same-uid `link` returns `Err` and creates nothing; on the real parent it
  returns `<parent>/out` at 0700 exactly as before; the existing
  `evals/tests/baseline.rs` tests pass unchanged.
- [ ] **T10 — macOS proofs (AC7, AC8).** Add the `#[cfg(target_os =
  "macos")]` unit test in `artifacts.rs` (resolve `/tmp/<unique>` to
  `/private/tmp/<unique>`, `retain_trial`, same `(dev, ino)` through both
  spellings, observed path in the failure message) and the macOS-gated CLI
  test in `evals/tests/runner.rs` (`trial` then `check` through
  `/tmp/<unique>`, oracle agent, metadata under `/private/tmp`).
  **Test:** both compile on Linux (cfg-gated out) and pass in CI's
  `test (macos-latest)` job; `implementation.md` records that the Linux
  verify seat cannot run them and cites the CI job as their proof.
- [ ] **T11 — Primitive and boundary audit (AC4, AC5).** Run the AC4 grep
  over `git diff main -- evals/src` and the AC5 grep.
  **Test:** the AC4 grep is empty; the AC5 grep shows `resolve_root` called
  exactly at the five sites and defined once, with no new `safe_root` or
  `ensure_root` caller on an operator path; recorded in `implementation.md`.
- [ ] **T12 — Docs (AC14).** Rewrite `evals/README.md:113-116` to the rule
  (including "name a per-user root that does not yet exist"), reword
  `evals/run-plan.md:84`, rewrite the `evals/tests/common/mod.rs:29-32`
  comment; leave `SHARED_TMP` and its comment alone.
  **Test:** `git grep -n "private/tmp" evals/` matches only the `SHARED_TMP`
  lines and the README rule sentence; `git grep -n "non-symlink parent"
  evals/` is empty; no `docs/decisions/` file in `git diff --stat`.
- [ ] **T13 — Gates (AC15, AC16).** Run the full quickstart list, `--locked`,
  with the evals tests against the candidate's own `swarmail`.
  **Test:** every command exits 0 and is recorded with its exit code in
  `implementation.md`; `git diff --stat main -- Cargo.toml Cargo.lock
  evals/Cargo.toml evals/Cargo.lock deny.toml` is empty; any tool that is
  missing is named as not run, never claimed.
- [ ] **T14 — Delivery paper (AC17).** Commit unsigned in the repository's
  style (imperative subject, a body that says why). The PR description
  states: no accepted decision states the old rule so no addendum is
  drafted; `benches/mail_compare.rs` `--output` is pre-existing and
  unvalidated; the two residuals (pre-existing root under a shared sticky
  parent; link-route asymmetry) are named for the operator; the macOS tests
  are proven by CI only.
  **Test:** `git log -1 --format=%G?` is `N`; `git status --porcelain` is
  empty; nothing was pushed (`git log @{u}..` is not consulted — there is no
  upstream for the branch).
