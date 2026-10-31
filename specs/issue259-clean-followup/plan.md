# Plan — Issue #259 clean follow-up

[Specification](spec.md) · [Ordered tasks](tasks.md).

## Technical context

One production Rust crate (`src/`) with real-protocol integration suites under
`tests/`, plus the standalone `evals/` workspace that the root gates neither
build nor measure. The report path is `deliverability::collect` →
`finish` (`src/deliverability.rs:847`, `:133`) over stored bytes, lazily, off
SMTP, behind the four-worker service and the 128-slot success-only cache. The
HTML collector is `src/deliverability/html.rs` (`collect_with` at `:118`;
`Context` frames at `:200`; `Work` at `:178`; URL helpers at `:514`–`:570`;
`terminated` and `tag` at `:588` and `:603`). No dependency, schema, rule,
version, executor, persistence or route change is planned.

No plan template exists under `.specify/templates/` (only `spec-template.md`),
so this document carries the dialect's sections directly, as the previous
design did. The intake task directory `.brokkr/state/tasks/` does not exist;
the intake results `d3b32236` and `1ffcbe72` and the recorded feature text are
the framing.

## Constitution check

- 0001–0003 and 0005 untouched: the plan diffs `src/store.rs`, `src/smtp.rs`,
  `src/persist.rs`, `src/model.rs` and `src/extract.rs` against `61320c6` and
  requires an empty result (spec AC10).
- 0020 placement: any repair test goes inline in the owning module when
  branch-stable, into the declared `concurrency_tests.rs` harness only for a
  cold path, and into `tests/deliverability.rs` for wire shape. No seam becomes
  a feature flag; no `coverage(off)`.
- 0045 §8: compatible work under proposed 0085/0089/0106/0107 may land; a
  semantic defect returns to design.
- 0056: commits are unsigned; nothing signs mid-run; nothing is pushed.
- 0050 §1 and §3: gates run from the pinned bundle; the design is identified
  by the chief's HEAD commit, which touches exactly `specs/issue259-clean-followup/`.
- 0057/0058 and the directed 0081 routing: the run uses `mixed-opus-terra`;
  this plan changes no recipe, adapter or fallback.

## Research and settled approach

The chief confirmed on tree `8750196`: the patch hash equality; a clean
worktree; `RULE_SET_ID = "swarmail-html/3"`; eleven `html/2` occurrences in
seven files, all historical or labelled historical; the decision index rows
0081–0107 contiguous with the 0107 row present; 0081's record, index row and
0058 pointer saying proposed; `evals/Cargo.toml` assigning standalone gates to
#201; README, llms.txt and OpenAPI describing the current empty-message
failure; every test named in the spec present at the cited lines. These are
readings, not executions.

Settled approach: **re-prove, control, classify, repair-on-reproduction.**

1. Re-prove the preserved Opus delta by reading it and running it.
2. Run the two disposable-copy mutation controls (declaration context, counted
   work) and bind their hashes to the candidate.
3. Classify every earlier finding and html/2 occurrence against the final tree.
4. Repair only a defect reproduced with a concrete failing input; extend the
   owning test; commit unsigned before `complete`.

Rejected: a permanent fault seam or rescan hook; a runtime work quota; a
tighter fitted `C`; a CSS tokenizer crate or variable resolution; an aggregate
empty-success shortcut, SMTP rejection or synthesized headers for empty mail;
a second empty matrix or a fourth set of CSS pairs; edits to 0081, the 0085/0106
pointers, historical specs or the appended implementation note; new
coordination tooling for #260 or the eval window.

## Project structure and files

| Path | Treatment |
|---|---|
| `specs/issue259-clean-followup/{spec,plan,tasks}.md` | The chief's commit, exactly. |
| `tests/deliverability.rs`, `tests/stdio.rs` | Read the `61320c6` delta line by line; run; change only to add an unmapped empty-matrix cell or a missing whole-report comparison (spec AC6, AC7). |
| `src/deliverability/html.rs` | Read `Context`, `Work`, URL and `terminated`/`tag` helpers against the census; run inline tests; mutate only on a disposable copy. Production change only on a reproduced defect, with its inline test. |
| `src/deliverability.rs`, `src/deliverability/html-rules.json`, `tests/fixtures/html-compatibility.json` | Identity and catalogue owners; read-only unless a test fails. |
| `src/store.rs`, `src/smtp.rs`, `src/persist.rs`, `src/model.rs`, `src/extract.rs` | Must be byte-identical to `61320c6`. |
| `README.md`, `src/static/llms.txt`, `src/static/openapi.json` | Served-docs owners; read-only. The html/2 historical sentence stays. |
| `docs/decisions/0081, 0085, 0089, 0106, 0107`, `docs/decisions/README.md`, `specs/html-review-repair/implementation.md`, `.forge/issue259-review/*` | Byte-unchanged; quoted in the notes. |
| `evals/` | Standalone suite only inside an evidenced nonoverlap window with a captured, hashed executable. |
| `tests/burst.rs`, `tests/p2.rs`, `tests/rate.rs`, remaining suites | Run unchanged under the full gate. |

## Data model and contracts

The report stays `{rule_set_id, findings}` with typed locations, fixed hints,
canonical order and deduplication; identity `swarmail-html/3`; the cache keys on
full immutable `Inputs` plus weak selected-mail identity and admits successes
only. Admission fails on empty raw before bulk and HTML run; HTML's own empty
branch only contributes nothing. Four states stay distinguishable on the wire:
empty report, Unknown, Failed, Unavailable.

Empty-message matrix the implementer must map to exact assertions:

