# Implementation plan — accept artifact roots named through root-owned symlinked ancestors

Implements [spec.md](spec.md). Compatible work under accepted 0045 §8: no
accepted ruling is contradicted, no decision is declared, drafted or
accepted. The chief commits spec.md, plan.md and tasks.md only.

## Technical context

The `evals/` workspace (its own `Cargo.lock`, outside the root coverage
denominator — accepted 0055) holds the artifact-root validation in
`evals/src/artifacts.rs` and its five operator boundaries in
`evals/src/runner.rs` and `evals/src/main.rs`. Everything the change needs is
in `std`: `std::fs::{symlink_metadata, metadata, read_link}` and
`std::os::unix::fs::MetadataExt::{uid, mode}`. `libc` is already a
dependency and is not needed. No new file outside this spec directory; no
new module, flag, exit code, stderr category or schema field.
`.specify/templates/` holds no realm template; this plan supplies the
context, constitution check, structure, research, contracts, quickstart and
risks the dialect asks for.

## Constitution check

- 0001–0005: untouched; nothing under `src/` changes.
- 0020: tests stay inline in `artifacts.rs`'s existing `mod tests`, in
  `main.rs`'s existing `mod tests` and in `evals/tests/runner.rs`; no new
  harness file; no sleep as synchronization; fresh tempdir per test; no
  shared mutable state; no wall-clock input.
- 0045 §8: compatible work; no decision body touched.
- 0050 §1 / 0056 addendum: nothing under `scripts/`, `.brokkr/`,
  `.github/`; no lockfile or `deny.toml` change; so the PR is mergeable on
  green without a hold.
- 0056: commit unsigned, never push.
- 0075: no path is echoed on stderr or stdout; the artifact schemas stay
  `deny_unknown_fields` with no new field.
- 0076: trials stay sequential; the CLI tests take `trial_scope()` as the
  existing ones do.

## Research and chosen approach

### Why not `canonicalize`

`std::fs::canonicalize` follows every link in the chain without letting the
caller inspect it, so a user-owned link hidden inside a root-owned target
would be followed. The checked walk below follows a link only after the rule
admits it and re-checks every component it splices in.

### Why the holder, the target and the link uid

Unlink-and-recreate of a directory entry is governed by write permission on
the directory containing it, not by the entry's owner — so the holder check
is what makes "a same-uid attacker could otherwise swap it" true. The target
check is the operator's literal sentence and named test. The link uid is the
operator's "owned by root (uid 0)". A root-owned sticky directory lets no
non-root user displace another owner's entry, which is the property the
write-bit test stands for, so it passes; `/private/tmp` (1777) is the case
the ruling exists for.

### The resolver

```text
pub fn resolve_root(root: &Path) -> Result<PathBuf, String>
  queue: VecDeque<Component> = root.components()
  real  = PathBuf::new(); hops = 0
  while let Some(c) = queue.pop_front():
    ParentDir                       -> Err("parent path refused")
    next = real.join(c)             // RootDir resets real to "/"
    match symlink_metadata(&next):
      Err(NotFound)                 -> real = next
      Err(_)                        -> Err("root unavailable")
      Ok(m) if m.file_type().is_symlink():
        if queue.is_empty()         -> Err("symlink refused")        // leaf
        holder = symlink_metadata(if real.as_os_str().is_empty() {"."} else {&real})
                 .map_err(|_| "root unavailable")?
        target = metadata(&next).map_err(|_| "symlinked ancestor target unavailable")?
        if !target.is_dir()         -> Err("symlinked ancestor target not a directory")
        admit_ancestor(own(&m), own(&holder), own(&target))?   // pure, see below
        hops += 1; if hops > 40     -> Err("symlink loop refused")
        t = read_link(&next).map_err(|_| "symlinked ancestor target unavailable")?
        push t.components() to the FRONT of queue, in order
        // real is unchanged: the link is replaced by its target's components
      Ok(m) if m.is_dir()           -> real = next
      Ok(_)                         -> Err("root component is not a regular directory")
  Ok(real)
```

The queue must be a real `VecDeque` of owned components (or an equivalent
owned representation) because link targets are read during the walk; the
implementer collects `read_link`'s `PathBuf` and pushes its components to the
front in order. A relative target thus resolves against the holder because
`real` still names the holder when the target's first component is joined.

### The pure predicate

