# Plan — deliverability DNS policy (#253)

Spec: [spec.md](spec.md). Decision: [0120](../../docs/decisions/0120-deliverability-dns-policy.md)
(proposed). Base `main` `8732445`; recipe `claude-careful`.

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
- **Performance.** A policy is at most 8,192 raw bytes.
  - Snapshot: one `Arc` clone under a short mutex per request.
  - Cache-key comparison: `Arc` pointer short-circuit, then exact content.
  - Report growth: at most 8,216 policy bytes plus about 70 bytes of framing.
  - No new thread, executor, permit or lock order.
- **Unknowns resolved at design.** Every one is ruled in the spec's panel
  adjudication, so none remains for implementation.

## Constitution check (house rules)

| Rule | How this plan holds it |
|---|---|
| 0001 accept == stored | `smtp.rs`, ingest, `Store::insert` and persistence are untouched. DNS is read only at report time (AC9). |
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
| `src/deliverability/dns.rs` (new) | `MAX_DNS_POLICY_BYTES`, `MAX_DECLARED_INBOXES`, `OwnerName`, `Policy` (canonical `BTreeMap<OwnerName, BTreeSet<String>>` + `BTreeSet<OwnerName>`), `Source`, `Effective { source, policy }`, `parse(&[u8]) -> Result<Policy, String>`, `load_file(&Path) -> Result<Policy, String>`, `Txt`, `Lookups`, serde views for the report and the echo, inline unit tests. |
| `src/deliverability.rs` | `mod dns;` and `RULE_SET_ID = "swarmail-html/4"`. `Inputs` gains `dns: Arc<Effective>` and `#[cfg(test)] lookups: Vec<OwnerName>`. `Report` gains `dns: DnsRecord`. `evaluate` builds `Lookups`, and `collect` resolves the fixture names under cfg(test). `finish(inputs, lookups)` fills `dns`. `Service` gains `server: OnceLock<Arc<Effective>>` and `inboxes: Mutex<HashMap<String, Arc<Effective>>>`, plus the methods `policy_for(inbox)`, `declare(inbox, Policy)` and `forget(inbox)`. `report(email, policy, ready_only)` takes the snapshot. |
| `src/store.rs` | `Store::report` takes a snapshot from `email.inbox`. `qualify` gains a `policy: Arc<Effective>` parameter. `wait_for_report` snapshots once, before `subscribe`. A `pub(crate) fn set_server_dns`. |
| `src/config.rs` | `ServeArgs.dns_policy: Option<PathBuf>` (`--dns-policy`, `SWARMAIL_DNS_POLICY`). `ServeArgs.dns_resolver: DnsResolver` (`--dns-resolver`, `SWARMAIL_DNS_RESOLVER`, default `declared`) with value parser `parse_dns_resolver`, which refuses `real`. `Config.dns_policy: Option<PathBuf>`. |
| `src/lib.rs` | `run_on` loads `cfg.dns_policy` first, maps any refusal to `io::Error::other("dns policy file <path>: <reason>")`, and installs the result with `set_server_dns` before the data file opens and before any bind. The `Config` literal at `lib.rs:356` gains `dns_policy: None`. |
| `src/api.rs` | Route `/api/v1/inboxes/{inbox}/dns` with `put(set_inbox_dns).delete(clear_inbox_dns)` and `DefaultBodyLimit::max(MAX_DNS_POLICY_BYTES)`. Handlers: `admit`, then `dns::parse` (400), then `declare` (503 when full), then echo. The assert call site passes the snapshot to `qualify`. |
| `src/mcp.rs` | The wait/assert call sites pass the snapshot. The report outputSchema (`mcp.rs:1074`) gains a required `dns` object. Description strings move to `swarmail-html/4` and mention `dns`. |
| `src/static/openapi.json`, `src/static/llms.txt` | The PUT/DELETE routes, the `DnsPolicy` / `DnsRecord` schemas, the report's `dns` member, and `swarmail-html/4`. |
| `docs/guide/deliverability.md` | A section on declaring DNS (file, API, real-mode refusal, process-local declarations). |
| `specs/deliverability-report-contract/spec.md` | §Contract and the "#253 extension seam" get a dated pointer to 0120. Historical text is kept. |
| `docs/decisions/README.md` | The 0120 row: proposed, awaiting the operator, PR held. |
| `tests/deliverability.rs` | Wire proofs AC1, AC3, AC6, AC7, AC8 (wire half), AC9. Exact-JSON edits for `/4` and `dns`. |
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
   makes duplication a refusal without a hand-written visitor.
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
| **Load.** | No load proof is planned. If the implementer adds one, it is a named, timed command with at most nproc/4 workers (32 on this 128-core host) for at most 10 minutes, and the applied load is reported. No paid trials. |

## Review return log

None yet. This is the first design. A review return revises this plan in a new
commit and records here which finding each change answers.
