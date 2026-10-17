# 0035 — The exact coverage gate refuses by effect: the build's own file set and the compiler's canonical expansion are the inventories

- **Status:** `proposed` — acceptance is the operator's. Until accepted,
  this document records the design run 12g commits to; it amends 0020 §3's
  leak-check sentence and nothing else. 0020's own file is not edited.
- **Date:** 2026-10-17
- **Amends:** [0020](0020-concurrency-tests-and-the-coverage-gate.md) §3
  (the sentence describing the leak check's refusal direction).
- **Transposes:** [0005](0005-extraction-at-ingest.md) (extraction at
  ingest) onto the gate's inventories.
- **Numbering note:** 0034 is cited by the delivery dialect for result
  sealing and is not yet landed under `docs/decisions/`; this record takes
  the next free number after it.

## Context

Issues #101's second half and #50: the exact coverage gate's denominator
could be left through *declarations* — a harness-named file compiled into
the production crate and dropped from the report by name, or a
`coverage(off)` in any spelling, including one emitted by a macro or
resolved through a `cfg_attr` chain. Fixtures A–G each passed the gate.
Runs 12–12e answered with a Rust lexer in bash that grew to 1176 lines,
and every review found one more spelling, because a source scanner cannot
prove the absence of something macros, `cfg_attr` chains and `include!`
can produce. The operator ruled the design itself was the problem; run 12f
restored main's declaration checks and marked the three recognizer specs
superseded.

Two facts found on the real tree shape this ruling (both verified this
session): the gate's own reports have **never** carried an `SF:` record
for `src/model.rs` — a `pub mod` of structs and derives with zero
functions, for which llvm-cov emits nothing countable — so a literal
"every dep-info `.rs` entry must appear in the report" check refuses main;
and the real expansion carries **exactly five** `coverage(off)`
attributes, all minted by `derive(Eq)` on `fn assert_fields_are_eq`
inside `#[automatically_derived] impl` blocks, so any function-inventory
check without the carve-out refuses main too.

A third fact was found while landing this design (implement seat, against
the real toolchain), and it moves stage 2's mechanism one notch: the
canonical expansion of the real tree **cannot be parsed by any syn**.
`src/api.rs:521` calls `std::pin::pin!`, and this toolchain's `pin!`
mints the experimental `super let` — accepted through the macro path, and
printed straight into the dump (line 494) — which syn refuses
(`unexpected token, expected ';'`), verified for syn 2.0.119 and for
3.0.9, the newest release. The residual that imagined "a lock bump" as
the remedy has no lock to bump to. But the canonical dump needs no parser:
by the time rustc prints it, comments and whitespace are gone, every
macro is expanded and every `cfg_attr` resolved — the token tree **is**
the canonical form, and the spelling space on it is still exactly one.
Stage 2 therefore walks the dump's tokens with `proc-macro2` (the
machinery syn itself is built on), and syn keeps the job it can do: the
fn probe over source files, which parse.

## Ruling

1. **The inventory is extracted at ingest — build time — and the checks
   are queries over the extracted record.** Nothing re-derives an
   inventory from source at check time. That is 0005 transposed, and it is
   the whole reason the recognizer died: a re-derivation can be fooled;
   the build cannot.
2. **Files, by effect.** The production units' dep-info — harvested from
   the gate's own instrumented build dir (the lib via its `.rlib` rule,
   the bins via executable rules whose prerequisites name the bin root and
   not the lib root) — is the file set the build itself testifies to. The
   gate refuses naming every **fn-bearing** entry with no `SF:` record in
   the report. *Fn-bearing*, not every `.rs`: a declarations-only file
   (`src/model.rs`) has no lines any dimension could count, and demanding
   a record for it demands the report lie. Non-Rust entries
   (`include_str!` assets) are exempt; an entry that does not resolve to
   an existing file under the workspace refuses, never skips. The
   commission's sentence stands as the contract: **a harness-name
   exclusion may only drop files that are NOT in the production build's
   file set; a harness-named file loaded into production is measured, not
   excluded.** On today's tool the excluded production file is absent from
   the report and the refusal fires; if a future tool measures it, its
   presence is the desired end state and nothing refuses.
