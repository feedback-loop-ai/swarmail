# ARC chain reports (issue #256)

Decision: 0133 — [ARC chain validation in the deliverability report](../../docs/decisions/0133-arc-chain-reports.md) (proposed; two operator holds)

- **Epic:** #251. Filed P3 ("only if a need is shown"); the operator ruled on
  2026-11-16 to deliver it. This story closes the epic.
- **Run:** `issue-256-report-arc-chains-part-b186e59e`, from `main` `f05f2ee`.
- **Authorities read:** house rules; accepted
  [0085](../../docs/decisions/0085-deliverability-report.md) with its dated
  pointers; accepted [0120](../../docs/decisions/0120-deliverability-dns-policy.md),
  [0122](../../docs/decisions/0122-deterministic-spam-rules.md),
  [0125](../../docs/decisions/0125-offline-dkim-verification.md) and
  [0126](../../docs/decisions/0126-spf-dmarc-alignment.md); the pinned
  `mail-auth-0.14.0` source (`arc/verify.rs`, `arc/parse.rs`, `arc/mod.rs`,
  `arc/output.rs`, `message.rs`, `dns/lookup.rs`, `dkim/output.rs`);
  `src/deliverability.rs`, `src/deliverability/dkim.rs` and
  `src/deliverability/auth.rs`.
- **Framing source:** the task block and the intake result. The run's
  `.brokkr/state/tasks/` directory was empty.

## Feature and rationale

A forwarder that changes a message breaks its DKIM signature. ARC (RFC 8617)
lets each forwarder record what it saw and seal that record, so the next
receiver can check the chain of custody. A test that sends mail through a
forwarder or a mailing list into Swarmail cannot say today whether the chain
it produced is valid. The report is silent about ARC.

This feature validates the ARC chain of a stored message inside the lazy
report evaluator, with keys from the declared DNS policy only, and reports:

- the chain validation state: `none`, `pass` or `fail` (RFC 8617 section 5.2),
  or `not_evaluated` when Swarmail cannot give a verdict;
- how many ARC sets the message carries;
- which instance `i=` broke the chain, and why, as one typed finding with a
  stable rule id, located at the header field that broke it.

## Scope

In scope:

- turning on mail-auth's `arc` feature;
- one new module, `src/deliverability/arc.rs`;
- thirteen `arc.*.v1` rules, at most one finding per report;
- one new required report member, `arc` (hold 1);
- the guard that keeps accepted 0125 section 6's work bound true with the
  feature on;
- `RULE_SET_ID` `swarmail-html/9`;
- OpenAPI, MCP schema and description, the message page, the guide and
  `llms.txt`.

Out of scope, and not built:

- reading what an `ARC-Authentication-Results` field claims. It is sealed
  text written by an intermediary, and nothing in this story trusts it;
- letting ARC change a DMARC, SPF, DKIM or `bulk.*` verdict or any
  `AuthEvidence` fact. That is a policy change with its own security
  argument and its own decision;
- trusted-sealer lists, sealing, real DNS;
- a new assert or await predicate, endpoint, MCP tool, configuration knob or
  `Location` variant.

## Operator holds

The delivery PR is not merged on green. Under 0045 section 8 it waits for the
operator on two items, both stated in
[0133](../../docs/decisions/0133-arc-chain-reports.md):

1. **The `arc` report member** amends accepted 0085 section 1's report shape
   (already amended by 0120's `dns` and 0122's `total`).
2. **The crate's clock read inside `verify_arc`** is executed on every chain
   that reaches the verifier, and its value can never reach a verdict. Whether
   an executed read with a dead value is inside accepted 0085 section 8 (as
   amended by 0125 section 8 and 0126 section 8) is a reading of accepted
   text, so the operator rules on it.

A refusal of either returns the run to design with the remedy 0133 names for
it.

## Contract

### Dependency

`Cargo.toml` changes one line: mail-auth's feature list becomes
`["aws-lc-rs", "arc"]`. The `arc` feature is declared as `arc = []` in the
crate's manifest and pulls in nothing. `Cargo.lock`, `evals/Cargo.lock` and
`deny.toml` stay byte-identical to `main` `f05f2ee`. Accepted 0125 section 1
said #256 turns the feature on.

