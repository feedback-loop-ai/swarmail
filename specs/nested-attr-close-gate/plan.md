# Plan — nested-attr-close-gate

## Technical context

`scripts/coverage-gate.sh` is one file with three layers: the contract
(:1–56 — exact 100/100/100, missed == 0, fail-closed), the static checks
(`rs_boundary_tokens` :168 — comment-stripped, whitespace runs → one 0x01
byte; `rs_tokens` :179 — comment-stripped, whitespace deleted;
`check_no_coverage_off` :187; `check_harness_declarations` :284), and
`selftest()` with `expect_pass`/`expect_refuse` helpers, dispatchable via
`bash scripts/coverage-gate.sh selftest` (:983). The harness walk's
load-shape scan (:361–433) reads the **boundary** stream — whitespace must
not be deleted, because `return include` + newline + `!(…)` would collapse
into `returninclude!(` and the identifier-boundary guard would see no
`include` token (the run-12c reviewer HIGH) — and recognizes a `path = "…"`
candidate as attribute-shaped by its post-quote tail (:391):

```awk
if ((tail !~ /^[\001]*\]/) && (tail !~ /^[\001]*\)[\001]*\]/)) {
```

Two disjuncts: a bare-`]` close, and exactly one `)` (separators allowed)
before `]` — the one-level `cfg_attr` shape. A doubly- or triply-nested
`cfg_attr` closes with `))]` / `)))…]`, matches neither, and falls into the
"ordinary assignment keeps passing" branch (:392–395): the walk never
compares the target to the harness name and the reserved file launders. The
base is `05ed2b8`, tree clean; the host awk is mawk 1.3.4 — no `?`
quantifiers, no unportable ERE; the last commits are unsigned (the tree's
current practice — disclose it in the result record). Run-local probe
evidence for this spec lives uncommitted under
`.brokkr/state/tmp/run12d-design/` (gitignored, like all `.brokkr/state/`).

The reference repair was **proven at the chief seat before this spec**:
`.brokkr/state/tmp/run12d-design/gate-statics-patched.sh` carries the exact
one-line change (diff vs the verbatim `05ed2b8` statics: line 391 only); the
red probe shows depth-2 and depth-3 laundering at the base and refusal under
the patch; the escape-carrying nested target launders at the base and
refuses under the patch; and the gate's own full selftest battery — all arms
verbatim — passes against the patched guard (`selftest pass`, exit 0).

## Constitution check

- **Frozen guarantees (0001/0002/0003) untouched** — gate tooling only; no
  Rust, no bundles, no wire surface, no dependency, no `src/` line.
- **Decision 0020 respected on every axis:** harness names stay reserved —
  this repair *strengthens* the reservation's enforcement; the new arms are
  deterministic shell fixtures (fresh `mktemp` trees per run, trap-removed;
  no sleeps, no wall-clock validity assertions, no chaos probabilities); no
  `coverage(off)` is introduced; 0020 is accepted (2026-10-16, `5671e0c`)
  and binds; nothing here amends it. 0020's placement rules govern Rust test
  files under `src/`; the gate's selftest is the gate's own battery and is
  where gate behavior is pinned (precedent: §5h/§5i/§5j).
- **The gate's own contract intact:** the floor still refuses anything but
  exact 100/100/100; the selftest stays fail-closed; a scan failure still
  refuses loudly ("could not scan $parent") rather than passing; the close
  test remains a single match — no peel loop (it hung the host mawk), no
  counting stage, no second scanner.
- **House style:** no unconditional ANSI; `.brokkr/state/` evidence stays
  gitignored; commits cite #101/#102 and close nothing; commits are unsigned
  per the tree's current practice, disclosed in the result record.
