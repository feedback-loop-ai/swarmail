# Implementation evidence — issue #245

The implementation is confined to `evals/tests/boundary.rs`. Source-mapped
logical blocks track quote/list content, paragraph boundaries, headings,
thematic breaks, fences and indented code. Scoped inline traversal consumes
code spans or admitted links in source order; definitions own their raw
destinations and titles. The test-only Markdown decoder uses the complete
existing WHATWG snapshot, consumes original escapes/references once, and
preserves the existing URL/percent/path classification stages.

No runtime source, manifest, lock, dependency, entity data, license allowance,
bundle, routing or gate change is needed. Accepted 0001–0005 remain intact.
HTML and renderer extensions remain outside the bounded convention checker.

## Test-first receipts

The four exact test filters below were each run through
`cargo test --manifest-path evals/Cargo.toml --locked --test boundary FILTER -- --exact`
before any scanner behavior edits. Each executed one test and exited 101;
none failed during fixture setup. The old scanner was byte-identical to
HEAD `91e638c24a1d727ab807db91be2f5ca5e7d14d19`, confirmed by comparing its
complete original file prefix before the appended regressions. The test-first
file SHA-256 was
`1e8bc2cd7dbad9139824856db2d3826b15cb146a8918199a1fcdc7de39a01d51`.

| Filter | Finding and observed old-scanner assertion failure |
|---|---|
| `tracked_code_spans_respect_blocks` | R1: a backtick opener masked a prohibited inline link across an empty paragraph boundary. |
| `tracked_destinations_own_backticks` | R1: masking changed raw destination `../.forge/` followed by backticks and `p` into the truncated diagnostic `../.forge/`; the full raw-target assertion failed. |
| `tracked_container_definitions_are_checked` | R2/G1: the indexed quote definition destination was missed. |
| `tracked_character_references_are_decoded_once` | G2: the indexed decimal-reference destination was missed. |

Raw receipts, command/status records and the test-first source snapshot live
in the ignored `.forge/epic192/issue245/implement-gates/` directory. The small
fixture helper's repair operation was corrected after the red runs to replace
the full raw target, because replacing literal `.forge` does not repair an
entity-encoded target. The refusal and raw filename/line/target assertions
remain intact. Four named regression families and all nine pre-existing
boundary tests pass after repair.

## Additional controls

Real indexed Markdown fixtures cover LF/CRLF, whitespace blank lines, soft
breaks, unmatched and unequal backticks, headings/thematic breaks and
container transitions; bare/angle/definition destinations and titles; ordered,
unordered, quoted, nested and continued definitions; tab/space indentation,
code fences, indented code and lazy/prose controls. Combined Unicode,
container/entity/backtick fixtures assert original physical destination lines
and raw spelling. Growing unmatched delimiter runs prove parser advancement
without wall-clock assertions.

Decoder controls exercise all 2,125 semicolon-terminated snapshot names,
multi-scalar values, case/semicolon boundaries, decimal/hexadecimal digit
limits, Unicode scalars/replacement policy, escaped ampersands, emitted
syntax, external URLs, queries/fragments, one percent pass and invalid
percent UTF-8. Existing grammar, inventory/discovery/read/encoding assertions
are unchanged.

## Gate accounting

The focused 15-test boundary suite and independent eval clippy pass locally.
Full independent eval checks and every local house gate must still run on the
committed implementation. An initial full eval attempt was terminated by the
workspace command's 30-second limit, exit 124; it is not a passing receipt.
The subsequent gate driver uses persistent background execution and records
actual command completion separately.

CI retains Helm/chart-bump/history, Docker smoke, network RustSec, privileged
port-80 and tag-only proofs. Issue #201 owns macOS eval evidence; the operator
owns live settings, sealing and decision acceptance. No external proof is
claimed by this implementation seat.
