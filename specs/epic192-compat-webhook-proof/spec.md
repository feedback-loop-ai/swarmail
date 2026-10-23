# Compat, webhook and extraction proofs

- **Date:** 2026-10-23
- **Status:** Draft design; implementation and verification outstanding.
- **Branch:** `epic192/lane-d`; base `168ac299e0fe16ecdd8fa0e0218cb7bc4e0a261c`.
- **Stories:** [#170](https://github.com/feedback-loop-ai/swarmail/issues/170) and [#190](https://github.com/feedback-loop-ai/swarmail/issues/190), including [all five comment findings](https://github.com/feedback-loop-ai/swarmail/issues/190#issuecomment-6284803433).
- **Run:** `epic192-first-wave-lane-d-compat-0f7ecd25`; triage result `engine`, full council and four independent review lenses.
- **Scope:** one candidate and one implementation stream, not epic completion.

Decision: 0061 — [Refusal recognition and extraction repair boundaries](../../docs/decisions/0061-compat-webhook-proof.md), **proposed**.

## Feature and rationale

Make existing search, extraction, rendering and webhook guarantees observable at the places where failures occur. An over-complex query must refuse over real HTTP without blocking async workers. Credential-safe errors must remain useful through actual DNS and TLS failures. Fresh extracted URLs must retain intentional encoded quotes without swallowing adjacent CSS URLs or sentence-ending curly quotes. Proofs must distinguish stored extracted fields from searched source fields, complete response work from serialization alone, and a committed delete prefix from an empty-prefix failure.

The outcome is existing-owner repairs and discriminating tests, with no new queue, cache, executor, schema, dependency, admission policy or general HTML/search grammar. All compatible work proceeds while normative amendments remain pending. A draft, generator receipt or green artifact check is not runtime proof or operator acceptance.

## Authority and scope partition

The [house rules](../../docs/house-rules.md) and accepted [0001](../../docs/decisions/0001-synchronous-insert-lossless.md), [0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md), [0003](../../docs/decisions/0003-oldest-only-pruning.md), [0004](../../docs/decisions/0004-per-test-inboxes-via-smtp-auth.md) and [0005](../../docs/decisions/0005-extraction-at-ingest.md) govern unchanged. Preserve commit/queryability before SMTP 250, synchronous insert, subscribe-before-check/recheck, oldest-only pruning, isolated inboxes, and extraction at fresh ingest. Preserve single-writer protection, FULL persistence, source bodies/raw and verbatim historical arrays.

[0023](../../docs/decisions/0023-off-worker-commits.md), [0044](../../docs/decisions/0044-compat-validation.md) and [0049](../../docs/decisions/0049-extraction-compat.md) authorize awaited whole-operation offload, literal search, existing dialect refusals and complete response construction. [0041](../../docs/decisions/0041-webhook-boundary.md) and [0048](../../docs/decisions/0048-rest-webhook-validation.md) require credential-free diagnostics with informative bounded causes. [0011](../../docs/decisions/0011-shim-delete-contract.md) and the existing ordered per-id store loop retain explicit-selector deletion and its committed-prefix behavior.

R4.1 narrows quote tolerance to a URL beginning a decoded attribute value; nested prose/CSS quotes delimit embedded URLs. This restores lexical ownership while preserving encoded quotes in direct URL values. R4.5 treats U+201D/U+2019 as trailing closing punctuation. Both are compatible clarifications of 0049 §1, to be recorded in a dated addendum and proposed 0061; neither changes source boundaries, ordinary comments or body order.

**R4.2 requires operator authority.** Accepted 0049 §6 explicitly orders one parenthesis, one exclusion marker and one leading quote. Stripping every quote before those steps changes that rule and newly refuses previously accepted literals. The chief rejects calling it only an implementation detail. Proposed 0061 §1 requests the exact amendment; preserve current recognition until accepted. Continue every independent compatible task. This is an additional concrete authority residual identified by synthesis, not a reason to stop the whole stream.

**R4.3 and R4.4 remain pending as directed at launch.** Keep text-first independent-part links and ordinary conditional-comment handling now. No dormant alternative, BodyOrder link API or conditional-comment parser is introduced. Neither this design nor acceptance of the separate R4.2 clause resolves those two questions.

## Panel synthesis

Both run-local positions were read: `.brokkr/state/design/positions/simplicity.md` and `robustness.md`, alongside the current intake and the prior stopped intake. They remain ignored evidence.

| Topic | Ruling: adopt, reject and reconcile | Cost of the losing option / chosen approach |
|---|---|---|
| Search placement | Adopt simplicity's one awaited existing page closure containing staging and scan/build. Reject robustness's optional second compiler handoff absent demonstrated necessity. Both require actual compilation-site proof. | One fallible closure needs careful dialect error mapping; a separate handoff would ease staging isolation but add scheduling and failure states. Pool contention remains. |
| R4.2 authority | Adopt simplicity's normative caveat; reject robustness's treatment as an unconditional refusal correction. Draft the exact amendment in 0061. | The five bypasses stay unresolved until operator acceptance; tests and broad intent cannot override accepted ordered wording. |
| Render proof | Adopt robustness's actual lookup/build observations and serialization-only mutant; reconcile with simplicity's direct Find/Build closure test and existing eleven-route harness. Reject more routes held only at the old gate. | More focused test instrumentation is necessary; a smaller gate-only test would continue certifying the wrong work. |
| DNS/TLS | Adopt both real-client requirements and simplicity's optional private execute/format helper. Prefer invocation-local resolver failure over a new DNS-server framework; require real reqwest resolution and generated TLS verification refusal. | A controlled resolver proves connector/error propagation, not an operating system DNS outage. Retain that scope; no public DNS dependence. |
| Extraction | Adopt simplicity's choice at the existing region boundary and robustness's decode-once/UTF-8/work-accounting constraints. Reject CSS/DOM parsers and global quote stripping. | Heuristic extraction and historical bad arrays remain; malformed browser recovery is not promised. |
| Delete prefix | Adopt robustness's selective middle-id failure, restart and retry; keep simplicity's existing per-id loop and local fixtures. Reject atomic batching. | Failed requests can have committed effects; clients must retry idempotently. SQLite restart fixtures cost test time. |
| Entity provenance | Adopt inherited generator receipt only for identical hashes, plus robustness's still-owed wrong-mapping evaluator mutation. Reject regeneration merely to create a diff. | Receipt identity must be checked; source inspection or a same-count assertion cannot replace full equality. |
| Broader resilience | Keep robustness's crash/burst/hanging-target failure ledger as preservation constraints; adopt simplicity's refusal of unrelated refactors or new infrastructure. | Existing pool/memory pressure, best-effort webhook loss and pending semantic defects remain explicit. This stream does not claim to repair them. |

## Acceptance Criteria

Every item below has a task/test pair in [tasks.md](tasks.md). “Pending authority” means an open story obligation, not an allowed completion claim.

### AC1 — #170 / 2.1: compiler refusal on the wire

A real Mailpit HTTP search sends a recorded over-complex literal that reaches compilation within admitted transport framing and returns HTTP 400 with the existing JSON `error` value `Error: the search needle is too complex`. Parser-size rejection, EOF, panic or generic 400 fails. Preserve empty/malformed query, invalid kind and successful escaped-literal controls; preserve MailHog's bare 400 for empty/unrunnable needles and missing/invalid kind, and its named prefix errors. Replacing compiler refusal with `.expect()` must fail this exact wire test; restoring source must pass. Do not change a production budget/parser limit merely to make a test reach the compiler.

### AC2 — #170 / 2.2: compile away from async workers

Both handlers run user-dependent lowering, escaping and compilation inside awaited existing blocking offload before any store scan. Validation precedence and panic symmetry remain unchanged. A current-thread-runtime proof observes the actual compiler invocation away from the runtime thread and coordinates a held invocation with real SMTP/health progress. Moving compilation back before offload fails that proof. No sleep synchronization or scheduler-dependent correctness ceiling; no new throughput or aggregate CPU bound.

### AC3 — #170 / 2.3: no userinfo in DNS/TLS failure causes

Real reqwest clients exercise separately identified resolver failure and TLS certificate-verification failure through production execute/format logic. Use deterministic per-client resolution and loopback freshly generated untrusted-certificate fixtures, not public DNS, global proxy/resolver mutation, static-expiry assumptions or synthetic chains passed directly to the formatter. Assert intended failure class, retained useful resolver/certificate cause, and absence of distinct username/password sentinels, their relevant encoded spellings and original userinfo. A meaningful mutation that restores credential-bearing URL metadata must fail; a mutant whose request has already lost userinfo is not proof. Restore and rerun green. Keep eight-cause bound, rustls verification, no proxies/redirects, functional Basic auth, CA cache isolation and existing retry/budget behavior. If real DNS proof is impossible, retain the precise security obligation and the issue's 0048 documentation alternative for parent/operator disposition; do not claim closure or edit unowned 0048.

### AC4 — #190 / 22.1: extracted-only values do not match

Insert an isolated legitimate test record with separate unique link/code sentinels absent from every candidate field, including raw. Real Mailpit and MailHog containing searches return zero for each sentinel; assert populated arrays and positive candidate-field controls. Do not use a normal SMTP body containing the sentinel as the sole fixture: raw is searched. Adding links or codes to searched haystacks independently must fail the respective wire assertion.

### AC5 — #190 / 22.2: reproducible entity proof

Retain the inherited actual generator invocation/equality and altered-snapshot refusal only after matching generator, snapshot and entity-module hashes in plan.md. The generated complete NAMED block is byte-equal; a changed snapshot exits nonzero with empty stdout. Execute `all_whatwg_entities_match_snapshot` against a same-cardinality wrong-scalar NAMED mutation: the real evaluator must fail, exact original bytes must be restored, and the evaluator must pass. No permanent table/snapshot/license change is justified by current equality. Date historical T02a/T11/T12 corrections honestly and tick only proven obligations, never whole tasks supported by only a subset of their proof.

### AC6 — #190 / 22.3: all response work is offloaded

Strengthen the existing eleven-route held-response proof with observations at actual lookup/latest selection and projection/render/copy/build sites, plus completed serialization. Include full/latest, plain aliases, raw/header/latest aliases, download and native JSON/raw. Retain body, content-type and missing-id 404 controls. During the controlled hold, real SMTP greeting and health progress; a separate unheld message accepted with DATA 250 is immediately queryable. Release map guards before builds and hold no mutation lock. A mutant explicitly moving lookup/build onto the runtime while retaining serialization and the old gate on the pool must fail. Use invocation/store-local seams only; cold watchdog cleanup belongs in the existing declared harness.

### AC7 — #190 / 22.4: nonempty committed shim prefix

Real shim DELETE receives at least three ordered IDs A/B/C against a real SQLite trigger refusing B alone. It returns named HTTP 503 `storage commit failed`; A is absent, B/C and an unrelated-inbox control remain, live and after reopening the same database with the previous Store released. Remove the trigger and retry the original selector: deletion converges; another retry removes zero without resurrection. Swallowed-error/false-success and prefix-loss/resurrection mutations must fail the owning wire/restart evaluator and restore green. No atomic batch, Store API or persistence behavior change.

### AC8 — #190 / 22.5: name the hang regression

Only the existing emptied-entry regression comment in src/store.rs changes: name the retained map guard during entry removal as the deadlock/hang cause. Retain its live/reopen assertions and executable test. No extra store seam, timeout or algorithm change.

### AC9 — #190 / R4.1: nested quoted URLs remain separate and clean

Fresh SMTP ingest followed by REST readback stores exactly the ordered clean URLs for every row below. Tolerant matching applies only when a URL starts the decoded attribute value after optional whitespace; deeper URLs end at their nested quote delimiter and subsequent URLs are still visited. Preserve `attribute_links_keep_encoded_quotes`, existing #136 wire regression, encoded schemes, interior commas, balanced punctuation, source-only links, decode-once and exact linear work counters. Restoring whole-value tolerance must fail the new wire cases. Append a dated 0049 §1 clarification without changing its historical body.

| HTML fixture | Exact links |
|---|---|
| `<td style="background-image:url('https://x.io/a.png'),url('https://x.io/b.png')">x</td><a href="https://x.io/verify?t=1">Verify</a>` | `["https://x.io/a.png","https://x.io/b.png","https://x.io/verify?t=1"]` |
| `<td style="background-image:url('https://cdn.x.io/hero.jpg')">` | `["https://cdn.x.io/hero.jpg"]` |
| `<td style="background-image:url(&quot;https://cdn.x.io/hero.jpg&quot;)">` | `["https://cdn.x.io/hero.jpg"]` |
| `<a onclick="location.href='https://x.io/go'">` | `["https://x.io/go"]` |
| `<meta http-equiv="refresh" content="0;url='https://x.io/r'">` | `["https://x.io/r"]` |
| `<span title="Visit 'https://x.io/t' now">` | `["https://x.io/t"]` |
| `<td background="https://x.io/hero.jpg" style="background:url('https://x.io/hero.jpg')">` | `["https://x.io/hero.jpg"]` |

### AC10 — #190 / R4.2: quote-first refusals (pending authority)

After explicit acceptance of 0061 §1, both containing searches refuse `"-to:x"`, `"!from:alice"`, `"-subject:a b"`, `to":x` and `"to":x` with HTTP 400 and the named unsupported prefix (to/from/subject as applicable). Remove every double quote from each whitespace token only for refusal recognition, then the existing single parenthesis and exclusion-marker steps. Keep accepted execution input unchanged, URL exceptions, ordinary literal controls and each dialect's supported explicit kinds. Restoring ordered-only wrappers must fail these wire tests. Before acceptance, preserve current implementation and retain this AC open; do not check it off using a proposal or expected-failure fixture.

### AC11 — #190 / R4.3: independent-part link order (pending authority)

The operator must rule on independent-part links. Preserve accepted text-first now. Retain fixture `multipart/mixed[multipart/alternative[text/html "code 121212 <a href=https://app.test/verify?t=abc>"], text/plain "footer 2026 https://lists.test/unsub?u=9"]`. Current links are unsubscribe then verify; codes are `["121212","2026"]`. If source-order is accepted, pass the same transient BodyOrder to extract_links and require verify then unsubscribe through fresh SMTP/REST, with direct-alternative preference, code order, raw/body truth and historical restore unchanged. The old text-first mutant must fail the authorized fixture. Neither scope repair nor this proposal accepts the amendment.

### AC12 — #190 / R4.4: conditional-comment URLs (pending authority)

Preserve ordinary-comment behavior now. The unresolved fixture is:
`<!--[if mso]><v:roundrect href="https://x.io/verify?name=O&#39;Brien&amp;t=abc"></v:roundrect><![endif]--><a href="https://x.io/verify?name=O&#39;Brien&amp;t=abc">Verify</a>`.
Current links are `["https://x.io/verify?name=O","https://x.io/verify?name=O'Brien&t=abc"]`. The operator must authorize quoted-attribute treatment inside conditional comments (then real SMTP/REST stores only the whole URL and old-comment handling fails the regression), or explicitly accept a decision naming this truncation as retained. Neither option is currently accepted. General ordinary comments, code suppression and browser recovery are not implicitly changed.

### AC13 — #190 / R4.5: curly closing punctuation

Trim trailing U+201D and U+2019 along with existing suffix punctuation, removing whole UTF-8 scalars and counting removed bytes accurately. Plain-text and HTML `“https://x.io/q”.` and `‘https://x.io/q’.` store `https://x.io/q`; mixed suffix punctuation, interior curly punctuation, IPv6 and matched parentheses retain their defined behavior. Existing exact statistics and linear work remain correct. Restoring the old trimmer must fail the regression. Choose this owned behavioral fix over the issue's README-retention alternative.

### AC14 — verification, evidence and honest handoff

All compatible criteria require exact commands, exit results, evidence paths, source/input identities and individual mutation-red/restored-green receipts. Preserve accepted guarantees, the exact 100% line/branch/function denominator and all [house gates](../../docs/house-rules.md#gates--part-of-every-change), plus configured design selftest, tamper-boundary, recipe structural and fail-closed native-tls proofs. Use the launcher-pinned engine/toolchain and immutable baseline proofs; no version-equal substitution. All four independent review lenses remain, security holds remain hard stops, and CI-only evidence stays separate.

The chief commit changes exactly this spec tree's three files and 0061. Its committed clean HEAD is the design identity. The deterministic final check alone returns designed. Implementation updates historical artifacts only with actual evidence. Report every finding's disposition, commits/files, tests, preserved rulings, proposal status and cross-lane requests. Do not claim whole #190 or epic completion while AC10–AC12 or any other obligation remains open.


## Implementation disposition — 2026-10-23

Compatible code is committed unsigned at
`a09a5483f1afb114ea09c21caf748c0b246a4608`. AC1–AC9/AC13 have targeted real-client/
runtime and restored-defect proof receipts; AC5 also retains identical-input
inherited generator evidence. The [finding-by-finding handoff](plan.md#implementation-handoff--2026-10-23)
records source identities, commands/exits, mutation/restoration receipts and
paths. No targeted proof becomes whole-story or exact-coverage completion.

AC10–AC12 remain OPEN pending separate R4.2/R4.3/R4.4 operator rulings. AC14
remains OPEN: required workspace tests and instrumented coverage refuse the
missing proposed-0061 index row in parent-owned docs/decisions/README.md.
Other house gates/selftests pass; exact coverage, independent verify/review/
ship, CI-only/external evidence and parent integration remain outstanding.
Phase result: **broken**, never complete or oversized. Proposed 0061 stays
proposed; no historical accepted body, baseline, recipe or frozen guarantee
was weakened to advance this attempt.
