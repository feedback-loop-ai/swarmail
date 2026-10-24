# Plan: residual test proofs

## Technical context

One Rust 2024 crate; `src/api.rs` aliases Registry to a real std mutex over a
BTreeMap. `cumulative_rows` holds one guard while summing and calling
`render_rows`. Its inline test already seeds the required cap/overflow fixture,
but uses a source literal as its discriminator. Code baseline is `68b4ef8`;
design-entry `9db0ecc` adds intake only. Linux is local, macOS is CI-owned.
No new dependency, feature, schema, public API or non-test behavior is needed.

No plan template exists in this candidate. This plan supplies technical context,
constitution check, structure, research, data model, contracts, quickstart and
risks directly; separate research/model/contract files would add no useful scope.

## Constitution check

Preserve 0001–0005 and 0047 §2 unchanged. The test remains inline and measured
under 0020 rules 1 and 4; all support outside it is `#[cfg(test)]` on its owner,
with an installer test and coverage of both installed and absent paths. No
cold-path harness migration, exclusions, coverage denominator manipulation,
feature seams, allowlist changes or test mocks. Use pinned executed proofs
(0050), unsigned commits (0056), and unchanged 0057/0058 review routing and
security hold. This is stronger evidence for an accepted invariant, not a
semantic amendment; no ADR is drafted or accepted.

## Project structure and files

| Path | Role and permitted change |
|---|---|
| `specs/residual-test-proofs/{spec,plan,tasks}.md` | Chief's only three committed design files; reconcile active framing and preserve dated history. |
| `specs/residual-test-proofs/intake-23.5.md` | Read-only resolved intake authority for this phase. |
| `src/api.rs` | Implementation changes only the named inline snapshot test and private test-only acquisition support beside its owner. |
| `Dockerfile`, `tests/common/mod.rs`, `tests/binary.rs`, `tests/webhook.rs` | Preserve bytes against `68b4ef8`; reviewed 23.1–23.4 and distinct webhook proof. |
| `.forge/epic192/issue191/` | Ignored receipts, exact mutation backup/diff and command logs; never stage. |

Other source, decisions, Cargo, dependencies, gates, tooling and routing are
preservation context. No new source files or shared registry refactor.

## Research and resolved choices

Both panel positions and the local review identify the same defect: idle output
plus source comparison cannot prove one executed snapshot. The requested intake
directory and operator-rulings file are absent; committed intake and current
operator framing resolve the necessary seam permission. The old binary-gap
narrative is obsolete on `68b4ef8`; AC1–AC5 remain unchanged.

Choose simplicity's scrape-local forwarding facade with next-call thread-local
installation, adopting robustness's real-guard, isolation and negative-control
requirements. Reject replacing Registry globally and converting constructors.
That would observe more consumers but widen the diff without strengthening the
required current split-lock proof. The narrower observer must own every access
within the scrape, including any callee: future bypasses require repairing the
proof, not claiming the count still covers them. No source pin or entry-only
notification can substitute for acquisition observation. No writer or scheduler
coordination is required because the count distinguishes the mutant even on an
idle registry. See the spec's explicit rulings and accepted limits.

## Implementation approach

1. Record actual HEAD, code baseline, platform and clean status. Confirm AC1–AC5
   preservation and read the `/metrics` call, `cumulative_rows` and `render_rows`.
   Save a baseline for shipping source comparison. Leave fixture and helper
   implementation untouched. Do not reuse old mutation/gate receipts.
2. Add private `#[cfg(test)]` support in `src/api.rs`: a borrowed-registry facade
   holding that real registry and an optional local acquisition counter. Its
   `lock()` delegates directly to the same std mutex and returns its actual
   `LockResult<MutexGuard<...>>`; it adds only observation, preserving poison
   errors and guard lifetime. Expose no Deref/unobserved lock route. Keep the
   alias, constructors and current non-test lock/sum/render statements unchanged.
   At `cumulative_rows` entry, a cfg(test)-only binding shadows the registry with
   this facade. Helpers used by a scrape must retain the observed access path.
3. Use a next-call thread-local slot, consistent with the owner module's existing
   seam ownership pattern. The installer stores a local `Rc<Cell<usize>>` counter;
   scrape entry takes the slot, transferring observer ownership to that invocation.
   The synchronous call does not await, so no cross-thread coordination is needed.
   Consume installation before any assertion, avoiding leftover hooks after
   failure. Setup uses its own private registry and precedes installation.
   Keep the implementation small; helper names are not a contract.
4. Remove the test's `include_str!`, extraction anchors, copied implementation and
   source assertion. Retain its six real registrations and exact expected bytes.
   Exercise an unobserved call and an observed call; each renders the fixture
   exactly. The observed call asserts count equals one, with a single-snapshot
   diagnostic (zero also fails). A subsequent unobserved call may verify slot
   consumption without adding failure-only seam branches. Cover presence and
   absence through normal passing execution. Do not assert/panic on the second
   lock inside the facade: count normally and let the test supply the diagnostic.
5. Run the exact named test below, requiring one selected test and green exit.
   Save the repaired `src/api.rs` bytes and hash, plus candidate identity and
   scoped diff. Install cleanup before any mutation. Temporarily change only
   the shared scrape statements to sum within a scoped first guard, release it,
   reacquire through the unchanged observed binding, and render with saved total.
   Keep test and seam unchanged. Use the identical command: it must compile,
   select one test and fail at the count assertion with observed two/expected
   one. Preserve exact output assertions; idle output is not the discriminator.
   Save mutant diff, output and exit. A build error, timeout, source mismatch,
   zero tests or unrelated panic refuses the experiment.
