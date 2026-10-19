# Ordered implementation tasks — release safety and truthful records

These tasks are unexecuted. Each includes the proof that permits checking it.
Requirement identifiers refer to [spec.md](spec.md); implementation details and
file ownership are in [plan.md](plan.md). Preserve the issue/test mapping in
signed implementation commit bodies. Do not mark a blocked proof passed.

- [ ] **T01 — Establish authorized base and evidence (#141/#142/#144).**
  Fetch current main, verify 2a325c9 ancestry, use `epic120-release-docs`,
  record the initial tree/version triple and operator #144 ruling. Re-read
  current code for D01–D24 and amendment claims. **Proof:** Git ancestry and
  clean-base evidence, unique 0051 allocation/reservation, itemized source
  disposition; no copied line numbers treated as current observations.
- [ ] **T02 — Add chart image preflight to existing pinned guard (#141,
  AC1).** Preserve absence/exact-ref modes; add bare require-present mode
  requiring explicit successful Docker result. **Test:** existing
  `publish-guard-test.sh` extended against real local HTTP: success+200
  passes; failure/skipped/cancelled/unset dependency, 404, 500, token failure,
  refused connection, bad mode/ref/version refuse. Existing 200-republish,
  both-tag checks and absence green twins continue passing.
- [ ] **T03 — Wire release dependencies and preflight (#141, AC1).**
  Docker needs chart verification/smoke; chart needs explicit Docker success
  and all current prerequisites; feed actual result through environment;
  guard precedes push; no read-only proof job gains package write.
  **Test:** extend real guard proof and existing workflow/delivery pins;
  old independent-chart wiring fails, corrected wiring passes. Actual
  preflight fixture for current v0.1.1/Cargo 0.1.0 refuses with no publish
  admission. Main's rate-skipped allowance stays confined to Docker.
- [ ] **T04 — Derive chart image contract (#141, AC2).** Validate
  appVersion=Cargo, rendered image and Artifact Hub annotation; remove
  hard-coded 0.1.0 expectation. **Test:** real Helm renders for existing
  profiles, coherent changed version passes, mismatched Cargo/appVersion or
  annotation fails, old hard-coded verifier fails the changed-version twin.
- [ ] **T05 — Add event-history bump check and selftest to chart verifier
  (#141, AC2).** Use full event baseline and fail closed on missing history;
  wire selftest and history check to chart CI. **Test:** real temporary Git
  repositories cover unbumped/decreased/malformed refusal, changed/increased
  and unchanged passing twins, multi-commit PR, push/non-ancestor before,
  zero-before new branch, prior-release tag, shallow missing base and numeric
  prerelease ordering. Restored baseline no-check behavior fails controls.
- [ ] **T06 — Repair examples and context (#141, AC3; D10/D12).** Both
  ephemeral quickstarts use --rm/30-second stop grace; document named-volume
  cleanup; main Helm flags include Always; remove stale main workaround;
  use recursive target exclusion; correct Dockerfile SIGTERM comment.
  **Test:** README option pins plus actual Helm render and T07's real
  anonymous-volume/context proof fail with the old commands/ignore file.
- [ ] **T07 — Implement and run real Docker smoke (#141, AC3).** Add
  reusable script and read-only CI job, unique resources/ports, non-root
  image user, bounded protocol readiness and cleanup. **Test:** DATA 250
  marker survives graceful restart and separately hard-kill restart on named
  storage; named volume survives removal, anonymous volumes from both
  quickstart forms do not; real context excludes root/tools/evals targets
  while keeping a non-target control. Failure path cleans owned resources.
- [ ] **T08 — Reconcile gate/process/license docs (#142, AC4;
  D01–D05).** Link house-rules inventory, fix compile/link/seat/panel text,
  clarify signing history, remove only proven-unused allowances and fix
  CDLA wording. **Test:** named-file link/path audit, existing constitution
  pins, current license tree and cargo-deny; active model routes and pinned
  verify dispatch remain byte-equivalent except intended descriptions.
- [ ] **T09 — Correct install/site/release claims (#142, AC4;
  D06–D11, D17–D20).** Use dated errata for historical entity/latest/subject/
  secret claims and #118; distinguish benchmarks from CI, remove unsupported
  crates.io claim/volatile counts, repair chart job/status/table and suite
  inventory/gen-cert wording. Prepare local operator handoff for closed-issue
  corrections. **Test:** D-row audit against workflow, chart values/renders,
  actual root test files and existing binary/compat/extraction/webhook
  regressions. Restore each named stale claim in a targeted pin where
  machine-checkable; prose-only historical corrections receive dated source
  citations. No external comment or publication is sent.
- [ ] **T10 — Document runtime limits and help (#142, AC4;
  D12–D16/D21).** Correct help to Host+Origin, pending-accept recovery comments,
  REST body/envelope limits, SMTP command/second-AUTH behavior and zero TLS
  timeout. **Test:** help-output pin plus existing/new real API oversized
  request, repeat-AUTH/non-UTF-8, TLS zero and listener-recovery proofs;
  T07 supplies Docker evidence. No mail-server behavior is changed.
- [ ] **T11 — Repair OpenAPI parameter/subject contract (#142, AC5;
  D22/D24).** Correct await defaults/minimums/cap and minimum-one waiting
  semantics; assert limit stays inert; subject uses oldest subject-bearing
  member with actual optional shape. **Test:** extend API pins with real
  omitted-limit, count=120→100 rows, limit=0→empty page, count=0,
  cap/over-cap and inert-assert inputs, plus distinguishing thread fixtures;
  old schema mutations fail and corrected twins pass.
- [ ] **T12 — Repair OpenAPI statuses (#142, AC5; D23).** Add applicable
  403, health 503, messages 400, clear 400/503, views 400 and body 413/415;
  keep already-correct batch 3 entries. **Test:** each affected method's
  observed status is present in its response map, with real requests and
  existing degradation/commit seams at deterministic poles. Schema removal
  mutations fail. Check every operation for guard 403; follow 0020 placement
  based on cold paths, not seam visibility.
- [ ] **T13 — Make decision inventory/status checks complete (#144, AC6).**
  Extend `tests/store_contract_docs.rs`, enumerating actual files and rows
  both ways, unique numbers/rows, valid links and matching statuses. Replace
  real-tree proposed search with independent fixtures. **Test:** missing
  0035/0055 row, dangling row, duplicate row/number, malformed filename and
  each one-sided status transition refuse; complete/coherent twins pass,
  including an all-accepted fixture and a mixed-status fixture.
- [ ] **T14 — Record operator acceptance and current references (#144,
  AC6/AC8).** Set 0034–0050 and 0055 current status to accepted (operator,
  2026-10-19), add missing rows, index 0051 as proposed, update current
  status references and directory description. Preserve historical text and
  0047/0050 dated amendments. **Test:** T13 real-tree audit plus current
  store/chart/listener/process status pins; compare accepted bodies before
  and after, allowing only authorized metadata and appended addenda.
- [ ] **T15 — Reconcile every amendment claim (#144, AC7).** Audit all
  records using plan's starting pairs; append dated scope-correct reciprocal
  pointers or corrections, especially 0045→0015/0018 and 0030→0018/0023.
  Clarify 0027/0031 acceptance provenance and 0035 numbering by dated
  addendum. Add proposed 0051 pointers to 0019/0045. **Test:** finite audited
  pair checks verify both ends and resolving links; remove each required
  pointer in fixtures and prove failure, repaired twin passes; original
  missing-0045-pointers mutation fails. Review semantic scope manually.
- [ ] **T16 — Publish proposal rule and preparation state (#144/#141,
  AC2/AC8).** State compatible-only proposal implementation for all
  0001–0005, accepted-ruling precedence and frozen protection; document
  nontransactional image-first publication and deliberate rerun recovery.
  If chart files changed, increase only chart preparation patch version and
  note it unpublished, leaving Cargo unchanged. **Test:** focused rule/status
  checks and accepted-text review, T04/T05 actual chart proofs, v0.1.1
  refusal fixture and unchanged runtime/version assertions.
- [ ] **T17 — Prove regression discrimination and final tree (#141/#142/#144,
  AC9).** Record per-issue reviewed-behavior failure and repaired pass; finish
  D01–D24 source/proof dispositions and all required gates in plan's order,
  formatting before each check. **Test:** fmt, clippy, workspace tests, exact
  coverage plus selftest, licenses, publish-guard proof, native-tls absence,
  pinned verify compile and CI-only chart/Docker/release proofs. Missing
  tools or execution refusal remain explicitly outstanding.
- [ ] **T18 — Deliver signed implementation evidence.** Commit only intended
  implementation changes with imperative subjects citing #141/#142/#144;
  bodies name T02/T03 guard tests for #141, T11/T12 OpenAPI tests for #142
  and T13/T15 inventory/marker tests for #144, using their final actual names.
  **Proof:** configured GPG signature verifies, branch is correct and clean,
  no run-local evidence is tracked, no push/PR/merge/issue action occurred.
