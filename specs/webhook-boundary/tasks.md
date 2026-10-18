# Ordered tasks: webhook boundary

Implementation handoff for [spec.md](spec.md), [plan.md](plan.md) and proposed
[0041](../../docs/decisions/0041-webhook-boundary.md). All boxes remain unchecked
until the paired proof exists. A completed design is not completed code.

- [ ] **1. Establish behavioral regressions and evidence (AC1–AC8).** Add
  baseline-compatible cases for each of #75/#76/#97/#98, retaining real-server
  and child-process cleanup. **Proof:** record a behavioral failure per issue
  on old behavior, with SHA, command and observed result; no compile-error red.
  Use the plan's authorized isolated checkout or labeled mutant discipline.
- [ ] **2. Unify secret validation and header construction (AC1).** Keep the
  octet bound, use the HTTP parser, reject edge SP/HTAB, and validate every
  target before replacing the list. **Proof:** real PUT control/padding/size
  refusals leave the previous whole list unchanged; inline parser-domain and
  multibyte boundaries; real delivery preserves accepted bytes; direct-set
  invalid delivery is safely exhausted; existing retries/redaction pass.
- [ ] **3. Centralize client policy and disable proxies (AC3).** Both cached
  client paths use one rustls/no-redirect/no-proxy builder. **Proof:** default
  HTTP and generated-cert extra-CA HTTPS binary cases deliver body/secret to
  the real target with zero recording-proxy connections under child-only
  upper/lowercase environment variables; redirect/TLS verification tests pass.
- [ ] **4. Redact outward URL credentials (AC4).** Share a whole-userinfo
  display policy and target echo projection; remove raw URL fallback from
  validation errors; sanitize URL-bearing delivery errors. **Proof:** GET/PUT
  and full captured default/debug lines reveal no plain/encoded credentials
  across budget, retry, transport and malformed direct-set paths; real Basic
  auth still reaches the sink; secret remains redacted and ca_pem visible.
- [ ] **5. Share serialized payloads through the request boundary (AC5).**
  Borrow Email into one serialized Vec/Bytes, clone references into tasks and
  retries, and remove Value/String copies. Add only local cfg(test) build/body
  observations as needed. **Proof:** one-versus-four targets and four retries
  each retain one backing allocation per admitted mail at actual Request
  construction, exact wire bytes match, pruned-mail retry remains green, and
  unmatched/all-refused/stopping cases build zero payloads. Reintroducing the
  request copy must fail the sharing test even if build count stays one.
- [ ] **6. Measure the framed memory behavior (AC5).** Run the matched
  one-versus-four child-process experiment with fixed 16 × 4 MiB mails and a
  second payload size, readiness barriers and nonretaining sink recorders.
  **Proof:** report baseline/ready RSS, payload lengths, slots and exact shared
  backing-byte totals; explain transport overhead and investigate any full
  payload growth per extra target before acceptance. Do not substitute a
  build counter or an arbitrary portable RSS threshold for this evidence.
- [ ] **7. Bound drop warnings without losing counts (AC6).** Store suppression
  alongside each existing normalized sink budget, retain it across successful
  admissions, and retire it with the budget. **Proof:** real stalled-sink burst
  produces N drops/one warning; supplied-instant tests pin before/exact 60s,
  duplicate/independent identities, recovery without reset and retirement.
- [ ] **8. Repair tail accounting (AC2).** Continue through all Lagged reports
  with one live/tail lag-effect helper, ending only at Empty/Closed; preserve
  existing abort/join and stop order. **Proof:** no-yield 8,200-mail test yields
  exactly lagged=8 and aborted=8192, all other outcomes/attempts zero; nonlagged,
  repeated-lag, filtering, Empty/Closed and terminal outcome cases pass;
  existing post-stop commit, bounded stop and independent-budget tests pass.
- [ ] **9. Expose the final ledger (AC7).** Emit one stable default-info event
  with all five counters after sweep and joins on ordinary dispatcher exits.
  **Proof:** real sink-read → SIGTERM → stdout EOF binary test finds exactly
  one event with aborted=1, other rows zero and clean exit; inline zero/closed/
  drained cases agree with retained post-stop snapshots. No sleep or HTTP
  shutdown reorder is used.
- [ ] **10. Align contracts and release documentation (AC8).** Update README,
  OpenAPI descriptions, source comments, proposed-decision index and dated
  release note with the chosen secret, proxy, redaction, memory, warning and
  final-log contracts. Keep 0041 proposed and accepted 0029 intact until
  operator acceptance. **Proof:** parse OpenAPI JSON, resolve document links,
  and audit each AC against the public description and its paired tests;
  remove obsolete verbatim-echo/final-scrape implications.
- [ ] **11. Prove the complete repair (AC8).** Run targeted suites, then format
  write/check, locked all-target clippy, locked full tests, bundle compile,
  coverage gate without TMPDIR override, and license check. **Proof:** all six
  exit zero with exact 100/100/100 and no coverage/policy exemptions; retain
  red-before/green-after records for each issue, all new seam installers and
  the memory observation. Environment refusals remain outstanding, never green.
- [ ] **12. Review the implementation handoff.** Reconcile any clarification
  into spec/plan/tasks and proposed 0041, preserving dated notes; follow the
  later protected review/landing workflow without closing operator-owned
  issues. **Proof:** diff review shows no frozen-guarantee weakening, unrelated
  protocol/store changes, gate changes or undeclared dependency changes, and
  all preceding acceptance evidence is present. The design seat itself only
  stages/commits the four design paths and never pushes.
