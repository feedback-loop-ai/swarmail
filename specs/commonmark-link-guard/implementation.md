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

All 22 recorded checks pass on implementation commit
`e5762ee63c7cac6ed340071fe54dab16225bc52c`, scanner SHA-256
`192a2f3674860a1033730c1fc61ac3964a5a68d0daaefb3370c546eaa2d7e3d0`.
The receipt manifest at
`.forge/epic192/issue245/implement-gates/results.jsonl` has SHA-256
`f65dc0318e96acae09029817ba2137db8586a3ee2592d6c8b75c78bd349ed886`.
Every receipt records command, commit, source/log digests, exit status and
elapsed time; each saved log digest was rechecked after the run.

| Proof | Result |
|---|---|
| Four exact regression filters | One executed test each, all pass after repair |
| Independent eval boundary/full suites, fmt, locked all-target clippy | Pass; boundary has 15 tests; full eval exited 0 after 396 seconds |
| Root fmt, locked all-target clippy, locked workspace tests | Pass |
| Candidate-local engine identity and self-bundle compile | Pass; SHA-256 matches `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`; no unread-input warning |
| Exact root coverage | Pass: 14,677/14,677 lines, 822/822 branches, 1,597/1,597 functions; zero misses |
| Coverage selftest, publish guard proof, keyless seal proof | Pass |
| Permissive licenses, fixed shim checks, fail-closed native-tls absence | Pass |
| Design selftest and delivery-boundary proof | Pass |
| Protected-file diff and whitespace checks | Pass; only this guard and its implementation/progress records differ from the design commit |

The canonical pinned bundle scripts were executed after checking the public
shims, with offline Cargo and the candidate-local engine on PATH. The root
coverage report is under `target/coverage/run.95ONr4/`; it measures the root
crate, not this separate eval workspace. Recipe routing is unchanged.

An initial full eval attempt hit the workspace command's 30-second limit,
exit 124, and is retained as interrupted evidence. A detached-driver attempt
produced no completion receipt. Subsequent gate batches ran directly with a
longer command budget. The full eval RPC response timed out at 300 seconds,
but the underlying command finished successfully and its complete log, exit
status and digests were recovered and verified. Neither timeout is counted
as a passing test; the completed 396-second run provides the full eval proof.

The final documentation commit records these already completed proofs;
the scanner bytes remain those of the implementation commit. Its relevant
follow-up checks are the tracked-document boundary suite, formatting and
whitespace checks.

CI retains Helm/chart-bump/history, Docker smoke, network RustSec, privileged
port-80 and tag-only proofs. Issue #201 owns macOS eval evidence; the operator
owns live settings, sealing and decision acceptance. No external proof is
claimed by this implementation seat.
