# Tasks — deliverability DNS policy (#253)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Decision:
[0120](../../docs/decisions/0120-deliverability-dns-policy.md) (proposed; never
accept it).

Work in order. Each task names the test that proves it. A task is done only when
that test runs, has been seen to fail without the change where that is possible,
and passes with it. Commit unsigned, in the repository's style. Never push.

## Phase 1 — the policy module (`src/deliverability/dns.rs`)

- [ ] **T1. Owner names.** `OwnerName::parse`: ASCII only, fold to lowercase,
  strip one trailing dot, labels of 1–63 bytes `[a-z0-9_-]`, total 1–253 bytes,
  `None` otherwise.
  *Test:* inline `owner_name_grammar`, a table covering:
  - `S1._DomainKey.Example.COM.` → `s1._domainkey.example.com`;
  - 253 bytes accepted and 254 refused;
  - a 63-byte label accepted and a 64-byte label refused;
  - an empty label (`a..b`), a leading dot, `.`, and the empty string;
  - NUL, `\r\n`, a space, `é`, `*`, and two trailing dots.
- [ ] **T2. Declaration parser.** `parse(&[u8]) -> Result<Policy, String>`:
  - size bound `MAX_DNS_POLICY_BYTES` = 8,192;
  - UTF-8 and JSON;
  - a `deny_unknown_fields` input struct with optional `txt` and `tempfail`;
  - printable-ASCII values, never trimmed;
  - the refusals: duplicate names after folding, duplicate values, empty
    `values`, duplicate `tempfail`, and a `txt`/`tempfail` overlap;
  - canonical `BTreeMap`/`BTreeSet` output.

  *Test:* inline `parse_accepts_and_canonicalizes` covers `{}`, absent members
  and a mixed-case trailing-dot name. Inline `parse_refusals` has one case per
  refusal (each asserting a fixed message):
  - malformed JSON, an empty input, an unknown member;
  - invalid UTF-8, 8,193 bytes;
  - a bad name, a value with `\t` or `\x7f` or a non-ASCII byte, `values:[]`;
  - a folded duplicate, a duplicate value, a duplicate `tempfail`, an overlap.
- [ ] **T3. Canonical views and the size bound.** `PolicyView` serialization:
  `txt` sorted by name, values sorted, `tempfail` sorted, both members always
  present.
  *Test:* inline `canonical_view_is_order_independent`: three permutations
  serialize byte-identically, and the view parses back to an equal `Policy`.
  Inline `canonical_view_worst_case` builds an accepted input of exactly 8,192
  bytes that omits a member, and asserts that its canonical form is at most
  8,216 bytes (spec §Declaration shape).
- [ ] **T4. Lookup recorder.** `Txt`, `Lookups::new(&Effective)`, and
  `resolve_txt(&mut self, &OwnerName) -> Txt`, which records each answered name
  once. Mark the items production does not reach with
  `#[cfg_attr(not(test), expect(dead_code, reason = "#254/#255 verifiers call it"))]`.
  *Test:* inline `lookup_outcomes`: over one map holding a DKIM selector name,
  an SPF name and a DMARC name, plus a `tempfail` name, assert:
  - `Records` with canonical values;
  - `TempFail`;
  - `NoRecords` for an undeclared name;
  - a repeated lookup recorded once;
  - `LookupView` JSON for each outcome.

  `cargo clippy --all-targets --locked -- -D warnings` stays clean, and the
  `expect` must be fulfilled.
- [ ] **T5. Server file loader.** `load_file(&Path)`:
  - `metadata` must say regular file;
  - `File::open` then `take(MAX_DNS_POLICY_BYTES + 1)`;
  - then `parse`;
  - errors carry the reason.

  *Test:* inline `load_file_refusals`, unix: a directory, a FIFO made with
  `mkfifo` (it must return, not hang), a missing file, an 8,193-byte file, and
  `/dev/zero` (bounded and refused); plus `{}` accepted.

## Phase 2 — report, inputs, cache, snapshot (`src/deliverability.rs`, `src/store.rs`)

- [ ] **T6. Report member and version.**
  - `RULE_SET_ID = "swarmail-html/4"`.
  - `Report` gains `dns: DnsRecord`, with `deny_unknown_fields` and no default.
  - `Inputs` gains `dns: Arc<Effective>` (default: `Default` and empty) and
    `#[cfg(test)] lookups: Vec<OwnerName>`.
  - `evaluate` builds `Lookups`. `collect` resolves `inputs.lookups` under
    cfg(test). `finish` fills `dns` before the byte check.

  *Test:* update `report_contract_variants_and_order`'s exact empty-report JSON
  to `{"rule_set_id":"swarmail-html/4","findings":[],"dns":{"source":"default","policy":{"txt":[],"tempfail":[]},"lookups":[]}}`.
  Round-trip through `serde_json::from_value::<Report>`. A report JSON missing
  `dns`, or carrying an unknown `dns` member, fails to deserialize.