### When ARC is evaluated

ARC is evaluated once per report, inside the existing lazy evaluation, after
DKIM's `prepare` and on the same strict parse, the same adapter over 0120's
`Lookups`, the same `AUTHENTICATOR` and the same receipt second. Nothing runs
on the SMTP path, at ingest, in `Store::insert`, at seed or restore, or in
fan-out.

The **ARC families** are the three root header names Swarmail's header scan
lowercases to `arc-seal`, `arc-message-signature` and
`arc-authentication-results`.

ARC always reads the **strict** parse of the stored bytes. It never reads the
non-strict re-parse DKIM makes for an `l=` signature, so the ARC verdict for
a message does not depend on whether an unrelated `DKIM-Signature` carries
`l=`.

### The `arc` report member (hold 1)

Every report carries one more required member, after `dns` and before `total`
(`total` stays the last key, 0122 section 2):

```json
"arc": {"state": "pass", "sets": 2, "failing_instance": null, "reason": null}
```

| key | type | meaning |
|---|---|---|
| `state` | `"none"`, `"pass"`, `"fail"` or `"not_evaluated"` | the chain validation state |
| `sets` | integer, 0 or more | the largest number of fields any one ARC family holds, in the root list or in the crate's parse, whichever is larger |
| `failing_instance` | integer 1 to 50, or `null` | the instance `i=` that broke the chain |
| `reason` | string or `null` | the rule id of the report's one `arc.*` finding |

Invariants, each proven over every fixture:

- `state` is `none` exactly when `sets` is 0. Then `failing_instance` and
  `reason` are `null` and the report holds no `arc.*` finding.
- `state` is `pass` only with `sets` at least 1, both other members `null`,
  and no `arc.*` finding.
- `state` is `fail` or `not_evaluated` exactly when the report holds one
  `arc.*` finding. `reason` is that finding's `rule_id`, and `state` is the
  state the rule table gives that id.
- A report never holds more than one `arc.*` finding. Chain validation has
  one verdict and one breaking point.
- `failing_instance` is non-null only when `state` is `fail`.

`not_evaluated` is Swarmail's fourth state. RFC 8617 has three. It is used
when Swarmail declines to judge (a declared temporary failure, an ambiguous
header boundary, a bound, or an expiry it could only judge with the wall
clock). Calling those `fail` would claim a broken chain nobody observed.

The member is computed once, in the evaluator, and cached in the report. REST,
MCP and the message page all render that one value. No surface derives the
state from findings.

### Evaluation order (normative)

The steps run in this order. The first that applies decides the state and
emits the one finding. `N` is the highest instance. Steps 0 to 6 are
Swarmail's and read no clock; step 7 is the crate's.

0. **No chain.** The root list holds no ARC-family field, and the strict
   parse (when there is one) lists none either. State `none`. Nothing is
   emitted and nothing is looked up.
1. **One body, one field list.** Otherwise all of these must hold, or the
   result is `arc.header_boundary_ambiguous.v1`:
   - the strict parse exists and DKIM's own boundary guard did not refuse it;
   - its body offset equals Swarmail's;
   - for each ARC family, the number of fields the crate lists (parsed, plus
     rejected fields of that name in `errors()`) equals the root list's count.

   This guard runs on every path, including mail with no `DKIM-Signature`,
   which is the usual case for forwarded mail and the one case where DKIM's
   guard has not run.
2. **Bound.** An `ARC-Message-Signature` whose `h=` lists more than 128 names:
   `arc.limits_exceeded.v1` at the first such field in wire order.
