# Ordered implementation tasks

Authority: [spec.md](spec.md), [plan.md](plan.md), proposed
[0044](../../docs/decisions/0044-compat-validation.md). Boxes are intentionally
unchecked: the design seat does not claim implementation. Execute in order;
every task includes the proof needed to close it. Use a feature branch,
real servers, per-test inboxes, deterministic seams and 0020 placement.

- [ ] **T01 — Pin current contracts and behavioral reds for all five children.**
  Add regression fixtures in the existing owning suites before production
  repair where possible. **Test:** #91 typo-clear/non-object MCP, #95
  cross-inbox latest, #96 unbounded snippet or offload gate, #99 divergent
  envelope, #100 localized subject each fail a named behavioral assertion on
  the reviewed behavior. Record commands; a build failure does not count.
- [ ] **T02 — Persist sender provenance with additive migration (#99).**
  Update Email, shared builder, SCHEMA/ADDED_COLUMNS/COLUMNS/bindings/decoder
  and deliberate construction fixtures. **Test (AC10):** old populated
  schema migrates and reopens twice; None/empty/nonempty states, raw, IDs,
  counters and ordering survive new inserts/restart. Existing full restore
  query-plan, pragma 5 s busy, sidecar and commit-failure suites stay green.
- [ ] **T03 — Route compat sender consumers through one accessor (#99).**
  MailHog From/Raw.From/kind=from and Mailpit ReturnPath/Return-Path use known
  envelope with legacy-only fallback; native from stays header-based.
  **Test (AC10–AC11):** real SMTP divergent header/reverse path and bounce,
  both seed transports, native sender filter, compat sender filter and
  envelope-only recipient matches before and after persistent restart.
- [ ] **T04 — Repair shared MCP classification and bounded byte parsing (#91).**
  Handle Bytes rejection, invalid parsed members and bridge response-ID
  classification together. **Test (AC1–AC2):** real HTTP malformed bytes,
  exact/over 2 MiB including chunked input, scalar/null/nested/mixed batches;
  real stdio successful and failed exchanges with duplicate/null IDs;
  notification effects/silence and empty/oversized pre-execution refusal.
- [ ] **T05 — Poll admitted batch members concurrently (#91).**
  Replace serial dispatch with ordered join_all and retain each wait's
  shutdown/deadline handling. **Test (AC2):** batch wait+seed succeeds,
  readiness proves all maximum-duration waits concurrently registered before
  delivery, existing individual deadline/cap tests pass, shutdown releases
  them. Existing
  cumulative-registry/_uncounted tests pass without invented admission caps.
- [ ] **T06 — Close argument/query shapes and null gaps (#91).**
  Add descriptor-driven unknown-key refusal for every tool, native closed
  query structs, strict optional-string serde helper and schema changes.
  **Test (AC3):** every tool's sentinel typo refuses; default-inbox clear and
  assert are unchanged by typos, waits register nothing; missing/null/type/
  string parity tables cover all optional seed fields and chaos error;
  per-event null still clears and valid advertised tool fields still stage.
- [ ] **T07 — Validate inbox admission consistently (#91).**
  One shared blank-after-trim validator serves HTTP/MCP/UI/compat and
  SMTP/POP3 admission while retaining nonblank spelling. **Test (AC4):**
  empty, spaces and Unicode whitespace refuse on all representable wires;
  missing URL segment retains 404; nonblank padded names round-trip; refused
  auth/seed/wait/clear cannot change stored data, session auth or waiters.
- [ ] **T08 — Page MCP search and both inbox listings (#91).**
  Stage offset, preserve total, add API/UI limit/offset and navigation.
  **Test (AC4):** 251+ messages/inboxes traverse disjoint complete sorted
  pages, default/0/250/251 sizes and huge/beyond-end offsets obey contracts,
  malformed offset refuses, UI links keep positive limits and never wrap.
- [ ] **T09 — Add one-pass scoped latest and shared route resolver (#95).**
  Use max(received_ms,id) across all scoped entries, with no sorted vector
  or mutation lock; refuse nonempty latest query. **Test (AC5):** two-inbox
  message/raw/headers isolation and empty scope; inverted timestamp vs
  insertion ordering and tie fixtures; byte-exact raw, corresponding headers,
  global lookup, empty query, named query refusal and ordinary-ID regression.
- [ ] **T10 — Bound snippets and short-circuit generic matching (#96).**
  Implement the 8192-byte safe prefix, 200-character output and source-cut
  suffix rule; retain string regex/lossy semantics with raw last. **Test
  (AC6/AC9):** inspected-byte seam proves bounded long whitespace/text/HTML;
  200/201, entities, unfinished tags, Unicode cuts and absent-body cases;
  early match skips raw while raw-only/U+FFFD and regex-size refusals stay.
- [ ] **T11 — Serialize borrowed MailHog pages with actual-byte cap (#96).**
  Keep both MIME fields, item-local lossy raw Cow and one capped buffer;
  publish no bytes until success. **Test (AC8):** independent shape fixtures
  for each MIME/body case and invalid raw; small injected exact/over budgets
  count escaping and duplicated MIME; real over-64-MiB response is named
  HTTP 400, smaller pages/raw remain usable, no body-sized Value tree exists.
- [ ] **T12 — Offload the complete four compat page paths (#96).**
  Move scan/filter/page/render/serialization to awaited Store::offload and
  return final bytes with no held map guard. **Test (AC7–AC8):** instance gate
  on a current-thread real server leaves SMTP 220 and health live while
  concurrent list/search builds are parked; independent watchdog releases
  on mutant failure. Record same-fixture before/after memory/greeting bench
  measurements and verify deterministic tests rather than timing thresholds.
- [ ] **T13 — Close prefix and unread-field drift (#99).**
  Apply token-prefix refusal inside common containing staging, with ://
  exemption and explicit kind values still literal; rename unread key.
  **Test (AC9/AC11):** known/unknown/mixed-case/later-token/leading-space
  prefixes name the field; URL and explicit-kind searches succeed; kind and
  page staging regressions pass; messages_unread exists and old key does not.
- [ ] **T14 — Extend subject fallback safely (#100).**
  Use finite localized markers, character indices, both colons, Unicode
  whitespace collapse and original header precedence. **Test (AC12):** full
  table of listed markers, Cyrillic uppercase, repeated/empty/numbered
  nonmarkers, NBSP/tabs/spaces and multibyte edges; exact header-based keys
  and chain separation remain, orphan variants group through real HTTP.
- [ ] **T15 — Publish the contracts alongside their implementation.**
  Update README/OpenAPI/MCP schemas/llms, comments and decision index with
  proposed 0044; retain accepted decisions unchanged. **Test (AC11):** JSON
  schema/doc pins cover nullable provenance, recipient predicate, strict
  null/unknown keys, concurrent effect ordering, latest query refusal,
  snippet/page limits, raw escape hatch, wipe/unread behavior, thread residuals
  and metrics-only registry cap; every declared route/example matches a wire
  test. Verify relative links and proposed status.
- [ ] **T16 — Close proof and all required gates before implementation success.**
  Restore mutants, record each child red/green and run plan quickstart gates
  with cargo PATH, write-format before check, locked workspace clippy/tests,
  bundle compile, coverage with no TMPDIR override and license check.
  **Test (AC13):** all six commands exit zero, coverage has exact nonzero
  100/100/100, no dependency/floor/exclusion changes, and diff preserves every
  do-not-undo contract. No completion claim while a gate or proof remains.
