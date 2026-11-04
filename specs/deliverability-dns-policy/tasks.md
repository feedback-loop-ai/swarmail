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
  ~~Inline `canonical_view_worst_case` builds an accepted input of exactly 8,192
  bytes that omits a member, and asserts that its canonical form is at most
  8,216 bytes.~~ *Superseded 2026-11-04 by RT1:* that input is now refused.
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
  unmodified. *Extended 2026-11-04 by RT5:* the change is also made through a
  real `PUT`.

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
  (recorded in the verify notes). *Widened 2026-11-04 by RT4.*
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

## Implementation record — 2026-11-03

The implement seat built T1–T25 as specified. Deviations and findings, each
small and each kept inside the design:

- **`Lookups` owns an `Arc<Effective>`** rather than borrowing `&Effective`
  (the spec sketch had `Lookups<'a>`). `Collector::finish(inputs, lookups)` and
  the existing tests that call it directly need a value with no lifetime, and
  `Lookups::default()` is the empty world. The interface is unchanged:
  `resolve_txt(&mut self, &OwnerName) -> Txt`.
- **`Service::report(email, ready_only)` keeps its signature** and snapshots
  the policy from `email.inbox`; `report_under(email, policy, ready_only)` is
  the entry that `qualify` uses with a request's frozen snapshot. This keeps
  the sibling stories' (#258, #260) test edits to a minimum.
- **The declaration parser reads objects by hand.** A derived serde struct
  accepts a JSON array as a tuple, so `[]` parsed as an empty declaration and
  `{"txt":[["a",["x"]]]}` as an entry. The two small visitors accept objects
  only and refuse a repeated member; `parse_refusals` pins each.
- **The mid-request and wait-snapshot proofs live in `concurrency_tests.rs`**,
  beside the `Held` gate they use (0020 rule 2); the other unit tests are inline.
- **`collect` takes a `&mut Lookups`** which production does not read yet; the
  parameter carries `expect(unused_variables)` for #254, like the other items.
- **Schemas carry no `maxLength`**: the structured-output suite's validator
  does not know the keyword, so the 253-byte name bound is stated in prose and
  enforced by the parser. `openapi-report-source.json` is re-pinned for the
  new `DeliverabilityReport`; `march-current-main.jsonl` line 8 and its
  provenance digest are hand-edited as #259 did, and recorded in the README.
- **Gates run here:** fmt, clippy, `cargo test --workspace`, the locked
  `swarmail` build, the evals fmt/clippy/test against that binary, the exact
  coverage gate (100/100/100) and its selftest, publish-guard, seal-branch and
  `cargo deny check licenses`. **Not run here:** `brokkr compile` over the
  recipe shelf, because the pinned brokkr binary is not provisioned in this
  sandbox; nothing under `.brokkr/` changed. No load proof was run.
- **Flake seen and removed:** an earlier draft built the FIFO with a spawned
  `mkfifo`; a fork inside the library test binary can hold a copy of another
  test's lock descriptor and fail `persist.rs`'s `try_lock` assertions. The
  FIFO is now made with `libc::mkfifo`.

## Review return — 2026-11-04

The review of `e80aa822` returned the design (run
`issue-253-the-deliverability-dns-c76ebc80`). Each task below answers a
finding named in the plan's review return log. Build them on top of the
branch; do not rewrite the earlier commits. Each test must be seen red
against `e80aa822` where the task says so, then green.

- [ ] **RT1. The canonical bound (AC2, AC6, AC12; guarantees finding).**
  In `dns::parse`, after building `Policy`, refuse when
  `serde_json::to_vec(&policy.view())` is longer than `MAX_DNS_POLICY_BYTES`,
  with `canonical declaration is larger than 8192 bytes`. Keep the raw
  pre-parse check and its message. Fix the doc comment on
  `MAX_DNS_POLICY_BYTES` (it still claims the "+24" rule).
  *Tests:*
  - inline `canonical_bound_closes_the_round_trip`, replacing
    `canonical_view_worst_case`: the exactly-8,192-byte raw input omitting
    `tempfail` is **refused** with the canonical message (red against
    `e80aa822`); an input whose canonical form is exactly 8,192 bytes is
    accepted; its canonical bytes parse to an equal `Policy` that serializes
    to identical bytes; one byte more of value is refused;
  - `parse_refusals` gains the canonical-over case;
  - `tests/deliverability.rs` `dns_policy_canonical_over_http` gains the
    boundary case: `PUT` the exactly-8,192-byte canonical form, read a
    report, `PUT` its `dns.policy` back byte for byte, and assert a
    byte-identical echo and next report; a body that fits 8,192 raw bytes
    but not canonically is `400` with the canonical message, and the
    previous declaration is unchanged;
  - `declared_policy_counts_toward_report_cap` uses a declaration whose
    canonical form is exactly 8,192 bytes and at most 256 entries, and still
    proves `AnalysisError::Failed` with the findings and success without the
    policy; the exact-cap test still lands on `MAX_REPORT_BYTES`.