- [ ] **T7. Recorded lookups through the real cache (AC11).**
  *Test:* inline `fixture_lookups_are_recorded`, which runs `Service::with_inputs`
  with `inputs.lookups` set to `[b, a, a, missing, tf]` over a declared map.
  The report's `lookups` is exactly `[a, b, missing, tf]` with outcomes
  records/records/no_records/temp_fail and their values. Empty fixture lookups
  give `lookups:[]`.
- [ ] **T8. Exact cap recomputed (AC12).**
  *Test:* rework the existing exact-cap test so its overhead comes from the
  serialized new shape, and assert exactly `MAX_REPORT_BYTES`. Add inline
  `declared_policy_counts_toward_report_cap`: an 8 KiB policy plus fixture
  findings that overflow gives `AnalysisError::Failed`, and the same findings
  without the policy pass.
- [ ] **T9. Per-inbox declarations in `Service`.**
  - `server: OnceLock<Arc<Effective>>`.
  - `inboxes: Mutex<HashMap<String, Arc<Effective>>>`, capped at
    `MAX_DECLARED_INBOXES` = 4,096.
  - `policy_for(inbox)` returns the inbox entry, else the server map, else
    `Default`.
  - `declare` returns `Err(Full)` for a new inbox at capacity and always allows
    a replacement.
  - `forget` is idempotent and returns the fallback.
  - The mutex is never held across an await.

  *Test:* inline `declarations_precedence_and_cap`:
  - inbox over server over default, with no merge;
  - an empty inbox declaration gives `source:inbox`;
  - 4,096 entries, then a new inbox is `Full` while a replacement succeeds;
  - `forget` twice.
- [ ] **T10. Cache identity (AC8).**
  *Test:* inline `policy_is_part_of_the_cache_key`, using the existing `gate`
  seam as an evaluation counter:
  - the same `Arc` and equal content each give 1 evaluation;
  - changed content (an unread name) gives a second evaluation and the new
    `dns.policy`;
  - the same content under `Server` versus `Inbox` gives distinct reports.
- [ ] **T11. One snapshot per request (AC10).**
  - `Store::report` snapshots from `email.inbox`.
  - `qualify(candidates, policy, threshold, stop_after, ready_only)` takes the
    snapshot.
  - `wait_for_report` snapshots once before `subscribe`.
  - `api.rs` assert and `mcp.rs` wait/assert pass it.

  *Test:* inline `declaration_change_mid_request_is_not_seen`:
  - hold an evaluation on the `gate`;
  - `declare` a new map for the inbox;
  - release the evaluation and assert that the held request's report (and the
    second candidate's in a two-message `qualify`) carries the old policy;
  - assert that a fresh request carries the new one.

  The existing 0002 subscribe-gate and Closed-arm tests in `store.rs` pass
  unmodified.

## Phase 3 — configuration (`src/config.rs`, `src/lib.rs`)

- [ ] **T12. Real-mode flag refused (AC5).** Add `DnsResolver { Declared }` and
  `parse_dns_resolver`. `real` is refused with
  `real DNS needs the resolver that #254 adds; this build has none`, and any
  other value with clap's invalid-value error. Add `--dns-resolver` /
  `SWARMAIL_DNS_RESOLVER`, defaulting to `declared`.
  *Test:* `tests/config_cli.rs`:
  - `try_parse_from` with `real` fails and the error text contains `#254`;
  - with `declared`, and with the flag absent, it parses.

  `tests/binary.rs`: `swarmail serve --dns-resolver real` with
  `SWARMAIL_DNS_RESOLVER` unset exits non-zero with the message, and a second
  run sets only the env var.
- [ ] **T13. Server file before bind (AC2).**
  - `ServeArgs.dns_policy` (`--dns-policy`, `SWARMAIL_DNS_POLICY`) and
    `Config.dns_policy: Option<PathBuf>`.
  - `run_on` calls `dns::load_file` first and maps refusals to
    `io::Error::other(format!("dns policy file {}: {reason}", path.display()))`.
  - Then `set_server_dns`, then the data file, then the listeners.
  - Add `dns_policy: None` to every `Config` literal.

  *Test:* inline in `lib.rs` (or `lifecycle_tests.rs` if the startup seam lives
  there) `bad_dns_policy_refuses_before_bind`:
  - `run_on` with a malformed file and a `data_file` path returns `Err`
    containing the path and the reason;
  - the data file was not created;
  - the configured SMTP and HTTP ports bind afterwards.

  `tests/binary.rs` `serve_refuses_bad_dns_policy`: non-zero exit and the
  message. `tests/config_cli.rs`: the flag and the env var populate
  `Config.dns_policy`.

## Phase 4 — API and adapters (`src/api.rs`, `src/mcp.rs`)

- [ ] **T14. `PUT`/`DELETE /api/v1/inboxes/{inbox}/dns` (AC3).**
  - `admit(&inbox)`.
  - `DefaultBodyLimit::max(MAX_DNS_POLICY_BYTES)` (over the limit is 413).
  - `dns::parse`, where a refusal is a 400 carrying the parser's message.
  - `declare`, where `Full` is a 503 `dns declarations full`.
  - Echo `{inbox, source, policy}`. `DELETE` echoes the fallback.

  *Test:* `tests/deliverability.rs` `inbox_dns_declarations_over_http`:
  - a PUT, then reports for `a` (`inbox`) and `b` (`default`);
  - an invalid body (400) and an oversized one (413) each leave `a`'s next
    report byte-identical;
  - a bad label is 400;
  - DELETE twice gives the fallback both times;
  - a server map plus an inbox map shows replacement, not merge;
  - `DELETE /api/v1/inboxes/a` keeps the declaration.

  Inline api test `dns_declarations_full_is_503`: prefill 4,096 entries through
  `declare`, then a new inbox PUT over the router is 503, and a replacement is
  200.
