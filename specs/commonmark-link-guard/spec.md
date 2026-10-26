# Specification — CommonMark tracked-link guard

- Created: 2026-10-26; status: draft for implementation.
- Feature: epic #192, issue #245; residual findings from #243.
- Scope: `evals/tests/boundary.rs` in this repository.

## What and why

The tracked-document guard must refuse citations whose local destination
normalizes into repository-root `.forge/`, including the reported CommonMark
forms. Such citations depend on ignored delivery evidence and break outside a
delivery worktree. Existing tracked citations and inert code examples must
retain their classifications. Target existence must never decide refusal.

R1 covers both document-wide backtick matching across paragraphs and premature
masking of backticks inside destinations. R2 and G1 describe the same missing
container reference definitions. G2 describes missing Markdown character
reference decoding, including misreading a numeric reference's hash as a URL
fragment. Repair these three mechanisms with four-finding traceability.

## Authority and scope boundaries

The checksum-verified issue packet and operator rulings were read from
`.forge/epic192/issue245/`. README SHA-256 is
`dba3a19c69a8b7ef6862363eceea04485d886715dc5224f76e4f271f4c0a12f7`;
rulings SHA-256 is
`b68560f6782247377e75e90698f95f33feef390cdbdb71ddd8c9a3cc0fbdfd9e`.
The packet's prior-runs section is empty. Intake is
`.forge/tasks/epic-192-follow-up-issue-245-clo-52600d1d.md`;
the historical `.brokkr/state/tasks/` directory is absent. Both panel positions
were read under `.brokkr/state/design/positions/`. These are run-local inputs,
not tracked citation targets or additional deliverables.