- [ ] **RT2. The entry count (AC19; security residual).**
  `MAX_DNS_POLICY_ENTRIES` = 256 in `dns.rs`. `parse` counts every `txt`
  owner, every value and every `tempfail` name as it accepts it, and refuses
  the 257th with `declaration has more than 256 names and values`.
  *Tests:* `parse_refusals` gains 257 entries (refused) and
  `parse_accepts_and_canonicalizes` gains exactly 256 (accepted), both inside
  8,192 bytes (for example one-letter `tempfail` names, and a mix of owners and
  values so both counters are exercised); `load_file_refusals` gains a
  257-entry file; the wire test gains a 257-entry `PUT` → `400`, with the
  previous declaration byte-identical in the next report.
- [ ] **RT3. `TempFail` is never clean (AC18; security contract gap).**
  `Report::qualifies` returns `false` when any `dns.lookups` entry has
  outcome `temp_fail`, before or regardless of the findings check.
  *Test:* extend `fixture_lookups_are_recorded`, or a sibling inline test
  in `src/deliverability.rs`, through `Service::with_inputs` and the real
  cache: a message with no findings whose fixture lookups include the
  declared `tempfail` name gives `qualifies(t) == false` for every
  `Severity` `t` (red against `e80aa822`); the same message resolving only
  a `records` name and a `no_records` name qualifies at every threshold. One
  `Store::qualify` call over the `temp_fail` report returns no match.
- [ ] **RT4. AC9's structural proof (AC9; security spec defect).**
  Rework `tempfail_policy_does_not_touch_delivery`'s scan in
  `tests/deliverability.rs`:
  - whole files via `include_str!`: `src/smtp.rs`, `src/persist.rs`,
    `src/extract.rs` and every file under `src/extract/` (list them
    explicitly; assert the list matches `std::fs::read_dir` of `src/extract`
    so a new file cannot escape it);
  - `src/store.rs`: split into function bodies at each line that begins a
    `fn` item at the `impl` indentation, keep those whose text contains
    `self.mutation.lock()`, assert that `pub fn insert(` is among them and
    that at least one was found, and assert none mentions `dns`,
    `Effective` or `policy_for` (case-sensitive for the type names, and
    `dns` case-insensitive);
  - a comment naming the read-side exemption (`report`, `qualify`,
    `wait_for_report`).
  The behavioural half (`250` and immediate listing under a
  tempfail-everything declaration) stays as it is.
  *Proof that it can fail:* record in the verify notes that adding
  `let _ = crate::deliverability::dns::MAX_DNS_POLICY_BYTES;` inside
  `Store::insert` turns the test red (then revert).
- [ ] **RT5. AC10 through a real `PUT` (AC10; security spec defect).**
  In `src/deliverability/concurrency_tests.rs`, add
  `put_mid_assert_is_not_seen` and `put_mid_wait_is_not_seen`, each on
  `#[tokio::test(flavor = "multi_thread")]`: bind `127.0.0.1:0`, build an
  `AppState` over the test's `Arc<Store>` (as `src/api.rs`'s inline router
  tests do), and spawn `api::serve` with a pending shutdown.
  - Assert: hold the first evaluation of a two-candidate `Store::qualify`
    with `Held`, send `PUT /api/v1/inboxes/box/dns` over a raw HTTP/1.1
    connection, assert `200` and the canonical echo, release, and assert both
    candidates' cached reports carry the old policy while a fresh
    `store.report` carries the new one.
  - Wait: start `wait_for_report` with a threshold, hold its first
    evaluation (after the subscribe) with `Held`, `PUT` as above, release,
    and assert the returned mail's report carries the old policy and a fresh
    request the new one.
  Keep `declaration_change_mid_request_is_not_seen` and
  `report_wait_snapshots_policy_before_subscribe_and_keeps_it` unchanged. No
  sleep: the test awaits the `PUT` response while `Held` keeps the
  evaluation parked.
