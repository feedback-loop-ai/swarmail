# Intake — port issue #200 onto current main

Run: epic-192-issue-200-port-the-ship-4ab8f513. Date: 2026-10-25.
Phase: intake only; disposition: resolved. Inspected base: aec4b46.

## Goal

Port the six shipped #200 commits onto aec4b46 so every eval row records reproducible provenance and matrix evidence can be replayed, preserving current main's accepted trap scoring, privacy and cancellation behavior.

## Evidence, authority and scope

Read the packet README.md and operator-rulings.md first, then verified all 53 entries with `(cd .forge/epic192/issue200 && sha256sum -c SHA256SUMS)`. The first checksum invocation used the wrong working directory; the corrected invocation passes, including both required documents. Read port.md, house rules, the decision index, 0001–0003, 0055, accepted 0071 and the relevant 0073/0075 preservation contracts. Consulted the shipped framing/design and implementation patches, including the final source-observation cancellation repair. No GitHub, CI, journal or sibling worktree was consulted. The initial worktree was clean.

The packet is the source of shipped acceptance evidence, not a current gate receipt. port.md supplies six ordered format-patches, ef8a3dd..0bd501f. Its listed textual conflicts are evals/README.md, evals/src/main.rs and evals/tests/{fixture,scoring}.rs. Local history confirms aec4b46 (#198) touches these and the semantic owners runner.rs, state.rs, tasks.rs, proof.rs and report.rs. Prior #194/#196 owners are 8e61ab4 and ef8a3dd. Do not replace current owners wholesale with old patch versions.

Spot-check: runner::TrialResult and RESULT_SCHEMA are already v2 but lack provenance; write_artifacts still records package version and a null binary path. Matrix has no artifact option; check currently scores a bare snapshot. SNAPSHOT_SCHEMA and CATALOGUE_SCHEMA are 2; tasks::trap_class/score implement accepted 0071. Fixture public layout choices remain unseeded; Private::record copies arbitrary claim strings. The candidate engine matches SHA-256 ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f, as does tooling.json's pin.

The old schema-1-only escalation is historical. The shipped patches carry the operator's scoped schema-2 recovery direction; current accepted 0071 independently requires result v2 and supersedes the shipped schema-1 catalogue/snapshot, mixed-v1/v2 reporting and old scorer assumptions. Preserve 0071's refusal of v1 snapshots/results and mixed historical reports. No automatic relabeling, legacy scorer or pooled historical rates. Historical v1 execution/replay belongs to its old source/binary. New retained evidence must bind the current v2 catalogue, snapshot and scorer. Do not copy the old business-score-v1/catalogue-schema-1 identity into new evidence. Separate artifact/projection envelope versions are not business schema versions.

Restore/adapt proposed 0063 and its single index row at its allocated number; preserve accepted 0071 and proposed 0073/0075 statuses. Any restored historical 0055 addendum must be explicitly subordinate to accepted 0071, rather than reactivating v1 readers. Record a dated integration explanation without rewriting accepted historical bodies or declaring any new acceptance. This is reconciliation under the user's explicit direction to preserve newer main, not authority to amend 0071.

Scope is the shipped eval provenance implementation, tests and owning documentation, including its existing privacy and cleanup repairs. No root runtime, root manifest/lock, dependency allowlist, coverage denominator/floor, engine pin or recipe changes. Candidate bundles and protected review/security hold govern subsequent phases unchanged. No model measurements, namespace work (#221), push, merge, tag, publication, issue closure or decision acceptance.

## Frozen guarantees

0001 is adjacent through real SMTP DATA 250 followed by MCP retrieval; 0002 through real mail waits. Both remain unchanged. 0003 oldest-only pruning is preserved with no new pruning claim. Preserve 0004 real-protocol tests and 0005 ingest extraction. No benchmark or root throughput change is required.

## Checkable acceptance criteria

1. Every newly emitted schema-2 trial/matrix row, including setup failure, cancellation, unstarted suffix and retention failure, carries the shipped typed provenance: Swarmail source commit and SHA-256, build-time harness commit, explicit model and argv identity/digest, and per-trial seed, or the shipped explicit unavailability reason. No runtime HEAD attribution for an unrelated binary. Identity tests cover changed runtime checkout, worktree/packed refs, dirty/archive builds and revision rebuilds. A replaced/misassociated SWARMAIL_BIN is refused before affected execution, or its mismatch is explicit in every affected row; it cannot appear verified.
2. Preserve the shipped command identity/privacy contract: private/legacy argv is not automatically hashed or published; opted-in public argv uses the shipped framed digest. Model labels are declarations. Native program paths and literal arguments still execute through CLI/worker boundaries. Independent canaries prove no private argv/path or caller environment reaches rows, reports, diagnostics or artifacts.
3. Fixed seed and fixture time reproduce the same public projection in fresh processes, including digest position and invite time; selected changed-seed vectors change public layout. Retain shipped seed/time parsing, semantic keys, duplicate-task/Unicode vectors and pre-admission matrix assignment. Invalid options refuse before launch. Tokens, OTPs, passwords, challenge/ticket secrets, admin secrets and invite UID use independent CSPRNG entropy, demonstrated with controlled independent entropy. Trial IDs/inboxes remain fresh. Real SMTP/MCP/HTTP tests retain MIME Date/DTSTAMP, attachment/page equality, OTP supersession and invite acceptance/expiry proof.
4. A real two-task × oracle/null × two-repetition matrix retains eight ordered rows with unique IDs, assigned seeds, per-row state/result/metadata and bound plan/index. Missing/ineligible state is explicit. Shared-root runs do not overwrite. Early failure, crash, wrong ID/exit, malformed/multiple/oversized output and output failure produce honest infrastructure accounting. Retain the 64 KiB child capture bound, exactly-one matching row validation and continued drainage.
5. Requested retention failure changes infrastructure outcome/exit and prevents a valid completion marker. Metadata is last; the final index binds all planned cells. Parent fallback occurs only after reaping and without child-owned output. Preserve exclusive 0700 leaf, descriptor-private 0600 temporaries before payload, complete writes, no-clobber publication and ownership-only cleanup. Collision, partial write, interruption, metadata/index failure and both umask extremes leave unowned bytes/modes unchanged and still exercise cleanup.
6. Trial/matrix replay works offline without original services/binaries and launches no saved command. Validate versions, byte hashes/lengths, semantic associations, dimensions, paths and bounds before any retained replay row. Mutated/swapped state/results, symlinks, duplicate/missing coordinates, wrong seeds, incompatible scorer/catalogue and over-limit inputs refuse, including an invalid later cell. Test at-limit and over-limit boundaries. Replay leaves evidence untouched, uses current tasks::score, separates original infrastructure/budget outcomes from business replay and marks absent/ineligible snapshots unscorable.
7. Preserve accepted 0071 across all integrations: catalogue/snapshot/result business versions remain 2; v1 snapshot and v1/mixed report rejection tests stay red on acceptance regressions. New rows have provenance, while no reader fabricates provenance or pools pre-trap history. Seven named traps still beat recovered goals in either order, infrastructure precedes deadline, deadline precedes traps, and earliest exact matching trap wins. Retain claim/flow/status/class matching, known-Decoy/null-flow sibling attribution, expired-but-not-superseded OTP recovery, reset replay, isolation and non-target recovery. All completed external naive controls still exit zero with healthy real mail and recovered goals yet fail with their expected trap classes. Catalogue goals continue to disclose the traps. Replay those retained completed controls through the actual CLI with matching verdict/class.
8. At shared fixture recording, new evidence retains canonical provisioned claimant or null; unknown/empty/missing claimants retain HTTP/action/category/order semantics. Known decoys/foreign agents keep wrong-message evidence and 0071 attribution. Live OTP and nonnumeric canary unknown identities produce HTTP 400/tool_misuse and null claims, with no canary in admin snapshots, stdout/stderr, reports, replay or retained trees. Cover link, OTP, reset/login, invite, support, release and marketing callers. Writer-only membership guards run before admin/artifact publication; they do not become new scorer membership rules. Preserve compatible historical claim handling within accepted v2 evidence, strict required-key checks and value-free malformed-input diagnostics.
9. Every #196 regression retains assertion strength: env_clear maps, private HOME/runtime directories, generated agent PWD, caller-resolved native executable, explicit preparation selectors, lazy HOME/custom-target offline builds, no_proxy clients, bounded typed diagnostics and no raw panic/serde/child bytes. Worker context uses CLI arguments without widening the worker environment map. No production test-control or ambient inheritance escape hatch.
10. Every #194 and shipped #200 cleanup regression retains assertion strength for both SIGTERM/SIGINT: pre-spawn subscription, latched admission, once-only forwarding, owned Cargo/source helpers, abort/join, retained group/reaper/drain identities, concurrent five-second TERM grace and five-second post-KILL observation, direct waits and honest observation failure. Cancellation observed during source-task failure/cleanup prevents later Cargo admission. Preserve causal Pending hold/release and bypass controls, repeated signals, startup/completion races, resistant descendants and output pressure. Full product order/cardinality, committed prefix, post-cleanup active row, fresh zero-time refused suffix, late-signal exit 2, statuses/lifetimes, no retries and 2 > 1 > 0 exits remain. Envelope expiry cannot bypass cleanup or reset its budget. At the acknowledged child-retention/row-commit seam cancellation wins; divergent immutable child results prevent matrix.json and exit 2 without mutation. Index failure changes exit without rewriting committed rows.
11. Port the shipped independent provenance/boundary controls and map baseline #194/#196/#198 tests to retained counterparts. Do not delete assertions, weaken time/size bounds, substitute mocks or use sleeps as synchronization. All-task oracle passes, null idle failures, wrong-action and isolation controls remain real SMTP/HTTP/MCP. Documentation describes current v2 scoring, provenance availability/privacy, deterministic public choices, retained replay and unconfined limitations without stale v1 compatibility promises. Decision inventory/status tests pass with no new acceptance. Final committed implementation passes every gate below under candidate routing; historical receipts do not substitute for fresh evidence.

The detailed shipped AC1–AC13 and their negative controls are in patch 02's spec.md hunk; patches 03–06 contain subsequent repairs. The only superseded obligations are their old business schema/scorer/history assumptions, reconciled above with accepted 0071. All other shipped checks remain obligations; this framing adds no unrelated feature.

## Verify commands and ownership

Intake records obligations; implementation and hermetic verify execute the full gates against committed candidate work. This intake is not a feature-completion or gate-pass claim. Verify is offline and installs nothing. Check shim integrity and execute pinned bundle copies, as required by 0050; these are the house inventory entries exposed by public scripts/ shims.

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
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

Run the pinned verify-seat's fail-closed native-tls proof too; arbitrary Cargo errors do not prove absence:

```bash
if out=$(cargo tree -i native-tls 2>&1); then
    exit 1
elif printf '%s' "$out" | rg -q 'did not match any packages'; then
    :
else
    exit 1
fi
```

Coverage must report exactly zero missed lines, branches and functions, with its selftest passing. Bundle policy-sweep warnings refuse. The standalone evals crate is outside root test/coverage gates; its independent suite is mandatory. No new root coverage exclusion is authorized.

Focused real-protocol/provenance checks supplement the full suites (provenance is a target supplied by the shipped patches):

```bash
cargo test --locked --test store_contract_docs
cargo test --manifest-path evals/Cargo.toml --locked --test provenance
cargo test --manifest-path evals/Cargo.toml --locked --test fixture
cargo test --manifest-path evals/Cargo.toml --locked --test scoring
cargo test --manifest-path evals/Cargo.toml --locked --test proof
cargo test --manifest-path evals/Cargo.toml --locked --test runner
cargo test --manifest-path evals/Cargo.toml --locked --test report
cargo test --manifest-path evals/Cargo.toml --locked --test boundary
cargo test --manifest-path evals/Cargo.toml --locked matrix_completion_fence_is_observed_and_its_negative_control_exits_early
cargo test --manifest-path evals/Cargo.toml --locked runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults
```

Design must map provenance schema fixtures and binding/scorer identity tests to v2 and preserve current #198 completed-control/replay tests. Aggregate green cannot replace missing discriminating controls. Restore temporary mutations before final gates.

CI owns macOS evidence (evals wiring is #201), privileged port-80 exchanges, chart/history/selftest and chart bump, Docker smoke, network RustSec and tag-only release-rate-5k. Operator owns live GitHub settings and branch sealing. These are ownership records, not new local acceptance criteria or execution claims. No network, namespaces or external evidence is required for this intake; nothing needed for framing is missing.

Commit this framing unsigned. Subsequent seats commit their work before verification/return; the engine alone chooses transitions. Intake result is resolved framing only; implementation, full gates and protected review remain required.
