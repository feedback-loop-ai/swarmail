# Implementation plan: extraction fidelity

Implements [spec.md](spec.md) and proposed
[0040](../../docs/decisions/0040-extraction-fidelity.md). This is a design
handoff, not evidence of implementation or passing feature tests.

## Technical context

Single Rust 2024 crate, mail-parser 0.11.9, regex and the existing offloaded
`build_email` ingest path. `src/smtp.rs` currently checks candidate zero;
`src/extract.rs` uses `html_to_text`, Unicode code matching and post-match
reference decoding. Existing local `RefScanStats` measures a 40-character
window. Persistence restores precomputed fields verbatim.

There is no plan template in `.specify/templates/`; its sole spec template
targets a different workspace. This plan supplies the house-required technical
context, constitution check, project structure, research, data model, contracts,
quickstart and risks directly. No new crate, feature flag or schema is planned.

## Constitution check

Extraction remains inside the existing offloaded parse, before synchronous
insert and SMTP 250 (0001/0005). Watchers and pruning remain unchanged
(0002/0003). Raw preservation remains governed by 0022 and POP3's established
normalization/transparency rules. Only extraction semantics change; 0040 is
proposed, never self-accepted. The 0031 amendment is a dated addendum.

Deterministic private tests stay inline, measured under 0020. Wire acceptance
tests use real servers and per-test inboxes. No cold-path harness is needed
for this sequential parsing work. Do not move tests merely to evade exact
100/100/100 coverage. Preserve 0035, all gate scripts, the reference pins and
the license allowlist. No sleeps, timing assertions or shared mutable counters.

## Project structure and files touched

| Path | Implementation responsibility |
|---|---|
| `src/smtp.rs` | Scan actual candidate types, update body-selection documentation and inline MIME cases. |
| `src/extract.rs` | Shared bounded multi-scalar reference decoder, code tokenizer, ASCII code regex, decode-before-match links, local stats and deterministic tests. |
| `src/extract/entities.rs` | Sorted static semicolon-required named mappings, provenance/digest, attribution and regeneration instructions; private data module, no runtime initialization/network dependency. |
| `tests/api.rs` | All #74/#93/#94 SMTP-to-REST cases, complete arrays and raw endpoint assertion using existing helpers. |
| `tests/pop3.rs` | Mixed MIME RETR preservation with existing CRLF/transparency expectations. |
| `src/store/persist_tests.rs` | Existing persistence harness: legacy stored values survive restore; compare fresh derivation of identical raw bytes. Use current persistence helpers and no new schema. |
| `docs/decisions/0031-sender-honest-stored-bodies.md` | Append dated historical-data clarification citing proposed 0040; preserve accepted text. |
| `docs/decisions/README.md` | Index 0040 as proposed. |
| `docs/releases/v0.1.0.md` | Dated extraction-repair entry and explicit historical-data limitation; do not imply the original release already shipped this repair. |
| `specs/extraction-fidelity/{spec,plan,tasks}.md`, proposed 0040 | Exactly four design artifacts for this seat; later evidence/clarifications stay in their owning documents. |

No production changes to persist/store, compat rendering, wire protocols,
listeners, bundles, Cargo manifests/lock or gates are required. Run-local
positions, gate logs, patches and result files remain ignored evidence.

## Research and resolved choices

Read both positions, all three issue bodies, task-directory framing, the
decision index and relevant rulings. Inspection confirms the candidate-zero,
renderer and post-match decode causes. No current intake document exists in
the task directory; use run17-feature.txt and its explicit issue commission.

A local Python standard-library HTML5 mapping cross-check found 2,231 keys,
2,125 semicolon-ended keys, valid `Lt`, and maximum UTF-8 replacement/source
ratio 6/5 (`nLt`/`nGt`). This corrects panel claims, but is not imported data
provenance. Implementation must pin an authoritative snapshot and applicable
license, verify its mappings and document deterministic generation before
adding the data. Do not fetch anything at build or test time. The semicolon
filter is a deliberate syntax rule, not an incomplete name table.

