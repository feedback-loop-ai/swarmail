# Plan — stripper-prove-or-refuse

## Technical context

`scripts/coverage-gate.sh` is one file with three layers: the contract
(:1–56 — exact 100/100/100, miss == 0, fail-closed), the static checks
(`rs_tokens` :157, `check_no_coverage_off` :165, `check_report_covers_src`
:223 — the `SF:` leak cross-check, `check_harness_declarations` :273), and
`selftest()` (:523) with `expect_pass`/`expect_refuse`/`expect_eq` helpers.
All static checks funnel through `strip_rust_comments` (:80–155) — an awk
state machine (`depth` for block comments, `instr`/`shash` for strings,
`inchar` for char literals) whose two guesses (capped substring look-back
:131–135, raw/plain conflation :152) resolve toward erasure. The selftest is
rebuilt every run and its arms are the durable net; fixtures are fresh
`mktemp -d` trees removed by the trap. The host awk is mawk 1.3.4; `substr`
with a negative start clamps to 1 — the hazard behind probe I. The base is
`f815366`, tree clean.

## Constitution check

- **Frozen guarantees (0001/0002/0003) untouched** — this batch changes one
  dev-plane function in one script; no Rust, no bundles, no wire surface.
- **Decision 0020 respected on every axis:** the new arms are deterministic
  (no sleeps, no timing assertions — R3 is closed structurally, measured only
  as design-phase evidence); no `coverage(off)` is introduced; harness names
  stay reserved (`check_harness_declarations` is not weakened — the arms
  still run through it); the walk reads the *stripped* stream, which is
  exactly what stops lying.
- **Decision 0033 and the release discipline untouched**; `.brokkr/` stays
  gitignored — all run-local evidence lives there and is never committed.
- **The gate's own contract intact:** floor still refuses anything but 100;
  the selftest stays fail-closed; the `SF:` cross-check still binds the
  stripper's tolerated over-emission direction.
- **Six gates** (`export PATH="$HOME/.cargo/bin:$PATH"`; coverage gate with
  `TMPDIR=/operator-scratch/cov-tmp`) re-run at the repair commit before
  reporting success.

## Project structure — every file the batch touches

| Path | Change |
|---|---|
| `scripts/coverage-gate.sh` | rewrite `strip_rust_comments` (:80–155); add 7 selftest arms in `selftest()`; true the decision comment |

Run-local, gitignored, never committed:
`.brokkr/state/tmp/chief-12b/` (probe sheet `probe.sh`, candidate
`stripper_candidate.sh`, harness `verify_candidate.sh`, patched-gate dry run
`coverage-gate-candidate.sh`), `.brokkr/state/tmp/panel-12b/`
(`probes.sh`, `stripper_fixed.sh`, `stripper_fn.sh`), positions under
`.brokkr/state/design/positions/`.

## Approach — the load-bearing details

### The reference body (verified at the chief seat)

Replace the body of `strip_rust_comments` (the awk program, :80–155) with:

