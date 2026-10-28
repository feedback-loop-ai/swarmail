# Plan — conditional envelopes

Date: 2026-10-26. Implements [spec.md](spec.md); ordered proof is in
[tasks.md](tasks.md). The design below is retained; the dated implementation proof follows.

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

## Implementation proof — 2026-10-26

Implemented the already-authorized R4.4 qualification and the same scanner's
LOW work-proof finding. Recorded 0061/0049 rulings and every unrelated path
remain unchanged. Qualification precedes callbacks, uses the first raw close,
requires the immediate ASCII opener/header and final case-insensitive marker
after ASCII-only trailing whitespace, and bounds attribute scanning before
the marker. Failed qualification keeps ordinary-comment/unfinished-tail behavior.
The local counters observe real qualification/search/tag predicates, failed
tails and decode calls. Test-only traces prove ordered payload progress,
bounded value ranges and complete, once-decoded source partitioning.

Inherited HEAD: 98c1812ced05e83a48a972896e9c296ce46d526a; inherited tree:
2a7919dcd695e4452df0ec427174aac4c6646206.
The staged code snapshot tree is **ed1e7369f7d173dec920f1e539b6d9d0d45c751a**, before
these proof-only Markdown updates. Final commit/tree identity is recorded in
the run-local commit.json after committing; source bytes below bind every
final gate and evaluator to the delivered implementation:

- src/extract.rs SHA-256: 55b8440f99ffa8cb4faa77e744baf9f98a7d7376153b1ad3d52e8bfcc1233aca
- tests/compat.rs SHA-256: bfa75ae0b03ea33c40c209fae723df2fb81cd8295299d912320d1548207cddf2
- Regression-first extract snapshot SHA-256: 173820e3be60d58726abffc7d8edf2cf8973fec4f4d418cbfdd5f140d7105f98.
  Its production prefix was checked byte-for-byte against inherited HEAD.
- Final receipt ledger: plain path
  .forge/epic192/conditional-envelopes/implementation/final-gates.json,
  SHA-256 660a1b8a5e7fc6735b1943007236f9095697410115491d077ea23433bd18c50d.
  Each entry retains exact command, UTC interval, exit, source identities and
  log SHA-256. Full logs/snapshots/patches are in that ignored directory;
  no tracked link points into it.

### Regression-first and negative-control receipts

All following exact invocations selected one test. Missing-marker and
non-final baseline failures returned full O'B/C'D URLs where ordinary
prefixes O/C were required; the corrected old no-marker fixture also failed.
Mixed case genuinely passed the baseline. After qualification, the temporary
case-sensitive-marker mutant produced the truncated URL and failed the mixed-case
test. The separate work mutant repeats actual suffix searches: exact URL output
and partition assertions still pass before the payload-work bound fails.
Both mutants compile, fail behaviorally with exit 101, restore byte-for-byte to
the final source hash above, and pass after restoration. Neither is committed.

| Receipt | Exit | Log SHA-256 |
|---|---|---|
| baseline-missing | 101 | 85380cbf107960f43a644df777f822a42d29dd75d8ce9d082613e45e22f5ddcf |
| baseline-case | 0 | 046a6bc01ce436faf3482b0c0977733c5c644d9cc56b238dcd895c1499e2cb92 |
| baseline-final | 101 | 568b5bcd01c4ea5152ffb0417f9d8f0ca9a8e718a59ad32ecf422d37b33fc27f |
| baseline-boundaries | 101 | 055e7ab820430a59e6dc1c9ea0c85fe1f3608ea94de0ea6db5ffde84501e9850 |
| case-mutant-red | 101 | f428fe6d146e5ddb8b1d64e659a137279f31d324b82bfae145a7818e71f30293 |
| case-restored-green | 0 | 1e38c6e0cc78efc7d15e1fef05d0511b8e77bc22de6deefb0bee14f67a334ca9 |
| work-mutant-red | 101 | 59c878c234a094495df501684eb9a7dd1dd2a24e3eec5304d5c79db8a6c94438 |
| work-restored-green | 0 | ad0308b4b36c7ce33c027dce6201f310156b6d3385297a7e95c7d22942cb086b |

| Mutation | Mutant source SHA-256 | Patch SHA-256 |
|---|---|---|
| case | 8aedbf5fc93bb8b44011fcc6aec2adfbe12a95028c85d0d7dde36c46c23e2dbe | b10e8fbe4c3b25eaa2d05cda0477150cf50a271f873e7b9d75772a92b0dfbc0a |
| work | 3d2c5d614283d46a2d60f935185a8ede14d9c455737f30ffc243089cb07627af | 31a266ea1712553133214eaf9b11bd80fbc3dad804396d8378deb2317a2be8e5 |

The three requested exact regressions, existing source-boundary evaluator and
work evaluator pass on final bytes. Work checks use sizes 1/16/256/4096 across
valid/missing/non-final envelopes, repeated malformed openers, multibyte payloads,
unfinished names/attributes and later angle brackets. All eight categories have
positive exact small-fixture pins; every separately specified linear bound is
asserted, together with ordered ranges and decoder invocation/partition checks.
The real SMTP/HTTP Outlook evaluator covers six isolated inboxes, exact native
stored arrays immediately after 250, raw/body bytes and both existing shim
projections. Complete conditional-only and duplicate-anchor URLs remain singleton
arrays. Historical live/reopen, independent-MIME text-first, nested URL and #136
evaluators each select one passing test.

### Final local gates

