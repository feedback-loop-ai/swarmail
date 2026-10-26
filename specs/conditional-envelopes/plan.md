# Plan — conditional envelopes

Date: 2026-10-26. Implements [spec.md](spec.md); ordered proof is in
[tasks.md](tasks.md). This design does not claim an implementation repair.

## Technical context and research

Single Rust crate with the existing byte tokenizer, source-region URL consumer,
bounded entity decoder and real SMTP/HTTP integration helpers. No new dependency.
The workspace has no `.specify/templates/plan-template.md`; its requested
technical context, constitution check, project structure, research, data model,
contracts and quickstart are supplied here.

Read the house charter, decision index, relevant accepted rulings, intake,
both positions and supplied review. The packet README, operator rulings and
review pass SHA256SUMS from the packet directory. An initial root-relative
checksum attempt used the wrong base; the directory-local successful check
is the authoritative receipt. The README has no populated prior-runs entries;
the supplied review and local commits supply the previous outcome. No external
source, journal or sibling worktree was consulted.

At inherited HEAD `4c3a98f29d166aa01c8a07b9fe922862916f9f6d`, `markup_at`
finds the raw comment close but only checks `[if` plus whitespace before
calling `conditional_values`. That helper searches for `]>` and scans through
the whole remaining comment body. It never checks a final marker. `tag`
already owns quote-after-equals boundaries and reports closed values immediately.
`LinkRegions` partitions and decodes source regions; rollback after callbacks
would be incorrect. The existing no-marker inline fixture expects the full URL
and must change. Both `LinkScanStats` and `TagScanStats` are instance-local;
their current consumption totals do not measure conditional traversal.

## Constitution check

Preserve synchronous accept == stored (0001), subscribe-before-check (0002),
oldest-only pruning (0003) and fresh-ingest derivation/verbatim restore (0005).
Extraction stays in the existing awaited ingest path. No new mutation owner,
queue, persistence seam or read-time derivation is authorized. The accepted
0061/0049 addenda already rule the semantics; no ADR or acceptance edit is needed.

Use 0020's measured inline default for deterministic tests and real protocols
for wire behavior. No sleep synchronization, random chaos, global counters,
fixed-date certificate test, production coverage exclusion or gate change.
Keep current recipe routing and dependency policy. The design commit contains
exactly this directory's three artifacts; position files remain ignored.

## Project structure and files touched

| Path | Ownership |
|---|---|
| `src/extract.rs` | Local envelope qualification, bounded payload visitation, counters through existing scanner and measured inline regressions. |
| `tests/compat.rs` | Extend existing Outlook wire evaluator with qualification/fallback controls; reuse isolated real-server helpers. |
| `specs/conditional-envelopes/spec.md` | Current requirements and any dated clarification within this follow-up. |
| `specs/conditional-envelopes/plan.md` | Approach and later concise, actual proof receipts. |
| `specs/conditional-envelopes/tasks.md` | Ordered checks, exact selectors and evidence-backed completion boxes. |

Read/test existing storage, SMTP, API and extraction controls; no production
edit outside the extractor is expected. Keep old specifications and decision
records unchanged. Full logs and mutation receipts are ignored run-local
evidence under `.forge/epic192/conditional-envelopes/`; tracked summaries use
identities and plain paths, never Markdown links into ignored evidence.

## Implementation approach

1. Add the three exact baseline evaluators before production edits, including
   the no-marker expectation correction. Capture missing/non-final behavioral
   reds and the actual mixed-case result. Snapshot source and log hashes.
2. Keep the comment arm of `markup_at` as outer boundary owner, with existing
   first-close and abrupt-close semantics. For a normal close, determine body
   bounds excluding `-->`. Abrupt closes return ordinary behavior before any
   potentially underflowing subtraction. Missing close keeps unfinished-tail
   behavior. Never search past the chosen close.
3. A small local qualification step returns payload start/end offsets or no
   qualification. Check immediate raw `[if` and ASCII whitespace; trim only
   trailing ASCII whitespace for the fixed-width final `<![endif]` comparison;
   find raw `]>` only in the preceding body portion. The header must finish
   no later than marker start. Conditions are opaque. Qualification invokes
   no region callbacks and allocates no speculative links or lowercased body.
4. Pass the original-source prefix ending at payload end plus payload start
   to `conditional_values`. Reuse `tag`, preserving absolute source offsets.
   Search forward for `<`; only existing recognized inner start tags invoke
   `tag`. Complete tags advance to their end; unfinished tags/quotes stop.
   Skip declaration-like markers without recursive parsing. Retain the
   already-closed-value semantics of `tag`, with no special ownership beyond
   the payload. Outer markup remains `Kind::Other`; inner text and element
   state never reach the outer suppression machinery.
5. Keep `LinkRegions` as sole partition/decode/match owner, including leading
   URL quote tolerance, embedded URL delimiters, punctuation and exact dedup.
   Add only local measurements at actual loops/calls. Codes still suppress
   all comment text; links still inspect ordinary comment source where no
   source-aware region was reported.