3. **Functions, by effect, on the compiler's canonical output.** The
   production lib and bin are expanded with the coverage toolchain
   (`-Zunpretty=expanded`, same target dir, same cfg set) and the helper
   refuses any `coverage(off)`/`no_coverage` token outside the body of an
   `#[automatically_derived]` impl — on the dump the attribute appears in
   exactly one token spelling, so the walk is a query over the compiler's
   own output, not a re-derivation from source (see the third context
   fact). On this toolchain the attribute is rustc's only per-function
   exclusion mechanism, so on the canonical expansion this is exactly the
   missing-record check — computed where the join is computable, with the
   carve-out the join cannot express. The carve-out is load-bearing: the
   five minted fns are deliberately uninstrumented.
4. **The leak check points the membership way.** 0020 §3 says the
   leak-check "fires on *presence*" — any harness-named `SF:` record
   refuses. That contradicts ruling 2's contract sentence: under a tool
   that measures harness-named production files, the presence-refusal
   would refuse the measured state the contract demands. **Amendment:** a
   harness-named `SF:` record refuses when its file is *not* in the
   production build's fn-bearing set (test harness measured as production
   — the original disease), and passes when its file *is* (production
   measured — the desired end state). 0020's tripwire
   (`check_harness_declarations`, rule 6) and hidden-dir check stay
   byte-identical; the tripwire keeps guarding the placement discipline
   the leak check never guarded.
5. **The record-join at function granularity is declined.** Comparing an
   expanded-source function inventory against the report's `FN:` records
   (demangled paths, generics stripped) refuses main today (the minted
   fns) and — even carved — makes every uninstantiated generic, cfg'd-out
   file, dead macro arm and backend-eliminated function a standing gate
   refusal on honest trees, reshaping the tree to satisfy the gate. The
   accepted cost is named: **uninstantiated generics stay invisible to the
   gate** — and to the exact floor, which cannot see them either. If the
   operator wants that stricter check, it is a new decision, not an edit.

## The helper

`tools/coverage-fn-scan/` — a standalone crate outside the measured crate
(own `[workspace]`), depending on `syn` 2.0.119 (the `files` fn probe over
source files) and `proc-macro2` 1.0.107 with `span-locations` (the `attrs`
token walk over canonical expansions); both with their
`proc-macro2`/`unicode-ident` trees (MIT OR Apache-2.0; already in the
measured crate's `Cargo.lock` and cached in the realm's cargo home, so the
hermetic build needs no network and the existing `cargo deny check
licenses` already vets the same versions). One sentence of defense: *a
real parser for source, because the last four runs prove bash cannot be
one; the compiler's own tokens for its own output.* No Rust source is
ever lexed by bash again.

## Known limits (not accepted exemptions)

This decision amends only 0020's leak-check sentence, as stated above. Its
§§1 and 3 stand as written — no exclusion is accepted under `src/`, inline
`cfg(test)` code stays measured as the vacuity detector, and a
`coverage(off)` in any spelling refuses. The items below are technical
limits of the checks, named so review can face them; none of them is a
license to exclude.

1. A forged `#[automatically_derived]` impl inside a macro body can launder
   a comment-split exclusion past both the grep and the expansion scan —
   closing it is derive-provenance analysis (the lexer reborn); it requires
   deliberate laundering by someone who has read the gate.
2. A future non-attribute exclusion mechanism evades the expansion scan —
   and every source-side check ever proposed; the leak check and review
   face it then.
3. The token walk treats the first brace group after an
   `#[automatically_derived]` attribute as the minted impl's body; an item
   whose self type carried a const-generic block (`impl X for Arr<{1}>`)
   would carve the wrong group and refuse loudly — fail-closed, named, and
   rarer than the mechanism it guards against. A dump whose delimiters do
   not balance refuses loudly (it is not the compiler's output).
5. A source file syn cannot parse refuses loudly (remedy: helper lock
   bump); the expansion must resolve the same cfgs as the instrumented
   build — the coupling is stated at the call site and a mismatch refuses,
   never silently.

## Acceptance

Each of #101's fixtures A–G refused by the new checks; each refusal arm
proven from a real cargo build of a fixture crate, with a green twin per
red arm; a `macro_rules!`-emitted `#[coverage(off)]` refused; the real
tree passing at exact 100/100/100 with the new checks silent; no Rust
lexing in bash; the helper hermetic and outside the measured crate. The
batch carries this decision `proposed`; the operator accepts or returns
it.