3. **Field verdicts.** Each ARC field is judged in wire order, and the first
   field with a verdict decides. A field is `arc.malformed.v1` when:
   - the crate rejected it for any reason other than its body-length tag;
   - (`ARC-Message-Signature`, `ARC-Seal`) a tag name is repeated, `a=` is
     missing, or a `t=` or `x=` is not a plain number that fits 64 bits;
   - its `i=` literal, read by Swarmail (the `i` tag of the tag list; for
     `ARC-Authentication-Results` the leading `i=` before the first `;`), is
     missing, is not ASCII digits with a value of 1 to 50, or differs from the
     instance the crate parsed.

   Otherwise it is `arc.unsupported.v1` when:
   - it is an `ARC-Message-Signature` that carries an `l=` tag, whatever its
     value;
   - its `a=` is `rsa-sha1`.

   These are the checks the crate would settle by a silent default: it reads
   a missing `a=` as rsa-sha256, a garbage number as absent, the last of two
   repeated tags, and an `i=` above 2^32 as its low 32 bits.
4. **Complete sets.** The three families must hold the same number of fields,
   or the result is `arc.set_incomplete.v1`. This includes ARC fields with no
   `ARC-Message-Signature` at all, which the crate reports as "no chain".
5. **Distinct instances.** Within each family no two fields may carry the
   same `i=`, or the result is `arc.instance_invalid.v1`. Swarmail decides
   this because the crate sorts its lists with an unstable sort: with two
   fields of one instance, which of them it reads at a position is not
   defined by the bytes. It also enforces RFC 8617's cap: instances are 1 to
   50 and distinct, so no family can hold more than 50 fields past this step.
6. **Expiry, at receipt time.** The newest `ARC-Message-Signature` is the one
   with the highest instance, which step 5 makes unique. When it carries an
   `x=` tag (a literal `x=0` included):
   - `x=` at or before the receipt second, or at or before its own `t=`
     when it has one: `arc.signature_expired.v1`;
   - otherwise: `arc.not_evaluated.v1`.

   In both cases the crate's verifier is not called and nothing is looked up.
7. **Verification.** `verify_arc` is called once. Its result maps as follows,
   with no other arm:

   | crate result | outcome |
   |---|---|
   | `Pass` | state `pass`, nothing emitted |
   | `Fail(Arc(InvalidInstance(p)))` | `arc.instance_invalid.v1`, instance `p` |
   | `Fail(Arc(InvalidChainValidation))` | `arc.cv_invalid.v1` |
   | `Neutral(Arc(BodyHashMismatch))` | `arc.body_hash_mismatch.v1`, instance `N` |
   | `TempError(_)` | `arc.not_evaluated.v1` |
   | `PermError(Dns(RecordNotFound(_)))` | `arc.key_missing.v1` |
   | any other `PermError(_)` | `arc.key_invalid.v1` |
   | `Fail(Crypto(IncompatibleAlgorithms))` | `arc.key_invalid.v1` |
   | anything else | `arc.signature_mismatch.v1` |

   "Anything else" is every other failed signature check: a wrong key of the
   right type, key bytes that are no key, altered sealed fields. The crate
   does not check key bytes before use, and its ed25519 backend words a
   failed check as a library error, so no narrower arm is sound (accepted
   0125, implementation record item 3). The mapping is one `match` with no
   arm that no fixture reaches. The crate's "too long", "broken chain",
   "expired" and parse results cannot occur after steps 3 to 6 and get no arm
   of their own.

A chain whose newest `ARC-Message-Signature` carries `x=` is decided at step
6 even if its instance sequence or `cv=` values are also wrong. This is
deliberate: the crate owns that structural verdict, and calling it with a
non-zero `x=` would let the wall clock decide which lookups are recorded.

### Rule table

Every rule is `error`. A pass emits nothing, and so does a message with no
ARC field.