- [ ] **RT6. The label cap (AC19; security residual).**
  `MAX_DNS_INBOX_LABEL_BYTES` = 512. `set_inbox_dns` refuses a longer label
  with `400` `inbox label is longer than 512 bytes` after `admit` and before
  `dns::parse` or `declare`.
  *Test:* `tests/deliverability.rs` `inbox_dns_declarations_over_http` gains
  a 513-byte label → `400` with that message and no declaration made (a
  following `DELETE` of that label echoes the fallback and a report for a
  512-byte-label inbox is unaffected), and a 512-byte label → `200`.
- [ ] **RT7. `DELETE` echoes the fallback (AC20; correctness finding).**
  Extract `fn fallback(&self) -> Arc<Effective>` (server snapshot, else the
  default) used by `policy_for`; `forget` removes the entry under the lock
  and returns `self.fallback()` without calling `policy_for`.
  *Test:* inline `forget_echoes_the_fallback` in `src/deliverability.rs`: with
  no server map, `forget` on a declared and an undeclared inbox returns
  `Source::Default`; on a `Service` with `set_server`, both return an `Arc`
  that is `Arc::ptr_eq` to the server snapshot. The wire `DELETE` tests keep
  asserting `server` or `default`.
- [ ] **RT8. The `load_file` residual, written down (correctness and
  independent info).** No code change in behaviour. Replace the comment
  "A FIFO is refused before it is opened, so it cannot block startup" with
  the truth: the metadata check refuses a FIFO, device or directory at the
  path; a FIFO swapped in between the check and the open would block
  startup; the path is operator configuration; closing it needs a production
  dependency (0120 §9).
  *Test:* `load_file_refusals` still passes; review reads the comment
  against 0120 §9.
- [ ] **RT9. Documentation pins (AC17; tests finding).**
  - `tests/api.rs`' route list gains `PUT /api/v1/inboxes/{inbox}/dns` and
    `DELETE /api/v1/inboxes/{inbox}/dns`, checked in the guide and in
    `llms.txt` (add them to `docs/guide/rest-api.md` if absent).
  - The served-OpenAPI test asserts that `components.schemas` holds
    `DnsRecord`, `DnsPolicy`, `DnsLookup` and `DnsEcho`, that
    `/api/v1/inboxes/{inbox}/dns` has `put` and `delete`, and that every
    `$ref` string in the document resolves to an existing node.
  - `tests/deliverability.rs`' `llms.txt` phrase list gains `temp_fail`,
    `"source"`, `canonical declaration is larger than 8192 bytes`,
    `256 names and values` and `512 bytes`.
  - Update `src/static/openapi.json`, `src/static/llms.txt` and
    `docs/guide/deliverability.md` for RT1, RT2, RT3, RT6 and RT7.
  *Test:* each new assertion is seen red by deleting its target once
  (recorded in the verify notes), then green.
- [ ] **RT10. Records and gates.** 0120 stays `- **Status:** proposed`
  and its README row keeps "awaiting the operator" and the PR hold.
  `git diff --exit-code 8732445 -- Cargo.toml Cargo.lock evals/Cargo.lock deny.toml`
  is clean. Run every house gate `--locked` as T25 lists, including the
  evals crate, the exact coverage gate and its selftest, publish-guard,
  seal-branch and `cargo deny check licenses`; every new production line
  (the two counters, the canonical check, the `qualifies` clause,
  `fallback()`, the label check) is executed by RT1–RT7's tests. No load
  proof is planned; any added one is bounded to nproc/4 workers for at most
  10 minutes, as a named, timed command, with the load reported.
  *Test:* the gate exit codes in the verify notes.

## Implementation record — 2026-11-04 (review return)

The implement seat built RT1–RT10 on top of `ef3c21c`, in the existing branch,
without rewriting earlier commits.