```awk
    function isident(ch) { return ch ~ /[A-Za-z0-9_]/ }
    function emit(s)      { printf "%s", s }
    {
      line = $0; n = length(line); i = 1;
      p1 = ""; p2 = ""; p3 = ""; run = 0;
      while (i <= n) {
        c = substr(line, i, 1); c2 = substr(line, i, 2);
        if (depth > 0) {
          # Comment interior: emitted as nothing.
          if (c2 == "*/") { depth--; i += 2; p1 = p2 = p3 = ""; run = 0 }
          else if (c2 == "/*") { depth++; i += 2; p1 = p2 = p3 = ""; run = 0 }
          else i++;
        } else if (instr) {
          if (shash > 0) {
            # A raw string ends at `"` followed by exactly shash `#`s.
            if (c == "\"") {
              j = i + 1; cnt = 0;
              while (substr(line, j, 1) == "#") { cnt++; j++ }
              if (cnt == shash) { instr = 0; emit(substr(line, i, j - i)); i = j; p1 = p2 = p3 = ""; run = 0; continue }
            }
            emit(c); i++;
          } else {
            # A raw string has no escapes; a plain string does.
            if (!sraw && c == "\\") { emit(substr(line, i, 2)); i += 2; continue }
            if (c == "\"") { instr = 0 }
            emit(c); i++; p1 = p2 = p3 = ""; run = 0;
          }
        } else if (inchar) {
          if (c == "\\") { emit(substr(line, i, 2)); i += 2; continue }
          if (c == "\047") { inchar = 0 }
          emit(c); i++;
        } else {
          if (c2 == "/*") { depth++; i += 2; p1 = p2 = p3 = ""; run = 0; continue }
          if (c2 == "//") { break }
          if (c == "\"") {
            # Raw-string opener proof at the quote: the # run ending here is
            # `run` and the character before it is p1 — the raw prefix is the
            # closed set r, br, cr, each at an identifier boundary (the
            # character before the whole prefix is no identifier char). The
            # lookback is exact for any run length, so the decision is a
            # proof, not a guess: there is no unresolved case to resolve
            # toward erasure. The only tolerated divergence is over-emission
            # (a comment stays visible — loud, and the SF: cross-check still
            # binds); erasing live code is the one direction this function
            # may not take.
            if (p1 == "r" && ((!isident(p2) && p2 != "b" && p2 != "c") || ((p2 == "b" || p2 == "c") && !isident(p3)))) { instr = 1; shash = run; sraw = 1 }
            else { instr = 1; shash = 0; sraw = 0 }
            emit(c); i++; p1 = p2 = p3 = ""; run = 0;
          } else if (c == "\047") {
            ch1 = substr(line, i + 1, 1); ch2 = substr(line, i + 2, 1);
            if (ch1 == "\\" || ch2 == "\047") { inchar = 1 }
            # else a lifetime: emit the quote and move on.
            emit(c); i++; p1 = p2 = p3 = ""; run = 0;
          } else {
            emit(c); i++;
            if (c == "#") { run++ }
            else { p3 = p2; p2 = p1; p1 = c; run = 0 }
          }
        }
      }
      # Line structure is preserved: exactly one newline per input line.
      print "";
      # String state persists across newlines for every literal kind: an
      # ordinary Rust string legally spans lines, and resetting its state
      # at the newline let a `/*` on the next line open a comment that
      # erased live code (the run 12c reviewer HIGH). An unterminated
      # ordinary string cannot occur in a tree that compiled, so
      # persistence cannot swallow live code of any input the gate sees;
      # raw strings and comments already persisted.
    }
```

State kept: `depth`, `instr`, `shash`, `sraw`, `inchar`, `run`, `p1`, `p2`,
`p3` — all O(1); `line`/`c`/`c2` are per-character reads, never accumulated.
State deleted: `out`, `pending`, `k`, `head`, `before`, `before2`. The
tracker's reset points mirror where `pending` resets today (:95 line start,
:100–101 comment edges, :109 raw close, :115 plain close, :139/`'` :144,
quote open) — the lifecycle is identical, the representation is not.

The prefix arm is subtle and was caught live: the boundary sits **before the
whole prefix**. `!isident(p2)` alone rejects `cr#"` (its `p2` is the `c`).
The correct form bounds a single `r` at `p2` and `br`/`cr` at `p3` — matching
the base's arm-1 logic (:136) extended with the `c` spelling (robustness R1).

### The seven selftest arms (added in `selftest()`, after the existing fixtures)

Each refuse arm is a fresh fixture tree in the established pattern
(:560–571); each equality arm calls `strip_rust_comments` directly through
`expect_eq`.

| Arm | Fixture (`src/lib.rs` unless noted) | Assert | Base |
|---|---|---|---|
| `co-raw-cr` | `static S: &str = cr#"abc\n/* "#;\n#[coverage(off)]\npub fn hidden() {}` | `expect_refuse check_no_coverage_off` | red |
| `co-raw-9hash` | `const S: &str = r#########"x"/*"y"#########;\n#[coverage(off)]\nmod hidden {}` | `expect_refuse check_no_coverage_off` | red |
| `co-raw-col0` | `static S: &str =\nr#"say "/*" hi"#;\n#[coverage(off)]\nfn hidden() {}` | `expect_refuse check_no_coverage_off` | red |
| `co-raw-multiline` | `static S: &str = r"say\n/* hi";\n#[coverage(off)]\nfn hidden() {}` | `expect_refuse check_no_coverage_off` | red |
| `raw-col0-keep` | `static S: &str =\nr#"say "/*" hi"#;\nfn visible() {}` | `expect_eq "$(strip_rust_comments …)"` == input verbatim | red |
| `raw-8hash-keep` | `const S: &str = r########"x"/*"y"########;\nfn visible() {}` | `expect_eq` == input verbatim | green |
| `raw-outr-keep` | `static S: &str = outer"/*"x";\nfn visible() {}` | `expect_eq` == input verbatim | green |

