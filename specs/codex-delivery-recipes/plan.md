# Plan — Codex delivery recipes

## Technical context

Implement [spec.md](spec.md) under accepted 0057 in the existing single Rust
crate. Prepared work includes four composed bundles, JSON offices/adapters/
policy, Markdown charters/docs, shell seats, Rust regressions, a stdlib-only
Python patch helper and CI. Finish demonstrated non-pinned test/document gaps
and outstanding proofs. No production dependency, package, mail schema, public
API, feature flag or runtime change belongs in this bootstrap.

Reconciled for `finish-and-deliver-the-operator--bbd50333` on 2026-10-23.
No plan template exists under `.specify/`; these three artifacts supply the
sections required by `dialects/speckit/design.md` without new scaffold.

Use the recorded composite, never version equality or an archived binary:

| Identity | Required value |
|---|---|
| Source base | `597f4868e3fb971b62e6a368a2d8aa23271f4531` |
| Ordered patch 0001 | `tooling-patches/0001-preserve-review-residual-facts.patch`; SHA-256 `5d4da2217135c5a2ca14354ae9907623cf67ff0076b078277cb77a09ad3020b5` |
| Ordered patch 0002 | `tooling-patches/0002-preserve-review-scope-and-fixes.patch`; SHA-256 `361272eff344a5d5e05b832281407a99c3f1a77fc1fbc5216e19fb4ce7487173` |
| Ordered patch 0003 | `tooling-patches/0003-bind-verified-candidate.patch`; SHA-256 `4098228e7649d1bda8961bb491aa29d982b462c420470e265f8b9acf0a148d96` |
| Composite diff SHA-256 | `49611f8d62c495467acdc4cb3f77c1e17bc5148339aaa3d48830e25214f3ac6b` |
| Executable SHA-256 | `e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c` |
| Build / activation | `.forge/epic192/engine-repair/untracked-config-release-receipt.json` / `.forge/epic192/active-engine.json` |

Patch paths are relative to `bundles/self/`. Compare actual bytes, tooling,
activation and build receipt before use. The receipt's staged status precedes
activation of the same binary; preserve that chronology and frozen prose.
The recorded six-file composite and unchanged manifests/lock are source
proof, never a clean-base claim. Normal PATH brokkr and `.forge/bin/brokkr`
must agree with the active digest. Prepared Rust 1.98/stable, nightly
2026-10-22 with LLVM, cargo-llvm-cov 0.9.1, cargo-deny and locked caches are
prerequisites, not passed gates. Put workspace `.forge/bin` and the provisioned
Cargo tool directory on PATH without committing operator-local home paths.
Offline verify installs nothing.

## Constitution check

This engine bootstrap takes full design and four-provider review. 0057 already
authorizes the routes and bounded escalation; this reconciliation adds no
semantic ruling or ADR. Preserve 0001–0005, protected independent review,
worst-verdict aggregation, security holds, explicit-safe residual admission
and transitive proof ownership under 0007/0050. Preserve 0017's delivered-result
and immutable-pin boundaries and 0056's unsigned/no-push discipline. Existing
dated markers preserve historical bodies.

All pinned bundle/office/adapter/charter/policy/script/tooling bytes remain
identical, including prose. A necessary defect requires concrete path and
execution evidence, a broken-run report and an assistant-prepared new pin/run.
Never repair those bytes, select another engine, weaken a gate or invent a
second evaluator during this run. Runtime gate failures remain outstanding;
they grant no scope for epic repairs. Later 0051/0047 rulings stay deferred.

## Project structure and touched files

