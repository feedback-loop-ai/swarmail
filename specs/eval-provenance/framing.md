# Intake framing — port issue #200 onto current main

Date: 2026-10-25. Run: `epic-192-issue-200-record-the-co-d61cd84e`.
Phase: intake only. Candidate base: `ef8a3dd`.

## Goal

Port the five shipped #200 commits' reproducible provenance, deterministic public fixture choices, historical replay and private-claim publication behavior onto current main while preserving every #194 cancellation/accounting behavior and #196 privacy property and regression test.

## Evidence and authority

The issue body, criteria and prior outcomes are in
`.forge/epic192/issue200/README.md`; operator directions are in that directory's
`operator-rulings.md`. All eight packet entries pass
`(cd .forge/epic192/issue200 && sha256sum --check SHA256SUMS)`.
The first checksum invocation used repository root rather than the manifest's
relative directory; the corrected invocation verified every entry.
The five numbered patches are ordered historical delivery evidence, culminating
in the supplied SHIP-COMPLETE receipt at a877db2. Historical receipts are not
current-candidate verification. No GitHub, CI, journal or sibling worktree was
consulted. Initial git status was clean.

Read house rules, the decision index, founding 0001–0003, accepted 0055 and
proposed 0073/0075. History identifies #194 at 8e61ab4 and #196 at ef8a3dd.
Spot-checks confirm TrialResult/RESULT_SCHEMA are v1, write_artifacts publishes
metadata with a null binary path, matrix lacks artifact options, and
Private::record copies arbitrary claimed text. Current runner/main tests and
the pinned verify inventory are present. The candidate-local `.forge/bin/brokkr`
matches SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.

Patch 0002 records the operator's explicit recovery direction: “amend accepted
decision 0055 to allow result schema 2 while preserving a schema-1 reader for
existing records.” Patch 0003 contains the authorized dated 0055 addendum.
Restore that exact scoped addendum once, preserving 0055's historical body and
accepted status; restore 0063-eval-provenance as proposed and its single proposed
index row. Patch 0001's schema-1-only design is superseded by that scoped
authority. The earlier schema-conflict stop is historical, not an unresolved
permission request. No broader acceptance follows. Preserve the later patch's
writer-only claim guard and compatible historical readers.

## Scope and frozen guarantees

The work sits beside accepted 0055 and current #194/#196 owners, described by
proposed 0073/0075. No change to 0001 synchronous accept/store, 0002 subscribe-
before-check, or 0003 oldest-only pruning is authorized. Real mail controls
exercise 0001/0002 indirectly; there is no new pruning or throughput claim.
Preserve 0004 real-wire testing and 0005 ingest extraction. Root production
semantics, coverage floor/denominator and license allowlist remain unchanged.

Expected owners are evals/src/{runner,main,fixture,mail,state,report}.rs,
evals/tests/, build-time identity support and the provenance/artifact/result
modules supplied by the patches, plus their owning documentation/design.
Port behavior into current owners. Replacing rewritten runner/main/tests with
old-base versions cannot satisfy preservation. Native literal argv execution
must survive across CLI/worker boundaries without publishing private paths.
This intake does not implement or supply a design verdict.

## Checkable acceptance criteria

1. **Every new row has truthful provenance.** Strict schema-2 trial/matrix
   rows, including admitted setup, cancellation, refused-suffix and retention
   failures, carry the shipped provenance contract: target binary digest and
   source commit, build-time harness revision, command/model identity and
   per-trial seed, or exactly the specified typed unavailability. Tests cover
   runtime checkout changes, worktrees/packed refs, dirty/archive builds and
   revision rebuilds. Never fabricate external source from current HEAD.
   Replacing/misassociating captured binaries refuses affected launches.
   Explicit model labels remain declarations. Legacy/private argv has no
   automatic hash; opted-in public argv uses the shipped framed digest, never
   literal argv/path publication. Private native argv still executes literally.
2. **Migration remains strict and compatible.** Historical v1 golden rows
   and mixed v1/v2 reports preserve outcomes, statuses and report formulas,
   with no invented provenance. Missing required nullable keys, duplicate,
   unknown, unsupported and hybrid fields refuse without fallback. New worker
   rows/envelopes require v2; old unversioned envelopes refuse. Schema-1
   snapshots and bare-state checks remain compatible. Invalid seed/time/new
   options refuse before launch. Document reader-first migration and exact
   permitted unavailable facts.
