# Plan — load-shape-token-stream

## Technical context

`scripts/coverage-gate.sh` is one file with three layers: the contract
(:1–56 — exact 100/100/100, miss == 0, fail-closed), the static checks
(`rs_tokens` :168, `check_no_coverage_off` :174, `check_report_covers_src`,
`check_harness_declarations` :284), and `selftest()` with
`expect_pass`/`expect_refuse`/`expect_eq` helpers. The harness walk (:284)
iterates every harness-named file under `src/`, derives the candidate
declaring parents (`lib.rs`, `main.rs`, `<dir>/mod.rs`, `<dir>.rs`, plus
*every* `*.rs` file — a declaration from any parent counts), and per parent
scans the comment-stripped line-preserved stream for load-shapes before
matching declarations. Two awk inputs exist: `strip_rust_comments` (line-
preserving) and `rs_tokens` (= strip + `tr -d '[:space:]'`, the token stream
the `coverage(off)` match runs over). The load-shape defect is that the
`#[path]` arm (:350–371) anchors to line start and extracts on the same line,
and the `include!` arm (:372–406) is likewise line-based. The host awk is
mawk 1.3.4 — no `?` quantifiers in ERE, `substr` never indexed below 1 in
this repair (the scan uses `match`/`RSTART`/`RLENGTH` forward only). The base
is `bcc7510`, tree clean; the last two commits are unsigned (the tree's
current practice — disclose it).

The reference repair was **built and verified at the chief seat** before this
spec: candidate at
`target/coverage/panel-chief-12c/candidate.sh` (gitignored run-local
evidence), selftest fully green — every existing arm plus the six new — and
the red-proof battery at verbatim `bcc7510` showing all five shapes laundering
(`target/coverage/panel-chief-12c/redproof.sh`).

## Constitution check

- **Frozen guarantees (0001/0002/0003) untouched** — gate tooling only; no
  Rust, no bundles, no wire surface, no dependency.
- **Decision 0020 respected on every axis:** harness names stay reserved —
  this repair *strengthens* the reservation's enforcement; the new arms are
  deterministic shell fixtures (no sleeps, no timing, no chaos); no
  `coverage(off)` is introduced; the walk still runs through
  `check_harness_declarations` unchanged in its declaration semantics. 0020
  is accepted (2026-10-16, 5671e0c) and binds; nothing here amends it.