```text
pub(crate) struct Ownership { uid: u32, mode: u32 }   // from MetadataExt
fn anchored(o: Ownership) -> bool   // o.uid == 0 && (o.mode & 0o022 == 0 || o.mode & 0o1000 != 0)
pub(crate) fn admit_ancestor(link: Ownership, holder: Ownership, target: Ownership)
    -> Result<(), &'static str>
  holder writable && !sticky -> "symlinked ancestor holder writable by group or other"
  target writable && !sticky -> "symlinked ancestor target writable by group or other"
  link.uid   != 0            -> "symlinked ancestor not owned by root"
  holder.uid != 0            -> "symlinked ancestor holder not owned by root"
  target.uid != 0            -> "symlinked ancestor target not owned by root"
  Ok(())
```

The order puts the two mode reasons first so an unprivileged test on the
real filesystem reaches them (a same-uid link always fails the ownership
arms). The two "holder/target not owned by root" arms are unreachable
unprivileged on a real filesystem; the table test over `Ownership` triples is
their proof.

### The five boundaries

Each is a one-expression edit that binds the resolved path and uses only it
afterwards (spec § Call sites). Site 1 keeps `finalize`'s error mapping to a
harness-error row and `artifact_write_failed`; sites 2–4 keep
`artifact_publication_failed` / `artifact_evidence_invalid`; site 5 keeps
`baseline refused`. `MatrixEvidence::start` receives the baseline's
`root.join("controls")` already real, and resolving a symlink-free path is a
component-wise identity, so no path is resolved to a different place twice.

### Behaviour on the task's platforms

- macOS: `/tmp/evals` → holder `/` (root 0755) ✓, target `/private/tmp`
  (root 1777 sticky) ✓, link root ✓ → `/private/tmp/evals`. `/var/folders/…/T/x`
  → `/private/var/folders/…/T/x` (target `/private/var` root 0755). `/etc/x`
  → `/private/etc/x`.
- Linux without symlinks in the path: identity.
- Linux merged-usr (`/bin -> usr/bin`, `/lib -> usr/lib`): admitted when `/`
  and `/usr/bin` are root-owned 0755, which is the common case; the AC11
  probe uses whichever candidate exists and asserts the arm it ran.
- A same-uid link anywhere in the ancestor chain: refused with
  `symlinked ancestor not owned by root` (or an earlier mode reason).

## Project structure — files touched

| File | Change |
|---|---|
| `evals/src/artifacts.rs` | add `Ownership`, `admit_ancestor`, `resolve_root`; rewrite the `artifact_root` helper's comment (keep `canonicalize`); extend `exclusive_destinations_and_symlinked_roots_are_never_reused`; add the predicate table test, the portable real-filesystem test (refusals plus the root-owned probe) and the macOS-gated unit test. `ensure_root`, `safe_root`, `read_file`, `publish`, `retain_with`, `trial`, `matrix`, `create_private_dir`: no changed line. |
| `evals/src/runner.rs` | `finalize`: resolve before `retain_trial` (site 1). `create_private_artifact_dir`, `SHARED_TMP`, modes: unchanged. |
| `evals/src/main.rs` | `MatrixEvidence::start` (site 2), `Cmd::Check` arms (sites 3, 4), `baseline_leaf` (site 5); unit tests in the existing `mod tests` for `baseline_leaf` through a same-uid linked parent (refuses, creates nothing) and `MatrixEvidence::start` on a same-uid linked root (refuses, creates nothing). |
| `evals/tests/runner.rs` | macOS-gated CLI test: `trial … --artifacts-dir /tmp/<unique>` then `check --artifacts-dir /tmp/<unique>/<trial_id>`; a portable CLI test that a same-uid linked ancestor still yields `artifact_write_failed` without the path on stderr. |
| `evals/tests/common/mod.rs` | comment at lines 29-32 rewritten; code unchanged. |
| `evals/README.md` | lines 113-116 replaced with the rule. |
| `evals/run-plan.md` | line 84 reworded. |

Nothing else. No `Cargo.*`, `deny.toml`, `src/`, `scripts/`, `.brokkr/`,
`.github/` or `docs/decisions/` change.

## Data model

None. No stored artifact type changes; `Metadata`, `MatrixPlan`,
`MatrixIndex` and the baseline `Plan` keep their fields.

## Contracts

- `pub fn resolve_root(root: &Path) -> Result<PathBuf, String>` in
  `evals::artifacts`: read-only; the reasons in spec § The rule are its
  only error values and are exact strings tests assert.