Parser ownership for nested message/rfc822 remains a proof obligation. Start
with the existing `text_bodies()`/`html_bodies()` iterators and actual types;
prove exclusion with the attached-message fixture. If the parser exposes
attached bodies, add the smallest ownership guard at these helpers and pin
it. Do not weaken attachment exclusion or introduce a general MIME walk.

## Data model and implementation approach

Stored Email fields, serialization, IDs, timestamps and raw bytes do not
change. Temporary decoded strings and scanner state never become stored bodies.

1. Keep two explicit sender-body helpers. Find the first matching actual type
   in the corresponding parser iterator, cloning only the selected body.
2. Keep `decode_refs_scanned`, `decode_refs_with_stats`, `RefScanStats` and
   their existing scan accounting. Replace the char-only result with a small
   representation capable of either a numeric scalar or a static named string,
   or append the validated replacement directly. Do not allocate per lookup.
   Binary-search the sorted mapping table using only the bounded token. Keep
   exactly the existing character-window construction and failed-ampersand
   progress. Test every entry, ordering/uniqueness, maximum name length and
   expansion. Named lookup is O(log 2125) with bounded-length comparisons.
3. Implement a private byte-indexed forward scanner for HTML code runs. ASCII
   structural bytes cannot occur within UTF-8 continuation bytes; slice only
   at structural boundaries or string endpoints. Tag parsing tracks before-
   value, quoted-value and unquoted-value state. A small stack for nested
   head/template suppression and a raw-text state for script/style/title are
   enough; no recursive calls or DOM. Only a matching top container close
   pops suppression; mismatched closes stay suppressed. Memory is O(depth)
   for the stack, bounded by input length. A suppressed HTML start tag ending
   in `/>` does not bypass suppression.
4. Emit/scan a text run before entering markup; discard its decoded buffer
   after regex matching. Independently scan the final text run at EOF. In
   suppressed regions parse only enough structure to maintain nesting/raw
   states. Do not look for template/head closes inside script/style/title.
   Ordinary EOF-in-text emits; EOF-in-markup discards the unfinished suffix.
5. Extend local stats with markup/text bytes consumed and actual bounded name
   probe work. Disjoint consumption counters must sum to source length;
   separately count lookahead comparisons so a repeated suffix search cannot
   hide behind an accounting-only byte partition. Cursor progress is monotonic,
   and each probe compares a fixed tag-name width. Keep the existing reference
   stats meanings unchanged. No global observer or feature flag.
6. Compile the ASCII code regex once. Keep plain-first fallback and stable
   deduplication. Remove only extraction's `html_to_text` import/call; leave
   the display callers in `src/compat.rs` alone.
7. Decode one complete HTML source before URL matching. Match plain text
   literally. Scope case-insensitivity to ASCII scheme characters, keep the
   delimiter class and punctuation loop, and deduplicate after both legs.
   Use the decoded source only as regex input; never feed it to the tokenizer.

Work is linear in input bytes with fixed reference/name limits and a fixed
mapping table. URL decoding may allocate at most 6/5 source bytes; code
decoding retains at most one run plus suppression stack and result storage.
This is a bound relative to decoded MIME bodies, not a claim that the SMTP
raw cap equals total process memory or that ingest must become faster.

## Contracts and proof design

Use existing real-server helpers with unique inboxes. Send CRLF MIME via SMTP,
wait for 250, then immediately read the record. Assert entire bodies/arrays,
not membership. Include all issue-table inputs verbatim. POP3 compares the
retrieved payload under its existing normalization and dot-transparency rules,
while `/raw` compares exact accepted bytes.

Inline tests pin tag-entry, quoted `>`, apostrophes in unquoted values,
case-insensitive delimited closes, nested template/head, raw text containing
fake markup, abrupt comments, every unfinished state, literal less-than signs,
encoded markup, 3/4/10/11-digit lengths, Unicode boundaries and decoder errors.
Replace the renderer-shaped assertions and obsolete `&AMP;`/never-grows
assertions; preserve the two reference-work pins unchanged.