Honest notes on the keep arms: `raw-8hash-keep` pins the old cap's edge stays
modelled, but an 8-hash run is modelled even by a re-capped scanner — the
*discriminator* for the cap is `co-raw-9hash`. `raw-outr-keep` cannot
discriminate the tolerated over-emission direction (a misread `outer"` still
emits its content); it pins the documented control so the shape is witnessed.
Neither is a red probe; both are boundary pins, per ruling 4.

### Red-first protocol (the regression proof)

1. Extract the base function verbatim (`sed -n '80,155p'`) and run the chief
   harness (`.brokkr/state/tmp/chief-12b/verify_candidate.sh` pattern) —
   record the five red shapes and the green controls at `f815366`.
2. With the base stripper still in place, run the **new arms** and capture
   each `expect_refuse` failure — the arms must fail *at the base* (the
   shape erases the attribute) — that output is the landing note's proof.
3. Only then land the function rewrite and watch each arm flip to its
   post-state.

### Output-identity property (why this is safe to land)

The repair changes output **only where the base output is wrong**. Verified
at the chief seat: 16 green probe shapes byte-identical to the base; all 20
`src/**/*.rs` files byte-identical; the five wrong shapes (R1, R2, I, I′,
R4) now keep live code, and the escape-shape over-emission
(`r"a\"; /* real */`) is repaired. The implement seat re-runs the same
harness before committing.

### Verification sequence

`bash scripts/coverage-gate.sh selftest` (all arms, old and new) → the six
house gates with the standing env (`PATH` + `TMPDIR`) → `cargo deny check
licenses` → `brokkr compile --bundle bundles/self` → hostile-line
measurement re-run and recorded in the landing notes → landing per the run's
path (review, push, merge are the operator's; no issue closed by this seat).

## Risks

- **A hand-rolled awk regression.** The tracker is state-machine surgery in a
  language with no tests of its own. *Mitigation:* the byte-identity
  property (16 shapes + whole `src/` tree), the 30+ existing arms, the 7 new
  arms, and the panel's own probe sheet all re-run before landing; the
  selftest rebuilds every run, so the net is durable, not remembered.
- **Divergence in garbage-Rust corners.** Malformed input may over-*emit*
  (a comment stays visible). That is the tolerated direction and it is loud
  — and the `SF:` cross-check still binds the token stream. Over-*pass* has
  no reachable path: the tracker decides every quote from exact state.
- **Scope creep.** The change is one function + arms. Any further residual
  found while landing gets one probe, one arm, one local guard
  (simplicity's binding statement 3), or it is deferred with the evidence
  recorded — never silently absorbed.
- **awk portability.** The reference body uses only POSIX awk (`substr`,
  `printf "%s"`, `length`); it no longer indexes below 1 anywhere, so the
  mawk/gawk clamp family is unreachable. The arms run under the gate's own
  awk on every host that runs the gate.
- **The measurement is host-dependent.** The 49.9 s / 1.1 s figures are
  mawk-on-this-host facts, not contract. The contract is structural (no
  per-character accumulator), which is what AC-4 checks; the numbers are
  recorded as evidence, never asserted in a test.
- **Arms grow the selftest** by 7 fixtures (~40 lines). Removed by the
  existing trap; each is fresh-tree isolated, so no cross-arm state.
- **The `cr` spelling is new to the scanner.** C-string raw literals do not
  appear in this tree today, but the prefix set is closed per the Rust
  reference, not per this tree's contents — the next `.rs` that uses one
  cannot silently erase anything after it.
