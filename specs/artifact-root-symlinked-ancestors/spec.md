# Accept artifact roots named through root-owned symlinked ancestors

Date: 2026-11-01. Status: design draft. Operator ruling of 2026-11-01 (the
#201 review raised it; the operator chose to relax the rule rather than
document around it). Zero-Codex `claude-careful` lane (decisions 0098, 0099;
review residuals rework under accepted 0102 and its spec-defect addendum).
Base: `main` at `a4f552d`.

## Feature and rationale

`evals/src/artifacts.rs` refuses every symlinked component of an artifact
root: `ensure_root` and `safe_root` walk the supplied path with
`symlink_metadata` and refuse the first link they meet. On macOS `/tmp`,
`/var` (so the per-user `TMPDIR` under `/var/folders`) and `/etc` are
symlinks into `/private`, so `--artifacts-dir /tmp/evals` refuses, and
`evals/README.md` tells operators to type `/private/tmp/evals` instead.

The ruling: an artifact or output root may be named through symlinked
**ancestor** components when those links cannot be swapped by any non-root
user. The harness resolves the supplied name **once**, to its real path, and
uses only the real path from then on. Everything at or below the real root
keeps today's rule: the root's own leaf and everything the harness creates
or reads under it still refuse symlinks; `O_NOFOLLOW`, exclusive creation
and the 0700/0600 modes are untouched; `..` components still refuse. A
symlinked ancestor owned by anyone other than root, or one a non-root user
could replace, still refuses with a named reason. Linux behaviour for paths
without symlinks is unchanged.

The change is confined to `evals/` (unpublished, outside the root coverage
denominator — accepted 0055) plus the eval docs and this spec. No file under
`src/`, `.github/`, `scripts/`, `.brokkr/`, no lockfile and no `deny.toml`
changes.

## Authority and scope

No accepted decision states the old rule. `git grep -i symlink docs/decisions`
finds symlink text only in 0033, 0042 and 0047 (server data files and
identity sidecars), 0057 (the brokkr recipe symlink) and 0099 (the brokkr
box); 0055, 0063, 0071, 0075 and 0076 never mention artifact-root symlinks.
The old rule is stated only in `evals/README.md:113-116`,
`evals/run-plan.md:84`, and two comments (`evals/src/artifacts.rs:493-495`,
`evals/tests/common/mod.rs:29-32`). The task's addendum clause ("if an
accepted decision states the old rule") therefore does not fire: **no
decision is declared or drafted by this spec, no addendum is written, and
the PR description must say so in those words** rather than leave it
implied. The operator's ruling, quoted in the task, is the directive for the
code change; this spec records it. The work is compatible under accepted
0045 §8: it contradicts no accepted ruling and accepts nothing.

Frozen guarantees 0001–0003, and 0004/0005, are not in this diff: nothing
under `src/` changes. The artifact analogue the design must preserve is
0075's "metadata.json is published last and binds the bytes": no new path by
which that marker could be substituted is opened by this change (see
*Residuals*).

## Panel synthesis — rulings

Both positions back the relaxation and agree that the strict functions below
the root do not move, that `..` refuses before anything is created, that
`canonicalize` is not used (it follows chained links unchecked), that the
real path is bound and used rather than re-derived, and that no dependency,
flag, exit code, stderr text or schema field is added. Where they differ:

