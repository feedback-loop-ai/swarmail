# Spec — nested-attr-close-gate

| | |
|---|---|
| **Status** | **superseded** (run 12f) — the source recognizer for declaration bypasses did not converge across runs 12–12e: a source scanner cannot prove the absence of an attribute or load that macros, `cfg_attr` chains and `include!` can produce. Run 12g replaces it with checks by effect; this spec is kept as the record. |
| **Run** | 12d — chief synthesis seat (`brokkr-state-run12d-feature-txt-15e129ff`) |
| **Feature slug** | `nested-attr-close-gate` |
| **Issues** | #101, #102 (the gate-and-tests batch this run verifies and ships) |
| **Decisions** | none minted — see [Decision ruling](#decision-ruling--none-minted); no number is consumed |
| **Base** | `05ed2b8` (the operator's repair of run 12c's review residuals G1/G2/G3; six house gates green; tree clean) |
| **Shape** | one awk close-test replaced by a single character class; one comment sentence trued; three selftest arms (two red-first, one keep); one subordinate fixture rename. Everything else this run does is verification and landing. No `src/`, no `tests/`, no dependencies, no bundles, no new file. |

## The question

Run 12d is a **verification batch**: re-run the six house gates at `05ed2b8`,
re-review the two run-12c repairs adversarially, ship with
`has_security_residual=false` and no residual above medium. The panel's
adversarial re-review found **one new HIGH of exactly the species the run-12c
repairs close** — the framing's own probe "nested parens before the closing
bracket", answered red. A verify batch cannot ship past a HIGH: the bar is
absolute, so this run closes the red with the smallest repair the boundary
demands and then proves the batch.

## The red — found independently by both seats, re-confirmed at this seat

The load-shape walk accepts a `path = "` candidate as attribute-shaped only if
the text after the closing quote matches one of two disjuncts
(`scripts/coverage-gate.sh`:391):

- `^[\001]*\]` — a bare-`]` close (`#[path = "…"]`);
- `^[\001]*\)[\001]*\]` — exactly **one** `)` (with separators) before `]` (the
  one-level `cfg_attr` close).

A **doubly-nested** `cfg_attr` closes with `))]` and satisfies neither — the
walk falls into the "ordinary assignment keeps passing" branch and never
compares the target to the harness name:

```rust
#[cfg(test)]
mod persist_tests;                       // the honest reservation still present
#[cfg_attr(all(), cfg_attr(all(), path = "store/persist_tests.rs"))]
pub mod alias;                           // compiles today, loads today
```

Probes, each re-run by this seat against verbatim `05ed2b8` statics
(`.brokkr/state/tmp/run12d-design/`, uncommitted run-local evidence):

- **rustc honors the nesting** — a nested effective path attribute resolves the
  module (an unapplied one would fail E0583); compiling one file at two module
  paths is legal Rust (robustness probe A, re-confirmed).
- **The check launders** — `check_harness_declarations` exits 0 on the fixture
  above, at depth 2 **and** depth 3, with the harness-named file present and
  the honest reservation intact. Every other layer misses with it: the
  declaration walk only inspects attribute stacks above `mod persist_tests;`
  lines, the reservation check is satisfied because the declaration is honest,
  and cargo-llvm-cov drops harness-named files from the report by path — so
  the exact 100/100/100 verdict silently shrinks. The one failure the gate
  exists to make structurally impossible, one spelling deeper than the two
  shapes `05ed2b8` already fixed.
- **An escape-carrying nested target launders too** at `05ed2b8`
  (`#[cfg_attr(all(), cfg_attr(all(), path = "store/persist_\x74ests.rs"))]`)
  — the fall-through branch never reaches the escape rule.

## The positions, ruled

Both seats located the same red independently and agree on the architecture:
one-eye, refuse-loud, no counting stage, no lexer, no decoder, no stream
unification, no blanket refusal, no `src/` change, no decision document, no
issue closed, history append-only. Three points genuinely conflicted; each is
ruled, with the losing side's cost named.

### Ruling 1 — the repair form: the single character class

**Simplicity** proposed replacing both disjuncts with one class:
`if (tail !~ /^[\001)]*\]/)`. **Robustness** proposed keeping the first arm
and generalizing the second to `(\)+[\001]*)+\]` (a grouped repetition,
strictly additive). **Ruled: the character class.** The two forms accept the
same language — any run of closing parens and 0x01 separators, in any order,
before the `]` — so there is no functional delta to arbitrate; the tie-breakers
are:

- **One disjunct fewer than today**, and no nested quantifier: a character-
  class run is the most portable ERE construct mawk can be asked to run, it
  cannot backtrack, and it cannot hang — the framing's standing rule forbids
  unportable ERE, and robustness's own peel-loop probe **hung the host mawk**,
  a lesson adopted here as a hard constraint: the repair stays a single match.
- **The stronger evidence trail**: the gate's own full selftest battery — all
  arms verbatim — was executed against the char-class form and passes
  (`selftest pass`, exit 0; re-run by this seat), alongside the red probes.

*Cost of ruling against robustness's form:* none functional — the accepted
language is identical; the "strictly additive" review-ability its shape bought
is paid instead by one trueing sentence that states the subsumption explicitly
(`[\001)]*` allows zero closers, so the bare-`]` case is subsumed, not lost).

### Ruling 2 — the arm budget: the 12c rule applied to both directions of the moved boundary

**Simplicity** budgeted two arms (one red, one keep). **Robustness** asked for
the red arm plus extra spelling pins (`)))]`, marker-glued `) )]`) and a
`coverage(off)`-via-`cfg_attr` arm. **Ruled: three arms** — the run-12c rule
("arms only where a probe went red or the boundary moved") applied to *both*
directions of the boundary this repair moves, for every `path = "` occurrence:

1. `co-nested-attr-harness` — **refuse, red at `05ed2b8`**: the doubly-nested
   `cfg_attr` load of a harness-named file (the red itself).
2. `co-nested-attr-escape` — **refuse, red at `05ed2b8`**: the same nesting
   with an escape-carrying target; at the base it passes (fall-through), after
   the repair the escape rule reaches it and refuses. This is the refuse
   direction of the moved boundary, and it goes unobserved without an arm.
3. `co-nested-attr-clean` — **keep, red at neither**: the same nesting loading
   a backslash-free, non-harness file, with the honest reservation present.
   Green at the base (fall-through passes it) and green after the repair
   (recognized and cleared) — the `co-stacked-clean` pattern re-instantiated
   at depth 2.

*Cost of ruling against simplicity:* one fixture (~4 lines) beyond its
two-arm budget — justified by simplicity's own keep-arm rationale ("the
boundary moved for every `path = "…"` occurrence") applied symmetrically to
the refuse direction. *Cost of ruling against robustness:* deeper spellings of
the same red (`)))]`, marker-glued) and green probes (`coverage(off)` via
`cfg_attr` — probe-confirmed to **refuse** via the deleted stream; a literal
0x01 in an ordinary string — probe-confirmed to **pass silently**, the escaped
quote blocking the match; attr-shaped text in a raw string — **refuses
loudly**) are **landing-note records, not arms**: an arm budget that grows
with curiosity is how a gate script becomes unmaintainable.

### Ruling 3 — no decision document

Simplicity refused to mint one (verdict semantics untouched — this is the same
species as the operator's `05ed2b8` repair of the same contract, which landed
doc-free); robustness did not ask for one. **Ruled: none minted.** The change
strengthens decision 0020's *enforcement* toward its already-accepted text;
the frozen guarantees, the gate's verdict semantics and the floor are
untouched. A char class is not a ruling. See
[Decision ruling](#decision-ruling--none-minted).

### Adopted from robustness — the spec sentences (documentation is free)

Three sentences the next reviewer will otherwise re-derive, carried in
[Spec sentences](#spec-sentences-carried-forward): the two-stream asymmetry
argument, the sentinel's conditionality, and the non-literal-RHS layering
(`concat!` probe: rustc itself refuses — "malformed `path` attribute input" —
re-confirmed at this seat). None costs a line of code.

### Adopted from simplicity — the cut list and the standing debt

No lexer, no third stream, no decoder, no blanket-refusal rung, no arms for
green probes, no `src/` change, no knob, no GPG repair, no issue closed. The
subsumed line-preserved `include!` arm (provably dead — the boundary-stream
scan refuses a superset of its shapes) is **not** deleted this run: deletion
has no red probe behind it and bundling an unrequested simplification into a
verify verdict muddies what was verified. It is **standing debt with a proof
obligation**: the next ordinary change that touches the walk deletes it, with
the selftest as the net. One subordinate exception, since the selftest is
touched anyway: the duplicated fixture name `co-split-include` (used at both
:865 and :886) is renamed in the second use — a one-word reader-trap fix,
dropped without argument if it draws review fire.

## The repair

One test, one line (`scripts/coverage-gate.sh`:391):

```awk
# before — two disjuncts, exactly one `)` recognized:
if ((tail !~ /^[\001]*\]/) && (tail !~ /^[\001]*\)[\001]*\]/)) {
# after — any run of closers and separators before the bracket:
if (tail !~ /^[\001)]*\]/) {
```

with the comment above it (lines 379–384) trued to say what the test now
accepts: a close is any run of `)` and separators before `]` — **any `cfg_attr`
nesting depth** — and an assignment (`let path = expr;`,
`let path = dir.path().join("persist.db");`) whose post-quote tail is not such
a run keeps passing. The subsumption is stated in the comment: the class
allows zero closers, so the bare-`]` case is subsumed, not lost.

## Spec sentences carried forward

1. **Two streams, two boundary guarantees.** `rs_boundary_tokens`
   (whitespace runs → one 0x01) feeds the load-shape scan; `rs_tokens`
   (whitespace deleted) feeds the `coverage(off)` match. Deletion is sound for
   a grammar-anchored subject — the attribute grammar supplies a punctuation
   boundary before `coverage` in every spelling, and deletion can never delete
   it (`#[cfg_attr(all(), coverage(off))]` probe-confirmed refusing). Markers
   are required for load-shape subjects — `return include` and `path =` sit in
   expression position, where whitespace is the *only* boundary. **Every
   consumer of a normalized stream must state its subject's boundary
   guarantee; whitespace deletion is permitted only where the subject is
   grammar-anchored.** The streams are not unified; the split is load-bearing.
2. **The sentinel is conditionally unforgeable.** 0x01 is inert-to-
   over-refusing in the arms (`[\001]*` absorbs it exactly as it absorbs a
   marker; `[^A-Za-z0-9_]` treats it as one more boundary) — it can neither
   break a real match nor forge a miss. The condition is **the tree
   compiles**: static checks run pre-build, and a tree that does not compile
   refuses at the instrumented run. The contract is "the gate refuses", never
   "each check is total in isolation". Probe-confirmed outcome, stated here so
   it is not re-derived: attr-shaped text inside an **ordinary** string passes
   silently (the escaped quote blocks the match); inside a **raw** string it
   refuses loudly — a named, accepted species, fired only while a
   harness-named file exists under `src/`.
3. **Non-literal path RHS: rustc is the refusing layer.** A `#[path]`/
   `cfg_attr` value that is not a single plain string literal is refused by
   rustc itself (`#[path = concat!("x", ".rs")]` → "malformed `path`
   attribute input"); the walk checks literal spellings only; the composition
   of the two layers is the refusal. The declaration walk's line-anchored
   check stays as a second layer; its miss direction (stacked attributes) is
   backstopped by the unanchored scan — two layers with named misses.
4. **Keep arms are scoped to backslash-free targets.** Any `\` in a target
   refuses — distinguishing `\\` from `\x74` requires an escape-grammar lexer
   in awk, the arms race the repair refuses to enter. A Unix filename
   containing a real backslash is un-`include!`-able: accepted, loud, named.

## Acceptance Criteria

1. **Red→green refuse arm (the red).** A new selftest arm
   `co-nested-attr-harness`: fresh fixture tree with the harness declared
   under `#[cfg(test)]`, a hidden `store/persist_tests.rs`, and
   `#[cfg_attr(all(), cfg_attr(all(), path = "store/persist_tests.rs"))]
   pub mod alias;`. The arm **fails at `05ed2b8`** (the check passes: the
   shape launders) and **refuses after the repair**. The red-proof output —
   the battery run with the arms in place at the unrepaired walk — is recorded
   in the run's landing notes.
2. **Red→green refuse arm (the refuse direction of the moved boundary).**
   `co-nested-attr-escape`: the same nesting with target
   `store/persist_\x74ests.rs`. Fails at `05ed2b8` (passes: laundered),
   refuses after the repair (recognized, then the escape rule refuses).
3. **Keep arm, red at neither.** `co-nested-attr-clean`: the doubly-nested
   `cfg_attr` loading a backslash-free **non-harness** file
   (`store/ordinary.rs`), honest reservation present, hidden harness file
   still in the tree. Green at `05ed2b8` and green after the repair. Each
   fixture carries the declared harness so the check is non-vacuous — a pass
   arm with no harness-named file present is rejected as vacuous.
4. **No regression in the durable net.** `bash scripts/coverage-gate.sh
   selftest` passes with every existing arm unchanged — the `coverage(off)`
   spellings, the §5h/§5j repair pins (including the one-level `cfg_attr` and
   raw-identifier refuses and both `let path = …` keeps), the §5i arms, the
   leak and counting checks — and the battery stays fail-closed. The framing's
   own keep pins (`let path = "x";`, `let path = dir.path().join("persist.db");`)
   are re-proven at the repaired commit.
5. **Closed-form close rule, structurally.** The post-quote close test is a
   single match, `tail !~ /^[\001)]*\]/` — no peel loop, no counting stage, no
   depth model, no second scanner; the trued comment sentences match the code;
   the changed comment is the only prose touched. Verified by diff review plus
   criteria 1–4.
6. **All six house gates pass at the repaired commit** — `cargo fmt --all --
   --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
   `brokkr compile --bundle bundles/self`, `scripts/coverage-gate.sh`
   (selftest green; **exact 100/100/100, missed == 0**; `TMPDIR=/home/
   valentin/scratch/cov-tmp`; `export PATH="$HOME/.cargo/bin:$PATH"`),
   `cargo deny check licenses`. The repair touches no Rust, so the measured
   denominator is unchanged — the verdict must come back identical.
7. **Scope holds.** The only changed tracked file is
   `scripts/coverage-gate.sh` — the one test at :391, the one trued comment
   sentence, the three §5k arms, the subordinate rename of the second
   `co-split-include` fixture use. No `src/`, `tests/`, `bundles/`, `docs/` or
   dependency changes; no new file; no `coverage(off)` anywhere; no decision
   document.
8. **Landing and record.** The batch lands on the run's landing branch per the
   usual path (review and push remain the operator's). The landing notes
   record the red-first evidence *and* the green-probe outcomes with the
   outcomes stated: `coverage(off)` via `cfg_attr` **refuses** (deleted-
   stream asymmetry holds); a literal 0x01 plus attr-shaped text in an
   ordinary string **passes silently**; the same shape in a raw string
   **refuses loudly**; `concat!` as a path RHS is refused **by rustc**;
   `)))]` and marker-glued `) \001 )]` tails are closed by the same class
   (mawk-confirmed). No GitHub issue is closed by any seat — the operator
   cites and closes #101/#102. Commits cite #101/#102. All gates are proven
   from a pristine tree — never from a worktree carrying probe residue.

## Decision ruling — none minted

No decision document is minted and no decision number is consumed. The ruling
rests on the same legs the run-12c synthesis used: the three frozen guarantees
(0001 sync insert, 0002 subscribe-before-check, 0003 oldest-only pruning) are
untouched — this is dev-plane gate tooling; the gate's *verdict* semantics are
unchanged (exact 100/100/100, refuse arms, fail-closed selftest — the contract
survives, its blind spot closes); the change amends no ruling — it strengthens
decision 0020's enforcement in the same species as the operator's own
`05ed2b8` repair of the same contract, which landed doc-free; and both seats
ruled no document is demanded. One character class is not a ruling. If the
operator disagrees at review, minting the document later costs nothing and
this spec is its draft.

## What this batch refuses to do

No lexer (`proc-macro2`/`syn`, no helper binary, no new dependency — the
compile-tree premise bounds what the scan must resolve). No third token
stream and no unification of the two existing ones (each is matched to its
subject's boundary guarantee). No decoder — no escape decoding, no cfg
predicate evaluation, no `cfg_attr` arity model, no path canonicalization;
unresolvable means refuse. No blanket-refusal rung (refusing every `path =`/
`include!` deletes green reviewed machinery). No arms for green probes.
No peel loop — it hung the host mawk; the repair stays a single match. No
`src/` change, no new env var or knob (the 0x01 byte stays a literal in one
place). No GPG repair, no history rewrite — commits follow the tree's
disclosed unsigned practice. No issue closed, nothing pushed; review, push
and merge are the operator's.