| Paths | Ownership and permitted work |
|---|---|
| `specs/codex-delivery-recipes/{spec,plan,tasks}.md` | Chief's entire three-file commit; implementation evidence edits precede final verification |
| `bundles/{self,codex,codex-careful,codex-fast}/`, `bundles/README.md` | Prepared inheritance, policy, design, exec seats and guidance; commit pinned bytes unchanged |
| `bundles/self/tooling.json`, `bundles/self/tooling-patches/000{1,2,3}-*.patch` | Frozen composite and upstream regressions; no source/pin mutation |
| `agents/codex-*.json`, `agents/charters/codex-{triage,fast-implementer,bug-reviewer}.md` | Prepared offices, hands, triage and scope contracts; frozen |
| `agents/review-correctness.json`, `agents/charters/review-*.md`, `adapters/dsh.json` | Prepared effort amendment, scope reporting and provider routes; frozen |
| `recipes`, `realms.json` | Relative `recipes -> bundles` alias and self-contained realm/canonical journal |
| `tests/delivery_recipes.rs`, `tests/delivery_constitution.rs` | Existing regressions; extend only demonstrated non-pinned gaps |
| `scripts/apply-brokkr-patches.py`, `tests/tooling_patches.rs` | Prepared stdlib helper and nine real-Git tests; no application dependency |
| `.github/workflows/ci.yml` | Exact patches, core/aggregation/candidate-binding tests, build and four compiles |
| `README.md`, `agents/README.md`, `docs/house-rules.md`, `docs/decisions/README.md` | Authorized guidance/provenance; finish non-pinned gaps |
| `docs/decisions/0057-codex-delivery-recipes.md`, dated markers in `0007`, `0009`, `0013` | Commit authorized acceptance/markers; no invented acceptance or historical-body rewrite |
| `.forge/results/`, `.forge/tasks/`, `.brokkr/state/{tasks,design}/`, `.forge/ledger/` | Ignored run-local evidence; positions never staged |
| `.forge/epic192/`, canonical `.forge/forge.db` | Receipts, immutable history, reservations and shared journal; no fabricated findings |

Current Git status is the authoritative implementation inventory, including
fast, safety hardening and normal-TUI guidance. Review an explicit list before
implementation staging; do not discard authorized files. Chief stages only
three specs. No `src/`, Cargo inputs, allowlist or coverage exclusions change.
Existing owners remain; no scheduler, registry, sidecar or new receipt schema.

## Research and evidence — 2026-10-23

Chief read both bbd50333 positions, intake, house rules, decision index and
0007/0009/0013/0017/0050/0056/0057. It inspected policies, patch boundaries,
ship/design scripts, test/CI inventory, tooling and receipts. Actual binary
and three patch hashes matched; all 16 policy evidence hashes matched. These
are inspections/checksums, not fresh fixture, provider or house-gate execution.
Build claims below are recorded receipts, not chief's independent rebuild.
Keep dated spec clarifications and historical artifact versions in Git.

| Evidence under `.forge/epic192/` | Observed proof and limit |
|---|---|
| `engine-repair/untracked-config-release-receipt.json` | Final digest, exact composite, unchanged manifests/lock, recorded full core/runtime/fmt/clippy and release build; later activation binds it. Source tests are not product gates. |
| `member-matrix-final/receipt.json` | 124 final-binary actual CLI rows, zero failures: six 1/2/4 clean/residual controls complete, 99 stop, 19 await operator. No providers or verify phase, hence no candidate-binding proof. Fixture true-fixes stops do not redefine production reverification. |
| `assistant-candidate-binding/cli-controls-final/receipt.json` | Twelve final-binary verify/review CLI cases: six unchanged complete; six committed reviewer edits await operator with zero successful reviews despite clean status. No providers or product delivery. |
| `policy-probe/receipt.json`, `evidence.sha256` | 102,110 evaluations, zero failures; 24 rules each for self/codex/careful and 39 fast; current source/binary/resolved identities. All 16 checksum entries matched. Resolver/core proof, not live selection or hermetic dispatch. Older counts/digest complaints are historical. |
| `pin-repairs/ship-probe-receipt.json` | Four cases on older `744ea1f3...` binary; cannot close final-engine ship dispatch. Rust real-shell controls prove only their shell boundary. |
| `provider-smoke/` | Availability only; no protected review consumption or completed delivery. |
| `history/`, adoption/re-export evidence | Supplied count: 67 recovered plus three stopped bootstraps, 70 before this run; chief did not recount. Source/remote stay immutable; import is not resume or discharge. |

Patch 0002 replaces false defaults with optional scope/fixes joins, preserving
true, and rejects explicit clean security/above-none contradictions. Patch
0003 enforces that check at protected singleton consumption and stores
`verified_candidate` in engine-owned journal inputs. Git inspection requests
`--untracked-files=all`. The old source blocker is not a current observation.
Proof coverage still requires an audit against exact current inputs.

## Implementation approach

1. Audit authorized inventory and immutable pin. Match source, lock, binary,
   patches, compositions and receipt inputs. Classify reuse by executed
   boundary. Preserve original receipts, stopped runs and baseline reds/greens;
   do not repeat unchanged proofs merely for newer timestamps.
