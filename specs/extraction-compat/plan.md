# Implementation plan — extraction and compat

Implements [spec.md](spec.md) and proposed
[0049](../../docs/decisions/0049-extraction-compat.md).

## Technical context and research

Single Rust crate, mail-parser 0.11.9, regex extraction, Tokio/Axum HTTP,
SQLite persistence and real SMTP/HTTP tests. Existing owners were inspected:
`src/extract.rs` has `tag`, `markup_at`, `html_codes`, bounded `parse_ref`
and `RefScanStats`; `smtp.rs` selects genuine bodies but loses their identity;
`compat.rs` has shared prefix refusal, scoped candidates and `page_work`;
`Store::offload` already carries pages. The existing held-page proof is
`src/compat/page_work_tests.rs`, with an instance-local gate and real deadline.
`scripts/gen-entities.py` already accepts a WHATWG snapshot and emits sorted
semicolon-ended mappings. Reuse these owners and confirm mail-parser's
child-ID and byte-offset accessors before coding; do not infer MIME ancestry
from the mere presence of an alternative anywhere in the message.

The source snapshot is an implementation input, not evidence fabricated by
this design. Obtain an operator-provided or permitted cached WHATWG snapshot,
verify its digest and license, and retain it as
`scripts/data/html-entities.json` for offline reproduction. No network build
step or CPython substitution. Missing input blocks AC2, not a rationale to
weaken it. Verify all mappings against that independent JSON, not just a
table-length assertion or comparisons of a table to itself.

## Constitution check

Preserve synchronous insert/commit before SMTP success (0001/0014), subscribe
before scan (0002), acceptance-order pruning (0003), ingest-only derivation
(0005), exact-count assertions (0010), raw/body honesty (0022/0031) and
off-worker parse/commit boundaries (0023/0030). The new decision proposes
public extraction/refusal/view-order changes without editing accepted text.
No dependency or allowlist change is planned. Keep exact 100% measured
coverage and 0020's inline default; only cold diagnostic paths warrant
declared sibling harnesses. No mocks, probabilistic races or sleep-as-sync.

## Project structure and touched files

| File | Implementation responsibility |
|---|---|
| `src/extract.rs` | Shared source-boundary recognition, context-aware URL matching, linear punctuation trim, numeric guards, bounded code collector and inline deterministic tests. |
| `src/extract/entities.rs` | Generated complete table, actual provenance/license and lookup/table proofs. |
| `scripts/gen-entities.py`, `scripts/data/html-entities.json` | Offline reproducible source-to-table generation and independent mapping verification. No runtime/build dependency. |
| `src/smtp.rs` | Preserve chosen body identities, determine direct alternative siblings and source order, route code collection without changing stored bodies. |
| `src/compat.rs` | Refusal-only wrapper recognition, scoped timestamp sort, finished-response offload for read shapes and aliases. |
| `src/api.rs` | Native single-message JSON/raw response construction inside offload. |
| `src/store.rs`, `src/model.rs` | Correct order/cap comments; reuse existing instance-local work seam as necessary. No mutation/storage-order change. |
| `src/compat/page_work_tests.rs` | Extend held-response proof and deterministic store-fixture real HTTP ordering tests. |
| `tests/p2.rs`, `tests/compat.rs`, `tests/api.rs`, `tests/webhook.rs` | Extend existing appropriate suites for real ingest/readback, MIME, query, cap/size and webhook regressions. Locate the existing extraction fixtures before adding cases. |
| `src/store/persist_tests.rs` | Fresh capped-row and historical verbatim-row restart proofs using existing persistence harness. |
| `README.md`, `src/static/llms.txt`, `src/static/openapi.json` | Publish cap/fresh-only scope, exact order and refusal contracts, precise offload claim and named residuals. |
| `docs/decisions/README.md`, relevant older decision tails, current release notes | Index 0049 as proposed, add dated pointers where needed and publish behavior changes; never rewrite accepted rulings. |

The chief's design artifacts are exactly spec.md, plan.md, tasks.md and
0049. All other edits in this table belong to implementation.

## Implementation approach

1. Establish the authorized branch from fetched current main; preserve all
   three previous batches. Record baseline SHA and new regression failures.
   Finish entity provenance acquisition before claiming the extraction repair.
