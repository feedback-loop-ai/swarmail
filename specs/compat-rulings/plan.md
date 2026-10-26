# Plan — compat rulings

Date: 2026-10-26. [spec.md](spec.md) is the current follow-up requirement;
[tasks.md](tasks.md) assigns each proof. No implementation success is claimed.

## Technical context and research

Single Rust crate; existing Tokio/Axum servers, literal search, source tokenizer,
entity decoder and SQLite persistence. No dependencies or data-model changes.
The repository has no `.specify/templates/plan-template.md`; this plan supplies
its required technical context, constitution check, structure, research, data
model, contracts and quickstart inline, keeping the chief's commit to three files.

Inspection confirms `wrapped_field_prefix` returns a borrowed field and
`refuse_field_syntax` visits trimmed whitespace tokens before generic matching.
`LinkRegions` already partitions ordinary and quoted source; `markup_at` skips
comments without reporting attributes, causing the prefix defect. `tag` reports
closed quote-after-equals values and permits quoted `>`. `links_scanned` visits
text before HTML. `smtp::build_email` calls links independently of codes-only
`BodyOrder`. The existing persistence regression is
`store::persist_tests::restore_keeps_legacy_derived_fields_while_fresh_ingest_repairs_them`.
Reuse these owners and existing real-server helpers. Read both panel positions
and verified intake; the new ruling supersedes D3's ownership restrictions.

## Constitution check

Preserve accepted 0001–0005: synchronous commit/queryability before SMTP 250,
subscribe before scan/recheck, oldest-only pruning and fresh-ingest extraction.
No store/SMTP acceptance or watcher algorithm needs editing. Preserve awaited
offload, raw/body truth, single-writer persistence and unchanged delivery controls.
The operator has authorized the exact 0061 changes; a second proposal would
reopen a decided choice. Recording acceptance is an implementation task, not
new operator acceptance by a seat. No other record changes status.

Tests use real SMTP/HTTP, isolated inboxes, ephemeral listeners and existing
persistent-server ownership. Inline deterministic controls stay measured under
0020; keep cold diagnostics in existing harnesses, never move code to reduce
coverage. No sleep synchronization, probabilistic chaos or timing validity claim.
All gates stay at their current thresholds. Use current candidate bundle routing;
historical D3 reviewer names, paths and engine digests are not this run's settings.

## Project structure and files touched

| Path | Planned work |
|---|---|
| `src/compat.rs` | Recognition-copy normalization and measured inline controls; leave execution staging unchanged. |
| `src/extract.rs` | Conditional-comment quoted-value visitation in existing lexical owner; deterministic boundary/work tests. |
| `tests/compat.rs` | Four named real-protocol tests below, using existing local server/persistence helpers. |
| `src/store/persist_tests.rs` | Extend historical-derived-field regression with Outlook arrays; no production storage edits. |
| `docs/decisions/0061-compat-webhook-proof.md` | Dated acceptance/choices, accepted status, historical text retained and identified. |
| `docs/decisions/0049-extraction-compat.md` | Dated §6 amendment and independent R4.3/R4.4 provenance, historical accepted body preserved. |
| `docs/decisions/README.md` | Only existing 0061 row becomes accepted with operator/date. |
| `specs/epic192-compat-webhook-proof/{spec,plan,tasks}.md` | Replace current pending obligations with decided requirements/proofs; label old D3 material historical. |
| `specs/compat-rulings/{spec,plan,tasks}.md` | Chief artifacts; later evidence summaries and actual completion boxes. |

Read/test `src/smtp.rs`, `src/store.rs`, `tests/api.rs`, `tests/smtp_edges.rs`,
webhook/compat rendering owners and existing extraction controls. No permanent
SMTP/store/webhook edit is expected. A temporary AC11 mutant in the existing link
owner is authorized solely for proof and must be restored exactly. No new fixture
file, production module, generic parser, dependency or license change is planned.

## Implementation approach

1. Add the exact AC10–AC12 wire evaluators before repairs and record behavioral
   red for AC10/AC12. For AC11 record baseline green, temporary source-order red
   and exact-restoration green. Preserve commands and complete logs.
2. In `wrapped_field_prefix`, remove every double quote into a temporary String,
   then strip one parenthesis and one exclusion marker and call `field_prefix`.
   Return a small owned recognized name, or construct the error while the copy
   lives. Keep shared handling for both shims. Never alter `SearchParams`, the
   execution needle, explicit-kind bypass or staging/error precedence. Repeated
   wrappers are not recursively stripped. Update the now-obsolete quoted-leading-
   parenthesis expectation and add input-preservation hit/miss controls.