2. Map member/core/candidate cases to AC3–AC5/AC7. Reuse matching 124-case,
   twelve-case and core receipts within limits. Supplement actual 1/2/4 office
   selection/consumption, missing/partial/stale/misassociated outputs, first/
   repeated scope growth, all-strategy returns and third-visit refusal.
   Restart retains run/pin/attempt/visits and candidate witness, absorbs
   findings and avoids downgrade. Use disposable contexts and the recorded
   core harness; no local policy reimplementation.
3. Prove remaining final-engine ship dispatch before expensive gates: unchanged
   positive control; missing/empty/mismatched ledger; dirty tree/index;
   HEAD/tree drift; hidden untracked files under Git configuration. Use
   verify-bearing fixtures for candidate binding. Retain consumed facts,
   rules and exits, attributing refusal to its engine or shell owner. Plain
   shell status cannot prove hidden-untracked refusal; test the composed
   boundary. Never mutate live pinned resources for negative controls.
4. Run locked recipe, constitution and helper regressions; extend only missing
   non-pinned tests/docs. Preserve twelve recipe tests and nine real-Git helper
   tests, including real-shell close-out. The helper checks root/HEAD and
   tracked cleanliness, validates all buffered hashes before mutation, then
   applies in order. It allows untracked output and promises no rollback after
   later apply failure. Separately attest clean build inputs, relevant
   untracked files and exact composite/lock. Partial source cannot certify a
   build; no transaction framework is needed.
5. Compile/discover all four with the active binary; compare named/explicit
   composition and standalone realm resolution. Retain logs/exits, zero
   unruled valuations and no sweep warnings. Prove complete inherited design/
   verify/ship origin. CI uses the same patches and runs core, aggregation and
   candidate-binding regressions before build.
6. Finish non-pinned guidance on classification, provider/tool limits,
   canonical TUI/journal, historical adoption, reuse and future isolation.
   Label unexercised live variants honestly; another full paid careful delivery
   is not required solely for its effort change.
7. Run every final-candidate local house gate and supplementary design/tamper/
   native-tls proofs. Reconcile implementation-owned evidence before committing
   all authorized changes unsigned with rationale. Preserve exact 100/100/100,
   locked inputs and allowlist. Later tracked edits require renewed proof.
8. Actual inherited offline verify executes on the committed candidate, checks
   shims and runs transitive pinned helpers. Only advancing verification
   establishes the journal witness. Manual gates do not substitute.
9. Actual protected review consumes all four requested singleton provider
   routes: high Opus/Flash, omitted DashScope effort, no fallback/Pro. Check
   worst verdict and candidate identity. Missing provider/fact proof remains
   outstanding; no author certifies its own work.
10. Actual inherited ship renders the active run's genuine ledger and closes
    on stable verified/reviewed HEAD/tree and clean index/worktree. Ready is
    preliminary. Later receipts stay ignored; never dirty the verified tree to
    tick future tasks. Deliver the local branch for operator sealing only.

## Data model and contracts

No mail migration or protocol change. Composition is the configuration model;
strategy selects a lane, triage owns framing, journal owns visits/attempts,
and policy owns transitions. Fast supports chore/feature/design/engine with
1/2/4 review and no unknown default. Non-triage full recipes refuse scope
defects. Holds and above-medium debt dominate scope returns.

Results belong to run, phase, attempt and member. Missing/partial/stale/
misassociated outputs cannot advance. Typed facts stay inside `inputs`.
Chief writes exactly `result`, `inputs`, `notes` with `result: drafted` through
workspace hands after its three-file commit. Only the deterministic check
rules design; chief does not select the next phase.

Scope/fixes remain unknown unless true is witnessed; false requires known
false members. Residual safety never defaults safe. Every review-to-ship allow
requires false scope/fixes. Supported clean reverification requires false
scope/true fixes, cannot override candidate binding and grants no reviewer
write authority. Explicit clean security/above-none residuals refuse, including
singleton. Explicit non-security, valid at-most-medium tracked debt remains.

`verified_candidate` is engine-owned HEAD/tree from advancing verification,
persisted across restart. Review/ship reject missing/spoofed witnesses,
dirty/untracked/unreadable Git and committed edits. Keep gate HEAD-span checks;
clean status is insufficient. Receipts name candidate, engine/pin, run/attempt,
command/provider, exit/verdict and result/log paths without credentials.