2. Generate the full table using the existing script. Add all-mapping tests,
   exact scalar checks, sortedness and the existing expansion proof. Put the
   nonempty ASCII digit guards before numeric parsing without changing
   `RefScanStats` accounting, scalar policy or decode-once behavior.
3. Extend the existing tag quote-state walker to expose ordered source
   spans to the link consumer. A callback or borrowed spans suffices; do not
   allocate a region object per source byte or collect a second full token
   stream. Share lexical recognition, keep consumer suppression separate.
   Quoted attribute spans use quote-permitting matching after decode;
   ordinary spans retain quote delimiters. Every byte is handled once per
   scan, including unfinished tails; decoding never changes source structure.
   Count candidate parentheses/brackets once and decrement while trimming
   suffix punctuation. Preserve every prior #94 result and work counter pin.
4. Introduce one code-cap constant and one collector owned by extract.rs.
   Check/deduplicate borrowed matches before allocating a new retained string;
   a linear lookup through at most 64 strings is sufficient. Feed independent
   HTML text runs into this collector. Keep existing scan-counter semantics
   even after it fills. Do not build complete per-body arrays then merge them.
5. Keep selected MIME part identity/offset alongside the unchanged strings
   during `build_email`. Scan multipart child lists for a parent containing
   both selected IDs directly and having alternative subtype. Otherwise order
   the selected sources by original byte offset. Pass that transient choice
   to extraction without a persisted MIME tree or Email field. Text preference
   falls back only on zero codes; the cap is shared across the merge.
6. Extend `field_prefix` recognition locally with the finite wrapper policy
   from the spec. Keep accepted query text untouched. Sort scoped cloned
   candidates by the global comparator before filtering/paging; retain the
   current one-pass maximum for latest. Document tz's existing refusal and
   distinguish native acceptance order. No store deque, UUID or stamp changes.
7. Reuse `page_work`/`Store::offload` for all named read paths. Include lookup
   or latest selection where useful, release guards, project/render/copy and
   serialize inside the closure, and return finished Response/bytes. Avoid a
   second blocking hop for latest when one closure can select and render.
   Reuse shared raw/plain helpers and existing error/panic handling. Native
   JSON serialization must finish before returning to Axum. Do not move
   unbounded response serialization into `IntoResponse` on the async worker.
8. Update product contracts, proposed-decision index and dated pointers.
   Reconcile the old extraction spec's incomplete full-table task using real
   generation/test evidence; never claim the earlier interim subset was full.
   Record residual resource exposure and verification receipts accurately.

## Data model and contracts

No persistent schema or public response field changes. `Email.codes` remains
an ordered vector; newly derived vectors have at most 64 distinct elements.
Historical rows stay verbatim. SMTP, seeds, persistence and webhook consumers
share the ingest-derived record. Query-time display rendering never becomes
extraction. MIME relationship/order is transient and affects codes only;
links keep text-before-HTML order.

Compat ordering is descending timestamp then ID, independent of acceptance
order used by native per-inbox listings, POP3, pruning and restore sequence.
Latest uses the same comparator as compat pages, with existing scope errors.
HTTP refusal names the unsupported field; accepted literals keep original
spelling. No new byte budget: MailHog v2's existing 67,108,864-byte inclusive
page limit remains the only affected page-budget contract.

## Proof design

Task test names are proposed names, not claims of existing passing tests.
Use real SMTP/HTTP for issue fixtures and response shapes. For deliberately
reversed timestamps, construct resident messages through the existing
crate-private test fixture and issue real HTTP requests from the in-crate
harness; no public test-only endpoint or oversized-mail scheduling race.
Use unequal IDs at equal stamps, multiple inboxes, offset pages, and a
persistent reopen to prove the order independently of insertion sequence.

Extend the held-page harness's gate at the actual heavy work site. A lookup
gate alone is insufficient. Test current-thread runtime progress during
blocked response construction and verify actual bytes after release, with
HTML-only and native JSON cases. The cold deadline path stays in the
declared `#[cfg(test)]` harness under 0020. An isolated negative control
moving the whole response build inline must fail; a seam left on the
blocking pool while serialization is moved inline must not count as proof.