6. Restore exact saved bytes on success and failure, verify equality, then rerun
   the same named test green. Never stage the mutant. A failed proof stays open;
   restoration alone does not make the negative control pass. Inspect the final
   diff against `68b4ef8`: only the inline test and cfg(test)-only support change
   source. Inspect non-test expansion to confirm the seam is absent and shipping
   statements unchanged; this is not a claim of reproducible executable hashes.
7. Execute the existing real-protocol tests and full local house inventory on the
   restored candidate. Save candidate-bound commands, platform, exit and relevant
   diagnostics in ignored evidence. Require successful zero-missed coverage and
   coverage selftest, without moving code or lowering any threshold. Keep
   implementation tasks open until their evidence exists; hand off independently
   to verify/review with unsigned scoped commits, never publication or acceptance.

## Data model and contracts

No application data model, metric names, encoding or persistence changes.
Test-only state consists of a next-call optional counter, a facade borrowing one
registry, and the local test-owned counter handle. Lifecycle: seed private map →
install observer → take slot on synchronous scrape entry → observe each real
lock → sum/render under actual guard → return/drop facade → assert output/count.
The cleared slot cannot affect the next scrape or another test thread.

The fixture stays total 6, LF overflow 2, a 2, b 1, c 1. Test-only observation
measures lock acquisitions in this path, not every conceivable copied snapshot
or cross-family atomicity. `render_rows` still uses the actual held map. The
negative control releases the first guard before reacquiring; trying to lock
twice while holding the first would deadlock and supply no valid proof.

Preserve independent webhook delivery and stored-mail truth; no new ordering
between SMTP receipt and observed retry exhaustion is claimed. Frozen storage,
waiter and pruning guarantees, ingest extraction, hostile DATA handling and TLS
verification remain unchanged and are not newly proved by this local seam.

## Quickstart and verification

```bash
# Use exactly this command on repaired, mutant and restored source:
cargo test --locked --lib api::tests::cumulative_scrape_uses_one_registry_snapshot -- --exact
# Preserve the already-reviewed real-server proofs:
cargo test --locked --test binary sigterm_with_a_delivery_in_flight_prints_the_final_counts -- --exact
cargo test --locked --test webhook refused_connection_never_blocks_the_smtp_path -- --exact
cargo test --locked --test api --test lifecycle --test binary --test webhook --test mcp --test smtp_edges
```

Each focused command selects exactly one test. Execute the complete
[sole house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
including fmt, locked clippy/test, bundle compile, coverage,
coverage selftest, publish-guard proof, seal-branch proof and licenses. Use
`.forge/bin/brokkr compile --bundle bundles/self` only after its SHA-256 equals
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`
and `bundles/self/tooling.json`'s binary identity. Check shims against
`bundles/self/scripts/shims.sha256`; independent verify executes pinned copies.
Exact coverage requires zero missed lines, branches and functions plus exit zero.

Run fail-closed native-tls absence with
`cargo tree --workspace --locked --offline --invert native-tls`: success means
refusal; a nonzero exit proves absence only if its output contains the exact
line ``error: package ID specification `native-tls` did not match any packages``.
Other errors cannot prove absence. Cargo here quotes the package with backticks;
this supersedes the intake example's apostrophe-only diagnostic matcher without
changing the absence requirement. The charter remains the inventory authority.

CI owns macOS, chart contract/history/selftest, Docker smoke, RustSec, tag-only
release-rate and real port-80 proof exactly as the house inventory specifies.
No benchmark or new spelling test is needed. Chief-phase baseline gates describe
the unrepaired source and cannot discharge the behavioural experiment or final
verification. The chief validates committed design structure and leaves its
three-file commit last; the deterministic sequence check owns advancement.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Instrumentation counts an entry notification instead of acquisitions. | Attach observation to real delegated lock operations; unchanged seam/test must detect the compiling split-lock mutant as two. |
| A callee or alias bypasses the local facade. | No Deref escape; audit all scrape accesses and require the mutant uses the same path. Future paths must preserve observation. Broader registry tracing is deliberately not claimed. |
| Test-only forwarding changes locking semantics. | Return actual LockResult/guard; preserve the shared non-test statements and renderer; inspect expansion and scoped diff. |
| Installation leaks between tests or counts fixture setup. | Private map, seed before install, thread-local next-call slot consumed at entry, local counter ownership and explicit absent path. |
| Deadlock or chance masquerades as a red proof. | Sequential sum/drop/relock mutation; no writer, sleep, timeout-as-success or retry loop. Require the specific two-versus-one assertion. |
| Coverage punishes unused observation branches. | Keep deterministic test inline; exercise both installed/absent paths, no special failure branch, exclusions or harness relocation. Require gate exit zero and exact counts. |
| Mutant survives failure. | Save bytes and install unconditional cleanup before edit; verify restoration and restored green before staging. |
| Prior repairs or production behavior drift. | Byte-compare protected files to `68b4ef8`; restrict non-test changes to none and inspect cfg(test) expansion. |
| Historical green receipts hide unfinished work. | Record new candidate-bound evidence; distinguish design baseline, repaired green, mutation red, restoration green and final gates. |
| Local tests are mistaken for platform or durability proof. | Keep CI ownership and explicit contract limits; no new crash/outbox/TLS claim. |
