# Deliverability DNS policy (#253)

Decision: 0120 — [docs/decisions/0120-deliverability-dns-policy.md](../../docs/decisions/0120-deliverability-dns-policy.md) (proposed; only the operator accepts it)

Epic #251, issue #253, priority P1. Run `issue-253-the-deliverability-dns-c704444e`,
design phase, 2026-11-03, base `main` `8732445`. Framing: [intake.md](intake.md).
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
- **Size.** The raw declaration is at most `MAX_DNS_POLICY_BYTES` = 8,192
  bytes. This applies to the file and to the API body.
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
  - input over the size bound.
- **Canonical form.**
  - `txt` is sorted by name, and each `values` list by byte order.
  - `tempfail` is sorted.
  - The compact canonical serialization is at most the accepted input's
    length plus 24 bytes, which is the spelling of the two members an input
    may omit (`{"txt":[],"tempfail":[]}`). Folding, sorting and serde's
    minimal escaping never lengthen anything else. So a report embeds at most
    8,216 policy bytes, and a unit test pins that worst case.

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
  - The inbox label is validated like every other inbox route.
  - The body is limited to 8,192 bytes; over the limit is `413`. An invalid
    body is `400`. Either way, nothing changes.
  - `200` returns `{"inbox","source":"inbox","policy"}`.
  - At most 4,096 inboxes hold a declaration. A new one beyond that is `503`
    and evicts nothing. Replacing an existing one is always allowed.
- `DELETE /api/v1/inboxes/{inbox}/dns` clears the inbox map.
  - It is idempotent.
  - `200` returns the resulting effective `{"inbox","source","policy"}`, where
    `source` is `server` or `default`.
- An inbox map replaces the server map for that inbox. There is no merge.
- Declarations are process-local and not persisted. Clearing an inbox's mail
  does not clear its declaration.

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
| Size limits | **Simplicity:** one raw byte bound (8,192), which also bounds the embedded canonical form. Robustness's per-owner, per-value and per-count constants are redundant under it. | Robustness: refusal messages say "too large", not which count was exceeded. |
| Declaration shape | **Neither as proposed:** `{"txt":[{"name","values"}],"tempfail":[...]}`. An array avoids serde's silent last-wins on duplicate object keys (robustness's object shape), and grouping per owner (rather than simplicity's flat record list) makes input, echo and report one shape, so a report's `dns.policy` can be declared back verbatim. | Simplicity: a duplicate after folding, or a duplicate value, is refused rather than collapsed. |
| Name and value grammar | **Robustness:** label grammar, exact printable-ASCII values, folding of names only. | Simplicity: one more validator branch per rule, each needing a failing fixture. |
| Lookup interface | **Simplicity:** a concrete recorder, no trait, and no `Real` variant. Adopt robustness's typed `OwnerName`, so header garbage never reaches a lookup or a report. | Robustness: there is no fail-closed `Real` arm in the evaluator, and #254 must add the arm with its resolver. |
| Inbox declaration bound | **Robustness:** 4,096 entries, `503` at capacity, no eviction. Simplicity's own fallback offered the same. | Simplicity: one constant and a wire proof that fills it. |
| Server file load | **Neither as proposed:** a `Config` path loaded in `run_on` before the data file and every bind, with robustness's file hardening. Simplicity's clap-parser load would make the policy type public through `Config`. | Simplicity: about ten `Config` literals in tests gain `dns_policy: None`, and pre-bind ordering is proven by a startup test, not by clap. |
| `GET` route | **Simplicity:** cut. `PUT`/`DELETE` echo (robustness) and the reports are the observation surface. | Robustness: a client reads the current policy only by writing it or by reading a report. |
| Split delivery | **Refused.** The record in the report is the issue's purpose. | Robustness: no part of #253 merges before the operator accepts 0120. |
| `dns_tests.rs` harness, lookup-count limit, wake on change, persistence, MCP setter, UI | **Simplicity:** all cut. There is no cold path, the report byte bound already refuses, inputs are frozen per request, and the brief did not ask for the rest. | Robustness: process-local declarations vanish on restart. Every report's `source` makes that visible. |

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
    next report.
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
  - Declaring a report's `dns.policy` back verbatim gives the same policy.
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
- **AC9 — Off the SMTP path.** With inbox `a` declaring `tempfail` for every
  name the message's headers mention, SMTP `DATA` returns `250`, the message is
  immediately listed, and its report is computed only when requested. SMTP,
  ingest, persistence and the Store mutation code gain no DNS reference.
- **AC10 — One snapshot per request.**
  - Using the existing evaluation gate seam: a `PUT` made while a report
    evaluation or an assert/wait scan is held does not change that request's
    reports, and the next request sees it.
  - The wait path takes the snapshot before its subscribe, and the 0002
    ordering tests stay green.
- **AC11 — Recorded lookups.** The `#[cfg(test)]` lookup fixture in `Inputs`
  resolves a list of names through `Lookups` inside `collect`. The report then
  lists exactly those names, once each, sorted, with outcome and values, through
  the real cache. Resolving no names gives `lookups:[]`.
- **AC12 — Report bound still holds.**
  - A declared 8,192-byte policy plus findings over 65,536 bytes is
    `deliverability analysis failed`, not a truncated report.
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
  - No test is deleted or weakened.
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
    `docs/guide/deliverability.md` describe `dns` and `swarmail-html/4`.
  - No decision is accepted, and no number other than 0120 is used.