3. Extend the comment arm of `markup_at` with a local bounded attribute visitor.
   Keep the existing raw comment terminator/abrupt-close rules as envelope owner.
   Recognize a downlevel-hidden conditional opener immediately after `<!--`:
   ASCII-case-insensitive `[if` followed by ASCII whitespace, a condition ending
   in `]>`, and a final ASCII-case-insensitive `<![endif]` immediately before
   `-->` (allow surrounding ASCII whitespace within the comment). Conditions are
   opaque source text, not evaluated; `[iffy]`, encoded openers and ordinary
   comments do not qualify. The exact supplied `[if mso]` form must qualify.
4. Within that qualified payload, walk forward from raw `<` boundaries and reuse
   `tag`'s quote-after-equals ownership on a slice bounded by the payload end,
   retaining original-source offsets for `Regions::value`. A successfully scanned
   inner tag consumes its complete extent; on an unfinished tag/quote, stop this
   inner scan after preserving any already-reported closed values. Do not restart
   from each subsequent `<` in that unfinished suffix. Skip declaration-like
   payload markers without recursion. Only values are reported: never emit inner
   text runs or apply inner start/end tags to outer raw/container suppression.
5. Missing opener/condition terminator/end marker/outer close falls back to the
   established ordinary-comment/unfinished-tail treatment. It does not reinterpret
   arbitrary comments. A malformed inner attribute cannot consume beyond the
   established payload boundary or hide an anchor after the comment. Closed
   values before an unfinished inner tail retain existing tokenizer semantics;
   unassigned source is ordinary link source exactly once. All slice boundaries
   are ASCII positions or source ends. No decoding precedes boundary recognition.
6. Preserve `LinkRegions` partition: ordinary bytes up to a reported value, that
   value exactly once, then resume at its end. Reuse its leading-URL tolerance,
   nested delimiter behavior and deduplication. Do not append a second whole-
   comment scan or delete URL prefixes after extraction. The outer markup remains
   `Kind::Other`, so comments never introduce codes or change outer suppression.
7. Extend deterministic counters at actual new scanning loops/probes, not by
   assigning input length as a proxy for work. Test exact ordinary-plus-quoted
   byte partition, actual scan/probe counts on repeated malformed openers and
   non-ASCII input, one decode per region, bounded end-marker search and monotonic
   cursor advancement. A constant number of linear passes is allowed; no suffix
   rescanning. Preserve existing unrelated exact counters.
8. Keep links text-first and `extract_links(text, html)` unchanged. Add historical
   Outlook fixtures in the existing persistence regression plus HTTP live/reopen
   test; compare exact raw/bodies/arrays and fresh SMTP result. Do not repair old
   rows during any read. Preserve existing non-Outlook historical and code-cap tests.
9. Record the operator's supplied choices as self-contained dated addenda in 0061
   and 0049, including packet identity and attribution. Mark 0061/index accepted
   together; retain dated old paragraphs with an explicit historical boundary.
   Update old current requirements/tasks to refer to the decided follow-up, without
   turning old failed gates into success. Do not copy unrelated ruling rows or
   change their acceptance. No tracked link into the ignored evidence tree.
10. Restore all mutations, run owning suites and every final-candidate gate, commit
    all implementation deliverables unsigned, and leave actual verify/review/ship
    decisions to the engine. This design phase ends with the chief commit at HEAD.

## Data model and contracts

No persisted field or schema changes. Recognition uses an ephemeral String;
attribute source ranges are transient and ordered. Stored `links`/`codes` remain
first-occurrence ordered arrays; links are text-first, codes use the existing
MIME ordering. Historical arrays remain verbatim even when wrong. Ordinary
comments remain heuristic source for links and suppressed for codes. This does
not model browser recovery, Outlook versions or arbitrary condition execution.

The bounded lexical choices above are compatible implementation boundaries
within the operator's selected behavior, not acceptance of additional semantics.
Crash after commit but before reply can still produce client retry ambiguity;
memory-only mail can disappear with process death. Existing post-commit webhook
loss/duplicates, cap eviction and aggregate pool/memory exposure remain unchanged.

## Quickstart and proof ledger

New wire selectors in `tests/compat.rs`:

