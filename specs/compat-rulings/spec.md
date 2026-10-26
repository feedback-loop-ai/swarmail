# Implement the operator's compat rulings

Date: 2026-10-26. Phase: design. Implementation and protected verification remain outstanding.

## Feature and rationale

Close AC10/R4.2, AC11/R4.3 and AC12/R4.4 of the
[existing feature](../epic192-compat-webhook-proof/spec.md): refuse misleading
quoted field syntax on both compatibility shims, retain text-first links
across independent MIME parts, and extract complete verification URLs from
quoted attributes in Outlook conditional comments. A zero-result exclusion
search can falsely reassure a caller; a truncated verification URL is unusable.
Correctness requires named refusals and exact stored arrays, not permissive
status checks or URL membership alone.

The operator's **Decision rulings of 2026-10-26** accept
[0061](../../docs/decisions/0061-compat-webhook-proof.md) §1 and separately
choose text-first R4.3 and source-aware R4.4. The supplied request explicitly
requires marking 0061 accepted (operator, 2026-10-26). That is existing
operator authority, not acceptance by this chief. The dated
[intake](../epic192-compat-webhook-proof/intake-2026-10-26.md) records it.
The issue packet README and ruling file were verified against their supplied
SHA256SUMS: respectively `191013a2d7b1f79d6983b7b063f4a51ae6251ea2c03acab1f44b829a4393367a`
and `ea8d9c1045b9a38fe992f7544c482bbf7ecab98f8cae0c42487a23cc5785f897`.
The historical tasks directory is absent; this tracked intake is the framing.
No journal, GitHub or sibling checkout is needed.

This dated design supersedes the earlier *current* pending-authority and D3
ownership instructions only for these three criteria and their proofs/records.
Earlier receipts and dated dispositions remain history. The semantic amendment
already has its numbered record, 0061, and exact operator authority: no new
proposal duplicates or reopens it, and no other decision is accepted. The
implementation updates its status/index and adds dated provenance in 0061 and
0049; this chief commits only the three design artifacts.

## Panel synthesis

Both positions were read in full. They agree on existing owners and the three
operator choices; no disagreement about those choices is invented.

| Topic | Ruling | Cost and rejected alternative |
|---|---|---|
| Refusal normalization | Adopt simplicity's small temporary recognition copy and robustness's strict separation from execution. Share the existing recognizer on both shims. | Input-proportional allocation and newly named refusals are accepted. A general lexer or normalized execution needle adds unsupported semantics. |
| Conditional attributes | Adopt both positions' reuse of `tag`/`Regions::value`. Robustness wins on spelling out bounded malformed-tail behavior and measured work; simplicity wins on keeping that machinery local. | More branch/control proofs than the smallest patch. A second whole-comment scan followed by prefix deletion can lose legitimate URLs and retain the defect; it is rejected. |
| Proof depth | Adopt robustness's conditional-only fixture and historical live/reopen checks, with simplicity's existing server helpers and local fixtures. | Extra discriminating tests are necessary; a singleton obtained solely from the external anchor cannot prove conditional extraction. |
| Ordering | Adopt both: text-first links, codes-only `BodyOrder`, unchanged SMTP extraction call. | Link and code order intentionally differ. Unifying their traversal loses the operator's selected contract. |
| Failure analysis | Retain robustness's preservation constraints; adopt simplicity's two-owner implementation limit. | Existing memory amplification, shared-pool contention, post-commit retry ambiguity and best-effort webhook loss remain. No queue, outbox, executor or broad stress framework is justified here. |
| Records | Adopt both: record only the supplied acceptance, preserve dated history and update current requirements. | Historical wrong arrays and historical failed receipts remain visible; rewriting them would destroy provenance. |

## Acceptance Criteria

These are implementation obligations. No checkbox is closed by this design.

### AC10 — refusal-only normalization on both shims

For a generic containing query, trim as today, visit each whitespace token,
remove **every double quote in a recognition copy**, remove at most one leading
`(`, then at most one leading `-` or `!`, then apply the existing ASCII
identifier-colon recognizer with its `://` exception. Recognized field spelling,
validation/error precedence and dialect-specific errors remain unchanged.

Both Mailpit and MailHog return JSON HTTP 400 naming the field:

| Query | Field |
|---|---|
| `"-to:x"` | `to` |
| `"!from:alice"` | `from` |
| `"-subject:a b"` | `subject` |
| `to":x` | `to` |
| `"to":x` | `to` |

