# Implementation plan — release proof residuals

## Technical context

One Rust crate, existing Bash proof scripts, Python loopback/Git fixtures,
installed curl, Cargo and the pinned brokkr. No new dependency, library, service
or runtime code. The plan template is absent from this checkout (only
`.specify/templates/spec-template.md` exists); this plan supplies the requested
technical context, constitution check, structure, research, data model,
contracts, quickstart and risks directly.

## Constitution check

The spec preserves 0001–0005, accepted 0019/0020/0045/0050 and proposed 0051's
existing compatible behavior. No new semantic ruling is needed. Keep exact
100% measured lines/branches/functions and all required gates. Public publish
shims and `shims.sha256` stay byte-identical. A mutable candidate proof and
an immutable dispatched proof have separate identities and receipts. Commit
unsigned under 0056, never push. The recorded engine route under 0057 retains
four independent review lenses; scope/security changes return for escalation.
Root owns integration and CI, and 18.5 is external pending throughout local work.

## Project structure and files

| Path | Work |
|---|---|
| `bundles/self/scripts/publish-guard-test.sh` | Replace three generic presence needles; add precise per-case request/stage assertions as needed; retain the observer, one server and holder. |
| `scripts/chart-verify.sh` | Prove three existing cases; edit only demonstrated fixture/assertion defects, retaining metadata/history/render owners. |
| `Cargo.toml` | Make every intended include explicitly root-anchored and keep accurate comments; preserve all other package settings. |
| `bundles/self/scripts/publish-guard.sh` | Conditional only: a demonstrated defect within the existing contract, with a paired regression. No production change is currently justified. |
| `specs/release-proof-residuals/{spec,plan,tasks}.md` | Design artifacts; later record truthful local disposition and outstanding external receipt in the owning documentation. |
| Run-private ignored evidence under `.forge/` | Disposable copies, generated synthetic package fixtures, logs, mutation diffs and receipts. No new journal; never rewrite inherited evidence. |

If repeatability requires a committed package fixture helper, keep it directly
adjacent to the owning package documentation in this spec directory and narrow
to AC5. Prefer generated disposable fixtures and an explicit runnable receipt
over a committed fake cache or a general packaging framework. No CI wiring or
other owner changes are authorized. The design commit itself contains only the
three required design files.

## Research and decisions already settled

Inspection confirms that the chart cases, holder and manifest-close/000 controls
already exist. Fresh execution must establish their discrimination. Presence
unknown-state assertions are the known weak point. The two positions differ
only materially on blanket root anchoring: use explicit anchors, but require
Cargo membership evidence before calling a particular old pattern defective.
A slash-containing old pattern may already behave identically. No further
architectural research is needed; installed tool compatibility and actual
packaging behavior are execution questions, not guessed requirements.

## Data model and contracts

No application schema or public API changes. Retain the existing fixture's
control file (token/manifest poles), per-case request log and port readiness
file. The same child owns both HTTP server and refusal holder; teardown tracks
and joins that exact PID. The 000 observer remains a status-only classifier
control following a successful real request.

A proof receipt records AC/case, candidate identity, exact script/tool hashes,
command/environment with secrets omitted, exit, named diagnostic, mutation
identity and restored-byte identity. This is an evidence convention, not a new
engine result schema. Record status as pass, fail or outstanding honestly.
Package evidence contains actual Cargo lists, asserted required members, decoy
paths, membership deltas and archive byte comparisons. 18.5 never receives a
local pass. No current evidence is inferred from the older epic spec/handoff.

## Ordered implementation approach

1. Capture starting commit/tree and changed-file status, tool versions and
   script hashes. Check exact brokkr base source, ordered patch digests and
   executable SHA-256 against `bundles/self/tooling.json`; version alone does
   not authorize compile. If absent/mismatched, record the missing proof for
   root rather than installing, rewriting pins or bypassing verification.
   Keep all fixture state private; use only synthetic registry credentials.
2. Run the real chart selftest positively. In a disposable candidate copy
   preserve its relative layout (the selftest copies `scripts/chart-verify.sh`
   and reads workflow bodies). Mutate one validation at a time: malformed
   appVersion validation, empty push-before refusal, malformed-baseline
   refusal. Each must fail its owning named assertion; a later mismatch or
   earlier metadata failure is not sufficient. Scope a mutation to the target
   context if shared validation masks it. Restore exact bytes and rerun the
   complete selftest after each control. Retain optional Helm render coverage
   and record whether installed Helm actually exercised it; absence does not
   certify rendered behavior. Do not install tooling in verify.
3. Tighten presence HTTP-500/token-500/refused-connection needles to the
   spec's distinct arms. Reset per-case request logs and assert exact scope,
   repository/tag and stage progression. Reuse the existing HTTP fixture and
   assertion helpers. Preserve credential-safe argv, headers, both image tag
   forms, bare chart mode and workflow checks. Prove each changed presence
   assertion with a presence-only wrong-reason mutation so the earlier absence
   case cannot mask it. For each targeted shared unknown-state arm, exercise
   wrong-reason and incorrect-admission controls in disposable adjacent
   guard/test copies and restore positive. Record actual raw-000 curl behavior
   and separately prove the status-only 000 classifier control.