```bash
cargo test --locked --test compat quote_normalization_is_refusal_only_on_both_shims
cargo test --locked --test compat independent_mime_links_remain_text_first
cargo test --locked --test compat conditional_comment_attributes_keep_complete_urls
cargo test --locked --test compat historical_extraction_arrays_survive_reads_and_reopen
cargo test --locked --lib store::persist_tests::restore_keeps_legacy_derived_fields_while_fresh_ingest_repairs_them
cargo test --locked --test compat
cargo test --locked --test api smtp_issue_136_link_fidelity
cargo test --locked --test smtp_edges
cargo test --locked --lib extract::
cargo test --locked --test store_contract_docs
```

Proofs are sequential during mutation; cleanup restores byte-for-byte snapshots
on every exit. Record mutation patch and original/mutant/restored SHA-256, actual
failing assertion (not compile error), test count, red/green exits and log digests.
The run-local evidence root is `.forge/epic192/compat-rulings/implementation/`;
its `evidence.json` maps AC/test to source identity, command and immutable logs.
Tracked plan summaries contain the actual dispositions without Markdown links
into `.forge/`. The historical persistence unit plus wire test are complementary.

Run all house gates and configured supplemental checks offline with installed
tooling, keeping full outputs and exits. Public shims are checked; execute the
candidate's pinned copies for local preflight. The sequence's externally pinned
check and protected verify verdict remain engine-owned.

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
cargo test --locked --test delivery_recipes
```

Bundle compilation must produce no policy-sweep warning. The native-tls absence
proof runs `cargo tree --locked -i native-tls`: accept only a nonzero exit with
Cargo's explicit `did not match any packages` diagnostic; success or an unrelated
error refuses. Require exact zero missed lines/branches/functions in the coverage
report plus a passing selftest. Missing tooling/cache is reported by name; never
install, use network, lower a gate or infer success. CI/operator evidence named
in the spec stays external to local ACs.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Recognition copy reaches execution | Distinct quoted/unquoted positive/negative mail IDs on both shims; keep staging input immutable. Linear temporary allocation remains. |
| Prefix survives alongside full URL | Disjoint source ranges and singleton equality, including conditional-only case; reject post-extraction prefix pruning. |
| General comment behavior drifts | Qualify raw conditional envelope, ordinary-comment controls and explicit malformed fallback; no DOM recovery. |
| Inner scan leaks codes or corrupts outer suppression | Report values only; keep outer `Kind::Other`; nested head/template/raw controls and following-anchor test. |
| Malformed input creates quadratic work or invalid UTF-8 slices | Bounded payload, monotonic cursor, stop unfinished suffix, ASCII boundaries and actual deterministic work counters. |
| Correct baseline receives fake red claim | AC11 uses a documented temporary source-order mutation; restored source hash must equal original. |
| Fresh-only proof hides historical cleanup | Deliberately wrong stored arrays read live/reopen plus fresh SMTP sibling; old wrong values intentionally remain. |
| Old authority language or receipts mislead | Current requirements updated, dated history retained, acceptance attributed only to operator; no new status for other decisions. |
| Full gates unavailable or fail | Preserve command/log/exit, name exact missing prerequisite or failure and do not claim runtime completion; implementation/verify must renew gates. |
| Scope expands to pool/storage/webhook redesign | Existing suites protect guarantees; unchanged contention and best-effort delivery limitations are explicit. |

## Chief validation — 2026-10-26

The chief ran the entire local house inventory and configured supplemental
proofs with offline Cargo on the inherited code at `9cc3984`, while authoring
only these three Markdown files. All house commands exited 0: fmt, locked
workspace clippy/tests, digest-checked candidate brokkr compile, exact coverage,
coverage selftest, publish guard, seal proof and license check. Shim integrity,
design selftest, delivery proof, recipe tests and documentation-contract tests
also exited 0. Bundle policy sweep reports 76 valuations, 14 groups, 0 unruled.
Native-tls absence returned 101 with exactly Cargo's package-ID no-match
error, satisfying the fail-closed absence proof.

Coverage is **14677/14677 lines, 822/822 branches, 1597/1597 functions**,
zero missed in each dimension, report `target/coverage/run.CfufS2`.
Command/UTC interval/exit/log SHA-256 receipts are recorded in the run-local
`.forge/epic192/compat-rulings/design/gates.json`. The tool transport timed out
while execution continued; completed receipts through 10:03:09 UTC, not the
timeout, establish these outcomes. Relative design links and required headings
were checked. These are local chief checks of unchanged implementation, not
proof that the new behavior exists or a protected verify/review/ship verdict.
Implementation must execute its new regressions and renew final-candidate gates.
