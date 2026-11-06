# Plan — deliverability DNS policy (#253)

Spec: [spec.md](spec.md). Decision: [0120](../../docs/decisions/0120-deliverability-dns-policy.md)
(accepted (operator, 2026-11-06)). Base `main` `8732445`; recipe `claude-careful`. Revised
2026-11-04 after the review of `e80aa822`; the
[review return log](#review-return-log) maps each finding to its change.

## Technical context

- **Language and build.** One Rust crate on the existing locked toolchain.
  New code uses only `serde`, `serde_json`, `std::collections::{BTreeMap, BTreeSet}`
  and `std::sync::{Arc, Mutex, OnceLock}`, all already in the tree.
- **Existing seams this builds on.**
  - `deliverability::Inputs` is the exact cache key.
  - `Collector::finish` is the one canonicalizer and holds the 65,536-byte
    check.
  - `Service::report` / `with_inputs` / `pin` are the cache and job
    admission.
  - `Store::qualify` / `wait_for_report` serve exact asserts and waits.
  - `Service.gate` is the evaluation seam (0020).
  - `run_on` already opens the data file before any listener binds.
  - `api::admit` validates inbox labels.
- **Performance.** A policy is at most 8,192 raw bytes and 8,192 canonical
  bytes, with at most 256 names and values.
  - Snapshot: one `Arc` clone under a short mutex per request.
  - Cache-key comparison: `Arc` pointer short-circuit, then exact content.
  - Report growth: at most 8,192 policy bytes plus about 70 bytes of framing.
  - The canonical check serializes the parsed policy once per accepted
    declaration (at most 8 KiB), only on `PUT` and at startup, never per
    report.
  - No new thread, executor, permit or lock order.
- **Unknowns resolved at design.** Every one is ruled in the spec's panel
  adjudication, so none remains for implementation.

## Constitution check (house rules)

| Rule | How this plan holds it |
|---|---|
| 0001 accept == stored | `smtp.rs`, ingest, `Store::insert` and persistence are untouched. DNS is read only at report time (AC9, whose structural proof now covers every function holding the Store's mutation lock). |
| 0002 subscribe before check | `wait_for_report` takes its policy snapshot before `subscribe` and leaves the subscribe-then-scan order unchanged (AC10). |
| 0003 oldest-only pruning | No change to the cap or eviction. Declarations are not mail and are not pruned. |
| 0005 extraction at ingest | Nothing is extracted. Analysis stays lazy over stored bytes. |
| 0085 accepted contract | §8 is fulfilled. The literal report shape is amended only through 0120, and the PR is **held for the operator** (0045 §8). |
| 0020 test placement | Inline `#[cfg(test)]` tests in `dns.rs` and `deliverability.rs`. Wire tests go in `tests/deliverability.rs`. No new harness, because there is no cold path. Seams stay `#[cfg(test)]`. |
| Coverage exact 100% | Every new line and branch has a named test in `tasks.md`. No `coverage(off)`. The `expect(dead_code)` lint is not an exclusion. |
| Dependencies | None added. AC16 diff-checks the four files. |
| Siblings #258 / #260 | DNS code lives in the new `src/deliverability/dns.rs`. Edits to `deliverability.rs`, `api.rs`, `mcp.rs`, `config.rs` and `store.rs` are small, contiguous blocks. |

## Project structure (files touched)

| File | Change |
|---|---|
| `src/deliverability/dns.rs` (new) | `MAX_DNS_POLICY_BYTES`, `MAX_DNS_POLICY_ENTRIES` (2026-11-04), `MAX_DNS_INBOX_LABEL_BYTES` (2026-11-04), `MAX_DECLARED_INBOXES`, `OwnerName`, `Policy` (canonical `BTreeMap<OwnerName, BTreeSet<String>>` + `BTreeSet<OwnerName>`), `Source`, `Effective { source, policy }`, `parse(&[u8]) -> Result<Policy, String>`, `load_file(&Path) -> Result<Policy, String>`, `Txt`, `Lookups`, serde views for the report and the echo, inline unit tests. *Revision:* `parse` counts entries as it accepts them and checks the canonical serialization's length after building `Policy`; the `load_file` comment states the metadata/open residual truthfully. |
| `src/deliverability.rs` | `mod dns;` and `RULE_SET_ID = "swarmail-html/4"`. `Inputs` gains `dns: Arc<Effective>` and `#[cfg(test)] lookups: Vec<OwnerName>`. `Report` gains `dns: DnsRecord`. `evaluate` builds `Lookups`, and `collect` resolves the fixture names under cfg(test). `finish(inputs, lookups)` fills `dns`. `Service` gains `server: OnceLock<Arc<Effective>>` and `inboxes: Mutex<HashMap<String, Arc<Effective>>>`, plus the methods `policy_for(inbox)`, `declare(inbox, Policy)` and `forget(inbox)`. `report(email, policy, ready_only)` takes the snapshot. *Revision:* `Report::qualifies` also requires that no `dns.lookups` entry is `temp_fail`; a private `fallback()` (server snapshot, else default) is shared by `policy_for` and `forget`, and `forget` no longer calls `policy_for`. |
| `src/store.rs` | `Store::report` takes a snapshot from `email.inbox`. `qualify` gains a `policy: Arc<Effective>` parameter. `wait_for_report` snapshots once, before `subscribe`. A `pub(crate) fn set_server_dns`. |
| `src/config.rs` | `ServeArgs.dns_policy: Option<PathBuf>` (`--dns-policy`, `SWARMAIL_DNS_POLICY`). `ServeArgs.dns_resolver: DnsResolver` (`--dns-resolver`, `SWARMAIL_DNS_RESOLVER`, default `declared`) with value parser `parse_dns_resolver`, which refuses `real`. `Config.dns_policy: Option<PathBuf>`. |
| `src/lib.rs` | `run_on` loads `cfg.dns_policy` first, maps any refusal to `io::Error::other("dns policy file <path>: <reason>")`, and installs the result with `set_server_dns` before the data file opens and before any bind. The `Config` literal at `lib.rs:356` gains `dns_policy: None`. |
| `src/api.rs` | Route `/api/v1/inboxes/{inbox}/dns` with `put(set_inbox_dns).delete(clear_inbox_dns)` and `DefaultBodyLimit::max(MAX_DNS_POLICY_BYTES)`. Handlers: `admit`, then `dns::parse` (400), then `declare` (503 when full), then echo. The assert call site passes the snapshot to `qualify`. *Revision:* `set_inbox_dns` refuses a label over 512 bytes with `400` after `admit` and before `dns::parse`. |
| `src/mcp.rs` | The wait/assert call sites pass the snapshot. The report outputSchema (`mcp.rs:1074`) gains a required `dns` object. Description strings move to `swarmail-html/4` and mention `dns`. |
| `src/static/openapi.json`, `src/static/llms.txt` | The PUT/DELETE routes, the `DnsPolicy` / `DnsRecord` schemas, the report's `dns` member, and `swarmail-html/4`. *Revision:* the canonical bound, the 256-entry count, the 512-byte label cap (in the PUT's `400` text), the `DELETE` echo and the `temp_fail` qualify rule. |
| `docs/guide/deliverability.md` | A section on declaring DNS (file, API, real-mode refusal, process-local declarations). *Revision:* the same additions as `llms.txt`. |
| `specs/deliverability-report-contract/spec.md` | §Contract and the "#253 extension seam" get a dated pointer to 0120. Historical text is kept. |
| `docs/decisions/README.md` | The 0120 row: proposed, awaiting the operator, PR held. |
| `tests/deliverability.rs` | Wire proofs AC1, AC3, AC6, AC7, AC8 (wire half), AC9. Exact-JSON edits for `/4` and `dns`. *Revision:* the AC6 boundary round trip, the AC3/AC19 refusals, AC9's widened structural scan, and the `llms.txt` DNS phrases. |
| `src/deliverability/concurrency_tests.rs` | *Revision:* the two AC10 tests that issue a real `PUT` through an in-crate loopback `api::serve` while `Held` holds an evaluation (assert and wait). The existing direct-declare tests stay. |
| `tests/api.rs` | *Revision:* the route list gains the two DNS routes; the served-OpenAPI test asserts the four DNS schemas, both operations and that every `$ref` resolves. |
| `tests/bulk_sender.rs`, `tests/mcp_revisions/structured.rs`, `tests/fixtures/mcp-structured-output/*`, `tests/fixtures/html-compatibility.json` | Only the rule-set id and `dns` member updates required by AC14. Fixture provenance files are updated in the way their README prescribes. |
| `tests/binary.rs`, `tests/config_cli.rs` | Binary exit proofs for AC2 and AC5. `cli_config()` gains `dns_policy: None`. |
| `Config` literals in tests and `src/compat/page_work_tests.rs` | `dns_policy: None`, and nothing else. |

No new `tests/*.rs` file, no `src/**/*_tests.rs` harness, no SMTP, POP3,
persistence or UI change.

## Research (decisions with their evidence)

1. **No resolver in the tree, so none is taken.**
   - `std` resolves only A/AAAA through `getaddrinfo`, and reqwest's resolver
     serves webhook connects only.
   - Real mode is refused, so it needs no resolver, and the `mail-auth`
     widening stays with #254 under the 2026-10-26 ruling.
   - Evidence: the intake's finding 1, and `Cargo.toml` and `Cargo.lock` at
     `8732445`.
2. **Duplicate JSON object keys.** `serde_json` keeps the last of two
   duplicate object keys when deserializing a map. An array of
   `{name, values}` entries plus an explicit check for duplicates after folding
   makes duplication a refusal. (The implementation found that a derived
   struct also accepts tuple-shaped arrays, so two small hand-written visitors
   read objects only; see the tasks' implementation record.)
3. **Identity without hashing.** 0085's `Service::pin` already compares
   `Inputs` with `==`. `Arc<Effective>`'s `PartialEq` short-circuits on pointer
   equality and then compares canonical content. No digest crate is needed,
   and none is added.
4. **Where the server file is loaded.**
   - Loading inside a clap value parser would put the parsed policy type into
     the public `Config`.
   - Loading in `run_on` from a `PathBuf`, before `Store::open` and every
     bind, keeps the types `pub(crate)` and mirrors the data file's
     established "restore before any listener binds" order.
   - The real-mode refusal stays in clap (`parse_dns_resolver`), following the
     pattern `parse_route_domain` set.
5. **The dead-code posture.**
   - `resolve_txt`, `Txt` and `Lookups::resolve_txt`'s recording path have no
     production caller until #254.
   - `#[cfg_attr(not(test), expect(dead_code, reason = …))]` keeps clippy's
     `-D warnings` honest, and it fails the build once #254 adds a caller and
     forgets to remove it.
   - The test build exercises every line through the cfg(test) lookup fixture
     (AC11), so coverage stays exact.
6. **The exact-cap test.** `deliverability.rs` builds a report of exactly
   `MAX_REPORT_BYTES` from a computed overhead. That overhead must be
   recomputed from the new shape, never hard-coded, so the test keeps landing
   on the cap.
7. **The bound algebra (2026-11-04).** The canonical form always spells both
   members, so it can be up to 24 bytes longer than an accepted input. A bound
   measured only on the input therefore lets a report embed a policy the API
   refuses. Measuring the same 8,192 on the canonical form closes it: every
   embedded form is at most 8,192 bytes, is itself canonical, and re-parses to
   the same bytes, so `PUT`ting it back passes both checks. A larger raw bound
   is not needed: the bytes a client declares back are that canonical form,
   which is at most 8,192 bytes.
8. **Resident memory (2026-11-04).** The parsed tree costs a `String` header,
   an allocation and a B-tree slot per name and value, roughly 100 bytes each,
   so a body of thousands of one-byte entries amplifies about 20 times. One
   combined count of 256 caps the tree near 4 times the wire size; a
   names-only count would leave values free to amplify.
9. **A real `PUT` inside the held harness (2026-11-04).** `tower` is not a
   direct dependency, so `Router::oneshot` is unavailable without a
   `Cargo.toml` change. `src/api.rs`'s inline tests already bind a loopback
   listener and spawn `api::serve` over a fresh `AppState`; the held tests do
   the same over the `Store` whose `Service` carries the gate, on the
   multi-thread runtime, so the server's tasks run while the test thread waits
   on `Held`.

## Data model

```text
OwnerName(String)                       canonical; Ord by bytes
Policy  { txt: BTreeMap<OwnerName, BTreeSet<String>>, tempfail: BTreeSet<OwnerName> }
Source  = Default | Server | Inbox      serde "default" | "server" | "inbox"
Effective { source: Source, policy: Policy }        Eq; carried as Arc<Effective>
Txt     = Records(Vec<String>) | NoRecords | TempFail
Lookups<'a> { effective: &'a Effective, used: BTreeMap<OwnerName, Txt> }
DnsRecord { source, policy: PolicyView, lookups: Vec<LookupView> }    report member
PolicyView { txt: Vec<{name, values: Vec<String>}>, tempfail: Vec<String> }
LookupView { name, type: "txt", outcome: "records"|"no_records"|"temp_fail", values }
Service.server:  OnceLock<Arc<Effective>>            unset == Default, empty
Service.inboxes: Mutex<HashMap<String, Arc<Effective>>>  ≤ MAX_DECLARED_INBOXES (4096)
```

- `DnsRecord`, `PolicyView` and `LookupView` derive `Deserialize` with
  `deny_unknown_fields` and no defaults on output fields. `Report` must still
  round-trip.
- The input type (`txt`/`tempfail`, both `#[serde(default)]`) is a separate
  `deny_unknown_fields` struct that `parse` validates into `Policy`.

## Contracts

The wire contracts are in the spec's §Contract:
- the declaration shape;
- the report's `dns` member;
- `PUT` and `DELETE` with their status codes (`200`, `400`, `413`, `503`);
- the CLI flags and the fixed refusal messages.

The OpenAPI document adds `DnsPolicy` (the declaration and the echo's
`policy`), `DnsEcho` (`{inbox, source, policy}`) and `DnsRecord` (the report
member), and marks `dns` required on `DeliverabilityReport`.

## Quickstart

```sh
cat > dns.json <<'JSON'
{"txt":[{"name":"s1._domainkey.example.com","values":["v=DKIM1; k=rsa; p=MIGf"]},
        {"name":"_dmarc.example.com","values":["v=DMARC1; p=none"]}],
 "tempfail":["example.org"]}
JSON
swarmail serve --dns-policy dns.json                      # server map
curl -X PUT --data-binary @dns.json localhost:8025/api/v1/inboxes/a/dns   # inbox map
curl localhost:8025/api/v1/messages/$ID/deliverability | jq .dns          # what the report used
curl -X DELETE localhost:8025/api/v1/inboxes/a/dns        # back to the server map
swarmail serve --dns-resolver real                        # refused: names #254
```

## Risks

| Risk | Mitigation |
|---|---|
| **Operator hold.** The `dns` member and the `/4` bump amend 0085's accepted literal shape. Merging on green would contradict an accepted ruling (0045 §8). | 0120's header, its README row and the PR description each say "held for the operator". No split delivery (ruled in the spec). The implementer must not mark 0120 accepted. |
| **Sibling collision.** #258 and #260 also bump `RULE_SET_ID` and edit `deliverability.rs`, `mcp.rs`, OpenAPI and the exact-JSON tests. | DNS code is in its own file, and other edits are small contiguous blocks. The second merge takes the next rule-set version as a one-line integration fix. No rebase onto `main` without the operator. |
| **Report growth breaks an existing bound test.** The exact-cap test and any report near 65,536 bytes shift by about 70 bytes of default `dns` framing. | Recompute the overhead from the serialized shape (Research 6). AC12 proves both the exact cap and the over-cap failure with a full 8 KiB policy. |
| **A large declared map plus many findings fails a report** that would have passed without the map. | Documented cost (0120 alternatives). An explicit `deliverability analysis failed`, never truncation. Bound sized at 8 KiB so ordinary DKIM/SPF/DMARC fixtures fit about 10 keys. |
| **Mixed-policy request.** A `PUT` during an assert or wait could split one request across two policies. | One snapshot at request entry, passed as a parameter to `qualify` (never re-read per candidate). Proven with the evaluation gate (AC10). |
| **0002 regression** from touching `wait_for_report`. | The snapshot is taken before `subscribe`, and only a parameter is added. The existing post-subscribe-gate and Closed-arm tests stay unmodified and green. |
| **Startup with a bad file serves false-clean reports.** | No fallback. `run_on` fails before the data file and every bind. FIFO and `/dev/zero` are handled by the regular-file check and the bounded read (AC2). |
| **Process-local declarations vanish on restart** and a test silently reverts to the server policy. | Every report carries `source`. The docs, OpenAPI text and 0120 §8 say "process-local". The server file is the durable path. |
| **Inbox declaration exhaustion.** | 4,096 cap, `503`, no eviction. A wire test fills it (bounded: 4,096 small PUTs on loopback, one test, no load proof). |
| **Dead code drifts or is hidden.** | `expect`, not `allow`. It fails the build when #254 adds a caller. Coverage still executes every line through AC11's fixture. |
| **Config literal churn** across about ten test sites. | One mechanical `dns_policy: None` per literal. AC14 limits test edits to exactly this, `/4` and `dns`. |
| **Hostile names from mail headers** (for #254). | `OwnerName::parse` is ASCII-only, label-validated and bounded, and returns `None` without panic (AC4). Invalid names never reach a lookup or a report. |
| **The real-mode flag is mistaken for a feature.** | A fixed refusal message naming #254. The API cannot select it. Docs say "refused in this build". |
| **Canonical-bound refusal surprises a client (2026-11-04).** An input that fits on the wire is refused because its canonical form does not. | A distinct message, `canonical declaration is larger than 8192 bytes`, and `400` rather than `413`, documented in OpenAPI, `llms.txt` and the guide. Only inputs within 24 bytes of the bound that omit a member are affected. |
| **The entry count or label cap rejects a legitimate test world.** | 256 names and values covers several domains with DKIM selectors, SPF, its includes, DMARC and tempfails; 512 bytes covers every RFC 5321 path. Raising either is an ordinary change. Both refusals are fixed messages with boundary tests. |
| **`temp_fail` qualify rule hangs a wait.** A wait with a threshold over a permanently temporarily-failing world never returns that mail. | Intended by 0120 §4; bounded by the wait's existing deadline outcome. Nothing in production records a lookup until #254. |
| **The AC9 scan of function bodies is shape-sensitive.** A refactor could move the mutation lock and leave the scan finding nothing. | The test asserts that it found `insert` among the lock-holding functions and that it scanned at least one function, so it goes red rather than vacuous. |
| **A loopback server inside the held harness flakes.** | Port 0 bind, no sleep: the test awaits the `PUT`'s response while `Held` keeps the evaluation parked, and `Held`'s 10 s condvar deadline panics rather than hangs. Multi-thread runtime so the server progresses while the test waits. |
| **`load_file` metadata/open race (accepted residual).** A FIFO swapped in between the two calls hangs startup. | Recorded in 0120 §9 and the code comment. The path is operator configuration; closing it needs a production dependency this story may not take. |
| **Load.** | No load proof is planned. If the implementer adds one, it is a named, timed command with at most nproc/4 workers (32 on this 128-core host) for at most 10 minutes, and the applied load is reported. No paid trials. |

## Review return log

### 2026-11-04 — review of `e80aa822` returned the design

Run `issue-253-the-deliverability-dns-c76ebc80`, rule
`REVIEW-RESIDUAL-SPEC-SECURITY-REDESIGN`: spec defect, security residual of
medium at most, no fixes applied. The spec's review return adjudication holds
the rulings; this log maps each finding to the change and the task.

| Finding | Member, severity | Change | Task |
|---|---|---|---|
| The 8,192 bound is not closed over the canonical form a report embeds; AC6's verbatim round trip fails at the boundary; 0120 §2's "one bound also caps what a report embeds" is false | guarantees, medium (spec defect) | spec §Declaration shape (size, withdrawn "+24" rule), AC2, AC6, AC12; 0120 §2 and its alternative; `parse` checks the canonical length | RT1, RT2 |
| AC9's proof covers half its claim | security (spec defect) | AC9 restated with a structural scan over SMTP, ingest, persistence and every lock-holding Store function, and the named read-side exemption | RT4 |
| AC10 says a `PUT`, the held test calls `Service::declare` | security (spec defect, secondary) | AC10 keeps its words; the proof moves to a real `PUT` over a loopback `api::serve` | RT5 |
| `TempFail` "never clean" is not enforced | security (contract gap) | `Report::qualifies` enforcement; spec §Lookup interface and §Report; 0120 §4; AC18 | RT3 |
| Resident memory bounded in wire bytes only; inbox key unbounded | security, low residual; independent, low | entry count 256, label cap 512, stated worst case; spec §Resource bounds; 0120 §2 and §8; AC19 | RT2, RT6 |
| `DELETE` echo can say `source:"inbox"` under a racing `PUT` | correctness, low | `forget` returns `fallback()`; spec §Configuration and API; 0120 §8; AC20 | RT7 |
| `load_file` metadata/open race | correctness and independent, info | no code change; truthful comment and 0120 §9 residual | RT8 |
| Routes, schemas, `llms.txt` and guide unpinned | tests, info | AC17's pins and a `$ref` resolvability check | RT9 |
| `Service::pin` order; public struct fields; sibling `RULE_SET_ID` | guarantees/independent, info | none (spec adjudication says why) | — |

**Criteria changed, and why.** No criterion was loosened to make the review
pass. AC6, AC9, AC10, AC12 and AC17 are strengthened or given a proof that can
fail. AC9's phrase "the Store mutation code" was not provable as written,
because `src/store.rs` legitimately holds the policy on its read path; it is
now defined (functions holding `self.mutation.lock()`) and the exemption
named, rather than narrowed. AC14 allows exactly one change to a test this
story added: `canonical_view_worst_case` asserted that an exactly 8,192-byte
input with an 8,205-byte canonical form is accepted, which is the defect the
guarantees member proved, so RT1 replaces it with the boundary tests. No
accepted decision conflicted with a criterion. AC18, AC19 and AC20 are new.