| Question | Ruling | What the losing side costs |
|---|---|---|
| Which object must be "owned by root and not writable by group or other" | **Both the directory holding the link entry and the directory the link resolves to**, plus the link node's own uid. Simplicity checked only the holder; the intake and robustness checked only the target. The holder check is what makes the operator's own rationale true (write permission on the containing directory, not ownership of the link node, governs unlink-and-recreate — robustness B1). The target check is the operator's literal sentence and the task's named test ("a group-writable symlink target directory refuses"). | Simplicity loses symmetry: `/srv/data -> /home/shared` (0775 group) refuses through the link though `/home/shared/evals` is accepted directly. Named in *Residuals*; the operator asked for the test. |
| Mode test on a root-owned sticky directory | **Exempt.** `/private/tmp` is root-owned 1777; a literal write-bit test refuses the exact case the ruling exists for. In a root-owned sticky directory no non-root user can unlink or rename another owner's entry, which is the property the write-bit test stands for. One predicate (*anchored*, below) applies to holder and target alike. Flagged for the security review by name. | None of the operator's examples needs the exemption on the holder (`/` is 0755); it is kept there for one predicate rather than two. |
| Where resolution lives | **One new read-only function `resolve_root` in `evals/src/artifacts.rs`, called once at each operator boundary** (simplicity). `ensure_root`, `safe_root`, `read_file`, `publish`, `retain_trial`, `trial`, `matrix`, `create_private_dir` and `Admission::at` keep their signatures and bodies. Robustness's `ensure_root -> Result<PathBuf>` is rejected: it ripples through every internal caller and would let a trusted link planted *below* a root be followed. | Robustness's B6 concern (the unresolved string being used afterwards) is answered by AC6 instead: each boundary binds the returned path and the unresolved one is not used again. |
| Pre-existing root owner/mode check under sticky or world-writable ancestors (robustness R3) | **Rejected for this change.** It changes behaviour for symlink-free paths (`/tmp/evals` on Linux, `/private/tmp/evals` on macOS — the README's current advice), which the ruling forbids, and the exposure it names exists today under exactly that advice. It is a pre-existing residual, named below for the operator, not widened here. | The honest cost: a root another local user pre-created under sticky `/tmp` is still accepted, as today. The README tells operators to name a per-user root that does not yet exist. |
| Anchor every real component of the canonical result (robustness R1 part 5) | **Rejected.** The harness has never checked ownership of real ancestors (`$HOME/evals` is accepted); doing so only on the symlink route makes one directory behave differently by spelling, and the hidden-ancestor rename it would close is already possible against real paths today. | A root-owned link inside a root-owned, non-writable directory that itself sits inside a user-writable directory can be redirected by that user — but so can the real path. Named in the plan's *Risks*. |
| `..` ordering (robustness R2) | **Adopted**, and already satisfied: the walk refuses `..` as the first check on every component, including components spliced in from a link target, and `resolve_root` creates nothing, so no refusal is preceded by a filesystem change. | None. |
| Non-UTF-8 roots refuse; `/` refuses (robustness R4 extras) | **Rejected.** Nothing prints, logs or persists a root path (0075 privacy posture; `PathBuf` carries any bytes), so there is no lossy record to protect; both would be new refusals of symlink-free paths. | None in this change. |
| Doc inventory (robustness R5) | **Adopted**: every occurrence is enumerated by grep below; `evals/tests/common/mod.rs` and `evals/run-plan.md` are added to the intake's list. `runner.rs`'s `SHARED_TMP` comment describes private runtime directories, a different rule that does not change, and stays. | None. |
| macOS proof | **Both** a `#[cfg(target_os = "macos")]` unit test in `artifacts.rs` (the intake's AC7) and one macOS-gated CLI test in `evals/tests/runner.rs` for `trial` then `check` (simplicity). Only the CI macOS test job can prove them; the Linux verify seat cannot. | One more macOS-only test than either position asked for; it is the only wire-level proof the ruling's own example gets. |
| Inline `artifact_root` test helper | Keep `canonicalize`; rewrite its comment. Switching it to the resolver (simplicity) would fail every inline test on a developer host whose `TMPDIR` passes through a user-owned link, for no gain the dedicated tests do not already give. | None. |

## The rule (normative)

**Definitions.** The *supplied root* is the path the operator named (or a path
derived from one before any resolution). Walking its components in order, a
*symlinked ancestor* is any component other than the last whose
`symlink_metadata` reports a symbolic link; the last component is the root's
*leaf*. For a symlinked ancestor, its *holder* is the real directory that
contains the link entry (the path resolved so far, or `.` when nothing has
been resolved yet), and its *target* is the directory the link resolves to
(`std::fs::metadata` on the link path, which follows).

A directory is **anchored** when its owner uid is 0 and either its mode has
no group or other write bit (`mode & 0o022 == 0`) or its sticky bit is set
(`mode & 0o1000 != 0`).

**Admission of a symlinked ancestor.** All of:

1. its holder is anchored;
2. its target exists and is a directory, and is anchored;
3. the link node itself is owned by uid 0.

The mode bits of the link node are never consulted (Linux reports every
symlink as 0777 and ignores them). Checks run in the order *holder
writable → target available and a directory → target writable → link uid →
holder uid → target uid*, so that every reason an unprivileged test can
produce on Linux is reachable before the ownership reasons.

**Resolution** (`resolve_root(&Path) -> Result<PathBuf, String>`, read-only;
it never creates, opens for write or changes anything):

- components are taken in order from a work queue; a `..` component, whether
  from the supplied path or spliced in from a link target, refuses with
  `parent path refused`;
- a component that does not exist is appended to the real path and the walk
  continues (`ensure_root` creates it afterwards, under the real path);
- a component that exists and is a real directory is appended;
- a component that exists and is neither a directory nor a symlink refuses
  with `root component is not a regular directory`;
- an unreadable component refuses with `root unavailable`;
- a symlink in the last position (the leaf) refuses with `symlink refused`;
- a symlinked ancestor is admitted by the rule above, then its `read_link`
  target's components are placed at the front of the queue in place of the
  link (a relative target resolves against the holder; an absolute target's
  root component resets the real path), so every link inside a target is
  checked by the same rule; more than 40 link expansions in one resolution
  refuses with `symlink loop refused`;
- a path with no symlinks resolves to its own components joined back
  together.

**Named reasons** (function-level `Err(String)` values, asserted by tests,
never echoed on stderr or stdout — the CLI's categories are unchanged):

| Reason | When |
|---|---|
| `parent path refused` | a `..` component (existing text) |
| `symlink refused` | the leaf is a symlink (existing text) |
| `symlinked ancestor holder writable by group or other` | holder mode has a group/other write bit and no sticky bit |
| `symlinked ancestor target unavailable` | `metadata` or `read_link` on the link fails (dangling, unreadable) |
| `symlinked ancestor target not a directory` | the target is a file, device or FIFO |
| `symlinked ancestor target writable by group or other` | target mode has a group/other write bit and no sticky bit |
| `symlinked ancestor not owned by root` | link node uid ≠ 0 |
| `symlinked ancestor holder not owned by root` | holder uid ≠ 0 |
| `symlinked ancestor target not owned by root` | target uid ≠ 0 |
| `symlink loop refused` | more than 40 expansions |
| `root component is not a regular directory`, `root unavailable` | as today's `ensure_root` |

**Unchanged below the root.** `ensure_root` and `safe_root` keep refusing
every symlink they meet, so a link swapped in after resolution is refused,
never followed. `read_file`'s `O_NOFOLLOW`, `create_private_artifact_dir`'s
exclusive 0700 create, `write_artifact_with`'s exclusive 0600 publication,
`baseline_leaf`'s exclusive 0700 create and `baseline_stream`'s
`create_new` are byte-for-byte unchanged. `Admission::at`
(`evals/src/admission.rs`) creates and `O_NOFOLLOW`-opens a leaf it owns
under an already-resolved root; it is not a validation site and does not
change.

## Recording the real path

"Records and uses the real path" means: each boundary binds the `PathBuf`
`resolve_root` returns and performs every later operation of that invocation
through it; the supplied path is not used again. No stdout line (both
`trial` and `matrix` emit a JSON row contract), no stderr text (0075: no
paths are echoed) and no schema field (the artifact types are
`deny_unknown_fields`; a field is a schema change) is added. The baseline
`--dry-run` plan keeps echoing the operator's own `output` spelling: it runs
before any filesystem access by contract and is an echo of the invocation,
not a record of a created root.

## Call sites

Every place the crate validates an artifact or output root, and what changes:

| # | Site | Today | Change |
|---|---|---|---|
| 1 | `trial --artifacts-dir`: `runner::finalize`, `evals/src/runner.rs:2658`, the only use of `req.artifacts_dir` | `retain_trial(root, …)` | `resolve_root(root).and_then(|r| retain_trial(&r, …))`; a refusal is still a harness-error row with `artifact_write_failed`. Matrix workers reach this with the matrix's already-real `trials` path. |
| 2 | `matrix --artifacts-dir` and baseline retention: `MatrixEvidence::start`, `evals/src/main.rs:757` | `ensure_root(root)` | `let root = &resolve_root(root)?;` before `ensure_root`; a refusal is still `artifact_publication_failed`. |
| 3 | `check --artifacts-dir`, `evals/src/main.rs:375` | `artifacts::trial(&root)` | `resolve_root(&root).and_then(|r| artifacts::trial(&r))`; a refusal is still `artifact_evidence_invalid`. |
| 4 | `check --matrix`, `evals/src/main.rs:376` | `artifacts::matrix(&root)` | the same shape with `artifacts::matrix`. |
| 5 | `baseline --output` parent: `baseline_leaf`, `evals/src/main.rs:2002` | `safe_root(parent)` then `parent.join(name)` | `let parent = resolve_root(parent)?;` then `safe_root(&parent)` and `parent.join(name)`; the returned leaf is therefore real, and `root.join("controls")` / `root.join("paid")` reach site 2 already real. A refusal is still `baseline refused`. |

Not call sites, and why: the internal callers of `ensure_root` and
`safe_root` (`retain_with`, `read_file`, `publish`, `matrix`,
`create_private_dir`, `metadata_binding`) receive paths already real;
`create_private_artifact_dir` has no validation of its own and must stay
reachable only below `ensure_root`/`safe_root`; `Admission::at` validates
a leaf it creates; `wrappers.rs` private config and credential parents use
their own descriptor walk and are not artifact or output roots;
`--state` is a file and `--agents` is an input; the root crate's
`benches/mail_compare.rs:339` checks `--output` only with `exists()` and
calls no root validation — pre-existing, out of scope, named in the PR.

## Documentation

- `evals/README.md:113-116`: drop the `/private/tmp/evals` workaround and
  state the rule: an artifact or output root may be named through symlinked
  ancestors only when the link, the directory holding it and the directory
  it resolves to are root-owned and not writable by group or other (a
  root-owned sticky directory such as `/private/tmp` qualifies); macOS
  `/tmp`, `/var` and `/etc` qualify; the harness resolves the name once and
  uses the real path; the root itself, everything under it, `..` and any
  other symlink still refuse; name a per-user root that does not yet exist.
- `evals/run-plan.md:84`: "existing non-symlink parent" becomes "existing
  parent (symlinked ancestors only under the README's root-owned rule)".
- `evals/src/artifacts.rs:493-495` and `evals/tests/common/mod.rs:29-32`
  comments: rewritten to say why the helpers still name the real path
  (the tests name real paths directly so the helper does not depend on the
  rule under test; private paths still refuse every link).
- `evals/src/runner.rs:547-555` (`SHARED_TMP`) is unchanged: private
  runtime directories keep refusing every link and the constant is already
  `cfg(target_os)`-gated.
- Earlier `specs/` records (eval-privacy, eval-provenance, …) are the
  history of what the code was built to and are not revised.
- No release note: `evals/` is unpublished.

## Acceptance Criteria

- **AC1 Rule.** `resolve_root` admits a symlinked ancestor only when its
  holder is anchored, its target is an existing anchored directory and the
  link node is owned by uid 0, where anchored means uid 0 and (`mode &
  0o022 == 0` or sticky). It returns the real path with every admitted
  link replaced by its target. Any other symlinked ancestor refuses with
  exactly one of the named reasons in the table above.
- **AC2 Below the root, unchanged.** The root's leaf and everything created
  or read under it still refuse symlinks: `ensure_root`, `safe_root`,
  `read_file`, `publish`, `create_private_dir` and `Admission::at` have no
  changed line; `artifact_leaf_refuses_existing_directories_files_and_symlinks`,
  `every_publication_stage_refuses_without_a_completion_marker` and the
  leaf case of `exclusive_destinations_and_symlinked_roots_are_never_reused`
  pass unchanged.
- **AC3 `..` refuses before anything is created.** A `..` in the supplied
  path or in a link target refuses with `parent path refused`;
  `resolve_root` performs no create, write or chmod on any path, so a test
  for `<link>/../x` and for a relative `../x` sees nothing created.
- **AC4 Primitives untouched.** `git diff` shows no changed line containing
  `O_NOFOLLOW`, `create_new`, `DirBuilder`, `mode(0o700)`, `0o600`,
  `ARTIFACT_DIRECTORY_MODE` or `ARTIFACT_FILE_MODE` in `evals/src/`.
- **AC5 One rule at every boundary.** The five call sites route through
  `resolve_root` exactly once each;
  `git grep -n "resolve_root\|safe_root\|ensure_root" evals/src` shows no
  other caller of `resolve_root` and no new caller of the strict functions
  on an unresolved operator path.
- **AC6 Resolve once, use the real path.** At each site the returned path is
  bound and used for every later operation of that invocation; the
  supplied path is not passed to any filesystem call afterwards. On a
  symlink-free path the returned path has the same components as the
  input, so Linux behaviour is unchanged for paths without symlinks.
- **AC7 macOS unit proof** (`#[cfg(target_os = "macos")]`, in
  `artifacts.rs` tests): a fresh `/tmp/<unique>` resolves to
  `/private/tmp/<unique>`; `retain_trial` through the resolved root
  publishes `/private/tmp/<unique>/<trial_id>/metadata.json`, and the file
  seen through `/tmp/<unique>/…` has the same `(dev, ino)`; the failure
  message carries the observed resolved path. Only the CI macOS test job
  proves this; the Linux verify seat cannot.
- **AC8 macOS CLI proof** (`#[cfg(target_os = "macos")]`, in
  `evals/tests/runner.rs`): `trial --task signup --agent oracle
  --artifacts-dir /tmp/<unique>` exits 0 with no `artifact_write_failed`
  on stderr and `/private/tmp/<unique>/<trial_id>/metadata.json` exists;
  `check --artifacts-dir /tmp/<unique>/<trial_id>` exits 0. No paid trial.
- **AC9 Portable predicate table** (Linux and macOS): the admission
  predicate is a pure function over the (uid, mode) of link, holder and
  target, and a table test produces every ownership and mode reason
  (holder writable, target writable, link not root, holder not root,
  target not root) and both accept arms (0755 and 1777 sticky targets),
  plus refusal of a root-owned non-sticky 0777 target. This is the only
  portable proof of the root-owned accept branch.
- **AC10 Portable real-filesystem refusals** (Linux and macOS, unprivileged,
  inside a fresh tempdir): a same-uid symlinked ancestor held in a 0700
  directory refuses with `symlinked ancestor not owned by root` and
  nothing is created under its target; a link held in a 0770 directory
  refuses with the holder-writable reason; a same-uid link to a 0770
  directory refuses with the target-writable reason; a dangling link
  refuses with `target unavailable`; a link to a regular file refuses with
  `target not a directory`; a symlinked leaf refuses with `symlink
  refused`; `..` refuses with `parent path refused`.
- **AC11 Root-owned acceptance on the real filesystem when available.** The
  same test probes `/tmp`, `/var`, `/etc`, `/bin`, `/sbin`, `/lib`,
  `/lib64`; for the first that is a symlink it calls
  `resolve_root(<link>/<unique>)`: if `/` and the target are anchored and
  the link is root-owned the result equals
  `canonicalize(<link>).join(<unique>)` and nothing is created; otherwise
  the refusal is one of the named ancestor reasons. When no candidate is a
  symlink the test prints that no root-owned symlinked ancestor exists on
  the host and that acceptance is proven by AC7 and AC9; it never passes
  silently without printing which arm ran.
- **AC12 Updated test.** `exclusive_destinations_and_symlinked_roots_are_never_reused`
  additionally asserts `resolve_root(link/must-not-create)` fails with
  `symlinked ancestor not owned by root`, keeps asserting
  `ensure_root(link/must-not-create)` fails, that `must-not-create` is not
  created, and that the `../metadata.json` read refuses.
- **AC13 Determinism and race shape.** Two sequential `resolve_root` calls
  on the same absent path return equal paths; no sleep, retry loop,
  global cache or wall-clock input is added (0020 §5).
- **AC14 Docs.** The README, run-plan and the two comments read as in
  *Documentation*; `git grep -n "private/tmp" evals/` matches only
  `runner.rs`'s `SHARED_TMP` lines and the README's rule sentence (as the
  example of a root-owned sticky directory, not as a workaround). No
  accepted decision body is edited; no decision file is added or changed.
- **AC15 Gates, all `--locked`, all clean:** `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  `cargo test --workspace --locked`; `cargo build --locked --bin swarmail`;
  in `evals/`: `cargo fmt --all -- --check`, `cargo clippy --all-targets
  --locked -- -D warnings`, `SWARMAIL_BIN=<the candidate's swarmail> cargo
  test --locked`; `brokkr compile --bundle .brokkr/recipes/self`;
  `bash scripts/coverage-gate.sh`; `bash scripts/coverage-gate.sh selftest`;
  `bash scripts/publish-guard-test.sh`; `bash scripts/seal-branch-test.sh`;
  `cargo deny check licenses`. A missing tool is reported, never claimed.
- **AC16 No dependency change.** `Cargo.toml`, `Cargo.lock`,
  `evals/Cargo.toml`, `evals/Cargo.lock` and `deny.toml` are unchanged;
  the rule uses `std::fs`, `std::os::unix::fs::MetadataExt` and nothing
  else new.
- **AC17 Delivery.** Commits unsigned; nothing pushed, merged, published or
  accepted. The PR description states that no accepted decision states the
  old rule so no addendum is drafted, names `benches/mail_compare.rs` as
  the pre-existing unvalidated `--output`, and names the residuals below.

## Residuals named for the operator (not changed here)

- **Pre-existing root under a shared sticky parent.** `ensure_root` accepts
  any existing real directory without an owner or mode check, so a root
  another local user created first under sticky `/tmp` is accepted, today
  and after this change (the README's `/private/tmp/evals` advice already
  had this shape). The README now tells operators to name a per-user root
  that does not yet exist. Tightening this is a behaviour change for
  symlink-free paths and so is a separate ruling.
- **Asymmetry on the link route.** A root-owned link whose target is a
  non-anchored directory refuses through the link but is accepted when the
  target is named directly. This is the operator's requested test.
- **`benches/mail_compare.rs` `--output`** has no root validation.