The original accepted query executes unchanged with existing folding/escaping.
Real SMTP fixtures distinguish quoted and unquoted literals; exact result IDs
and counts prove both hits and misses. URL controls and every supported explicit
kind on each shim preserve their existing contracts. Cover existing wrappers,
one-wrapper limits, embedded quotes and field case. The old ordered-only
recognizer must fail the same wire evaluator behaviorally, then the repair passes.

### AC11 — retain text-first links across independent MIME parts

Deliver the exact mixed shape through SMTP:
`multipart/mixed[multipart/alternative[text/html "code 121212 <a href=https://app.test/verify?t=abc>"], text/plain "footer 2026 https://lists.test/unsub?u=9"]`.
Immediately after DATA 250, REST returns links exactly
`["https://lists.test/unsub?u=9","https://app.test/verify?t=abc"]` and codes
exactly `["121212","2026"]`. Selected bodies and raw bytes retain their
existing fidelity contract; direct-alternative text preference still passes.
Do not add `BodyOrder` to links. Current production is correct: obtain red with
a temporary source-order link mutant and green after exact restoration, using
the same evaluator. Do not mislabel the baseline as a defect.

### AC12 — complete conditional-comment attributes

Fresh SMTP/REST delivery of
`<!--[if mso]><v:roundrect href="https://x.io/verify?name=O&#39;Brien&amp;t=abc"></v:roundrect><![endif]--><a href="https://x.io/verify?name=O&#39;Brien&amp;t=abc">Verify</a>`
stores exactly `["https://x.io/verify?name=O'Brien&t=abc"]`. The truncated
`https://x.io/verify?name=O` is absent. The counterpart without the external
anchor must return that same singleton, proving the conditional attribute is
visited. Old conditional-comment handling must fail this wire evaluator,
then the repair passes.

Source quotes delimit each attribute before its single entity decode. A quoted
value cannot also pass through ordinary-source matching. Preserve ordinary
comments/source-only links, suppressed comment codes, encoded quotes, nested
attribute URL delimiters, exact-string deduplication and punctuation handling.
Bound the inner scan to the raw comment envelope, preserve forward progress and
UTF-8 boundaries, and keep inner tags from changing outer suppression state.
Malformed/unterminated inputs and actual work counters are pinned as in the plan.

### AC15 — fresh ingest and historical truth

Preserve [0005](../../docs/decisions/0005-extraction-at-ingest.md): only fresh
ingest derives links/codes. A deliberately stored historical Outlook row has
both old truncated and full URLs, populated codes and original raw/bodies.
Live HTTP reads and reads after shutdown, release and reopening the same
database retain those exact arrays and bytes. Fresh SMTP ingestion of the same
source yields the corrected singleton before 250/readback. Query/MCP/compat,
webhook construction and restore continue to read stored arrays; no migration,
query fallback, historical cleanup or write-back is introduced. Preserve 64-code
limits, body selection, synchronous insert, subscribe-before-check and oldest-only
pruning. AC15 is a local follow-up identifier, not a renumbering of old criteria.

### AC16 — records and proof agree

Append dated 2026-10-26 operator provenance in 0061 and the 0049 amendment: replace
§6's refusal-recognition order exactly with accepted 0061 §1, independently
record retained R4.3 and source-aware R4.4, and preserve historical bodies.
Mark only 0061 and its existing single index row accepted (operator, 2026-10-26).
Update current AC10–AC12 and dependent plan/tasks in the older feature; preserve
dated old receipts and superseded instructions as history. Link tracked records,
not run-local packet paths. The tracked rulings home currently stops before the
supplied decision-rulings table, so the dated addenda must be self-contained.
`store_contract_docs` must pass. No other packet decision is accepted.

Each red/restored-green receipt includes exact command, executed test count,
behavioral assertion, exit status, source identity, mutation/restoration digests
and log identity. Compilation failure and a selector matching zero tests do not
count as red or green. Run every final-candidate house gate, exact zero missed
lines/branches/functions and coverage selftest, with the supplied engine digest
and unchanged candidate routing. New source changes require renewed affected
proofs; old receipts do not certify new bytes. No whole-epic or protected-phase
completion is inferred.

## Scope and external ownership

No dependency, license allowance, schema, public extraction API, feature flag,
engine pin, gate or recipe change. Existing compatible work outside AC10–AC12
is preserved and covered by the full suites. CI owns macOS/platform, Docker/chart,
RustSec network audit and privileged port-80 exchange. The operator owns live
GitHub settings, namespace provisioning, eventual sealing and landing; tag-only
release proofs belong to CI/operator. They are not local acceptance criteria.
No push, merge, tag, publication, paid trial or issue closure is authorized.