| rule id | state | when | location |
|---|---|---|---|
| `arc.header_boundary_ambiguous.v1` | `not_evaluated` | step 1 | the first root ARC field in wire order (its name, occurrence 0); `arc-seal` with `occurrence: null` when the root list holds none |
| `arc.limits_exceeded.v1` | `not_evaluated` | step 2 | that `arc-message-signature` |
| `arc.malformed.v1` | `fail` | step 3 | that field |
| `arc.unsupported.v1` | `fail` | step 3 | that field |
| `arc.set_incomplete.v1` | `fail` | step 4 | the family with the fewest fields (ties in the order `arc-authentication-results`, `arc-message-signature`, `arc-seal`), `occurrence: null` |
| `arc.signature_expired.v1` | `fail` | step 6 | the newest `arc-message-signature` |
| `arc.instance_invalid.v1` | `fail` | step 5 or 7 | step 5: the second field, in wire order, that carries the lowest duplicated instance (of the first family that duplicates it, in the order `arc-authentication-results`, `arc-message-signature`, `arc-seal`). Step 7: at sorted position `p`, the first of `arc-seal`, `arc-message-signature`, `arc-authentication-results` whose `i=` is not `p` |
| `arc.cv_invalid.v1` | `fail` | step 7 | the first `arc-seal`, by instance, whose `cv=` is not `none` at `i=1` or not `pass` later |
| `arc.body_hash_mismatch.v1` | `fail` | step 7 | the newest `arc-message-signature` |
| `arc.key_missing.v1` | `fail` | step 7 | the field whose key was asked for |
| `arc.key_invalid.v1` | `fail` | step 7 | the field whose key was asked for |
| `arc.signature_mismatch.v1` | `fail` | step 7 | the field whose check failed |
| `arc.not_evaluated.v1` | `not_evaluated` | step 6 or 7 | the newest `arc-message-signature` (step 6); the field whose key lookup failed temporarily (step 7) |

Weights follow accepted 0122 section 2 with no new exception: 10 for each
rule, 0 for `arc.not_evaluated.v1`.

The key rules follow accepted 0125's DKIM rows:

- `arc.key_missing.v1`: the lookup found no record. The name is undeclared,
  or it is outside 0120's grammar and was never looked up. Under 0120 the
  declared map is the world, so an undeclared name is a definite "no record",
  not missing evidence.
- `arc.key_invalid.v1`: a record is declared and is not a usable key: it is
  not a key record, it is revoked (an empty `p=`), or its type does not match
  the field's `a=`.
- A declared `tempfail` is `arc.not_evaluated.v1`.

Each fix hint is a constant with no character the page or a JSON string must
escape. The guide lists each id with its severity, state and hint, and a test
binds the guide to the code.

### Instance and location attribution

- **`occurrence`** is always the root occurrence: the field's index among the
  root list's fields of the same name, in wire order. It is never the chain
  index. The crate's lists are sorted by `i=`, so a crate field is mapped to
  its root occurrence by its position in the message, as DKIM does. Step 1's
  count equality makes the two views line up.
- **The failing seal** comes from the number of key lookups the crate made in
  this one `verify_arc` call. The crate looks up the newest
  `ARC-Message-Signature`'s key first, then each `ARC-Seal`'s key from
  instance `N` down to 1, one lookup each, and stops at the first failure. So
  a failure after one lookup (or none) is the newest `ARC-Message-Signature`,
  instance `N`, and a failure after `k` lookups, `k` at least 2, is the
  `ARC-Seal` of instance `N - k + 2`. This is exact for every chain length and
  costs no extra parse and no extra signature check.

  The instance named is the newest seal that does not verify. Each seal
  covers every older set, so bytes altered in an older set after a newer hop
  sealed are reported at the newest seal that covers them, which is where RFC
  8617 validation stops too. An older seal is named when the newer ones
  verify: a hop sealed over a seal that was already broken, or the declared
  key for the older seal's selector is not the one it was sealed with.
- **`failing_instance`** per rule:
  - `malformed`, `unsupported`: the field's own `i=` literal when it is 1 to
    50, else `null`;
  - `set_incomplete`: the lowest instance for which the three families do not
    hold the same number of fields;
  - `signature_expired`, `body_hash_mismatch`: `N`;
  - `instance_invalid`: the duplicated instance (step 5) or the crate's `p`
    (step 7);
  - `cv_invalid`: that seal's instance;
  - `key_missing`, `key_invalid`, `signature_mismatch`: the instance of the
    located field;
  - every `not_evaluated` state: `null`.

### Time

Swarmail reads no clock. Expiry is judged at the receipt second
(`max(received_ms, 0) / 1000`), as accepted 0125 section 4 does for DKIM.