## Quickstart and validation

Stay on the prepared branch. Verify executable digest against frozen tooling,
activation and build before `.forge/bin/brokkr` or normal PATH use. Targeted
implementation commands:

```bash
cargo test --locked --test delivery_recipes
cargo test --locked --test delivery_constitution
cargo test --locked --test tooling_patches
.forge/bin/brokkr compile --bundle bundles/self
.forge/bin/brokkr compile --bundle bundles/codex
.forge/bin/brokkr compile --bundle bundles/codex-careful
.forge/bin/brokkr compile --bundle bundles/codex-fast
.forge/bin/brokkr recipes show codex
.forge/bin/brokkr recipes show codex-careful
.forge/bin/brokkr recipes show codex-fast
```

Run the entire [house inventory](../../docs/house-rules.md#gates--part-of-every-change)
plus design selftest, mandatory tamper-boundary and fail-closed native-tls
proofs. Do not duplicate/reduce that inventory. Verify compiles self; retain
separate actual compiles for the other three. CI owns the listed chart/history/
selftest, real Docker, advisory, tag-rate and real port-80 checks; claim none
locally without actual receipts.

Only after bootstrap actually completes may reserved A–H trials start. Use
prepared setup/run-lane helpers under `.forge/epic192/`. Eventual launch from
an isolated checkout is
`brokkr run --recipe codex-fast --db "$SWARMAIL_JOURNAL" --feature '<scoped story>'`,
where `SWARMAIL_JOURNAL` names the absolute canonical `.forge/forge.db` path.
Normal `brokkr tui` sees that journal. Never relocate/alias open SQLite/WAL
files or create invisible active copies. Eight lanes share journal coordination
but isolate Git stores/config/branches, results/evidence/build/coverage/cache,
ports/inboxes/data and reserved 16-CPU groups. Reserve overlaps, throttle gates
and rerun integration proofs. Their committed recipe baseline stays immutable;
later gate repairs use separate writable candidate scripts and execute repaired
proofs. Eight is a subsequent target/ceiling, not a bootstrap success claim.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Cheap classification misses sensitive scope | Evidence burden, independent scope review, bounded return/no downgrade; unknown stops |
| Restart resets budgets or admits stale output | Exact-engine restart/association proof; preserve journal identity/counters |
| Joins erase unknowns or clean contradictions | Existing join/consumer owners and matched 1/2/4 controls; no safe defaults |
| Reviewer commits leave clean status | Journal HEAD/tree witness plus gate HEAD-span refusal and positive controls |
| Git hides untracked inputs | Explicit engine enumeration and composed-boundary negative control |
| Inheritance relocates proof scripts | Complete inherited seats; verify origin and actual exec consumption |
| Structural/core receipts are overclaimed | Separate source, evaluator, CLI, hermetic and live-provider evidence |
| Ship uses old proof | Final-engine dispatch controls and genuine active-run close-out remain mandatory |
| Provider/effort differs | Actual protected selection/consumption; no DashScope effort, fallback or Pro; outages stop |
| Dsh lacks native tool restrictions | Honest conduct limitation and measured identity; drift refuses |
| Helper partly applies or overlooks untracked source | Refuse partial source; separately attest source/lock and relevant untracked inputs |
| Pinned defect remains | Concrete broken evidence, new pin/run; accept delay rather than bypass |
| Evidence edits change verified candidate | Finish tracked work before verify; later receipts ignored |
| History import becomes false discharge | Preserve snapshot/remote and stopped attempts; no inferred acceptance/supersession |
| Eight runs collide/exhaust resources | Isolate writable state, shared canonical journal, reservations and throttling; no early launch |
| Gates expose epic runtime defects | Retain failures and exact floor; repair only in later scoped lanes |


## Implementation evidence boundary

The run's [task evidence](tasks.md#implementation-evidence--run-bbd50333)
records the audited matching receipts and fresh final-engine dispatch controls.
Only implementation-owned checks are marked there. The implementation commit
keeps actual committed-candidate hermetic verify, protected four-provider
review and genuine ship pending. Their later ignored receipts must prove the
outcome without editing the candidate. Compile/selection fixtures and provider
smoke do not grant a delivery verdict.