| State and operation | Expected outcome |
|---|---|
| Stored zero raw, `GET …/report`, repeated | `503 {"error":"deliverability analysis failed"}` both times; nothing cached |
| Stored zero raw, MCP `swarmail_get_deliverability_report`; stdio | `isError: true`, text exactly "deliverability analysis failed" |
| Stored zero raw, exact assert, each threshold, count 0 and 1 | analysis failed, never a clean or partial count |
| Stored zero raw, positive-deadline REST/MCP wait, each threshold, count 0 and 1 | analysis failed; count 0 means minimum one |
| Stored zero raw, zero-time cold wait | `503 {"error":"deliverability analysis unavailable"}`; no job submitted; a previous failure does not become a ready result |
| Truly empty inbox, exact assert | count 0 → `200`; count 1 → `409`; no analysis required |
| Truly empty inbox, wait | `408` with `matched: 0` for count 1; zero-count behaviour as the existing suite records |
| Every case | `250` before analysis; lookup `200`; raw exactly zero bytes; UI retains the message |

Mutation-control evidence record (run-local, never tracked): candidate HEAD and
tree; disposable copy path; `sha256` of the mutated and restored source file;
`sha256` of the mutation patch; exact test command; red log hash and the
failing assertion line; green log hash; `git status --porcelain` of the
candidate empty afterwards.

## Implementation sequence

1. Bind: record HEAD, tree, clean status; reproduce the patch hash equality and
   the packet SHA256SUMS; confirm the pinned brokkr digest against
   `bundles/self/tooling.json` and the shim hashes.
2. Read the `61320c6` delta in both test files; confirm whole-object comparison
   and literal oracles; write the empty-matrix cell map. Extend only an unmapped
   cell or a subset comparison, in the owning suite.
3. Run focused suites, then the declaration-context and counted-work controls
   on a disposable copy under a fresh `CARGO_TARGET_DIR`. Restore byte-exactly;
   confirm the candidate is untouched.
4. Classify: re-run the html/2 grep, render the decision index table, quote both
   0081 statements, confirm the appended note is unchanged.
5. If step 2 or 3 reproduced a defect: repair in the owning function, extend the
   owning test, re-run steps 2–4, commit unsigned.
6. Run the full gate inventory from the pinned bundle on the final tree; record
   command, exit and log digest.
7. Evals: only with same-user nonoverlap and cleanup evidence; capture and hash
   the executable before and after; otherwise record unresolved.
8. Return `complete` with the notes the spec's criteria require; the engine
   owns hermetic verify and the protected five-member review.

## Validation and quickstart

Gate inventory, run from the pinned bundle with `--locked` and no network:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
cargo tree --locked -i native-tls   # must fail with the package-not-found diagnostic
```

Focused suites that own the spec's criteria:

```bash
cargo test --locked --lib deliverability
cargo test --locked --lib inspected_work_has_a_fixed_linear_bound_separate_from_progress
cargo test --locked --test deliverability
cargo test --locked --test stdio report_stdio_text_contract
cargo test --locked --test bulk_sender
cargo test --locked --test burst
cargo test --locked --test p2
cargo test --locked --test rate
```

Mutation controls (disposable copy only):

```bash
cp -a . "$CTRL" && cd "$CTRL"            # on /home, not /tmp
# control A: collapse Context::Component into Context::Declarations in html.rs
# control B: add one counted repeated suffix read inside terminated() or tag()
sha256sum src/deliverability/html.rs
CARGO_TARGET_DIR="$CTRL/target" cargo test --locked --lib deliverability   # expect red at the named assertion
git checkout -- src/deliverability/html.rs && sha256sum src/deliverability/html.rs
CARGO_TARGET_DIR="$CTRL/target" cargo test --locked --lib deliverability   # expect green
```

Standalone evals, only after the coordination window is evidenced:

```bash
cargo build --locked --release && sha256sum target/release/swarmail   # capture, record
export SWARMAIL_BIN=/abs/path/to/captured/swarmail
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked
sha256sum "$SWARMAIL_BIN"                                                # must equal the capture hash
```

Design handoff: commit exactly the three spec files, unsigned, imperative
title and a body that says why; leave it as HEAD. The pinned
`design-check.sh` rules the artifacts.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| A green re-run is mistaken for new verification | Every receipt is bound to HEAD and tree; candidate-copy gates are observations; only the engine's hermetic verify emits `verified_candidate`. |
| A mutation control contaminates the candidate | Disposable copy, fresh target dir, byte-exact restore, `git status` empty afterwards; no committed hook. |
| The loose bound misses a small superlinear regression | Accepted: the claim is linearity, proved by a fixed pre-measurement bound plus a red/green control at `n = 32768`; a fitted constant would be brittle. |
| The work census is incomplete | Exclusions stated (mail-parser internals, allocation, hash-map cost, RSS, latency); sensitivity is not completeness. |
| Whole-report oracle drifts with the catalogue | Oracle is literal in the test and pinned separately by the snapshot test; cost is fixture maintenance. |
| An unmapped empty-matrix cell hides a false clean | Cell-by-cell map; any gap gets a durable assertion in the owning suite. |
| Living docs still mention html/2 | Accepted and flagged: the sentence is labelled historical and followed by current behaviour; deletion would change served bytes for no criterion. |
| 0081 or a stale pointer is "fixed" by a seat | Byte-unchanged check on the decision files and the packet; both statements quoted with attribution. |
| Eval suite overlaps a live same-user run | Run only with nonoverlap and cleanup evidence; otherwise unresolved. A free trial lock is not evidence. |
| #260 and this candidate diverge | Identities recorded; neither certifies the other; an integrated descendant needs fresh bound proof. |
| Reviewer or verify outage | No fallback, no author substitution; the run stops with the refusal visible. |
| Scope creep into a "robust" rewrite | Repair only on a reproduced defect; everything else is refused in this design. |
