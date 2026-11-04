# Deliverability DNS policy (#253)

Decision: 0120 — [docs/decisions/0120-deliverability-dns-policy.md](../../docs/decisions/0120-deliverability-dns-policy.md) (proposed; only the operator accepts it)

Epic #251, issue #253, priority P1. Run `issue-253-the-deliverability-dns-c704444e`,
design phase, 2026-11-03, base `main` `8732445`. Framing: [intake.md](intake.md).
Revised 2026-11-04 in run `issue-253-the-deliverability-dns-c76ebc80` after the
review of `e80aa822` returned the design (decision 0102's 2026-10-31
addendum); [§Review return adjudication](#review-return-adjudication--2026-11-04)
lists each finding and the ruling that answers it.
Authorities:
- accepted [0085](../../docs/decisions/0085-deliverability-report.md) and its
  [contract spec](../deliverability-report-contract/spec.md), including §8 and
  the "#253 extension seam";
- accepted 0045 §8 (code under a proposed decision);
- accepted 0020 (where tests live);
- the frozen guarantees 0001–0003 and 0005.

## Feature and rationale

The authentication stories #254 (DKIM), #255 (SPF/DMARC) and #256 (ARC) need DNS
answers. Real DNS would make a test mail server nondeterministic: answers
change, resolvers time out, and CI has no network. This story therefore builds
the source of DNS answers those stories will read, and it builds no verdict.

- **Offline by default.** A declared TXT map: DKIM selector records, SPF and
  DMARC. It is empty unless configured. A server-wide map comes from a file. A
  per-inbox map comes from the REST API, so parallel tests on one server can
  declare different worlds.
- **Real DNS only by explicit configuration.** In this story that
  configuration exists and is refused. The resolver arrives with #254.
- **Every report says which policy it used and which records it read.** The
  report then explains itself, and a verdict can be reproduced from the report
  alone.
- **A Swarmail-owned lookup interface.** #254 and #255 call one recording
  method. They never touch a resolver, a clock or the network inside the
  evaluator.

The analysis stays where 0085 put it: lazy over the stored bytes, cached, never
on the SMTP path (0001, 0005). The policy is configuration, frozen once per
request and part of the cache key. It is never derived from mail, and it is never
written into mail or the data file.

## Scope

**In:**
- `src/deliverability/dns.rs`: the policy model, the parser, `OwnerName`, and
  the `Lookups` recorder.
- The `dns` report member and `RULE_SET_ID` `swarmail-html/4`.
- `serve --dns-policy FILE` and `serve --dns-resolver declared|real`.
- `PUT` and `DELETE /api/v1/inboxes/{inbox}/dns`.
- Per-request snapshot and cache identity.
- The documentation surfaces that describe the report: OpenAPI, `llms.txt`, the
  MCP output schema and descriptions, and the deliverability guide.
- The contract spec's §Contract and §#253 seam.
- Decision 0120 and its index row.

**Out:**
- Any DKIM, SPF, DMARC or ARC verdict or rule. Those belong to #254, #255 and
  #256.
- A real resolver, or any dependency. #254 owns both under the 2026-10-26 ruling.
- Persistence of per-inbox declarations.
- A `GET` route.
- An MCP setter.
- UI rendering of `dns`.
- TTLs, NXDOMAIN/NODATA distinction, record types other than TXT, wildcard
  names and IDNA.

## Contract

### Declaration shape (file, API body, and the report's `dns.policy`)

```json
{"txt":[{"name":"s1._domainkey.example.com","values":["v=DKIM1; k=rsa; p=MIGf..."]},
        {"name":"example.com","values":["v=spf1 -all"]},
        {"name":"_dmarc.example.com","values":["v=DMARC1; p=reject"]}],
 "tempfail":["_dmarc.example.org"]}
```

- **Members.** `txt` and `tempfail` are both optional on input; an absent
  member is empty. Both are always present on output. Any other member is
  refused.
- **Size (revised 2026-11-04).** One constant, `MAX_DNS_POLICY_BYTES` =
  8,192, bounds two measurements of every declaration:
  - the **raw** bytes: the file and the API body. Over it, the API answers
    `413` and the file is refused at startup;
  - the **canonical** form: the compact serialization of the parsed policy,
    checked after folding and sorting. Over it, the declaration is refused
    with `canonical declaration is larger than 8192 bytes`: `400` on the API
    (the body fitted, the declaration did not) and a startup refusal for the
    file.

  Because both bounds hold for every accepted declaration, and the canonical
  form is a fixpoint (below), a report embeds at most 8,192 policy bytes and
  its `dns.policy` can **always** be declared back verbatim.
- **Entries (added 2026-11-04).** At most `MAX_DNS_POLICY_ENTRIES` = 256 names
  and values together: every `txt` owner, every value and every `tempfail`
  name counts one. One more is refused with
  `declaration has more than 256 names and values`. The byte bound does not
  bound the parsed tree, whose per-string overhead is many times a short
  name's wire size; this count does (§Resource bounds).
- **Owner name.**
  - Must be ASCII.
  - Folded to lowercase, with one trailing dot stripped.
  - Then dot-separated labels, each 1–63 bytes of `[a-z0-9_-]`, with a total
    length of 1–253 bytes.
  - A declared name matches only itself: no wildcard expansion and no IDNA.
- **TXT value.**
  - The record's logical value, with its character-strings already
    concatenated.
  - Exact printable ASCII bytes `0x20`–`0x7E`; the empty string is allowed.
  - Never trimmed or case-folded.
- **Refusals.** None of these is repaired:
  - two `txt` entries whose names fold to one owner;
  - a duplicate value under one owner;
  - an empty `values` list;
  - a duplicate `tempfail` name after folding;
  - a name in both `txt` and `tempfail`;
  - a name or value outside the grammar;
  - non-UTF-8 or non-JSON input;
  - raw input over the size bound, or a canonical form over it;
  - more than 256 names and values.
- **Canonical form.**
  - `txt` is sorted by name, and each `values` list by byte order.
  - `tempfail` is sorted.
  - Both members are always spelled. Canonicalization is idempotent: parsing
    a canonical form gives the same policy, which serializes to the same
    bytes.
  - *Withdrawn 2026-11-04:* round one allowed the canonical form to be the
    accepted input plus up to 24 bytes (the two omittable members) and claimed
    the one raw bound also capped the embedded form. The review showed an
    accepted 8,192-byte input whose 8,205-byte canonical form a report embeds
    and the API then refuses. The canonical bound above replaces that rule.
    Its cost: an input within 24 bytes of the bound that omits a member is
    refused, with a message that says so.

### Lookup interface (crate-private; #254/#255 call it)

```rust
pub(crate) struct OwnerName(/* canonical */);
impl OwnerName { pub(crate) fn parse(name: &str) -> Option<OwnerName>; }
pub(crate) enum Txt { Records(Vec<String>), NoRecords, TempFail }
pub(crate) struct Lookups<'a> { /* frozen snapshot + used: BTreeMap<OwnerName, Txt> */ }
impl Lookups<'_> { pub(crate) fn resolve_txt(&mut self, name: &OwnerName) -> Txt; }
```

- **Answers.** A name under `txt` answers `Records` with its canonical values.
  A name under `tempfail` answers `TempFail`. Any other name answers
  `NoRecords`.
- **Pure.** No network, clock, global or lock.
- **Recording.** Every answered name is recorded once. A repeated lookup returns
  the same answer and records nothing new.
- **Meaning of the answers.** `NoRecords` is a definite negative. `TempFail` is
  never a pass and never a clean result: a verifier that meets it owes a
  `not_evaluated` finding or an analysis failure (0120 §4).
- **Enforced, not only promised (added 2026-11-04).** `Report::qualifies`
  returns `false` at every threshold when any entry of `dns.lookups` has
  outcome `temp_fail`, whatever the findings say. A verifier that forgets its
  `not_evaluated` finding still cannot make a temporarily failing world
  clean. An exact assert therefore never matches such a report, and a wait
  with a threshold does not return it: it reaches its existing deadline
  outcome (0085's wait deadline) unless another candidate qualifies. In
  production today `lookups` is always empty, so no behaviour changes until
  #254 adds a caller.
- **No production caller yet.** Nothing in production calls `resolve_txt` until
  #254. The items production does not reach carry
  `#[cfg_attr(not(test), expect(dead_code, reason = "#254/#255 verifiers call it"))]`.
  `expect`, not `allow`, so the first real caller must delete the attribute.
  That is a lint attribute, not a coverage exclusion.

### Report

```json
{"rule_set_id":"swarmail-html/4","findings":[...],
 "dns":{"source":"default","policy":{"txt":[],"tempfail":[]},"lookups":[]}}
```

- `source` is `default`, `server` or `inbox`.
- `policy` is the complete canonical effective policy.
- `lookups` lists each resolved name once, sorted by name, as
  `{"name","type":"txt","outcome":"records"|"no_records"|"temp_fail","values":[...]}`.
  `values` is empty unless the outcome is `records`.
- No path, time or generation appears anywhere in the report.
- `Report` stays `deny_unknown_fields`, and `dns` has no default.
- The 65,536-byte report bound covers the whole report. Exceeding it is
  `deliverability analysis failed`, never truncation.
- Member order is `rule_set_id`, `findings`, `dns`.
- A report whose `dns.lookups` holds a `temp_fail` outcome never qualifies
  (§Lookup interface).

### Configuration and API

- `serve --dns-policy FILE` (env `SWARMAIL_DNS_POLICY`) sets the server map.
  - `Config` gains `dns_policy: Option<PathBuf>`.
  - `run_on` loads the file before it opens the data file and before any
    listener binds.
  - The file must be a regular file. It is read through a bound of 8,193 bytes.
  - Any refusal makes `run_on` fail with `dns policy file <path>: <reason>`, so
    `serve` exits non-zero. There is no fallback to `default`.
  - `{}` is a valid, empty server map.
- `serve --dns-resolver declared|real` (env `SWARMAIL_DNS_RESOLVER`) defaults to
  `declared`.
  - The argument parser refuses `real` with
    `real DNS needs the resolver that #254 adds; this build has none`.
  - Real mode is never selectable through the API.
- `PUT /api/v1/inboxes/{inbox}/dns` sets the inbox map.
  - The inbox label is validated like every other inbox route, and in
    addition may be at most `MAX_DNS_INBOX_LABEL_BYTES` = 512 bytes (RFC
    5321's command-line length, so every inbox a compliant `RCPT TO` names
    fits). A longer label is `400` `inbox label is longer than 512 bytes`
    before the body is parsed. Such an inbox can still hold mail; it cannot
    hold a declaration.
  - The body is limited to 8,192 bytes; over the limit is `413`. An invalid
    body, including a canonical form over 8,192 bytes or more than 256 names
    and values, is `400`. Either way, nothing changes.
  - `200` returns `{"inbox","source":"inbox","policy"}`.
  - At most 4,096 inboxes hold a declaration. A new one beyond that is `503`
    and evicts nothing. Replacing an existing one is always allowed.
- `DELETE /api/v1/inboxes/{inbox}/dns` clears the inbox map.
  - It is idempotent.
  - `200` returns the resulting effective `{"inbox","source","policy"}`, where
    `source` is `server` or `default`.
  - The echo is the fallback itself: the server map, else the default.
    `Service::forget` removes the entry and returns that fallback without
    reading the inbox declarations again, so a `PUT` racing the `DELETE` can
    never make the echo say `source:"inbox"` (revised 2026-11-04). A racing
    `PUT` is simply the next state; the next report sees it.
  - `DELETE` stores nothing, so it has no label cap beyond `admit`.
- An inbox map replaces the server map for that inbox. There is no merge.
- Declarations are process-local and not persisted. Clearing an inbox's mail
  does not clear its declaration.

### Resource bounds (added 2026-11-04)

The 4,096-inbox cap counts declarations; the two byte bounds count bytes.
Neither alone bounds resident memory, so the entry count and the label cap
above exist, and the worst case is stated rather than left to be discovered:

- one declaration holds at most 8,192 bytes of names and values plus at most
  256 strings and tree slots of allocator overhead (about 100 bytes each, so
  about 25 KiB), plus its inbox key of at most 512 bytes: under 34 KiB;
- 4,096 declarations: under 140 MiB at a completely full registry, from at
  most about 34 MiB of accepted request bytes;
- the 128 report-cache slots may each keep one replaced snapshot alive: under
  5 MiB more.

The memory is process-local and is released on `DELETE` or restart. It sits
inside the house's accepted model of an unbounded mail store on a test server
(0038). No allocator measurement is made in a test: it would depend on the
allocator, and the bounds above are enforced by counts that are tested
exactly. The route stays unauthenticated under the plane-wide Origin guard,
as every inbox route is (0004).

### Snapshot, cache, expiry and wake

- **Snapshot.** Each request resolves the effective policy once, from the
  message's inbox, at its entry: a message report, an exact assert, a REST await
  or an MCP wait. All its candidates and iterations use that `Arc` snapshot.
- **Waits.** A wait takes its snapshot before it subscribes. The
  subscribe-then-scan order (0002) is unchanged.
- **Cache.** `Inputs` carries the effective policy, source plus content, so
  0085's exact cache key covers it.
- **Expiry, refresh and wake.**
  - Declared maps never expire.
  - Refresh is a new declaration.
  - A declaration change wakes no waiter. In-flight requests keep their
    snapshot, and the next request sees the change.
  - #254 defines live-answer TTL, refresh and wake before live answers exist.

## Panel adjudication — 2026-11-03

The simplicity and robustness positions agreed on most of the design, and both
were adopted where they agreed:
- one new module;
- canonical `BTreeMap`/`BTreeSet` content compared exactly in the cache key, so
  no hashing crate is needed;
- three outcomes;
- process-local inbox declarations;
- no network, clock or dependency;
- refusal of real mode before bind;
- the `swarmail-html/4` bump under an operator hold;
- no new rule.

Where they conflicted:

| Question | Ruling | Losing side's cost |
|---|---|---|
| What the report embeds | **Robustness:** the complete effective policy plus the used lookups. With no rule in this story, used-only reports are identical under every map, so AC8 is unobservable and no verdict can be reproduced. | Simplicity: a report grows by up to 8 KiB when a map is declared, and the map bound drops to 8 KiB. |
| Size limits | **Simplicity:** one byte constant (8,192). Robustness's per-owner, per-value and per-length constants are redundant under it. *Corrected 2026-11-04:* the claim that the raw bound "also bounds the embedded canonical form" was false (canonical could be raw + 24), and the claim that count constants were redundant was false for resident memory. The constant now bounds the raw and the canonical form, and one combined entry count (256) is added; see the review return adjudication. | Robustness: refusal messages say "too large", not which per-owner count was exceeded. |
| Declaration shape | **Neither as proposed:** `{"txt":[{"name","values"}],"tempfail":[...]}`. An array avoids serde's silent last-wins on duplicate object keys (robustness's object shape), and grouping per owner (rather than simplicity's flat record list) makes input, echo and report one shape, so a report's `dns.policy` can be declared back verbatim. | Simplicity: a duplicate after folding, or a duplicate value, is refused rather than collapsed. |
| Name and value grammar | **Robustness:** label grammar, exact printable-ASCII values, folding of names only. | Simplicity: one more validator branch per rule, each needing a failing fixture. |
| Lookup interface | **Simplicity:** a concrete recorder, no trait, and no `Real` variant. Adopt robustness's typed `OwnerName`, so header garbage never reaches a lookup or a report. | Robustness: there is no fail-closed `Real` arm in the evaluator, and #254 must add the arm with its resolver. |
| Inbox declaration bound | **Robustness:** 4,096 entries, `503` at capacity, no eviction. Simplicity's own fallback offered the same. | Simplicity: one constant and a wire proof that fills it. |
| Server file load | **Neither as proposed:** a `Config` path loaded in `run_on` before the data file and every bind, with robustness's file hardening. Simplicity's clap-parser load would make the policy type public through `Config`. | Simplicity: about ten `Config` literals in tests gain `dns_policy: None`, and pre-bind ordering is proven by a startup test, not by clap. |
| `GET` route | **Simplicity:** cut. `PUT`/`DELETE` echo (robustness) and the reports are the observation surface. | Robustness: a client reads the current policy only by writing it or by reading a report. |
| Split delivery | **Refused.** The record in the report is the issue's purpose. | Robustness: no part of #253 merges before the operator accepts 0120. |
| `dns_tests.rs` harness, lookup-count limit, wake on change, persistence, MCP setter, UI | **Simplicity:** all cut. There is no cold path, the report byte bound already refuses, inputs are frozen per request, and the brief did not ask for the rest. | Robustness: process-local declarations vanish on restart. Every report's `source` makes that visible. |

## Review return adjudication — 2026-11-04

The review of `e80aa822` (tree `be3dc0d`) returned the design with a spec
defect and a security residual. All five members found the frame, the scope,
the deferral of the resolver to #254 and the frozen guarantees sound, so this
is a revision of the committed design, not a new one. The simplicity and
robustness positions were argued against the findings. Where they agreed,
both were adopted: enforce `TempFail` in `Report::qualifies`; make `DELETE`
echo the fallback without a second read; bound resident memory with counts
and state the product; pin the documentation surface; decline the
`load_file` race fix and record why. Where they conflicted:

| Finding (member, severity) | Ruling | Losing side's cost |
|---|---|---|
| The 8,192 bound does not cap the embedded canonical form, so a boundary report cannot be declared back (guarantees, medium, spec defect) | **Simplicity:** the one constant applies to the raw bytes **and** to the canonical form; the "+24" rule is withdrawn. Robustness's two constants (raw 16,384, canonical 8,216) also close the round trip, as its own position concedes, but add a second number to every surface for no extra guarantee. Robustness's point is kept: the bound is measured on the form a report embeds, and an over-canonical body is `400`, not `413`. | Robustness: an input within 24 bytes of the bound that omits a member is refused although it fitted on the wire. |
| AC9's proof scans only `smtp.rs` and `persist.rs` (security, spec defect) | **Robustness:** keep a structural proof that can go red on the Store mutation code, not only a restatement. The scan covers `src/smtp.rs`, `src/persist.rs`, `src/extract.rs` and every `src/extract/*.rs`, and every `src/store.rs` function that takes the Store's mutation lock (`self.mutation.lock()`), which includes `Store::insert` and its oldest-only prune. It names the read-side exemption (`qualify`, `wait_for_report`, `report`). Simplicity's widening of the file list is adopted inside it. | Simplicity: a text scan of function bodies is shape-sensitive; the test must also assert it found `insert` among the scanned functions, so a refactor that hides the lock cannot make it vacuous. |
| AC10 says "a `PUT`", the held proof calls `Service::declare` (security, secondary) | **Robustness:** keep AC10's words and move the proof to a real `PUT`. The held tests in `src/deliverability/concurrency_tests.rs` start an in-crate loopback `api::serve` over the same `Store` (the pattern `src/api.rs`'s inline router tests already use; no new dependency, since `tower` is not a direct one) and issue the `PUT` over HTTP while the gate holds the evaluation, once inside an exact assert and once inside a wait's scan after its subscribe. The existing direct-declare tests stay. | Simplicity: a heavier test; one more loopback server inside the harness. |
| Resident memory unbounded in items; inbox key unbounded (security, low; independent, low) | **Simplicity's count, robustness's discipline:** one combined count of names and values (256), because a names-only count (robustness's 64) still lets about 2,000 tiny values amplify; a 512-byte label cap on `PUT`; the worst case is stated in §Resource bounds and 0120 §8. | Robustness: the worst case is about 140 MiB rather than its estimated 64 MiB. Both are inside 0038's accepted model; raising or lowering either count later is an ordinary change. |
| `TempFail` "never clean" is prose only (security) | **Both:** enforced in `Report::qualifies`, with a failing and a passing fixture, and the wait's deadline behaviour stated. | None. |
| `DELETE` may echo `source:"inbox"` under a racing `PUT` (correctness, low) | **Both:** `forget` returns the fallback by construction. | None. |
| `load_file` stats, then opens (correctness/independent, info) | **Both:** declined in code. Closing it needs `O_NONBLOCK` and therefore `libc` as a production dependency, which AC16 forbids and which would spend the epic's one widening. 0120 §9 and the `load_file` comment state the residual truthfully: a FIFO swapped in between the two calls can hang startup; the path is operator configuration. | The startup hang stays possible for whoever controls the configured path. |
| Routes, schemas, `llms.txt` and guide text are unpinned (tests, info) | **Both:** extend the existing route and phrase lists, assert the four schemas exist, and add a `$ref` resolvability check over the served OpenAPI document. | None. |
| `Service::pin` comparison order; public `Config`/`ServeArgs` gain fields; sibling `RULE_SET_ID` bumps (info) | No change: correct as written, the `pop3_idle_timeout` precedent, and the one-line integration fix 0120 already records. | None. |

## Acceptance Criteria

Each criterion names the proof that can fail it.

- **AC1 — Default policy, exact JSON.** With no DNS configuration, the report
  for a message carrying a `DKIM-Signature` header is exactly the previous
  report with `rule_set_id` `swarmail-html/4` and
  `"dns":{"source":"default","policy":{"txt":[],"tempfail":[]},"lookups":[]}`.
  The existing exact-JSON assertions in `tests/deliverability.rs` and
  `tests/bulk_sender.rs` compare whole bodies.
- **AC2 — Server map and its refusals.**
  - `serve --dns-policy FILE` and `SWARMAIL_DNS_POLICY` load a map, and reports
    then carry `source:"server"` and the canonical policy. `{}` gives
    `source:"server"` with empty lists.
  - Each of these makes `run_on` return an error naming the file and the
    reason, and makes the binary exit non-zero, with the configured SMTP and
    HTTP ports still bindable afterwards:
    - malformed JSON, an unknown member, or an empty file;
    - non-UTF-8 bytes, or 8,193 bytes;
    - a declaration whose canonical form exceeds 8,192 bytes, or one with 257
      names and values;
    - a FIFO or a directory;
    - a bad name, a bad value, or `values:[]`;
    - two names that fold to one owner, a duplicate value, or a `txt`/`tempfail`
      overlap;
    - a missing file.
  - Proofs: parser unit tests (one per refusal), a `run_on` startup test, and
    one binary exit test.
- **AC3 — Per-inbox map over real HTTP.**
  - `PUT /api/v1/inboxes/a/dns` returns `200` with the canonical echo. Reports
    for inbox `a` show `source:"inbox"`. Inbox `b` still shows the server or
    default policy.
  - `DELETE` returns `200` with the fallback policy and is idempotent.
  - An invalid body is `400`, an oversized body is `413`, and an invalid inbox
    label is `400`. Each leaves the previous declaration byte-identical in the
    next report. The invalid bodies include a canonical form over 8,192 bytes
    and 257 names and values; the invalid labels include one of 513 bytes,
    while a 512-byte label is accepted.
  - The inbox map replaces the server map. It is not merged.
  - Clearing inbox `a`'s mail keeps its declaration.
- **AC4 — Lookup outcomes.** Unit tests over one declared map cover:
  - `Records` with canonical values for a DKIM selector name, the SPF name and
    the DMARC name;
  - `TempFail` for a declared name;
  - `NoRecords` for an undeclared name;
  - a repeated lookup recorded once;
  - an `OwnerName::parse` table (uppercase, trailing dot, underscore labels,
    253/254 bytes, 63/64-byte label, empty label, NUL, CRLF, space, non-ASCII),
    with `None` and never a panic.
- **AC5 — Real mode refused.**
  - `--dns-resolver real` and `SWARMAIL_DNS_RESOLVER=real` fail argument
    parsing with the fixed message naming #254, so nothing binds or looks up.
  - `declared` parses, and is the default.
  - Proofs: a `Cli::try_parse_from` unit test and a binary exit test.
- **AC6 — Canonical, order-independent and reproducible.**
  - Two declarations with the same content in different name, value and
    `tempfail` order give byte-identical echoes and reports.
  - Declaring a report's `dns.policy` back verbatim gives the same policy,
    **at the boundary too**: a declaration whose canonical form is exactly
    8,192 bytes is accepted, its report embeds those bytes, and `PUT`ting the
    report's `dns.policy` back returns a byte-identical echo and a
    byte-identical next report. A unit test shows that parsing any accepted
    canonical form gives the same policy and the same bytes, and that the
    exactly-8,192-byte raw input omitting `tempfail` (accepted by `e80aa822`
    with an 8,205-byte canonical form) is now refused with the canonical
    message.
- **AC7 — Deterministic across processes.** One message under one server file,
  served by two processes started from different working directories with the
  file at different paths, gives byte-identical report JSON. No path appears in
  either.
- **AC8 — Cache identity.**
  - An owner-local test that counts evaluations shows:
    - an unchanged effective policy reuses the slot (one evaluation);
    - changed content, even a name no rule reads, evaluates again and reports
      the new policy;
    - the same content under `server` and under `inbox` gives two distinct
      reports.
  - A wire test shows a `PUT` between two report requests changes the second
    report.
- **AC9 — Off the SMTP path (proof revised 2026-11-04).**
  - Behavioural: with inbox `a` declaring `tempfail` for every name the
    message's headers mention, SMTP `DATA` returns `250`, the message is
    immediately listed, and its report is computed only when requested.
  - Structural: SMTP (`src/smtp.rs`), ingest (`src/extract.rs` and every
    `src/extract/*.rs`), persistence (`src/persist.rs`) and the Store
    mutation code gain no DNS reference (`dns`, `Effective`, `policy_for`).
    The Store mutation code is every `src/store.rs` function whose body takes
    `self.mutation.lock()`, which includes `Store::insert` and its oldest-only
    prune. The test asserts that `insert` is among the functions it found, so
    it cannot pass vacuously, and it goes red when a DNS reference is added to
    any of them.
  - Named exemption: the Store's read path (`report`, `qualify`,
    `wait_for_report`) holds the effective policy, because 0085 puts analysis
    there.
- **AC10 — One snapshot per request.**
  - Using the existing evaluation gate seam: a `PUT` made while a report
    evaluation or an assert/wait scan is held does not change that request's
    reports, and the next request sees it.
  - Revised proof (2026-11-04): the `PUT` is a real HTTP request to the real
    `set_inbox_dns` handler, served by an in-crate loopback `api::serve` over
    the same `Store`, issued while `Held` holds an evaluation inside an exact
    assert (`qualify`) and, separately, inside a wait's scan after its
    subscribe. Its echo is the new canonical declaration; the held request's
    reports carry the old policy; the next request carries the new one.
  - The wait path takes the snapshot before its subscribe, and the 0002
    ordering tests stay green.
- **AC11 — Recorded lookups.** The `#[cfg(test)]` lookup fixture in `Inputs`
  resolves a list of names through `Lookups` inside `collect`. The report then
  lists exactly those names, once each, sorted, with outcome and values, through
  the real cache. Resolving no names gives `lookups:[]`.
- **AC12 — Report bound still holds.**
  - A declared policy whose canonical form is exactly 8,192 bytes (within
    256 names and values) plus findings over 65,536 bytes is
    `deliverability analysis failed`, not a truncated report; the same
    findings without the policy pass.
  - The existing exact-cap test is recomputed for the new shape, still lands on
    exactly `MAX_REPORT_BYTES`, and still passes.
- **AC13 — Rules and fixtures.** No rule id is added. The every-rule
  passing/failing fixture requirement is met vacuously, and 0120 §11 says why.
  `bulk.dmarc_presence` stays `not_evaluated` even when `_dmarc` is declared.
- **AC14 — Accepted rulings and existing suites.**
  - These suites pass: `tests/burst.rs`, `tests/p2.rs`, `tests/lifecycle.rs`,
    `tests/deliverability.rs`, `tests/bulk_sender.rs`, `tests/mcp_revisions.rs`,
    `tests/api.rs`, `tests/mcp.rs` and `tests/stdio.rs`.
  - Existing tests change only in three ways: the rule-set id, the `dns` member
    (including MCP structured-output fixtures and the output schema), and
    `dns_policy: None` in `Config` literals.
  - No test is deleted or weakened. Tests that this story added in
    `e80aa822` change only where the 2026-11-04 tasks say: the worst-case
    canonical test is replaced by the boundary tests of AC6, because the
    behaviour it pinned (accepting a declaration whose report could not be
    declared back) is the defect.
- **AC15 — Gates.** Every house gate passes `--locked`, including:
  - the evals crate against the candidate binary;
  - every recipe compile, with the "lets the run go on without reading" output
    checked;
  - `scripts/coverage-gate.sh` at exact 100% lines, branches and functions, and
    its `selftest`;
  - `publish-guard-test.sh`, `seal-branch-test.sh`, and
    `cargo deny check licenses`.
- **AC16 — No dependency change.** `Cargo.toml`, `Cargo.lock`,
  `evals/Cargo.lock` and `deny.toml` are byte-identical to `main` `8732445`.
- **AC17 — Records.**
  - 0120 is `proposed`, with a row in `docs/decisions/README.md` marked
    "awaiting the operator" and stating the PR hold.
  - The contract spec's §Contract and "#253 extension seam" point to 0120 for
    the `dns` member.
  - OpenAPI, `llms.txt`, the MCP output schema and descriptions, and
    `docs/guide/deliverability.md` describe `dns`, `swarmail-html/4`, the
    canonical bound, the 256-entry count, the 512-byte label cap and the
    `temp_fail` qualify rule.
  - Pinned (added 2026-11-04): `tests/api.rs`' route list holds
    `PUT /api/v1/inboxes/{inbox}/dns` and `DELETE /api/v1/inboxes/{inbox}/dns`
    for both the guide and `llms.txt`; the served OpenAPI document has the
    `DnsRecord`, `DnsPolicy`, `DnsLookup` and `DnsEcho` schemas and both
    operations; every `$ref` in it resolves; `tests/deliverability.rs`'
    `llms.txt` phrase list holds the DNS phrases. Deleting any of them goes
    red.
  - No decision is accepted, and no number other than 0120 is used.
- **AC18 — `TempFail` is never clean (added 2026-11-04).** Through the
  `#[cfg(test)]` lookup fixture and the real cache: a report with no findings
  whose lookups include a declared `tempfail` name has `qualifies(t) ==
  false` for every severity `t`; the same message resolving only `records`
  and `no_records` names qualifies. The first assertion fails against
  `e80aa822`'s `Report::qualifies`; the second passes on both. An exact
  assert over the `temp_fail` report does not match it.
- **AC19 — Declaration resource bounds (added 2026-11-04).** The parser
  accepts exactly 256 names and values and refuses 257 with the fixed
  message, from the file and the API alike; the `PUT` handler accepts a
  512-byte label and refuses a 513-byte one with `400`, leaving the registry
  unchanged. 0120 §8 and §Resource bounds state the worst case.
- **AC20 — `DELETE` echoes the fallback (added 2026-11-04).**
  `Service::forget` does not consult the inbox declarations after removing
  the entry. A unit test shows that `forget` on a declared and on an
  undeclared inbox returns the server snapshot (`Arc::ptr_eq`) when a server
  map is installed and the default otherwise; the wire `DELETE` tests keep
  asserting `source` `server` or `default`.