The crate has no entry point that takes a time. `verify_arc` reads
`SystemTime::now()` once, after its count checks, and uses the value in one
place: `signature.x == 0 || (signature.x > signature.t && signature.x > now)`
on the newest `ARC-Message-Signature`. Step 6 guarantees that field has no
`x=` tag whenever the verifier is called, and step 3 guarantees the crate's
`x` is then exactly 0. The left side of the `||` is true, and the clock value
is never compared. A report is a function of the stored bytes, the receipt
second and the policy. This is hold 2.

The price: a chain whose newest `ARC-Message-Signature` carries an `x=` after
the receipt second is not verified. It reports `arc.not_evaluated.v1`, an
error, so it is never clean. Whether real sealers set `x=` on that field was
not measured here.

### Key source and lookups

0120's declared TXT map, through `Lookups`, is the only key source. Every key
name inside 0120's grammar that the crate asks for is recorded in
`dns.lookups`. A name outside the grammar is not looked up, not recorded, and
answers "no record". One evaluation asks at most 51 times (one for the newest
`ARC-Message-Signature`, one per seal, at most 50 seals).

### Work bound

- The strict parse hashes the body at most once per distinct
  (canonicalization, hash) pair of an `l=`-less signature field, DKIM and ARC
  together: at most four hashes over at most 8 MiB. It rejects every
  `ARC-Message-Signature` with a non-zero `l=` unhashed. ARC adds no hash to
  it.
- **The DKIM re-parse.** With `arc` compiled in, DKIM's non-strict re-parse
  would also hash every `ARC-Message-Signature` that carries a distinct `l=`.
  `dkim::prepare` therefore re-parses only when the number of
  `DKIM-Signature` fields carrying `l=` plus the number of
  `ARC-Message-Signature` fields carrying `l=` is at most 16. Otherwise it
  emits the existing `dkim.limits_exceeded.v1`, located at the first
  `DKIM-Signature` field that carries `l=`, and does not re-parse. Accepted
  0125 section 6's "at most one re-parse (at most 16 more hashes)" stays
  literally true. Only a message with at least one DKIM `l=` and many ARC
  `l=` fields is affected.
- Verification: at most one `ARC-Message-Signature` check and 50 seal checks
  (step 5 caps a chain at 50 sets before the verifier runs).
- A report gains at most one finding and one small member, inside 0085's 256
  findings and 65,536 bytes.

### Authentication-Results correspondence (normative)

`arc.state` holds the token `Authentication-Results` would carry:

- `arc=none`: `state` is `none`.
- `arc=pass`: `state` is `pass`.
- `arc=fail`: `state` is `fail`; `reason` and the one `arc.*` finding say why.
- No `arc=` clause: `state` is `not_evaluated`.

The guide's existing correspondence paragraph gains these lines beside
`spf=`, `dkim=` and `dmarc=`. `arc=pass` says the chain validated. It does
not say the sender is authentic, and nothing else in the report reads it.

### Report and version

`RULE_SET_ID` becomes `swarmail-html/9`, the next never-shipped id (0122
section 1.4), and every literal that names the id moves with it. The report
is `{rule_set_id, findings, dns, arc, total}`. MCP serializes the same report
with sorted keys. The cache key is unchanged: the rule-set id, the mail
identity (which carries the receipt time) and the policy already cover every
input.

`qualifies` is unchanged. Every `arc.*` finding is an error, so a broken or
unevaluated chain qualifies at no threshold, and a passing chain or no chain
adds nothing.

### Live surfaces

- **REST:** the report JSON carries `arc`. OpenAPI's `DeliverabilityReport`
  gains the member in `properties` and `required`, with a closed schema.
- **MCP:** `swarmail_get_deliverability_report`'s `outputSchema` gains the
  member and the `required` entry; `structuredContent` carries it; the tool
  description names the ARC results. The structured-output fixtures are
  regenerated with their provenance.
- **Message page:** the report section shows one escaped ARC line: the state,
  the set count, and the failing instance when there is one.