4. Prove the existing refusal socket lifecycle. Insert a temporary holder close
   after port publication but before ownership validation; do not close it so
   early that writing the port file merely fails. Require the real selftest's
   ownership diagnostic, not incidental transport failure. Restore and rerun.
   If necessary repair lifecycle in the same embedded fixture, without new
   processes, fixed ports or sleeps as synchronization. Check cleanup joins
   the exact child; do not claim cleanup survives SIGKILL.
5. Explicitly anchor intended include patterns. Build a private candidate copy
   with synthetic `.cargo-home/registry/src/decoy/` source/test/bench/asset/data
   paths and license/NOTICE basenames. Omit nested Cargo.toml files, which
   would conceal inclusion mistakes through Cargo's nested-package rule.
   Do not replace or copy the real Cargo home. Run offline actual package list
   on pristine candidate and fixture, using `--allow-dirty` only for deliberate
   disposable modifications. Check every required root family and automatic
   Cargo manifest/lock/readme files and reject all decoy prefixes. Individually
   restore each unanchored license/NOTICE basename in private controls and
   require the assertion to name a leaked member. Compare slash-pattern old
   and anchored membership individually; equivalence is valid evidence. Create
   a real offline `.crate` (for archive inspection, `--no-verify` may separate
   archiving from build verification, explicitly recorded) and compare license,
   NOTICE and entity-data bytes plus normalized package metadata. This is not
   a substitute for full locked test/build gates.
6. Run final candidate selftests and package proof, then all gates from the
   sole [house inventory](../../docs/house-rules.md#gates--part-of-every-change).
   This includes fmt, locked clippy/tests, pinned compile, exact coverage plus
   coverage selftest, publish/seal proofs, license deny and the fail-closed
   native-tls check. Dependency-tree command failure must not be mistaken for
   absence. Verify offline, install nothing; distinguish candidate canonical
   script receipts from the real pinned verify dispatch and preserve shim
   checks. No immutable baseline resources are rewritten.
7. Hand off current evidence and unresolved items to the existing full route.
   CI retains actual-event chart history/render/bump proof, Docker persistence/
   volume/context smoke, network advisory audit, tag same-run rate proof and
   prescribed real port-80 exchange. Local chart fixtures do not certify those
   CI runs. Root reconciles integration and CI. Preserve exact 18.5 evidence
   checklist and open #186/#192; author commits unsigned with no push.

## Quickstart for proof collection

From the candidate root, with installed approved tooling and isolated evidence:

```bash
bash scripts/chart-verify.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
cargo package --list --locked --offline
```

These are entry commands, not the complete acceptance proof: steps 2–5 add
independent red/restore controls, in-tree cache fixtures and archive inspection.
Run `cargo package --list --locked --offline --allow-dirty` only within the
private modified fixture. Execute the house inventory after restoration with
Cargo offline configuration; invoke brokkr only after its exact pin is checked.
Use candidate roots for candidate proof and dispatched pinned paths for verify;
never make a shim select alternate code. Save command exits and applicability,
not credential values. The deterministic design check can be preflighted with
`bash bundles/self/scripts/design-check.sh check .` after the design commit;
that local preflight does not replace the sequence's pinned check.

## Risks

| Risk | Mitigation and retained cost |
|---|---|
| Shared validators or generic needles make a mutation fail for the wrong reason | Named owning assertions, valid other inputs, per-case logs and presence-targeted mutations; more proof runs are intentional. |
| Invalid HTTP 000 is conflated with status classification | Record real curl transport outcome and separate status-only observer proof; neither proves GHCR/TLS behavior. |
| Socket close/reuse races or orphan fixtures | Keep one bound non-listening holder, port-0 allocation, explicit readiness, ownership checks and exact-PID joined cleanup. Platform behavior is exercised, not assumed; SIGKILL cleanup is not promised. |
| Cargo matching differs from visual interpretation | Real package-list/archive oracle, no nested-package camouflage, per-pattern mutation and equivalence reporting. Explicit anchors can be behavior-preserving. |
| Fixture copies include credentials or modify shared state | Generate synthetic cache files and credentials only; private directories and isolated copies, no real cache import or live registry. |
| Candidate green is misrepresented as pinned verify green | Separate script/tool/tree identities and real dispatch; no shim/hash/pin changes. Missing exact tools remain outstanding. |
| Release-contract scope drifts into tests | Preserve workflow and manifest identity, require escalation for changed release/security behavior, retain four-reviewer route. |
| Existing release failure modes remain | Outage refuses; check/push races and image-success/chart-failure persist; no rollback, retry, skip or digest-enforcement contract is added. |
| Inherited claims obscure present obligations | Fresh evidence only; 0051 stays proposed; 18.5 and CI-only evidence remain explicit and #186/#192 stay open. |