- **RT1/RT2.** `dns::parse` counts entries as it accepts them
  (`count_entry`) and refuses a canonical form over 8,192 bytes after building
  the policy. `canonical_view_worst_case` is replaced by
  `canonical_bound_closes_the_round_trip`: the 8,192-byte raw input that omits
  `tempfail` (accepted by `e80aa822`) is now refused with the canonical
  message; a canonical form of exactly 8,192 bytes is accepted and
  re-serializes to identical bytes. The canonical serialization uses `expect`,
  not `map_err`: a view of strings cannot fail to serialize, and an
  unreachable error closure would leave the exact coverage gate one function
  short.
- **RT3.** `Report::qualifies` is false at every threshold when a lookup is
  `temp_fail`. To drive it through `Store::qualify` without a production
  caller, `Service` gained a `#[cfg(test)]` `lookups: OnceLock<Vec<OwnerName>>`
  seam beside `auth` (0020 rule 4); `temp_fail_is_never_clean` installs it.
- **RT4.** `the_accept_path_has_no_dns_reference` scans whole files for SMTP,
  persistence, `src/extract.rs` and `src/extract/*` (the list is checked
  against `read_dir`), and every `src/store.rs` function that takes
  `self.mutation.lock()`, asserting `pub fn insert(` is among them. *Seen red:*
  adding `let _ = crate::deliverability::dns::MAX_DNS_POLICY_BYTES;` inside
  `Store::insert` failed the test with `pub fn insert(...) mentions DNS`; it
  was reverted and the test is green.
- **RT5.** `put_mid_assert_is_not_seen` and `put_mid_wait_is_not_seen` serve
  the real router with `api::serve` over the test's `Store` on a multi-thread
  runtime and issue the `PUT` with `reqwest` (already a dependency, used by the
  harness's own `client()`), not a hand-written HTTP/1.1 socket as the task
  sketched. It is the same real HTTP request to the real `set_inbox_dns`
  handler, answered while `Held` keeps the evaluation parked; no sleep.
- **RT6.** `set_inbox_dns` refuses a label over 512 bytes after `admit` and
  before the query and body. The wire test seeds mail into the 513-byte label
  and reads `source:"default"` from its report, which proves no declaration was
  made, rather than relying on the `DELETE` echo (which is the fallback either
  way).
- **RT7.** `Service::fallback()` is shared by `policy_for` and `forget`;
  `forget_echoes_the_fallback` asserts `Arc::ptr_eq`.
- **RT8.** The `load_file` comment states the metadata/open window truthfully.
- **RT9.** Route rows in `docs/guide/rest-api.md`; `llms.txt`, the guide, the
  OpenAPI texts and the MCP report description (`MODERN_REPORT_DESCRIPTION`,
  which no fixture pins) describe the canonical bound, the 256 count, the label
  cap, the `DELETE` echo and the `temp_fail` qualify rule. The MCP tool
  descriptions that the March fixture pins (`no_findings_at_or_above`) are not
  edited, to keep that fixture's hand edit to one line. *Seen red,* each by
  removing its target once and then restoring it: deleting the `DnsRecord`
  schema (`components.schemas lacks DnsRecord`); renaming one `$ref` target
  (`dangling $ref`); removing every `512 bytes` from `llms.txt` (phrase list);
  removing the `DELETE` row from `rest-api.md` (`omits DELETE ...`).
- **RT10 — gates run here, all `--locked`:** `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets -D warnings`;
  `cargo test --workspace` (1,309 passed, 0 failed);
  `cargo build --bin swarmail`; the evals fmt, clippy and `cargo test` against
  that binary (exit 0); `scripts/coverage-gate.sh` (exact 100/100/100:
  24,349 lines, 1,818 branches, 2,406 functions) and its `selftest`;
  `publish-guard-test.sh`; `seal-branch-test.sh`; `cargo deny check licenses`.
  `git diff --exit-code 8732445 -- Cargo.toml Cargo.lock evals/Cargo.lock
  deny.toml` is clean. **Not run here:** `brokkr compile` over the recipe
  shelf, because the pinned brokkr binary is not provisioned in this sandbox
  (`which brokkr` is empty, nothing found on disk); nothing under `.brokkr/`,
  `.github/` or `scripts/` changed. CI-only proofs are not claimed.
- **Load.** No load proof was run. `cargo test` ran with
  `RUST_TEST_THREADS=32` (nproc/4 of 128); the coverage gate and the evals
  suite ran as the house scripts run them.