- **assert / await:** no code change. `no_findings_at_or_above` reads the
  `arc.*` findings like any other.
- **Docs:** the guide gains an "ARC (`swarmail-html/9`)" section; `llms.txt`
  and the MCP description name the states and the rule ids.

## Frozen guarantees and failure modes

- **0001, accept == stored.** No line of `Store::insert`, the SMTP session or
  ingest changes. ARC runs after the `250`, from the stored bytes.
- **0002, 0003.** Await and pruning are untouched.
- **0005.** Nothing is extracted at query time into the row; the report is a
  separate derived product, as 0085 rules.
- **A crash mid-analysis** leaves the cache slot empty. The report is
  published whole or not at all, so no partial `arc` member can be served.
- **A pruned or deleted message** keeps 0085's behaviour: the slot's `Weak`
  is dead and the report is unavailable.
- **Hostile mail** meets the bounds above. A bound or a malformed field is a
  finding, never a panic and never `AnalysisError::Failed` for sender-shaped
  data (0125's ruling).
- **No shared mutable state** is added: each evaluation owns its `Lookups`,
  adapter and lookup counter.

## Panel adjudication — 2026-11-16

Two positions were argued. Where they conflict, the ruling and the loser's
cost are stated.

| question | ruling | what the losing side costs |
|---|---|---|
| A typed `arc` report member (robustness) or findings only, with `arc=` read off a table (simplicity) | **The member.** In a findings-only report `pass` and `none` are both an absence, so a test that asserts "my forwarder's chain validates" passes when the forwarder sealed nothing. The goal asks the report to carry the state and the instance, and a finding's location is a header occurrence, not an `i=`. | A second hold (0085 section 1), new bytes in every report, two schema changes, a hand-regenerated fixture and a page line. Simplicity's point that 0126 refused an `auth` member stands for SPF, DKIM and DMARC: there `none` is itself a finding. For ARC `none` is the healthy state of ordinary mail and cannot be one. |
| Which seal failed: lookup count (simplicity) or prefix bisection up to 8 sets (robustness) | **Lookup count.** Exact for all 50 sets, no extra parse, no extra crypto. | The attribution depends on the pinned crate's lookup order. Fixtures that break the seal at `i=1` of 2 and `i=2` of 3 fail loudly if an upgrade reorders it. Robustness said itself that the bisection's truncation proof was not built. |
| A Swarmail structural pre-scan of `i=`, `cv=`, counts and the 50-set cap (robustness) or the crate's verdict (simplicity) | **The crate's verdict** for the instance sequence and `cv=`. **Swarmail's** for what the crate cannot say or settles silently: none against incomplete, the boundary, the tag checks of step 3, and duplicated instances, which the crate would settle by an unstable sort. | Robustness loses RFC-natural precedence for a chain that is both structurally wrong and carries `x=`: it reports `not_evaluated`, not `fail`. A full pre-scan would be a second owner whose every line needs a fixture, and it would leave the crate's structural arms unreachable under the exact coverage gate. |
| `x=`: any `x=` is not evaluated (simplicity) or split at the receipt second (robustness) | **Split.** `x=` at or before receipt is a real expiry at the time 0125 section 4 judges by, so it is `fail`. | One more rule id and fixture pair. |
| A missing key: `key_missing`, weight 10 (simplicity) or `arc.not_evaluated.v1`, weight 0 (robustness, and the intake's AC2) | **`arc.key_missing.v1`.** Under 0120 the declared map is the world, and 0125 already rules an undeclared key name a definite absence for DKIM. | A test that forgets to declare a forwarder's key scores 10, as it does for DKIM. The intake's wording is corrected toward accepted 0125. |
| Not-evaluated outcomes: state `fail` (robustness) or no `arc=` (simplicity) | **A fourth state, `not_evaluated`.** | The member has four values where RFC 8617 has three. |
| The `l=` re-parse: fail the report (robustness) or a counted guard (simplicity) | **The counted guard**, and ARC always reads the strict parse. 0125 refused `Failed` for sender-shaped data. | A hostile message with a DKIM `l=` and many ARC `l=` fields loses its per-signature DKIM verdicts to `dkim.limits_exceeded.v1`. |
| Pre-parse ARC bounds from a root scan (robustness) | **Refused.** The strict parse's four-hash bound holds with `arc` on (read in `message.rs`, `parse_ams`), and the guard above bounds the re-parse. | None found. Robustness's amplification figures were read from source, not measured, as its position says. |
| How many ids: eight (simplicity) or one per cause (robustness) | **Thirteen**, the ones whose hint tells the reader to do something different, with the key rows named as DKIM's are. Selector-invalid and key-revoked are folded into `key_missing` and `key_invalid`. | Five more fixture pairs than simplicity's table. |
| `ARC-Authentication-Results` contents and any effect on DMARC or the facts | **Both positions agree: not read, no effect.** | — |

Robustness's claim that a literal `l=0` "verifies any body" is not adopted as
stated: the pinned crate hashes the whole body for `l=0`
(`dkim/canonicalize.rs`, `canonical_body`). Step 3 still refuses every `l=`
tag, so the question does not arise.

## Acceptance Criteria

Each criterion is proven by the task that names it in
[tasks.md](tasks.md).

- **AC1 — Closed rule table.** `arc.rs` holds exactly the thirteen rules
  above. Each has its id, `error` severity, state, constant hint and location
  as specified, and each has at least one passing and one failing inline
  fixture. A test binds the table to the guide.
- **AC2 — The framing's fixtures.** Inline, and over real SMTP in
  `tests/deliverability.rs` for the first three and the modified body:
  - a valid single-hop chain: `pass`, `sets` 1, no `arc.*` finding;
  - a valid two-hop chain: `pass`, `sets` 2;
  - a broken seal at `i=1` of a two-hop chain (the map declares a different
    key of the same type for that seal's selector, and the newer set
    verifies): `fail`, `arc.signature_mismatch.v1` at that `arc-seal`'s root
    occurrence, `failing_instance` 1;
  - the oldest seal's `b=` altered after the second hop sealed: `fail`,
    `arc.signature_mismatch.v1` at the newest `arc-seal`, `failing_instance`
    2;
  - a body modified after sealing: `fail`, `arc.body_hash_mismatch.v1` at the
    newest `arc-message-signature`, `failing_instance` `N`;
  - a gap in `i=` (sets 1 and 3): `fail`, `arc.instance_invalid.v1`,
    `failing_instance` 2;
  - a missing set member, in two forms: no `ARC-Authentication-Results` for
    one instance, and ARC fields with no `ARC-Message-Signature` at all. Both
    are `fail`, `arc.set_incomplete.v1`, never `none`;
  - a key absent from the DNS map, for the newest `ARC-Message-Signature` and
    for a seal: `fail`, `arc.key_missing.v1` at that field.
- **AC3 — The member's invariants** hold for every fixture in the suite, by
  one table test. `none` is reported only when no ARC-family field exists.
- **AC4 — Attribution.** In a three-hop chain whose `i=2` seal alone does not
  verify against its declared key, the finding names that seal and instance
  2. Every `occurrence` equals the index computed from the root list,
  including for a chain written oldest-first.
- **AC5 — Crate defaults do not decide.** Each is `arc.malformed.v1` and
  causes no lookup: a repeated tag; a missing `a=`; a non-numeric `x=` and
  `t=`; a missing `i=`; `i=0`; `i=51`; `i=4294967297`; an
  `ARC-Authentication-Results` whose `i=` is not first. Each is
  `arc.unsupported.v1`: an `ARC-Message-Signature` with `l=5`, with `l=0`;
  `a=rsa-sha1` on a signature and on a seal.
- **AC6 — Boundary.** With an ARC field present, each gives exactly
  `arc.header_boundary_ambiguous.v1` and no lookup: a colon-less line before
  and after the ARC fields; a leading whitespace-only line; `ARC-Seal :` (a
  space before the colon) beside a valid chain, and alone (location
  `arc-seal`, `null`). The same guard is shown to fire on mail with no
  `DKIM-Signature`. A test lists every spelling the root scanner names as an
  ARC family and shows the crate lists exactly those fields.
- **AC7 — Time.** With the newest `ARC-Message-Signature` rewritten to carry
  `x=1000`: at receipt 999 the report is `arc.not_evaluated.v1` with empty
  `dns.lookups`; at receipt 1000 it is `arc.signature_expired.v1`. With `t=`
  and `x=` equal it is `arc.signature_expired.v1`. No test asserts wall-clock
  validity, and no kit fixture that must pass carries `x=`.
- **AC8 — Keys and lookups.** Every key name asked for appears in
  `dns.lookups`. A declared `tempfail` on the signature's key and on a seal's
  key gives `arc.not_evaluated.v1` at that field, state `not_evaluated`, and
  the report qualifies at no threshold. A selector that puts the name outside
  0120's grammar gives `arc.key_missing.v1` and no recorded lookup. A
  declared value that is no key record, a revoked key and a key of the other
  type each give `arc.key_invalid.v1`. A wrong key of the right type gives
  `arc.signature_mismatch.v1`.
- **AC9 — Structure.** `cv=pass` at `i=1`, `cv=none` at `i=2` and `cv=fail`
  give `arc.cv_invalid.v1` at that seal. A chain whose instances start at 2
  gives `arc.instance_invalid.v1` with `failing_instance` 1. A duplicated
  instance with equal family counts gives `arc.instance_invalid.v1` at the
  second field that carries it, whichever order the two are written in.
  Fifty-one sets give `arc.instance_invalid.v1` and no lookup. An
  `ARC-Message-Signature` with 129 `h=` names gives `arc.limits_exceeded.v1`.
- **AC10 — Determinism.** For every fixture, two evaluations on fresh
  services with the same policy give byte-identical report JSON.
- **AC11 — Accept == stored.** Over real SMTP an ARC-bearing message gets its
  `250`, is queryable at once, and its stored row and raw bytes are identical
  before and after the report is read. The burst, p2 and rate suites pass
  unchanged.
- **AC12 — DKIM is undisturbed.** Every existing DKIM, SPF, DMARC and bulk
  test passes with the feature on, with only the rule-set id and the new
  member changed in expected bytes. ARC's verdict for a chain is the same
  with and without an unrelated `DKIM-Signature` that carries `l=`.
- **AC13 — The re-parse guard.** One DKIM `l=` field beside 15 ARC `l=`
  fields is re-parsed and judged as before. Beside 16 it gives
  `dkim.limits_exceeded.v1` at the first DKIM `l=` field and no other
  `dkim.*` finding.
- **AC14 — No fact moves.** For a message whose chain passes, fails and is
  absent, the six `AuthEvidence` facts and every `bulk.*`, `spf.*`, `dmarc.*`
  and `dkim.*` finding are identical.
- **AC15 — Surfaces.** Over the wire, REST, MCP `structuredContent` and the
  message page show the same `arc` member for the two-hop pass and the broken
  seal. The OpenAPI and MCP schemas require the member and validate both
  reports.
- **AC16 — assert and await.** An exact assert and an await with
  `no_findings_at_or_above=error` do not count the broken-seal message. For
  two messages that qualify without ARC, adding a valid chain to one leaves
  both qualifying.
- **AC17 — Version.** `RULE_SET_ID` is `swarmail-html/9`; no literal
  `swarmail-html/8` remains outside history (decision records, earlier specs,
  release notes, the guide's version history). The drift tests pass.
- **AC18 — Dependencies.** `git diff f05f2ee -- Cargo.lock evals/Cargo.lock
  deny.toml` is empty. `Cargo.toml` differs in the mail-auth feature list and
  its comment only. `cargo deny check licenses` is clean and `native-tls` is
  absent.
- **AC19 — Docs.** The guide, `llms.txt` and the MCP description name the
  four states, the thirteen ids and the correspondence. The doc-contract tests
  pass. 0133 has its README row, still proposed.
- **AC20 — Gates.** Every gate in house rules § Gates is clean, `--locked`,
  with exact 100% coverage, the coverage selftest and the publish-guard
  proof.