- `pub(crate) fn admit_ancestor(link, holder, target: Ownership) -> Result<(), &'static str>`:
  pure; its five reasons are the corresponding strings.
- `ensure_root`, `safe_root`: signatures and bodies unchanged.
- CLI: no new flag, output line, stderr category or exit code. `trial`,
  `matrix`, `check` and `baseline` accept on macOS the `/tmp`, `/var` and
  `/etc` spellings they refused before; every refusal keeps its existing
  category.

## Quickstart

```bash
# from the repository root, offline, locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
SWARMAIL_BIN="$(cargo build --locked --bin swarmail --message-format=json \
  | jq -r 'select(.reason == "compiler-artifact" and .target.name == "swarmail"
                  and (.target.kind | index("bin"))) | .executable')"
test -x "$SWARMAIL_BIN"
(cd evals && cargo fmt --all -- --check)
(cd evals && cargo clippy --all-targets --locked -- -D warnings)
(cd evals && SWARMAIL_BIN="$SWARMAIL_BIN" cargo test --locked)
(cd evals && SWARMAIL_BIN="$SWARMAIL_BIN" cargo test --locked artifacts::)   # the focused suite
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
git grep -n "resolve_root\|safe_root\|ensure_root" evals/src     # AC5
git grep -n "private/tmp" evals/                                 # AC14
git diff main -- evals/src | grep -n "O_NOFOLLOW\|create_new\|DirBuilder\|0o700\|0o600\|ARTIFACT_"   # AC4: empty
```

The macOS-gated tests (AC7, AC8) run only in CI's `test (macos-latest)`
job; the verify seat and this host are Linux and cannot execute them, which
the implementer records in `implementation.md` rather than claims.

## Risks

| Risk | Mitigation |
|---|---|
| The sticky exemption is read by the security reviewer as a hole. | It is named in the spec and here with its reasoning: a root-owned sticky directory lets no non-root user unlink or rename another owner's entry; the exemption applies only to directories owned by uid 0; without it the ruling's own example refuses. The predicate table proves a root-owned 0777 non-sticky directory still refuses. |
| Hidden-ancestor rename: a root-owned link inside a root-owned non-writable directory that itself sits in a user-writable directory can be redirected by that user between the holder check and `read_link`. | Accepted and named. The same user can already redirect a symlink-free path's real ancestors, which the harness has never checked; `ensure_root` and `safe_root` still refuse anything planted below the resolved root; closing it needs the whole-chain anchor the spec rejected for changing symlink-free behaviour. |
| A link whose target contains another link, or `..`. | Target components re-enter the same walk: inner links are admitted or refused by the same rule; `..` refuses; more than 40 expansions refuses. Tests: a two-hop same-uid chain refuses at the first link; `<link>/../x` refuses with `parent path refused`. |
| TOCTOU between resolution and `ensure_root`. | A link swapped in after resolution is met by the unchanged strict walk and refused, never followed. Nothing is cached across invocations. |
| `/tmp` on Linux is not a symlink, so the Linux job proves no accept arm on a real filesystem. | AC9's table proves every arm of the predicate; AC11 probes merged-usr links where they exist and prints which arm it ran; AC7/AC8 prove the real macOS case in CI. |
| The boundary edits silently keep using the unresolved path somewhere. | AC6 and the per-site unit tests (a same-uid linked parent yields a refusal and creates nothing under the target); on macOS the AC7/AC8 tests fail if any later step used `/tmp/<unique>` through `safe_root`. |
| The matrix worker gets an unresolved path. | Workers receive `MatrixEvidence`'s `root.join("trials")`, which is built from the resolved root; resolving it again in `finalize` is an identity. |
| Behaviour change for a symlink-free path. | `resolve_root` on such a path returns its own components; the existing suites (`artifacts::`, `runner.rs` artifact tests, matrix and baseline tests) run unchanged on Linux and prove it. |
| The README still contains `/private/tmp`. | Only as the example of a root-owned sticky target in the rule sentence; AC14's grep names the allowed matches. |
| A developer host whose `TMPDIR` passes through a user-owned link. | The inline `artifact_root` helper and `common::tempdir` keep `canonicalize`, so the existing suites still run there; the rule is proven by tests that build their own links. |
| The dependency allowlist or lockfiles drift. | Nothing new is imported; AC16 is a `git diff --stat` check. |