6. Extend edge fixtures and real-wire evaluator, then perform the two isolated
   mutations described below. Restore exact bytes after each, including on
   failure. Run owning controls and all gates, record real outcomes, reconcile
   tasks and commit implementation separately. Never describe design preflight
   gates as proof that the behavior has already been repaired.

## Work observations

Extend existing local scanner stats; pass them through `markup_at`, the local
qualifier, `conditional_values` and `tag` rather than introducing a scanner
framework. Count where predicates or traversal steps execute, including failed
searches. Do not increment by an input length or cursor jump as a substitute.
Existing `markup_bytes`, `ordinary_bytes`, `quoted_bytes`, reference-decoder and
raw-close pins retain their meanings. Both code and link extraction walk the
shared tokenizer; each walk has its own accounting, not a claim of one total
ingest walk.

For one walk let N be input bytes and C the number of normally closed comments
examined for qualification. Pin these separately measured categories (names
may follow existing Rust naming conventions; units and bounds are normative):

| Category | Unit observed at execution | Bound per walk |
|---|---|---|
| opener bytes | Raw byte comparisons/classification for `[if` plus its delimiter | <= 4C |
| trailing bytes | Each byte examined while trimming trailing ASCII whitespace, including the stopping byte | <= N + C |
| marker bytes | Each byte compared to the fixed nine-byte end marker | <= 9C |
| header probes | Each two-byte window actually compared with `]>`, including misses | <= N; each probe compares at most two bytes |
| payload search bytes | Each byte examined by the forward `<` search, including unsuccessful tails | <= N |
| tag-name bytes | Each byte examined to find a tag-name delimiter | <= N |
| attribute steps | Each executed attribute-loop byte dispatch | <= N |
| quote-search bytes | Each byte compared while seeking a source closing quote, including failed tails | <= N |

Tag counters may cover all tag visits in that walk; their same per-category
bounds still apply. Do not sum consumption counters and call it measured work.
Use exact counts for small fixtures and the explicit bounds for repetition
sizes 1, 16, 256 and 4096. Include missing/non-final markers, repeated malformed
openers, multibyte bodies, completed tags and a qualified unfinished quote with
many later `<` bytes. Each work category must be exercised; zero counters from
unreached instrumentation are not a proof.

Use a test-only observer/local trace through existing stats seams to assert
strictly advancing payload traversal and non-overlapping ordered value ranges
within payload bounds. At existing link-region decode calls, observe one
invocation per processed ordinary/quoted region (including any empty regions
that the existing consumer processes); the ranges partition the source exactly.
Decoded output never re-enters structural recognition. These observations must
not introduce a process-global seam or a second decoder.

The work negative control repeats actual observed suffix traversal for successive
payload positions in a local temporary mutant. Do not merely add a counter or
change an expected bound. Output and partition stay equal; the actual-work bound
must fail. The case negative control changes only final-marker comparison to
case-sensitive. Both must compile, select one test, fail its intended assertion
and return green after byte-exact restoration. No mutation is committed.

## Data model and contracts

The only new transient data is qualified source offsets and local observations.
No persisted fields, public API, config or schema change. Source offsets refer
to raw UTF-8 bytes; slice only at ASCII delimiters or source ends. Link/code
arrays, text-first link order, 64-code policy, raw/body truth and historical
live/reopen behavior remain as ruled. Empty payloads qualify without attributes;
ordinary fallback intentionally retains decoded quote delimiters. Wire assertions
read exact arrays from the native stored-field response and check both shims'
existing projections; no new shim response field is required or authorized.

## Quickstart and proof ownership

Use offline Cargo and installed tooling. The four new exact unit selectors and
existing wire selectors/counts are in [tasks.md](tasks.md). Full evaluator logs
must retain command, selected count, assertion, exit, UTC interval, source hash,
mutation patch and original/mutant/restored hashes. A tracked dated summary
maps each AC to those identities; logs are not stand-ins for committed code.