For the numeric fixture, generate exactly 1,048,576 distinct values starting
at 1,000,000, each followed by one space (8 MiB total). Use short fixed ASCII
headers and one inbox/message, so page size thresholds are stable. Native
lists contain full Email bodies: pin <9 MiB there, <16 KiB on Mailpit's
summary page, and <40 MiB on the complete MailHog page. Do not remove bodies
to satisfy a summary-sized bound on native responses.
Pin collector occupancy as well as output length; final truncation alone
must fail the collector proof. Compare a complete real webhook payload and
reopened stored row. Test the documented historical exception separately.

Run regressions in an isolated baseline checkout when available; if a new
private seam cannot compile on `2a325c9`, reproduce only the old faulty
behavior in an isolated negative control and label that substitution. Do
not revert later-batch repairs in the final tree. Record tests by issue in
the implementation commit, including which tests are preservation-only.

## Quickstart and validation

After seeding an isolated inbox, these real HTTP probes check the refusal
and documented timestamp order (the automated fixtures prove the hard cases):

```bash
curl -i 'http://127.0.0.1:8025/api/v1/search?query=-to%3Ax'
curl -i 'http://127.0.0.1:8025/api/v2/search?kind=containing&query=%28to%3Ax%29'
curl -i 'http://127.0.0.1:8025/api/v1/message/latest?tz=UTC'
curl -s 'http://127.0.0.1:8025/api/v1/messages?inbox=batch4&limit=1'
curl -s 'http://127.0.0.1:8025/api/v1/message/latest?inbox=batch4'
```

Run `cargo fmt --all` before every check. Final implementation inventory,
in order: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, `bash scripts/coverage-gate.sh`,
`bash scripts/coverage-gate.sh selftest`,
`bash scripts/publish-guard-test.sh`, and `cargo deny check licenses`.
Coverage must be exact missed == 0 on lines/branches/functions, without a
TMPDIR override. Run the existing fail-closed native-tls absence proof.
Verify owns `brokkr compile --bundle bundles/self` using the pinned binary;
record any sandbox refusal honestly. Keep mawk compatibility and install
nothing in hermetic verification. CI retains chart/advisory/release-rate and
port-80 checks. Do not alter lifecycle test deadlines to hide host load.

Delivery is a signed local commit on `epic120-extraction-compat`, never a
push/PR/merge/issue close. If cargo itself is refused, stop immediately and
preserve `.brokkr/state/epic120-extraction-compat.patch`. Git/signing failures
also require an honest blocker; do not bypass permissions or commit unsigned.
Artifact shape validation proves neither signed delivery nor implementation.

## Risks

| Risk | Mitigation and accepted exposure |
|---|---|
| Encoded quotes reintroduce #94 delimiter failures. | Recognize source boundaries first; per-context matchers; unchanged seven-row regression and exact counters. |
| Tokenizer reuse becomes quadratic or drops links in suppressed HTML. | Borrow/callback source spans, monotone cursor, bounded references, one-pass trim counters; malformed-tail and source-only-link tests. |
| Full table is fabricated or sourced differently. | Pinned JSON with actual SHA-256/license, deterministic regeneration and all-mapping comparison. Missing input remains incomplete. |
| Shared ancestor silently suppresses mixed content. | Direct sibling check with nested-alternative negative fixture; preserve chosen body/raw bytes. |
| Cap only reduces response size, not allocation. | Bound collector before ownership, shared across sources; occupancy/65th-value/duplicate proofs. |
| Historical arrays and links still amplify memory. | Preserve and publish fresh-only cap and uncapped-link exposure; no global-memory claim. A link cap or migration needs separate scope. |
| Clock rollback surprises latest users. | Explicit timestamp meaning, deterministic disagreement fixture, scoped/global consistency; acceptance order remains native/POP3/pruning only. |
| Quoted field spans escape detection or literals change. | Finite prefix recognition on each token, named refusal matrix and literal positive controls; no query grammar. |
| Serialization occurs after offload or aliases retain inline work. | Finished responses inside closure, enumerate aliases, actual construction seam and inline negative control. |
| Blocking-pool contention delays SMTP acceptance. | Reuse awaited pool without store locks; progress proof establishes worker availability, not global latency/admission bounds. |
| Proposal or design validator is mistaken for shipped work. | Keep 0049 proposed, checkboxes unchecked until evidence, separate artifact/gate/commit receipts and disclose environment blockers. |