3. **Public fixture choices reproduce; secrets do not derive from them.**
   Preserve shipped --seed/--fixture-time formats, versioned semantic keys and
   framing. Fixed vectors include duplicate tasks and Unicode. Equal explicit
   inputs in fresh fixture processes reproduce projection v1, including digest
   link position and invitation time; selected changed-seed vectors change a
   field. Matrix assigns seeds and one fixture time before admission. Execution
   IDs/inboxes remain fresh. Controlled independent private entropy proves
   tokens, OTPs, passwords, challenges/tickets, admin secrets and invite UID
   independent of public seed. Real SMTP/MCP/HTTP proves Date/DTSTAMP/MIME,
   within-run attachment/page equality, OTP supersession, acceptance and expiry.
4. **Matrix artifacts bind the whole requested product.** A real two-task ×
   oracle/null × two-repetition matrix retains eight ordered rows, unique IDs,
   correct seeds and associated state/result/metadata plus bound plan/index.
   Absent/ineligible state is explicit. Concurrent shared-root runs cannot
   overwrite. Missing binary, early setup failure, crash, wrong ID/exit,
   malformed/multiple/oversized stdout and output failure produce honest
   infrastructure results for admitted slots. Retain #194's stricter 64 KiB
   child capture, exactly-one-row validation and continued drainage.
5. **Retention failures cannot masquerade as complete success.** Requested
   retention failure changes infrastructure outcome/exit and prevents a valid
   completion marker. Metadata is last; final index binds every planned cell.
   Parent fallback is post-reap and only without a child-owned directory.
   Existing/partial/conflicting evidence is untouched. Preserve #196 exclusive
   0700 leaf, descriptor-private 0600 temporary before payload, complete write,
   no-clobber publication and ownership-only cleanup. Existing parent/unowned
   bytes and modes remain unchanged. Test collision, partial-write, interruption,
   metadata/index failures and permissive/restrictive umasks; each failure
   still exercises owned-process cleanup.
6. **Replay validates before output and makes bounded historical claims.**
   Offline trial/matrix check validates every version, binding, semantic
   association, dimension, path and size before any replay row, then uses the
   existing scorer. Modified/swapped state/results, symlinks, missing/duplicate
   coordinates, wrong seeds, hashes/lengths, incompatible catalogue/scorer and
   oversized input refuse, including an invalid later cell. Test both at-limit
   and over-limit inputs. Check works without original binaries or services,
   launches no saved command and leaves evidence untouched. Original execution
   budget/infrastructure outcomes stay separate from business replay; missing
   or ineligible snapshots remain unscorable.
7. **New evidence excludes private claimants without breaking history.** At
   shared fixture recording retain only provisioned canonical identity or null.
   Unknown/empty/missing claimants preserve HTTP/action/category/order/scoring;
   known decoys and foreign agents retain wrong-message evidence. A live OTP
   obtained through real mail and a separate nonnumeric private canary used as
   unknown identity produce HTTP 400/tool_misuse with null claim and are absent
   from admin snapshots, stdout/stderr, reports, replay and entire retained
   trial/matrix trees. Cover shared recording and link, OTP, reset/login,
   invite, support, release and marketing callers. Writer-only membership
   guards reject invalid supplied snapshots before admin/retained payload or
   completion markers. Historical bare omitted/null/unknown and strict retained
   unknown claims remain scoreable without reflection or mutation; strict
   missing keys still refuse. Keep writer membership out of readers/scorer.
   Malformed private keys/types/facts produce value-free errors and unchanged
   exits; bad retained evidence emits no replay rows.
8. **Every #196 privacy proof survives.** Preserve cleared complete child
   maps, fresh private runtime directories, generated agent PWD, caller-bound
   native executable resolution, explicit preparation selectors, lazy HOME/
   custom-target offline builds and every client no_proxy. Independent canary
   scans prove new rows/reports/diagnostics/artifacts contain no caller
   environment, HOME/PWD or private path, including build/request/replay/error
   paths. No raw child output, serde fields, panic payload, URL or argv escapes
   through new code. No runtime test controls, ambient inheritance fallback or
   debug-output escape hatch. Canary failure receipts do not expose canaries.