- [ ] **T15. MCP schema and descriptions.** The report outputSchema gains a
  required `dns` object (`source` enum; `policy.txt[]` and `policy.tempfail[]`;
  `lookups[]` with an `outcome` enum). Descriptions say `swarmail-html/4` and
  "records the DNS policy and lookups".
  *Test:* the existing `tests/mcp_revisions/structured.rs` and the stdio report
  witness, updated to the new shape. The structured fixture provenance files
  are updated as their README prescribes, and the schema still validates a real
  report.

## Phase 5 — wire proofs (`tests/deliverability.rs`)

- [ ] **T16. Default exact JSON (AC1).**
  *Test:* update every exact report body in `tests/deliverability.rs` and
  `tests/bulk_sender.rs` to `/4` plus the default `dns` object. Add
  `default_dns_policy_exact_json` for a message carrying `DKIM-Signature`.
- [ ] **T17. Order independence and reproduction (AC6).**
  *Test:* `dns_policy_canonical_over_http`:
  - two permuted declarations give byte-identical echoes and report bodies;
  - PUTting a report's `dns.policy` back gives the same echo.
- [ ] **T18. Two processes (AC7).**
  *Test:* `dns_report_identical_across_processes`: write the same declaration
  to two temp directories and spawn two `serve` processes with different cwd and
  `--dns-policy` paths on free ports. Deliver one identical raw message to each
  over SMTP, then assert byte-identical report JSON with `source:"server"`, and
  that neither path appears.
- [ ] **T19. Cache identity over the wire (AC8, wire half).**
  *Test:* `dns_change_reaches_next_report`: get a report, PUT a changed map,
  get the report again, and assert the second carries the new policy.
- [ ] **T20. Off the SMTP path (AC9).**
  *Test:* `tempfail_policy_does_not_touch_delivery`: PUT a map that marks the
  message's domains `tempfail`, then SMTP DATA returns `250` and an immediate
  list shows the message. The report is computed on request and shows the
  `inbox` source. Plus `grep -n -i dns src/smtp.rs src/persist.rs` stays empty, as it is on `8732445`
  (recorded in the verify notes).
- [ ] **T21. No rule (AC13).**
  *Test:* `declared_dmarc_does_not_satisfy_bulk_rules`: declare
  `_dmarc.<from domain>`, and `bulk.dmarc_presence.not_evaluated.v1` is still
  present.

## Phase 6 — records and documentation

- [ ] **T22. Documentation surfaces.** OpenAPI (`DnsPolicy`, `DnsEcho`,
  `DnsRecord`, the routes, the required `dns`), `llms.txt`, and
  `docs/guide/deliverability.md` (file, API, process-local declarations,
  real-mode refusal, `TempFail` semantics).
  *Test:* the existing OpenAPI and `llms.txt` consistency tests (`tests/api.rs`,
  `tests/agent_setup_docs.rs`, `src/api.rs` inline doc tests) pass. Add
  assertions that the new routes and the `dns` schema exist.
- [ ] **T23. Decision records.**
  - Add the 0120 row to `docs/decisions/README.md`: proposed, awaiting the
    operator, report-shape amendment of 0085, PR held.
  - Add a dated pointer to 0120 in `specs/deliverability-report-contract/spec.md`
    §Contract and "#253 extension seam".
  - Change no other decision's status.

  *Test:* `git diff 8732445 -- docs/decisions/` touches only `0120-*.md` and
  `README.md`. `grep -c '0120' docs/decisions/README.md` is ≥ 1. The 0120
  status line still reads `- **Status:** proposed`.

## Phase 7 — gates (AC14–AC16)

- [ ] **T24. Dependencies unchanged.**
  *Test:* `git diff --exit-code 8732445 -- Cargo.toml Cargo.lock evals/Cargo.lock deny.toml`.
- [ ] **T25. Every house gate, locked.** Record each exit code in the verify
  notes:
  - `cargo fmt --all -- --check`;
  - clippy `--workspace --all-targets --locked -D warnings`;
  - `cargo test --workspace --locked`;
  - `cargo build --locked --bin swarmail`;
  - the evals fmt, clippy and test against the candidate binary;
  - every recipe compile, with the "lets the run go on without reading" output
    checked;
  - `scripts/coverage-gate.sh` (exact 100%) and its `selftest`;
  - `publish-guard-test.sh`, `seal-branch-test.sh`, and
    `cargo deny check licenses`.

  CI-only proofs are not claimed. No load proof is planned. Any added load
  proof is bounded to nproc/4 workers for at most 10 minutes, as a named, timed
  command, with the load reported.
