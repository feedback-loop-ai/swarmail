# Spec — the harness walk must read the token stream: no token-split load shape launders the reserved name

| | |
|---|---|
| **Run** | 12c — chief synthesis seat |
| **Feature slug** | `load-shape-token-stream` |
| **Issues** | #101, #102 (bodies: `.brokkr/state/run12-feature.txt` — absent; numbers known from the feature text and commit history) |
| **Decisions** | none minted — see [Decision ruling](#decision-ruling--none-minted); 0034 stays free |
| **Base** | `bcc7510` (run 12c review base; six house gates green; working tree clean) |
| **Shape** | the load-shape scan inside `check_harness_declarations` (`scripts/coverage-gate.sh`) moves from the line-preserved stream to `rs_tokens`; one inserted pass, one deleted block, six selftest arms; no Rust, no bundles, no new file, no new stage |

## The problem

The harness-declaration walk (`check_harness_declarations`, `scripts/coverage-gate.sh`:284)
reserves the harness names under `src/` (decision 0020): a file named `tests.rs`
/ `*_tests.rs` / `*-tests.rs` is dropped from the coverage report by
cargo-llvm-cov's default ignore regex, so any parent that *loads* one must
refuse — the file is uncompiled, and the exact 100/100/100 verdict silently
lies. The walk therefore scans every declaring parent for `#[path = "…"]`
targets and `include!` arguments. But the scan is **line-anchored**: the
`#[path]` arm matches only attributes that *start a line* (base :350,
`^[[:space:]]*#\[[[:space:]]*path[[:space:]]*=`) and extracts the target from
the same line (:351–352); the `include!` arm (:372–406) is likewise line-based.
The scan reads `strip_rust_comments` — the line-preserved stream — while
`rs_tokens` (:168), the whitespace-collapsed token stream the gate already
owns, sits unused by it.

Five shapes, each legal Rust that compiles and loads a harness-named file,
were probed against verbatim `bcc7510` and **all five launder** (the walk
passes; the reserved file loads; the verdict lies):

1. **Stacked attribute:** `#[allow(dead_code)] #[path = "store/persist_tests.rs"] pub mod alias;`
   — the `#[path]` is not line-initial, so the anchored regex never matches.
2. **Split `include!`:** `include` at end of line, `!("store/persist_tests.rs");`
   on the next — the line-based `include`+`!`+delimiter match never sees the
   tokens together.
3. **Split `#`:** `#` alone on one line, `[path = "store/persist_tests.rs"]`
   on the next — neither line matches the anchor.
4. **Split opener:** `#[path =` on one line, the string on the next — the
   anchor matches but the same-line extraction finds no closing quote and
   declines silently.
5. **Continued target:** `#[path = "store/persist_\` + newline + `tests.rs"]`
   — the backslash-newline hides the target's tail from the same-line
   comparison.

Each bypass is decision 0020's exact threat model in a new spelling family:
the file is dropped from the report by name, production code parks inside it,
`cargo build` succeeds, and the gate's exact verdict shrinks its denominator
silently. That is a **HIGH** and it is live at the reviewed base.

Controls verified green at the base (the boundary as it stands, all still
pinned by existing arms): a line-initial harness `#[path]` refuses; an
escape-carrying `include!`/`#[path]` target refuses; every `include!` macro
delimiter and spacing refuses; a literal `include!` of a non-harness file
passes.

**Root cause:** the scan's input is the wrong stream. A load-shape is a token
sequence — `#` `[` `path` `=` *string* — not a line shape; any layout that
separates those tokens is invisible to a line-anchored match. The repair is
not a new check: it moves the existing check onto the stream its tokens
actually live in.

## Verified at the base, carried into the record

Recorded so this batch does not re-litigate them:

- All six house gates pass at `bcc7510` (intake criterion 1): `cargo fmt
  --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo
  test`, `brokkr compile --bundle bundles/self`, `scripts/coverage-gate.sh`
  (exact 100/100/100, miss == 0, selftest green), `cargo deny check licenses`.
- The selftest contract at the base is green, including the five `5h` arms
  pinning the three laundering-repair residuals (intake criterion 2), and the
  three repair pins are red at `bcc7510^` and green at `bcc7510` (criterion 3).
- The run's adversarial probe battery was re-run at the chief seat this run,
  all at verbatim `bcc7510`:
  - **Persistent string state — closed.** An escaped-newline continuation
    keeps following code; a string ending exactly at EOF keeps its content
    (loud over-emission, the tolerated direction); multi-hash raw strings
    crossing lines keep following code; a lifetime/char literal at line start
    keeps following code.
  - **Macro-delimiter recognition — closed.** `#[doc =
    include!("store/persist_tests.rs")]` refuses; `include /* pick */ ! (…)`
    refuses; `include!(concat!(env!(…)))` refuses; and a target that merely
    *contains* the token `include` — `#[path = "include/util.rs"]` — passes.
  - **Escape refusal — closed.** `\u{74}`, `\x2F` and octal `\164` in a
    `#[path]` target all refuse; the pass direction is the backslash-free
    literal load (ruling 3 reframes the intake's wording of it).
  - **The load-shape battery — the residual that did open.** The five shapes
    above all pass the walk at the base (i.e. launder); the stacked-clean
    keep shape passes. This spec's subject.
- The gate's compile-tree premise (carried from run 12b): the static checks
  run over a tree that compiled — rustc rejects unterminated strings and
  impossible layouts — so every shape the checks must resolve is one rustc
  accepted. The five laundering shapes are all in that class: ordinary,
  compilable Rust.

## The ruling — synthesis of the panel

The seats agree on the species (gate-integrity, HIGH direction, gate-only
repair in this run), the evidence discipline (red probe per defect, selftest
as the durable net), and the refusal list (no Rust changes, no new file, no
new stage, no decision document). They conflict on four load-bearing points;
each is ruled, not averaged.

1. **Adopt robustness's finding: the ship is not byte-identical.** The probe
   battery live-falsifies simplicity's binding statement — "ship `bcc7510`
   byte-identical; there is nothing new; the bar is met" — the stacked
   attribute alone launders a harness-named file past every check while six
   gates stay green, so the security bar (`has_security_residual=false`, no
   residual above medium) is **not** met at the base. One sanctioned repair
   class ships this run, landing in `bcc7510`'s own discipline (small, local,
   self-arm-pinned). **Cost of the losing side:** simplicity's zero-tree-
   change discipline is overruled for exactly one check and six arms — no
   other file, no other behavior moves. **Cost had robustness been
   overruled:** a live HIGH laundering bypass stays open and the run's own
   bar is unreachable; byte-identity would have been fidelity to a lie.
2. **The mechanism: the scan moves to the token stream.** The load-shape scan
   runs over `rs_tokens "$parent"` with an occurrence loop per shape
   (`match()`/`RSTART`/`RLENGTH`, no line anchoring), applying the closed-form
   rule per occurrence: an argument that is not a plain `"` literal cannot be
   resolved by this walk and refuses; an escape-carrying target refuses; a
   target whose basename matches the reserved pattern refuses; `END { if
   (scan_bad) exit 1 }` is fail-closed. **Simplicity's ladder rung 1 —
   blanket refusal (refuse any `#[path]`/`include!` at all, i.e. delete the
   pass arms) — is refused for this run:** it deletes green, reviewed
   machinery and a pinned pass arm, which is strictly *more* change than the
   guard it wants to avoid, contradicting the minimality it argues from. It
   is reserved as the operator's policy option for a future residual that
   demands *new* vocabulary rather than a moved boundary. **Cost to
   simplicity:** the parse-rule liability survives in a wider form — the
   honest sentences below name two new loud over-refusal species (string
   text shaped like a load-shape; whitespace-collapsed spellings that land on
   the reserved pattern). Both refuse only while a harness-named file exists
   under `src/`, and both are loud. **Cost to robustness:** none material —
   its probe/refusal-arm/repair discipline is adopted verbatim, and its HIGH
   is closed by exactly the guard it demanded.
3. **The reframed pass arm.** The intake phrases criterion 4's pass direction
   as "a literal backslash in a legit path still passes"; the design seat's
   finding stands and is adopted: a backslash inside a load target is a
   string escape this walk does not decode — `\x74` spells `t` — so any
   backslash-carrying target refuses, fail-closed, and **the pass direction
   is the backslash-free literal load** (the existing §5h arm and the new
   keep arm). This is an accepted loud over-refusal under the operator's
   standing "no decoder ever" decision. **Cost:** Rust path spellings that
   legally use escapes must be rewritten literally to pass the gate — loud,
   named, and instantly fixable; no silent path exists in either direction.
4. **Arm budget — six arms, none speculative.** Five refuse arms, one per
   red shape (stacked, split-`include!`, split-`#`, split-opener,
   continued-target), each red at `bcc7510` and refusing after the repair.
   One keep arm, `co-stacked-clean`, pinning the moved boundary — and it
   carries a *declared* harness file so it exercises the scan (a pass arm
   with no harness-named file under `src/` passes vacuously and is not a
   pin; this was caught and reworked at the chief seat). **Robustness's
   "every framing probe becomes an arm" is refused:** arms only where a probe
   went red or the boundary moved; the passing probe outcomes (persistent
   string state, macro delimiters, escape spellings, `include`-named
   directory) are recorded in the landing notes per intake criterion 4, not
   frozen as fixtures. **Cost:** fewer tripwires than robustness wanted; the
   epic's own red-arm criterion is the budget's authority, per the 12b
   precedent.
5. **The dead disjunct stays; the subsumed arm stays.** The include! arm's
   condition carries a dead first disjunct (base :385 — inside a block that
   only runs when the line already matched that shape); it is out of scope,
   is pinned by the existing arms, and its removal would widen this diff
   without changing behavior. The decl walk's line-preserved `include!` arm
   (:372–406) is likewise **retained** although the token-stream scan now
   subsumes it (every refusal it can make, the scan also makes): smallest-
   diff discipline, and no comment in it becomes false. **Cost to
   simplicity:** two code sites check `include!` shapes for one more release;
   the spec is the record that the line-preserved arm is no longer the
   load-bearing one.
6. **Adopt simplicity's discipline in full where the positions don't
   conflict:** red-first implementation (the regression is proven at the base
   before the repair lands), one production file, no new architecture, any
   further residual found while landing gets one probe / one arm / one local
   guard or is deferred with the evidence recorded — never silently absorbed.

## The design

`check_harness_declarations` keeps its structure and becomes two passes over
each parent. The reference body — built and verified at the chief seat — is
in [plan.md](plan.md).

**1. Pass one: the token-stream load-shape scan.** For each candidate parent,
`rs_tokens "$parent"` (the comment-stripped, whitespace-collapsed stream the
gate already trusts for the `coverage(off)` match) is piped to a small awk
program with two occurrence loops — one for `#[path =`, one for
`include![({[`. No anchoring: an occurrence *anywhere* in the stream is
resolved, so a stacked attribute, a split spelling, a continued target and a
hash split are all seen as the contiguous token sequences they are. Per
occurrence the closed-form rule applies: the argument must be a plain `"`
literal (anything else refuses — `concat!`, `env!`, any macro spelling, an
unterminated string); the target must contain no backslash (an escape cannot
be decoded — refuse); and the target's basename must not match the reserved
pattern `(^|[_-])tests\.rs$` (refuse, naming the harness and the parent).
`END { if (scan_bad) exit 1 }` makes the scan fail-closed, and a non-1
non-zero exit from the pipeline refuses loudly ("could not scan") rather than
passing silently — the established posture of every check in this gate.

**2. Pass two: the declaration walk, unchanged.** The existing per-line decl/
stack walk keeps its regexes, its attribute-stack rules, its any-parent
iteration and its exit protocol; the only edit inside it is the *deletion*
of the old line-anchored `#[path]` block (:345–371), whose job pass one now
does over a strictly larger shape set. The line-preserved `include!` arm
stays (ruling 5).

**3. The selftest gains six arms** in a new `5i` block after the `5h`
residuals — five `expect_refuse … check_harness_declarations` arms over
fresh fixture trees in the established pattern, and one `expect_pass` keep
arm. All existing arms stay untouched and green.

**Honest sentences** (recorded so they are not rediscovered at the next
review):

- The scan reads the *comment-stripped* token stream: a load-shape inside a
  comment is not a load (rustc would not compile it) and must not refuse; a
  load-shape inside a *string literal* is seen and refuses — loud
  over-refusal, accepted (ruling 2's cost).
- `rs_tokens` removes all whitespace, including inside string literals: a
  target whose whitespace-collapsed spelling lands on the reserved pattern
  (e.g. `persist_ tests.rs` → `persist_tests.rs`) refuses. Loud, fail-closed,
  and impossible to confuse with a silent launder.
- The scan runs only while a harness-named file exists under `src/` — the
  whole check is scoped to the reserved names; with none present, no parent
  is scanned and nothing new refuses.
- `#[cfg]`-disabled code refuses all the same: the gate checks the tree as
  written, not as configured — the same policy the `coverage(off)` refusal
  enforces.
- The compile-tree premise bounds all of this: the gate refuses loudly where
  a compiler-verified tree cannot be resolved by this walk, and never
  silently.

## Acceptance Criteria

1. **Red→green refuse arms.** Five new selftest arms — `co-stacked-path`,
   `co-split-include`, `co-hash-split`, `co-split-open`,
   `co-continuation-target` — each a fresh fixture tree hiding a
   harness-named file behind its load-shape. Each arm **fails at the base**
   (`bcc7510` — the shape passes the walk and the file launders) and
   **refuses after the repair**. The red-proof battery output is recorded in
   the landing notes (intake criterion 4's record requirement).
2. **Keep-direction arm.** `co-stacked-clean`: a harness-named file declared
   by its parent under `#[cfg(test)]` *plus* a stacked
   `#[path = "store/ordinary.rs"]` load of a non-harness file, in one tree.
   Green at the base and green after the repair — red at neither. A pass arm
   with no harness-named file present is rejected as vacuous.
3. **No regression in the durable net.** Every existing selftest arm passes
   unchanged — the `coverage(off)` spellings, the leak check, hidden-harness
   dirs, the declaration-walk arms (including the §5h repair pins), counting,
   preflight and `SF:` cross-check arms — with the selftest still fail-closed.
4. **Closed-form scan rule, structurally.** Every `#[path =` and every
   `include![({[` occurrence in each parent's token stream is resolved by the
   scan; an unresolvable argument refuses; no fixture is special-cased and no
   allowlist exists. Verified by review of the diff plus arms 1–2; the scan
   contains no name-specific branch beyond the reserved-pattern regex the
   walk already uses.
5. **All six house gates pass at the repair commit**, with `export
   PATH="$HOME/.cargo/bin:$PATH"` and the coverage gate run with
   `TMPDIR=/home/valentin/scratch/cov-tmp`: fmt, clippy `-D warnings`,
   `cargo test`, `brokkr compile --bundle bundles/self`,
   `scripts/coverage-gate.sh` (selftest green, **exact 100/100/100,
   miss == 0**), `cargo deny check licenses`.
6. **Scope holds.** The only changed tracked file is
   `scripts/coverage-gate.sh` (the scan move and the six arms). No `src/`,
   `tests/`, `bundles/`, `docs/` or dependency changes; no new file; no
   `coverage(off)` anywhere; no decision document.
7. **Landing and record.** The batch lands on the run's landing branch per
   the usual path (review and push remain the operator's); the landing notes
   record the red-first evidence *and* the passing probe outcomes (persistent
   string state, macro delimiters, escape spellings, `include`-named
   directory). No GitHub issue is closed by this seat — the operator cites
   and closes #101/#102. Commits cite #101/#102.

## Decision ruling — none minted

No decision document is minted; **0034 stays free.** The ruling rests on four
legs: the three frozen guarantees (0001 sync insert, 0002
subscribe-before-check, 0003 oldest-only pruning) are untouched — this is
dev-plane gate tooling; the gate's *verdict* semantics are unchanged (exact
100/100/100, refuse arms, fail-closed selftest — the contract survives, only
its blind spot closes); the change amends no ruling — it *strengthens*
decision 0020's enforcement in the same species as the operator's three
repairs at `bcc7510`, which landed doc-free; and both seats independently
ruled no document is demanded. A truthing of comments inside the function is
not a ruling.

## What this batch refuses to do

- **No blanket refusal.** Ladder rung 1 (refuse every `#[path]`/`include!`
  load, delete the pass arms) stays refused: it is more change than the
  guard, and it deletes reviewed green machinery. Reserved for the operator
  if a future residual demands new vocabulary.
- **No decoder, ever.** No escape decoding, no path canonicalization, no
  "resolve the string like rustc would" ambition — unresolvable means refuse,
  loud, naming the parent (operator decision, standing).
- **No line-based patching of the five shapes.** A second anchor, a
  look-ahead, a "join the next line" pre-pass — each re-opens the same race
  with the next layout; the token stream is the input where the race does not
  exist.
- **No touching the dead disjunct (:385), the `SF:` cross-check, the
  counting, the preflight, or the instrumented-build logic.**
- **No arms for probes that passed.** Passing probe outcomes are landing-note
  records, not fixtures — the arm budget is red shapes plus moved boundaries.
- **No `src/` change, no new file, no dependency, no `coverage(off)`, no
  unconditional ANSI, no timing assertion** (0020 §5, house non-negotiables).
- **No issue closed by this seat**, and nothing pushed — review, push and
  merge are the operator's.