9. **Every #194 cancellation/accounting proof survives.** Both SIGTERM and
   SIGINT retain pre-spawn subscription, latched admission, once-only forwarding,
   independent Cargo ownership and one cleanup owner. Preserve body abort/join,
   retained group/reaper/drain identities, concurrent five-second TERM grace
   and five-second post-KILL observation, actual cessation/direct waits and
   honest platform-observation failures. Private directories survive through
   cleanup. No new provenance/matrix timeout supersedes that fence. Keep causal
   Pending hold/release and bypass controls, startup/completion/repeated-signal
   races, resistant descendants and output-pressure assertions. Preserve full
   product order/cardinality, committed prefix, post-cleanup active row, fresh
   zero-time refused suffix, cancellation-winning commitment, late-signal exit
   2, actual statuses/lifetimes, no retries, 2 > 1 > 0 exits and report math.
   Authorized schema migration changes representation, not those semantics.
10. **The complete real-protocol controls retain their strength.** Map every
    baseline #194/#196 regression to its ported counterpart; adapt only version/
    API expectations required by the authorized migration. All-task oracle
    passes, null idle failures, wrong-action, infrastructure-before-budget-
    before-business precedence and isolation controls pass through real
    SMTP/HTTP/MCP. Do not delete/dilute tests, weaken bounds, use sleeps as
    synchronization or substitute mock servers.
11. **Authority and contradictions remain explicit.** Restore only the exact
    scoped dated 0055 addendum, proposed 0063 at its allocated number and one
    proposed index row; inventory/status tests pass. Keep 0073/0075 proposed.
    If a genuine additional contradiction is demonstrated, name both exact
    requirements and affected code/test and stop that choice for the operator.
    Intake found no demonstrated unresolved contradiction; it grants no new
    acceptance or authority to choose away privacy/cancellation requirements.
12. **Fresh candidate gates and current routing govern completion.** Execute
    all commands below on committed work and record actual exits. Use candidate
    bundles as they stand, including protected review and hard security hold;
    do not import stale routing from the old patches. Current careful and fast
    design/engine lanes require five separate review roles. The engine owns
    transitions; intake supplies no review result. Commit unsigned before
    returning from the responsible phase. No push, merge, tag, publication,
    issue closure, decision acceptance or dependency-allowlist widening.

## Verify gate commands and ownership

Intake records commands; implementation and hermetic verify execute them on
committed work. Verify stays offline and installs nothing. Check engine digest,
tooling and pinned resource/shim integrity first. The pinned script commands
below execute the same house inventory entries named by public scripts/ shims;
verify checks shim bytes and uses pinned copies, per 0050 and verify-seat.sh.

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check -
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
```

Coverage requires exactly zero missed lines, branches and functions, with no
floor/denominator reduction. Bundle compile policy-sweep warnings refuse.
Also run the pinned verify-seat's fail-closed native-tls absence proof (only
its specific package-absence diagnostic passes, not an arbitrary Cargo error):

```bash
if out=$(cargo tree -i native-tls 2>&1); then
    exit 1
elif printf '%s' "$out" | rg -q 'did not match any packages'; then
    :
else
    exit 1
fi
```

Focused proofs supplement the full suites. The provenance target is supplied
by the patches to port; its listing does not claim it exists on this base.

```bash
cargo test --locked --test store_contract_docs
cargo test --manifest-path evals/Cargo.toml --locked --test provenance
cargo test --manifest-path evals/Cargo.toml --locked --test runner
cargo test --manifest-path evals/Cargo.toml --locked --test report
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo test --manifest-path evals/Cargo.toml --locked --test fixture
cargo test --manifest-path evals/Cargo.toml --locked --test scoring
cargo test --manifest-path evals/Cargo.toml --locked --test boundary
cargo test --manifest-path evals/Cargo.toml --locked matrix_completion_fence_is_observed_and_its_negative_control_exits_early
cargo test --manifest-path evals/Cargo.toml --locked runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults
```

Design maps inherited AC1–AC12 and their negative controls to current owners;
aggregate green cannot replace a missing discriminating proof. Restore all
temporary mutations before final gates. No bench is required for this
harness-only work; root throughput semantics do not move.

House CI owns chart/history/selftest, Docker smoke, network RustSec, tag-only
release-rate-5k and privileged real port-80 proof. The supplied operator ruling
assigns macOS eval proof to #201 CI gating; evals is outside CI until #201.
These are ownership statements, not execution receipts or local blockers.
Namespaces (#221), live GitHub settings and network evidence are not added
criteria. Preserve Linux preparation/custom-target/artifact proofs. All input
evidence needed for framing is present locally.

Intake result: resolved framing only. No implementation, full-gate pass, review
verdict or feature delivery completion is claimed.