The [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
is authoritative. Execute every listed gate, with `.forge/bin/brokkr compile
--bundle bundles/self` replacing PATH-based brokkr. Verify its SHA-256 equals
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` first.
Check `bundles/self/scripts/shims.sha256`, then execute the pinned
`bundles/self/scripts/` copies for coverage, its selftest, publish-guard and
seal-branch proof. Bundle compilation must have no policy-sweep warning.
Supplement with design-check selftest, delivery-proof-test, delivery_recipes,
store_contract_docs and fail-closed native-tls absence: `cargo tree --locked
-i native-tls` must fail specifically with Cargo's `did not match any packages`
diagnostic; any other error or successful tree refuses. Exact coverage needs
positive totals and zero misses in lines, branches and functions.

CI/operator checks remain assigned as stated in the spec; install nothing and
consult no remote source. Gate failures or missing prerequisites remain explicit
unfinished obligations. Only the deterministic sequence check owns its verdict.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| An early callback assigns wrong ownership before fallback | Qualify all raw bounds before callbacks; exact prefix/full-URL negative controls. |
| Whitespace or case handling broadens ordinary comments | ASCII-only byte checks; immediate opener and final-marker tables; mixed-case mutant. |
| A tag reads through the marker or follows a later comment close | Bound by first raw close and marker start; header ordering and following-anchor controls. |
| Unfinished quotes trigger repeated suffix scans | Stop inner traversal; actual failed-search counters and repeated-work mutant. Later inner attributes may intentionally stay ordinary. |
| Byte partition is mistaken for work proof | Keep work categories separate; exact small counts, size bounds and output-preserving mutation. |
| UTF-8 split or double decode changes URLs | ASCII cut points, multibyte and encoded-delimiter fixtures; observed decode calls and ranges. |
| Inner container-like tags hide outside codes | Values-only reporting and outer `Kind::Other`; outside code/anchor controls. |
| Test names or unchecked tasks drift again | Exact selectors/nonzero counts; reconcile this ledger from retained receipts, not old green logs. |
| Broad reliability claims expand scope | Existing suites protect ingest, store and webhooks. Crash-after-commit retries, best-effort notifications, later cap eviction and aggregate resource pressure remain. |
| Full tooling or gates fail locally | Name the command/prerequisite, retain exit/log, do not lower gates or infer runtime completion. |


## Chief validation — 2026-10-26

The chief ran all house gates offline against inherited code at HEAD
`4c3a98f29d166aa01c8a07b9fe922862916f9f6d`, tree `735a9c659a4b7d8697f27f92d6df108f3556d32b`.
Only this design directory changes; no production/test/gate/decision bytes were
edited. All nine house commands passed: format, locked workspace Clippy/tests,
digest-checked candidate engine compile, exact coverage, coverage selftest,
publish guard, seal proof and license check. Supplemental shim integrity,
design selftest, delivery proof, recipe tests, documentation-contract tests and
the fail-closed native-tls absence wrapper also passed. The absence probe has
Cargo's explicit no-package diagnostic. Bundle sweep: 76 valuations across
14 groups, zero unruled.

Coverage is **14742/14742 lines, 834/834 branches, 1602/1602 functions**,
zero missed in each dimension, report `target/coverage/run.uxEkTC`.
The run spans 10:53:05–11:00:16 UTC. The workspace transport timed out at
300 seconds while the runner continued; its completed command receipts and
rechecked log hashes establish the outcomes, not that transport result.
Receipts live at `.forge/epic192/conditional-envelopes/design/gates.json`
(SHA-256 `8a1772d45a2ff531f88c83d64d618e31201fcf642200f436d9e436f1127509b5`).

| Proof | Exit | Log SHA-256 |
|---|---|---|
| shims | 0 | `785e16dc6e9db3dcf3e2f94347f10c6c3508b97edc039d75e9676f3b0ea1b39c` |
| fmt | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| clippy | 0 | `f591e097f22f91dd5c31fbbf7e587d80c0a4e0ed594f9c9012a544f38c52d926` |
| test | 0 | `19d962d2b5dcbb716d7249befc4e7c2cd622f28f620d341fe33a5eba07caec68` |
| bundle | 0 | `ad83b8df41a4b44f1d748cf6c6ef17dd8a7db503246c50e2ff0dab8e04b895ff` |
| coverage | 0 | `7f43e52ebed3d46be10e1515afcabacf4e4766efb6d40407190cf75ff2293f73` |
| coverage-selftest | 0 | `d34e676c6b8e49c2791415ddb90e0f1733000cf1a18614a026bbc816060bd12a` |
| publish | 0 | `939ac40398ee171a956e8423b96d4a9c62b4b1c30250a6fa6146b585ddf0647f` |
| seal | 0 | `e022b8e780d716cdbafdf4723515738c62bf480308724cfbb2051566d835b200` |
| licenses | 0 | `71dadc8c8978da797b94dc581b666330c737551839c8fc820d7647130930c4b7` |
| design-selftest | 0 | `1d5eb6cef8be9f99e87fa8b1394ed2df9172cf2c4da06f6d74cc03c95245735e` |
| delivery-proof | 0 | `2969d73ecccc45098e6907c604407b44967f3fe4669573a6cd4c5e8b82785d8c` |
| recipes | 0 | `d2904e07b2419c335a0489dd5c847e8779c64e79728d49bc113498eaea58f7bc` |
| docs | 0 | `e5501de9a57ba2d15010fd117d34dda60080842b20232e607ae0dcbe650ed278` |
| native-tls | 0 | `c20322ba4adf68e75b4b544969d1187b0abaf3b0585b10cd47b0ce97c167025f` |

These are chief preflight results on inherited implementation, not proof that
AC1–AC5 are implemented or a protected verification verdict. The implementation
tasks remain unchecked and must supply their new regressions, mutation receipts
and renewed final-candidate gates. The chief validates committed artifact
structure after committing; the sequence's pinned check owns advancement.