Count `"<p>".repeat(500_000)` as 1,500,000 markup bytes and zero text bytes;
count `"<".repeat(500_000)` as 500,000 text bytes. Add exact probe-count
formulas for repeated raw-text near misses and nested suppressed containers,
derived from the actual fixed-width probe algorithm. Assert exact counts,
not elapsed time or a loose upper bound. Enumerate all strings of length up
to five over `< > / ! - = \" ' & ; # a 1 é`, with no panic, valid output and
code results drawn from decoded emitted runs. Add explicit longer multibyte
and malformed-reference fixtures. Enumeration complements the monotonic-cursor
and UTF-8 slicing argument; it does not prove arbitrary-string totality alone.

For the restore residual, construct a legacy persisted row whose derived
fields deliberately differ from fresh extraction, close/reopen with existing
helpers, and assert verbatim restored fields and unchanged raw bytes. Deliver
the same raw source fresh and assert the repaired values. No write-back or
version column is needed to make this distinction observable.

Capture a behavioral failure per child against the current unchanged
extraction or reviewed f39813d. Use an isolated authorized baseline checkout
or a clearly labeled old-behavior mutant, never reset the working tree. A
compile failure is not a regression result. Record SHA, command, expected/
observed arrays and passing repaired result in ignored run-local evidence.

## Quickstart and validation

Implement the ordered [tasks](tasks.md) on a feature branch. Targeted commands:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --locked extract::tests
cargo test --workspace --locked --test api --test pop3
cargo test --workspace --locked persist_tests
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
cargo deny check licenses
```

Use no TMPDIR override; preserve exact coverage in all three dimensions. Any
environment refusal is a blocked gate, never a green result. Implementation
owns protected review and the later PR/check/landing workflow; this design
seat never pushes, merges or closes issues.

Validate all four design files as nonempty, acceptance/risks headings,
checkboxes, resolving local links, and proposed 0040. Stage and commit exactly
those four paths with an imperative rationale and GPG signature. If `.git` is
read-only, preserve the artifacts and an ignored patch, report the missing
commit, and do not redirect Git metadata or claim committed paths.

### Design-seat validation receipt — 2026-10-18

The four artifact structure/link checks, format check (after formatting),
locked workspace clippy, bundle compilation and license check passed on the
unchanged production code. `cargo test --workspace --locked` stopped in the
library suite with 188 passing and 25 failing tests; socket operations were
denied with `PermissionDenied: Operation not permitted`. Coverage was not run
because it repeats those denied real-server tests. These results do not prove
the proposed feature, whose implementation tasks remain unchecked.

Creating `run17-extraction-fidelity` failed because `.git` is mounted read-only
and Git could not create its ref lock. The four artifacts are therefore
uncommitted; the design patch and test log are preserved under ignored
`.brokkr/state/design/`. No alternate Git directory or permission bypass was
used. A writable Git environment and socket-capable verification are still
required for the requested delivery.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Body candidates include attached content. | Mandatory nested multipart and message/rfc822 fixtures; minimal ownership guard if needed. |
| Small scanner exposes attributes or rejoins digits. | Per-run regex, explicit quote states, full-array wire assertions and malformed-input unit cases. |
| Conservative head/template suppression drops real codes. | Named malformed-input policy; tests pin omitted close behavior; raw mail stays available. |
| Hostile near misses rescan suffixes or split UTF-8. | Monotonic byte cursor, fixed-width probes, exact counters, multibyte cases and deterministic enumeration. |
| Full entity set changes assumptions and adds data maintenance. | Snapshot digest/license/provenance, full mapping roundtrip, multi-scalar tests and validated 6/5 expansion bound. |
| Full-source link decoding increases CPU/allocation. | One pass with existing 40-character bounds; one temporary source; no claim of zero overhead or browser validation. |
| Historical fields still mislead after restart. | Explicit 0031/release-note residual and persisted-versus-fresh test; no silent claim of backfill. |
| Added tests/data/helpers evade coverage. | Inline deterministic tests, measured production module, unchanged 0020/0035 gates and exact 100/100/100. |
| Git/toolchain permissions block delivery. | Preserve artifacts, patch and exact failure; do not claim a commit or passing gate that did not occur. |