- **The gate's own contract intact:** floor still refuses anything but 100;
  the selftest stays fail-closed; a scan failure refuses loudly ("could not
  scan $parent") rather than passing.
- **House style:** no unconditional ANSI; `.brokkr/state/` evidence stays
  gitignored; commit cites #101/#102 and closes nothing; commit is unsigned
  per the tree's current practice, disclosed in the result record.
- **Six gates** (`export PATH="$HOME/.cargo/bin:$PATH"`; coverage gate with
  `TMPDIR=/home/valentin/scratch/cov-tmp`) re-run at the repair commit
  before reporting success.

## Project structure — every file the batch touches

| Path | Change |
|---|---|
| `scripts/coverage-gate.sh` | insert the token-stream load-shape pass in `check_harness_declarations`; delete the old line-anchored `#[path]` block (:345–371); add six selftest arms in a new `5i` block |

Run-local, gitignored, never committed:
`target/coverage/panel-chief-12c/` (candidate `candidate.sh`, extracted
`base-lib.sh`, red-proof `redproof.sh`/`keepcheck.sh`, probe batteries
`probes2.sh`–`probes4.sh`), positions under `.brokkr/state/design/positions/`,
this spec under `specs/load-shape-token-stream/` (committed).

## Approach — the load-bearing details

### The reference body (verified at the chief seat)

In `check_harness_declarations`, immediately before the per-parent
`detail="$(strip_rust_comments …)"` declaration walk, insert:

```bash
      # Load-shape scan over the whitespace-collapsed token stream: every
      # #[path] and every include! in the parent, wherever it sits and
      # however its tokens are laid out, is checked here. An argument that
      # is not a plain string literal, an escape-carrying target, and a
      # harness-named target each refuse, fail-closed, naming the parent.
      detail="$(rs_tokens "$parent" | awk -v harness="$harness" -v parent="$parent" '
        {
          rest = $0;
          while (match(rest, /#\[[[:space:]]*path[[:space:]]*=/)) {
            rest = substr(rest, RSTART + RLENGTH);
            if (rest !~ /^"/) {
              print harness ": a #[path] whose target is not a plain string literal cannot be resolved by this walk in " parent " — spell the path literally";
              scan_bad = 1; continue;
            }
            rest = substr(rest, 2);
            q = index(rest, "\"");
            if (q == 0) { scan_bad = 1; break; }
            target = substr(rest, 1, q - 1);
            if (target ~ /\\/) {
              print harness ": a #[path] target with a string escape cannot be resolved by this walk in " parent " — spell the path literally";
              scan_bad = 1;
            } else {
              nseg = split(target, seg, "/"); base = seg[nseg];
              if (base ~ /(^|[_-])tests\.rs$/) {
                print harness ": harness-named file loaded via #[path = \"" target "\"] in " parent;
                scan_bad = 1;
              }
            }
            rest = substr(rest, q);
          }
          rest = $0;
          while (match(rest, /(^|[^A-Za-z0-9_])include[[:space:]]*![[:space:]]*[({[]/)) {
            rest = substr(rest, RSTART + RLENGTH);
            if (rest !~ /^"/) {
              print harness ": an include! that is not a plain string literal cannot be resolved by this walk in " parent " — load the file with #[path] or a literal include!";
              scan_bad = 1; continue;
            }
            rest = substr(rest, 2);
            q = index(rest, "\"");
            if (q == 0) { scan_bad = 1; break; }
            target = substr(rest, 1, q - 1);
            if (target ~ /\\/) {
              print harness ": an include! target with a string escape cannot be resolved by this walk in " parent " — spell the path literally";
              scan_bad = 1;
            } else {
              nseg = split(target, seg, "/"); base = seg[nseg];
              if (base ~ /(^|[_-])tests\.rs$/) {
                print harness ": harness-named file included via include! in " parent;
                scan_bad = 1;
              }
            }
            rest = substr(rest, q);
          }
        }
        END { if (scan_bad) exit 1 }
      ')" || rc=$?
      if [ "$rc" -eq 1 ]; then
        any_bad=1
        details="${details}${detail}
"
      elif [ "$rc" -ne 0 ]; then
        refuse "coverage refusal: the harness declaration walk could not scan $parent"
      fi
      rc=0
```

Shape notes, each load-bearing:

- **No anchoring.** `match()` finds an occurrence anywhere in the collapsed
  stream; `RSTART + RLENGTH` resumes after the matched shape, and
  `rest = substr(rest, q)` resumes *at the closing quote* — a `"` cannot
  begin either shape, so no occurrence can be skipped by resuming there.
- **The unresolvable arm is the closed-form rule, pointed loud.** An argument
  that is not a plain `"` literal — `concat!`, `env!`, a macro variable, a
  missing quote — refuses rather than being skipped. This is the include!
  arm's own established rule (run 12b), extended to `#[path]`.
- **The escape arm is unchanged from the base's semantics** — any `\` in the
  target refuses (ruling 3) — it merely now sees escapes *anywhere* in the
  stream, including after a line split.
- **The reserved-pattern regex is the walk's own** — `(^|[_-])tests\.rs$`,
  basename-normalized by `split` on `/` — no new name vocabulary.
- **Fail-closed twice.** `END { if (scan_bad) exit 1 }` for found shapes;
  any *other* non-zero pipeline status (awk cannot run, mawk parse error)
  takes the `could not scan` refusal — the walk never continues on an
  unscanned parent. (`rc=0` is re-armed afterwards so the following
  declaration-walk capture starts clean, mirroring the base's protocol.)

Then **delete** the old in-walk `#[path]` block (base :345–371) — the
`if ($0 ~ /^[[:space:]]*#\[[[:space:]]*path…/)` arm with its same-line
extraction — leaving the action block as `{ lines[NR] = $0; <include! arm>;
if ($0 !~ decl) next; … }`. The line-preserved `include!` arm stays (spec
ruling 5); the dead disjunct at base :385 stays untouched.

Also extend the function's header comment with the run-12c paragraph the
candidate carries (:307–317 there): the scan runs over the token stream
because every token-split load-shape is invisible to a line-based scan and
launders the reserved name — all probed red at the reviewed base.

### The six selftest arms (new `5i` block, after the `5h` arms)

| Arm | Fixture | Assert | Base |
|---|---|---|---|
| `co-stacked-path` | `src/store/persist_tests.rs` (production body) + `src/store/mod.rs` = guarded decl + `src/lib.rs` = `#[allow(dead_code)] #[path = "store/persist_tests.rs"] pub mod alias;` | `expect_refuse check_harness_declarations` | red |
| `co-stacked-clean` | `src/store/ordinary.rs` + `src/store/persist_tests.rs` + `src/lib.rs` = guarded decl of `persist_tests` **and** stacked `#[path = "store/ordinary.rs"]` | `expect_pass` | green |
| `co-split-include` | harness file + guarded decl + `src/lib.rs` = `pub fn x() {}\ninclude\n!("store/persist_tests.rs");` | `expect_refuse` | red |
| `co-hash-split` | harness file + guarded decl + `src/lib.rs` = `#\n[path = "store/persist_tests.rs"]\npub mod alias;` | `expect_refuse` | red |
| `co-split-open` | harness file + guarded decl + `src/lib.rs` = `#[path =\n"store/persist_tests.rs"] pub mod alias;` | `expect_refuse` | red |
| `co-continuation-target` | harness file + guarded decl + `src/lib.rs` = `#[path = "store/persist_\<newline>tests.rs"] pub mod alias;` | `expect_refuse` | red |

Every refuse arm carries a *declared* harness file plus the laundering shape
in a second parent — exactly the base's arm pattern; the keep arm carries a
declared harness file so the scan actually runs (a vacuous pass arm was
caught and reworked at the chief seat).

### Red-first protocol (the regression proof)

1. Extract the base walk verbatim (the reference implementation used the
   `# --- instrumented build location` marker to cut `base-lib.sh`) and run
   the six-arm battery against it — all five refuse shapes **pass** (launder)
   at `bcc7510`; the keep shape passes. Recorded by
   `target/coverage/panel-chief-12c/redproof.sh`.
2. Capture that output in the landing notes (the proof the arms can go red).
3. Land the scan move; re-run the battery — each red shape now refuses; the
   keep shape is unchanged green; the full selftest is green.

### Why the keep direction is safe

The scan adds refusals; it removes none. The only behaviors that change are
the five laundering shapes (now refusing) and shapes strictly implied by
them. Verified at the chief seat: the full selftest — every pre-existing arm,
including the §5h repair pins and the literal-`include!`-of-non-harness-file
pass arm — is green with the candidate; the keep battery (`keepcheck.sh`,
`probes2.sh`–`probes4.sh`) shows the passing probe families (persistent
string state, macro delimiters, `include`-named directory target, escape
spellings' *refusal* direction) unchanged. The two new over-refusal species
named in the spec's honest sentences are loud, named, and unreachable when
no harness-named file exists under `src/`.

### Verification sequence

`bash scripts/coverage-gate.sh selftest` (all arms, old and new) → the six
house gates with the standing env (`PATH` + `TMPDIR`) → `cargo deny check
licenses` → `brokkr compile --bundle bundles/self` → landing per the run's
path (review, push, merge are the operator's; no issue closed by this seat).

## Risks

- **Loud over-refusal in string text.** A string or raw string whose *text*
  contains a `#[path = "…tests.rs"]`- or `include!(`-shaped sequence refuses
  while a harness-named file exists under `src/`. Accepted (spec ruling 2's
  cost), loud, named, and fixed by rewording the literal. The comment-stripped
  stream is exactly why comments do *not* refuse.
- **Whitespace collapse can forge a reserved suffix.** `rs_tokens` removes
  all whitespace; a target spelled `persist_ tests.rs` collapses to
  `persist_tests.rs` and refuses. Accepted over-refusal — the alternative
  (whitespace-preserving comparison) re-opens the laundering race this run
  closes.
- **A vacuous pass arm.** A keep arm with no harness-named file passes
  without exercising the scan. Mitigated in kind: `co-stacked-clean` carries
  a declared harness file (and the chief seat caught exactly this failure
  mode in a first draft of the arm).
- **Hand-rolled awk regression.** The scan is ~50 lines of mawk in a file
  with no unit tests of its own. Mitigation: the durable selftest net (all
  existing arms + six new), the red-proof battery, and the keep-direction
  battery all re-run before landing; the selftest rebuilds every run.
- **mawk portability.** The scan uses only `match`/`substr`/`index`/`split`
  and POSIX classes; no `?` quantifiers, no negative `substr`, no gawk-isms.
  It runs under the gate's own awk on every host that runs the gate.
- **Two sites now check `include!` shapes** (the retained line-preserved arm
  and the token-stream scan). Subsumed but not deleted (spec ruling 5) — a
  future reader may think the line-preserved arm load-bearing; this plan and
  the spec are the record that the token-stream scan owns load-shapes.
- **Scope creep.** One check + six arms. Any further residual found while
  landing gets one probe, one arm, one local guard (ruling 6), or is deferred
  with the evidence recorded — never silently absorbed.