The [tracked operator ruling](../../docs/rulings/epic-192.md#rulings-of-2026-10-26-epic-164)
already requires refusing tracked links into delivery state. This is a repair
of that enforcement, not a new public contract or amendment to a ruling;
there is no new decision declaration. Preserve accepted
[0055](../../docs/decisions/0055-agent-eval-harness.md)'s standalone unpublished
eval workspace, [0050](../../docs/decisions/0050-delivery-gates.md)'s proof
ownership and [0056](../../docs/decisions/0056-signing-policy.md)'s unsigned
seat commits. Reuse the entity data authorized by
[0049](../../docs/decisions/0049-extraction-compat.md), without changing its
production HTML decoder.

No runtime code, storage/SMTP/wait/pruning behavior, ingest extraction,
dependency, lockfile, license allowance, gate floor or recipe routing changes.
HTML and arbitrary renderer extensions remain outside the bounded scanner.
No full renderer, reference-use resolver, filesystem/symlink URL resolution,
namespace work, paid trial or decision acceptance is included.

## Panel synthesis and rulings

| Question | Ruling | Rejected alternative and cost accepted |
|---|---|---|
| Local repair versus structural parsing | Adopt simplicity's single-file scope and robustness's explicit paragraph/container context. Both reject global prefix stripping. | Reject patching the global inline-code mask or splitting only on blank newline pairs. Structural correctness costs a modest block-state model and boundary fixtures. |
| Precedence | Adopt both positions: inline code owns only its eligible block; an admitted destination/title owns its bytes. Parse definitions in block context. | Reject either masking all code first or extracting all links first. A unified traversal is more involved but prevents false refusals as well as missed links. |
| Provenance | Adopt robustness's original byte ranges and physical-line assertions, consistent with simplicity's source-preserving view. | Reject reconstructed offsets after joining stripped strings. Mapping logical content to source adds bookkeeping. |
| Character references | Adopt simplicity's reuse of the pinned complete JSON snapshot and robustness's one-pass Markdown decoding rules. | Reject a tiny name allowlist, copied table, dependency or reuse/change of the ingest decoder. Accept one immutable lookup's initialization and memory cost in the test process. |
| Proof breadth | Adopt robustness's combined-case, CRLF, Unicode and decode-order controls inside simplicity's focused Git-fixture matrix. | Reject helper-only proof and general renderer corpus expansion. Extra focused tests cost runtime, while HTML/extensions remain an explicit limitation. |
| Runtime hardening | Adopt the preservation boundaries in both positions. Robustness's crash, webhook, chaos, TLS and burst discussion is rationale for leaving runtime owners alone. | Reject adding runtime acceptance work to this issue; existing house gates retain that proof responsibility. |

There is no genuine disagreement on the mandatory three repairs. Where the
positions differ in emphasis, explicit block state and interaction tests win
over minimizing scanner lines. Simplicity wins on avoiding an AST, module
framework and runtime decoder coupling. The losing minimal-patch approach
would save code but leave the same lexical-context defects; the losing full
parser approach would buy broader grammar coverage at an unjustified scope
and maintenance cost.

## Acceptance Criteria

1. **AC1 — Test first, all findings.** Before any scanner behavior change,
   add named tests with actual Markdown files indexed in fresh temporary Git
   repositories and execute them through `check_tracked_markdown`. Record
   exact test filters, scanner/source identity, nonzero status and the
   behavior assertion for R1 paragraph scope, R1 destination ownership,
   R2/G1 containers and G2 references. Record those same tests passing after
   repair. Setup failures or zero matching tests are not red evidence.
   Prohibited fixture documents live only in temporary repositories; Rust
   source strings hold fixtures in this repository.
2. **AC2 — Code-span scope and precedence (R1).** Equal-length backtick runs
   in different paragraphs or blocks cannot suppress a real link. Test empty
   and whitespace-only blank lines, LF/CRLF, and container/block transitions.
   A valid span may cross soft line breaks within one paragraph; different
   delimiter lengths and unmatched delimiters follow CommonMark. Backticks
   inside valid bare/angle inline or reference destinations remain destination
   bytes; titles are consumed with their link. Pair refusals with genuine
   code examples and allowed tracked citations. Malformed constructs cannot
   consume later valid links across a block boundary.
3. **AC3 — Container definitions (R2 and G1).** Recognize reference
   definitions within block quotes, unordered/ordered lists, nested
   combinations and their valid destination continuation/indentation forms.
   Apply container indentation to fences and indented code too. Refuse a
   normalized delivery-state destination; accept corresponding tracked
   citations and leave code examples inert. Preserve existing bare-definition
   classifications and the policy of checking definition destinations directly,
   without resolving uses. Prose containing marker-like characters must not
   manufacture a container or definition.
4. **AC4 — Markdown character references (G2).** Decode case-sensitive,
   semicolon-terminated named references using the complete existing snapshot;
   decode decimal references with 1–7 ASCII digits and hexadecimal references
   with 1–6 digits after `x` or `X`. Replace zero, surrogate and out-of-range
   numeric scalars with U+FFFD; preserve other valid scalars without HTML's
   legacy numeric remapping. Unknown names and malformed syntax remain literal.
   Process backslash escapes and references together once: an escaped ampersand
   cannot introduce a reference, and emitted text cannot be decoded again.
   Prove inline and definition refusals for encoded dots/slashes, including
   numeric hashes; pair with allowed destinations, external URLs, actual
   queries/fragments, escaped ampersands, invalid references and decode-once
   controls. Test exact case, semicolon and digit-length boundaries, replacement
   scalars and a multi-scalar named mapping.
5. **AC5 — Classification and source integrity.** Markdown decoding precedes
   scheme/network-path recognition and query/fragment splitting; the existing
   single percent-decoding and lexical root-clamped normalization then apply.
   Entity-produced percent text reaches that one percent stage; emitted
   Markdown escapes/entities are not rescanned. Keep invalid percent-UTF-8
   refusal. Diagnostics retain source filename, original physical destination
   start line and raw target spelling. Exercise Unicode before a destination,
   a continuation destination and a combined container/entity/backtick case.
6. **AC6 — Regression boundary.** Existing inline links/images, nested and
   escaped labels, angle destinations, titles, balanced parentheses, bare
   definitions, code spans/fences, path normalization, root clamping, percent
   encodings, external/anchor/query links and tracked-only inventory retain
   their tested meanings. Inventory/discovery/read/encoding failures remain
   fail-closed. No existing assertion is weakened or test skipped/ignored.
   Root/eval manifests and locks, license policy, production sources, entity
   snapshot and bundles remain unchanged. No runtime or public API changes.
7. **AC7 — Local delivery evidence.** The committed implementation passes
   the independent eval boundary/full suites, fmt and clippy plus every local
   [house gate](../../docs/house-rules.md#gates--part-of-every-change), with the
   digest-checked candidate-local engine, exact zero-miss coverage, coverage
   selftest, publish/seal proofs, shim and native-tls absence proofs. Retain
   command/status/candidate receipts. Root coverage is not eval coverage.
   The plan names commands and additional intake proof scripts. A structural
   design check proves only these artifacts, not implementation completion.

## External proof ownership

CI owns Helm/chart-bump/history proofs, Docker smoke, network RustSec,
privileged port-80 exchange and tag-only rate/publication proofs. Per the
supplied ruling, #201 owns macOS eval evidence. The operator owns live branch
settings, sealing and decision acceptance. These are not new local acceptance
criteria. Seats do not push, merge, tag, publish or close the issue; they do
not read GitHub, CI, journals or sibling worktrees. A missing local input is
reported by exact path without fabricating a pass.
