# Spec — the stripper must prove or refuse: raw-open detection and linear-time emission in the coverage gate

| | |
|---|---|
| **Run** | 12b — chief synthesis seat |
| **Feature slug** | `stripper-prove-or-refuse` |
| **Issues** | #101, #102 (bodies: `.brokkr/state/epic70/run12-feature.txt`) |
| **Decisions** | none minted — see [Decision ruling](#decision-ruling--none-minted); 0034 stays free |
| **Base** | `f815366` (run 12 repair batch; working tree clean) |
| **Shape** | one function rewritten in `scripts/coverage-gate.sh` (`strip_rust_comments`, :80–155) plus its selftest arms (:523+); no Rust, no bundles, no new file, no new stage |

## The problem

The gate's stripper (`strip_rust_comments`, `scripts/coverage-gate.sh`:80–155) feeds
`rs_tokens`, `check_no_coverage_off`, `check_report_covers_src` (the `SF:` leak
cross-check) and the harness-declaration walk. Its contract (:89–91) says a
missed raw string may only end early — loud, never an over-pass. Five shapes
now live-verified against verbatim HEAD `f815366` (chief probe sheet
`.brokkr/state/tmp/chief-12b/probe.sh`; panel evidence
`.brokkr/state/tmp/panel-12b/probes.sh`) resolve a guess toward **erasure** —
the one direction the contract forbids — and erase live code from **every**
static check that shares the stripper:

- **R1 (HIGH):** `cr#"…"#` — the C-string raw spelling is missing from the
  prefix arms (:136–137), so the opener is misread as a plain string, which
  force-closes at the newline (:152); a `/*` inside the content then erases
  everything after it. `static S: &str = cr#"abc\n/* "#;` erases a following
  `pub fn hidden() {}` and its `#[coverage(off)]`.
- **R2 (HIGH):** the `k < 8` cap (:131–132) turns any 9+-hash raw string into a
  phantom plain string — `r#########"x"/*"y"#########;` erases a following
  `mod hidden {}`. This falsifies the comment at :128–131 ("longer runs are no
  spelling this scanner meets, and a missed raw string only ends early — loud,
  never an over-pass"): the cap is precisely an over-pass.
- **I (HIGH):** a raw string opening at **column 0** — `substr(pending, …)` with
  a negative index clamps to 1 on this host's awk (mawk 1.3.4), so `before`
  reads the `r` itself as an identifier character and the opener is misread as
  a plain string. `static S: &str =\nr#"say "/*" hi"#;` strips to
  `r#"say "` and erases everything after. The same clamp bites `br#"` at
  column 0.
- **R4 (HIGH, found this run — both seats missed it):** a **hash-less** raw
  string `r"…"` shares its state (`instr=1, shash=0`) with plain strings, so a
  multi-line `r"…"` containing `/*` force-closes at the newline (:152) and
  erases what follows: `static S: &str = r"say\n/* hi";` erases a following
  `fn hidden() {}`. Raw strings legally span lines; plain strings cannot —
  the state has no flag distinguishing them.
- **R3 (MEDIUM):** `pending` grows one concat per normal-state character
  (:145) while its comment claims it is "capped" (:126–127) — and the *emission*
  accumulator `out = out c` (:115/:139/:144/:145) is equally quadratic, which
  no seat's prescription addressed. Measured on this host (mawk 1.3.4): a
  1 MiB single-line string literal costs the stripper **49.9 s**; 4 MiB exceeds
  60 s; 16 MiB exceeds 60 s. A generated fixture, an embedded base64 blob or a
  certificate constant of that size pins CI on **every** gate run — a dev-plane
  DoS aimed at the gate, exactly the species robustness named.

All five bypass the same way: the erased region can hide a `#[coverage(off)]`,
a harness `#[path]`/`include!`, or an unguarded `mod` line from the static
checks, silently. Controls verified green today and pinned below: 8-hash
boundary (modelled as raw), identifier merely ending in `r` (`outer"…"`,
plain), `xcr#"…"` (plain), nested comments, char literals, lifetimes.

**Root cause:** two unresolved guesses in one function — the capped,
substring-indexed look-back (:131–135) and the raw/plain conflation (:152) —
plus per-character string accumulators (:115/:145). Every guess resolves toward
erasure. The repair replaces the guesses with a proof and the accumulators
with constants; it does not add a check.

## Verified at the base, carried into the record

Both seats re-verified independently at `f815366` and the chief re-checked the
load-bearing claims; recorded so this batch does not re-litigate them:

- All six house gates pass at the base: `cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `brokkr compile --bundle bundles/self`, `scripts/coverage-gate.sh`
  (exact 100/100/100, miss == 0, selftest green), `cargo deny check licenses`.
- The declaration walk (:273+) holds in its widened form: every `src/**.rs`
  file is a candidate declaring its parent, `#[path]` targets are
  basename-normalized, `include!`/unguarded-`mod`/any-visibility arms refuse.
- The extract seam is honest: `REF_MAX = 40` scanned via
  `char_indices().nth(REF_MAX)` — 40 characters, structurally ≤ 160 bytes;
  exact-count tests inline and deterministic; `src/webhook.rs:497` still
  `for attempt in 0..4u32`; `RefScanStats` value-typed, no globals.
- The webhook end-to-end posture: 5 s POST timeout, four attempts with
  100/400/1600 ms backoff, drop-and-count on overflow, deliveries owned by the
  dispatcher's task (`store.rs:482–490`). **Honest sentences** (recorded at
  robustness's request, so they are not rediscovered at the next review):
  *duplicates are possible; a vanished `250` is not* (accept == stored is a
  storage promise; a failed commit lands as never-acknowledged — the sender
  retries into a duplicate, answered as a transient `451` per RFC 5321
  §4.2.2, distinct from chaos's fixed line); *a delivery can be lost — counted
  in the `dropped` ledger row, not retried forever; the loss being observed is
  the requirement*; *pruning stays oldest-only, including catch-up on restore*.
- `.brokkr/` is gitignored, the working tree is clean, and the next free
  decision number is 0034 (last committed: 0033).

## The ruling — synthesis of the panel

The seats agree on the species (gate-integrity, HIGH direction, repair in this
run), the evidence discipline (red probe per defect, selftest as the durable
net), and the refusal list (no Rust rewrite, no counting stage, no new file,
no decision document). They conflict on three load-bearing points; each is
ruled, not averaged.

1. **Adopt robustness's rule: prove-or-refuse at the quote.** The raw-open
   prefixes are a closed, enumerable set — `r`, `br`, `cr` — each at an
   identifier boundary; the `#` run is counted exactly, uncapped. This kills
   R1, R2 and the clamp (the lookback no longer indexes below the line start,
   so there is nothing to clamp). Simplicity's probe I joins the same fix.
   Robustness's refusal of a Rust rewrite and of a counting stage is adopted
   as-is: the shell scanner stays auditable, the arms rebuild every run, and
   only the decision's direction was wrong.
2. **Reject simplicity's binding statement 1** — "the one production-side
   change is a 3-line index guard + two selftest arms, nothing else ships."
   Its premise is live-falsified: §5 claims ">8-`#` spellings err toward
   *emitting*", but the reproduction shows the cap resolves toward **erasure**
   (R2). And the 3-line guard cannot see R4 at all (no `sraw` distinction) —
   so shipping only it would leave two live HIGHs open. **Cost of the losing
   side:** the diff grows from 3 added lines to one rewritten function
   (~90 lines of awk replacing ~75), and the arm budget grows from 2 to 7.
   What is *not* lost: no new file, no new vocabulary, no new stage, no Rust,
   no decision document, ship in this run — simplicity's discipline holds
   everywhere the evidence permits it.
3. **Supersede robustness's mechanism** — a 16-character sliding window plus a
   refuse-on-undecidable arm — **with the exact O(1) lookback and streamed
   emission.** Two reasons, both measured, not argued: (a) a window cannot
   count a run longer than itself, so with the cap removed it must *refuse*
   legal long-run raw strings — the refuse arm is itself an unresolved guess,
   just pointed loud; the exact tracker has no unresolved case at all, so the
   rule (prove-or-refuse) is satisfied more strongly than prescribed; (b) the
   sliding window keeps `pending` — but the measurement above shows the
   quadratic lives in `out` as much as in `pending`, so the windowed repair
   leaves R3 factually unclosed. Streaming deletes both accumulators.
   **Cost of the losing side:** robustness's explicit refuse arm disappears —
   there is nothing left to refuse — and the window's familiar shape goes with
   it. The refusal *discipline* survives as fallback law: if a future grammar
   ever makes the tracker's state insufficient, the scanner must refuse loud
   and name the file; erasure stays the one forbidden arm.
4. **Arm budget — 7 arms, each justified by a probe or a moved boundary, none
   speculative.** One adversarial refuse arm per red shape (cr#, 9-hash,
   column-0, multi-line `r"…"`) — each red at `f815366`, each refusing after
   the repair, each the *next* adjacent regression's tripwire in the spirit of
   robustness's H/I. Three keep-direction arms pin the moved boundary: the
   column-0 output-equality arm (simplicity's arm 2, red today), the 8-hash
   boundary keep (the old cap's edge stays modelled — the 9-hash arm is its
   discriminator), and the identifier-ending-in-`r` control. Simplicity's
   "arms only as red probes demand" is honored: no speculative spellings.
5. **Adopt simplicity's discipline in full where the positions don't
   conflict:** red-first implementation (the regression is *proven* by
   recording the arm red at the base before the repair lands), one production
   file, no new architecture, no decision document, and any *further*
   residual found while landing gets the one-probe/one-arm/one-guard
   treatment rather than a scope growth (simplicity's binding statement 3,
   adopted verbatim).

## The design

`strip_rust_comments` is rewritten around two invariants. The reference body —
built and verified at the chief seat — is in [plan.md](plan.md).

**1. The lookback is exact, so the quote decision is a proof.** Four scalars
replace the `pending` string and the cap: `run` (the length of the `#` run
ending at the cursor), and `p1`/`p2`/`p3` (the character before that run, and
the two before it), maintained per normal-state character — `run++` on `#`,
otherwise shift and reset. At a quote the opener proof is:

```awk
if (p1 == "r" && ((!isident(p2) && p2 != "b" && p2 != "c") ||
                  ((p2 == "b" || p2 == "c") && !isident(p3)))) { … raw … }
```

— the closed prefix set `r`/`br`/`cr`, with the identifier boundary checked
**before the whole prefix** (single `r` bounds at `p2`, two-char prefixes at
`p3`). This is where the naive reading (`!isident(p2)`) fails: it would reject
`cr#"`. The tracker is exact for any run length, so no case is undecided and
no refuse arm is reachable; the boundary check reads scalars, never
`substr(line, negative)`, so the mawk clamp cannot recur even in principle.

**2. Raw and plain strings are distinguished from open to close.** A new
`sraw` flag is set at every open: raw strings have no escapes (the
backslash branch is guarded with `!sraw`) and **span lines** (the :152
force-close becomes `if (instr && shash == 0 && !sraw) instr = 0`). The
closer check for `shash > 0` counts the run forward from the quote, exactly,
uncapped. This kills R4 and the escape-shaped over-emission in one stroke.

**3. Emission streams; nothing accumulates per character.** `out` and
`pending` are deleted. The scanner emits each kept span directly
(`function emit(s) { printf "%s", s }`), prints exactly one newline per input
line, and keeps only O(1) state — so a hostile multi-megabyte line costs
linear work (measured: 4 MiB → 1.1 s, vs > 60 s at the base). Line structure,
comment stripping, nesting, char/lifetime handling and the `//` break are
byte-identical to the base everywhere the base was right — verified across
16 probe shapes and all 20 files of `src/`.

**4. The comments are trued.** The decision comment states the closed prefix
set, the exact run count and the tolerated divergence (over-*emission* — a
comment stays visible, loud, and the `SF:` cross-check still binds). The
false sentences are gone: no comment in the function claims a bound the code
does not enforce.

The selftest (:523+) gains the seven arms of ruling 4 — four
`expect_refuse … check_no_coverage_off` arms over fresh fixture trees (the
established pattern, :560–571) and three `expect_eq` output-equality arms on
`strip_rust_comments`. No timing arm is added: R3 is closed structurally, not
by clock (decision 0020 §5). All existing arms stay untouched and green.

## Acceptance Criteria

1. **Red→green refuse arms.** Four new selftest arms — `cr#"…"` with `/*` in
   content, a 9-hash raw string, a column-0 raw string, a multi-line `r"…"` —
   each with a `#[coverage(off)]`/item hidden behind the shape. Each arm
   **fails at the base** (`f815366` — the shape erases the attribute and the
   check passes) and **refuses after the repair**. This is the epic's
   regression-per-child criterion: the proof of the red is recorded in the
   landing notes.
2. **Keep-direction arms.** A column-0 `r#"say "/*" hi"#;` fixture strips
   byte-identical to its input (a following `fn visible() {}` survives); an
   8-hash boundary fixture with `/*` inside the content keeps the code after
   it and stays byte-identical; an `outer"/*"x";` fixture keeps a following
   `fn` (identifier ending in `r` is not an opener). All three green at the
   base except the column-0 equality arm, which is red at the base and green
   after.
3. **No regression in the durable net.** Every existing selftest arm passes
   unchanged — the `coverage(off)` spellings (plain/comment/whitespace), the
   leak check, hidden-harness-dir check, declaration-walk arms, counting and
   preflight arms, and the SF: cross-check arms — with the selftest still
   fail-closed.
4. **Linear work, proven structurally.** `strip_rust_comments` contains no
   per-character string accumulator: the emission is streamed and the
   lookback state is a fixed set of scalars. Verified by review of the diff
   plus a run-local hostile-line measurement (≥ 1 MiB single-line literal:
   base > 45 s, repair < 2 s). **No timing assertion is added to the
   selftest** (0020 §5) — the measurement is design-phase evidence, recorded
   in the landing notes, not a test.
5. **True comments.** The decision comment names the closed prefix set, the
   exact run count and the over-emission tolerance; the "capped" claim and
   the "only ends early — loud, never an over-pass" claim are gone. A grep
   over the function finds no sentence asserting a bound the code lacks.
6. **All six house gates pass at the repair commit**, with `export
   PATH="$HOME/.cargo/bin:$PATH"` and the coverage gate run with
   `TMPDIR=/home/valentin/scratch/cov-tmp`: fmt, clippy `-D warnings`,
   `cargo test`, `brokkr compile --bundle bundles/self`,
   `scripts/coverage-gate.sh` (selftest green, **exact 100/100/100,
   miss == 0**), `cargo deny check licenses`.
7. **Scope holds.** The only changed tracked file is
   `scripts/coverage-gate.sh` (the function and the selftest arms). No
   `src/`, `tests/`, `bundles/`, `docs/` or dependency changes; no new file;
   no `coverage(off)` anywhere.
8. **Landing and record.** The batch lands on the run's landing branch per
   the usual path (review and push remain the operator's); the landing notes
   record the red-first evidence and the hostile-line measurement. No GitHub
   issue is closed by this seat — the operator cites and closes #101/#102.

## Decision ruling — none minted

No decision document is minted; **0034 stays free.** The ruling rests on four
legs: the three frozen guarantees (0001 sync insert, 0002 subscribe-before-
check, 0003 oldest-only pruning) are untouched by a dev-plane scanner; the
gate's *verdict* semantics are unchanged (exact 100/100/100, refuse arms,
fail-closed selftest — the contract survives intact, only its blind spot is
closed); the operator's own run-12 repairs (`f815366`) landed as doc-free
script fixes, and this is the same species; and both seats independently
ruled no document is demanded — simplicity explicitly ("the next free number
stays 0034"). A change to the decision *comment* inside the function is a
truthing, not a ruling.

## What this batch refuses to do

- **No Rust rewrite of the stripper, no counting stage, no new file, no new
  vocabulary** — the two-net architecture (static checks before toolchain,
  one stripper feeding all of them) is kept, per both seats.
- **No refuse arm inside the stripper.** The exact tracker leaves nothing
  undecided. The recorded fallback law stands for future grammars: if the
  state is ever insufficient, refuse loud and name the file — never erase.
- **No sliding window, no cap, no "approximation by contract" sentence.**
  Approximation is exactly what failed here; the decision is now a proof.
- **No timing test, no `coverage(off)`, no global counter, no sleep-based
  synchronization, no chaos probability in the mid-band** (0020, house
  non-negotiables).
- **No special-casing of fixtures.** The decision rule is fixed; the fixtures
  only witness it.
- **No touching the delivery plane** — `persist.rs`, the mutation boundary,
  the webhook budget, the wire answers. The verified posture above is a
  record, not an invitation.
- **No issue closed by this seat**, and nothing pushed — review, push and
  merge are the operator's.
- **No decision document** — 0034 stays free for the next ruling that needs
  it.