- **Six gates** re-run at the repaired commit before any success is
  reported (`export PATH="$HOME/.cargo/bin:$PATH"`; the coverage gate with
  `TMPDIR=/operator-scratch/cov-tmp`), from a **pristine tree** — never
  from a worktree carrying probe residue (the run's own disclosed lesson).

## Files touched

Exactly one tracked file: `scripts/coverage-gate.sh`.

| Region | Change |
|---|---|
| :379–384 (walk comment) | True one sentence: a close is any run of `)` and separators before `]` — any `cfg_attr` nesting depth; the class allows zero closers so the bare-`]` case is subsumed; assignments keep passing |
| :391 (the close test) | `(tail !~ /^[\001]*\]/) && (tail !~ /^[\001]*\)[\001]*\]/)` → `tail !~ /^[\001)]*\]/` |
| after :908 (new §5k, before the `# 6. Canonical counting` block) | Three arms: `co-nested-attr-harness` (refuse), `co-nested-attr-escape` (refuse), `co-nested-attr-clean` (keep), each mirroring the §5j fixture shape (declared harness; hidden `store/persist_tests.rs`; `src/alias.rs`) |
| :886 (fixture name) | Rename the second `co-split-include` use to a distinct name (`co-inc-line-split`) — the §5i reader-trap fix; subordinate, dropped if it draws review fire |

No other file: no `src/`, `tests/`, `Cargo.toml`, `bundles/`, `docs/`;
no new file; no decision document (spec's Decision ruling).

## Approach

Red-first, in the 12c discipline — the arms exist before the repair and are
proven red at the base:

1. **Arms in, repair out.** Add the three §5k arms to `selftest()` with the
   close test still at its `05ed2b8` form. Fixture shape mirrors §5j
   (`co-cfg-attr-path`): `src/store/persist_tests.rs` (the hidden harness
   file), `src/alias.rs`, and `src/lib.rs` carrying the declared harness plus
   the nested attribute. The keep arm swaps the load target to
   `store/ordinary.rs` (created, alongside the hidden harness file) — the
   `co-stacked-clean` pattern at depth 2.
2. **Red proof.** `bash scripts/coverage-gate.sh selftest` at the unrepaired
   walk: the two refuse arms must FAIL (the check passes — the shapes
   launder), the keep arm must pass, and every existing arm stays green.
   Capture the output for the landing notes. If a refuse arm is green at the
   base, it is not a red-first arm — stop and re-derive.
3. **The repair.** Replace the two-disjunct test at :391 with the single
   class and true the comment sentence above it. Nothing else in the walk
   moves; the loop's `rest` resumption points are untouched.
4. **Green proof.** `bash scripts/coverage-gate.sh selftest`: every arm
   green, including the three new ones — and the framing's keep pins
   (`let path = "x";`, `let path = dir.path().join("persist.db");`) are
   re-proven (they are existing arms/battery members; re-run them via the
   probe fixtures if not pinned).
5. **The subordinate rename** (only after the battery is green, so the
   rename's proof is a clean re-run).
6. **The six gates** at the repaired commit, from a pristine tree; the
   coverage verdict must come back identical (no Rust changed).
7. **Landing and record** per the run's usual path: landing notes carry the
   red proof, the green-probe outcomes with their stated results
   (`coverage(off)` via `cfg_attr` refuses; 0x01-in-ordinary-string passes
   silently; raw-string attr-shape refuses loudly; `concat!` RHS refused by
   rustc; `)))]` and marker-glued tails closed by the class), the unsigned
   commit disclosure, and no closed issue.

## Contracts

- **The close-recognizer contract:** a `path = "` candidate whose post-quote
  tail is a run of `)`/0x01 followed by `]` is attribute-shaped at any
  nesting depth; everything else keeps passing as an ordinary assignment;
  a recognized target with a `\` refuses; a recognized harness-named target
  refuses, naming the parent. One match, linear, no state.
- **The selftest contract:** fail-closed — an arm that cannot run is a
  failure; fixtures are fresh per run and trap-removed; no arm depends on a
  schedule, a wall clock, or shared mutable state.
- **The stream contract (unchanged, restated):** the load-shape scan reads
  `rs_boundary_tokens` (markers preserved — expression-position subjects);
  the `coverage(off)` match reads `rs_tokens` (deletion sound — the subject
  is grammar-anchored). Neither stream changes in this repair.

## Quickstart

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cd <repository-root>

# red proof (arms in, repair out) — expect exactly the two refuse arms red
bash scripts/coverage-gate.sh selftest

# … apply the one-line repair and the comment true …

# green proof — full battery, fail-closed
bash scripts/coverage-gate.sh selftest

# the six house gates
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
brokkr compile --bundle bundles/self
TMPDIR=/operator-scratch/cov-tmp bash scripts/coverage-gate.sh
cargo deny check licenses
```

## Risks

- **Over-refusal widens by one species.** Token soup shaped
  `path = "…"` + a run of `)` + `]` (macro bodies, raw strings carrying the
  shape with real quotes) now refuses where it previously fell through.
  *Mitigation:* it is loud, names the parent, and fires only while a
  harness-named file exists under `src/` — the same trade the one-level test
  already makes, at one more depth; the spec states the species and the
  selftest's keep arms prove the assignment direction still passes.
- **The bare-`]` case could be lost in the rewrite.** *Mitigation:* the
  class allows zero closers (`[\001)]*` matches the empty run), so the first
  arm is subsumed, not dropped — and every existing `#[path = "…"]` refuse
  arm in the battery proves it on every run.
- **Portability/termination of the new regex.** A nested quantifier
  (`(\)+…)+`) was the alternative; a peel loop hung the host mawk outright.
  *Mitigation:* the landed form is a single character-class run — no nested
  quantifier, no backtracking, no loop — probe-confirmed under the host awk
  (`)))]`, `) \001 )]`, and the depth-2/3 fixtures).
- **Comment drift.** A trued sentence that overstates what the test accepts
  is the next reviewer's false floor. *Mitigation:* the comment names the
  exact accepted language and the subsumption; AC 5 reviews the pair.
- **Gates proven from a dirty tree.** The run's disclosed incident: foreign
  probe residue in `src/` made the tree non-compiling mid-session. *Mitigation:*
  every gate run starts from `git status --short` clean (stash or fresh
  worktree otherwise); the landing notes state the tree the gates ran from.
- **The standing debt could rot.** The subsumed line-preserved `include!`
  arm stays this run (no red behind its deletion). *Mitigation:* recorded as
  standing debt in the spec; the next ordinary change that touches the walk
  deletes it with the selftest as the net.
- **Scope creep in a verify batch.** Any further red found by the verify
  seat is a new finding for the panel, not a quiet extra edit. *Mitigation:*
  the task list below is closed; deviations return to design.