All 29 final receipts exited 0 on the source identities above. Exact coverage:
**14988/14988 lines, 838/838 branches, 1610/1610 functions**, zero missed in each
dimension; report target/coverage/run.FPPem1. Coverage selftest reports every
refusal arm refuses and every pass arm passes. The initial gate pass found a
collapsible-if lint and three cold short-circuit branches in compound test
assertions; these were corrected with a let-chain and separate invariant
assertions/enclosing-payload lookup. Initial logs remain under first-pass/.
No gate, exclusion, harness placement or dependency policy changed.

Packet checksums, candidate engine digest and fixed-shim integrity pass.
Pinned bundle compilation retains current routing and reports 76 valuations
over 14 groups, zero unruled; no policy-sweep warning. Native-tls absence
accepts only Cargo's explicit no-package diagnostic (Cargo exit 101, wrapper
exit 0). Every house command and configured supplemental proof ran offline
with installed tools. Pinned script copies were executed after checking shims.
Extraction: 41 passing tests; compat: 36; documentation contracts: 25;
delivery recipes: 15. The full locked workspace suite passes.

| Final receipt | Exit | Log SHA-256 |
|---|---|---|
| final-bundle | 0 | ad83b8df41a4b44f1d748cf6c6ef17dd8a7db503246c50e2ff0dab8e04b895ff |
| final-clippy | 0 | d6b6e5e891252beafe874c20a7e5b775d8d81b97a69d66a8250c9d4ca6075708 |
| final-compat | 0 | 58afdfcbf6fb4ad7f089d9932adee276643db38a86066740ff3c82d3cb1cab4a |
| final-coverage-selftest | 0 | b592c0da51ed12d117e2c705e14aed0ee8c1d78e05a03496ffa0dd69fa928cad |
| final-coverage | 0 | f0cd395c1aa79face844dbd52ef3f6c5a3577a48356c8af0158b0c83105d557a |
| final-delivery-proof | 0 | 2969d73ecccc45098e6907c604407b44967f3fe4669573a6cd4c5e8b82785d8c |
| final-design-selftest | 0 | 1d5eb6cef8be9f99e87fa8b1394ed2df9172cf2c4da06f6d74cc03c95245735e |
| final-docs | 0 | eb0ef57aa096dfae06c0838fdf02d732b60e6b3f4ee1bd9e5d8b5cc6bf4c40c6 |
| final-engine | 0 | d59591f9d6f693cfccb0906c65ee27e7aa2a907ab4d85f90d8c8303abfee6c10 |
| final-extract | 0 | 843651dcf0300bd7903f6e392506fa1b8a205ec390aae725691f46c1965dac33 |
| final-fmt | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| final-green-boundaries | 0 | 8b61a27351bef4096d97e0b2a907b8b3276a0809cbf9ae91d769140a9374f310 |
| final-green-case | 0 | fe849427392cadb1ffc84bf4b2ff50d0097cf06736e72dff4844cfe1c7f46a28 |
| final-green-fidelity | 0 | 1bbba5ba49e0ac0fc3efeaf1ff76d7acad6c95cb4350df97418b96a73c9e0bf1 |
| final-green-final | 0 | bdea71f9d39760e3c604a0e7e46d12dcc5dee801544214dc4fd930530c32cb7c |
| final-green-historical | 0 | f8fa9baced0a208f9d7c4dd2c10c8404c95098a803e30151e3e095640b960a68 |
| final-green-mime | 0 | cf68b86d28bf1f2efa92da656914a021e508ec4ff8db5d5d682352e40be6d2b6 |
| final-green-missing | 0 | 39c85ca1195fe6eb852cb8aca393b0760f82ccb32168249434037c85099fc8b2 |
| final-green-nested | 0 | a6d469b908a8070be95cc04b95026b641a984eefe0f1984ca8b00990f69e16e8 |
| final-green-wire | 0 | dcf648085d4be38cfdf38ddd013fd7a60bf252dbb1dc0d0d7cf115aaaeea9ff7 |
| final-green-work | 0 | 6b78f3c16b90ac8e2b6856b26a7868aa658786217ab3a29181f06b0c50dac5d4 |
| final-licenses | 0 | 71dadc8c8978da797b94dc581b666330c737551839c8fc820d7647130930c4b7 |
| final-native-tls | 0 | 2bb6caf966b1f99bd1046e5b5c634e4526b914f80441762405e271227f59263b |
| final-packet | 0 | 9a560c912c084f4b51c8d5c47561e0bac9b4db5107b068af9b7de631fd09f0af |
| final-publish | 0 | 939ac40398ee171a956e8423b96d4a9c62b4b1c30250a6fa6146b585ddf0647f |
| final-recipes | 0 | 0e02d1b46736f50ef2e8f2df78f3412ac79358730c9e70647f511ba2e2bff9c7 |
| final-seal | 0 | e022b8e780d716cdbafdf4723515738c62bf480308724cfbb2051566d835b200 |
| final-shims | 0 | 785e16dc6e9db3dcf3e2f94347f10c6c3508b97edc039d75e9676f3b0ea1b39c |
| final-test | 0 | 95bffbc0132590a69f92a330f180ed0760579fe45829d655efb6e515de361259 |

The commands and exact selector names are in the gate inventory, quickstart
and task ledger above; receipt names map directly to their full commands in
the hashed JSON ledger. Local proof does not claim protected verify/review or
CI/operator evidence: macOS, chart/history, Docker, advisory/network, privileged
port-80, live GitHub settings, namespaces, release/tag, sealing and publication
remain with their recorded owners. No decision acceptance or broader prior-run
LOW disposition is changed. All scoped implementation and proof records are
committed unsigned; the tracked worktree is checked clean after committing.
