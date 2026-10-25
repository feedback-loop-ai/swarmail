# Plan: portable webhook refusal fixture

## Technical context

One Rust 2024 crate with existing Tokio sockets and real SMTP/HTTP integration
tests. Scope baseline: ac160ff4a74a975226d58dd447f5f107327694f6; design entry:
b7196b8. Local execution is Linux; macOS execution belongs to CI. Reuse the
already implemented connected-client arrangement in the SIGTERM test inside
the webhook regression. No dependency, API, schema or production change.

No plan template exists under this candidate's `.specify/`; provide its
required sections here. This bounded fixture needs no separate research,
data-model or contract artifacts beyond the three requested design files.

## Constitution check

Preserve synchronous accept/store, subscribe-before-check, oldest-only pruning
and ingest extraction. The real protocol test remains in tests/webhook.rs
under 0020, with observed metrics and bounded waits. Preserve 0041 best-effort
webhook behavior, retries and accounting. No production seam, coverage
exclusion or denominator change. Use pinned proofs under 0050, unsigned
commits under 0056 and unchanged candidate routing under 0057/0058. No ADR or
operator decision acceptance is required for this test-only repair.

## Project structure and files

| Path | Role and permitted change |
| --- | --- |
| specs/residual-test-proofs/{spec,plan,tasks}.md | Exactly the chief's committed artifacts; reconcile current scope and preserve dated history. |
| specs/residual-test-proofs/intake-webhook-macos.md | Read-only current framing and baseline receipts. |
| tests/webhook.rs | Implementation edits only refused_connection_never_blocks_the_smtp_path fixture, lifetime and related comments/diagnostics. |
| tests/binary.rs | Read-only connected-client reference; preserve entire file. |
| tests/common/mod.rs, src/api.rs, Dockerfile | Read-only inventory and preservation of prior #191 fixes. |
| .forge/epic192/issue191/ | Supplied ignored evidence and fresh ignored proof receipts. |
| .brokkr/state/design/positions/ | Ignored panel evidence, never staged. |

All other source, Cargo files, decisions, proof scripts, tooling and recipe
routing stay unchanged. Do not extract a shared fixture or reopen snapshot
mutation work.

## Research and resolved choices

The checksum-verified macos-ci-238.md records the precise failure boundary:
unconnected bound webhook target times out; connected SIGTERM target passes.
Both panel positions converge on adopting that existing arrangement. Keep
explicit ownership local, accept minor duplication and disclose reuse-enabled
macOS interference. No namespace, global serialization or kernel-policy
experiment is justified. The exact setup probe is not each-attempt evidence.

Scoped source/history inspection finds two #191 reservation targets: webhook
is unconnected and needs repair; SIGTERM is connected and needs no edit.
db0b517's helper and API test changes introduce no refused reservation;
35afa14 identifies the binary target subsequently repaired. A tests-wide
search corroborates those functions; repeat the inventory after implementation.
The SIGTERM unconnected contender is only a bind check. The historical
98b54e51 object is unavailable locally, so the inventory is explicitly bounded
to current source and available residual history, without invented evidence.

The metric helper waits for value >= baseline + delta. Preserve its existing
delta-one call and exact storage assertions rather than claiming exact
exhaustion equality. Runtime crash durability, hostile DATA, chaos and TLS
failure behavior remain unchanged; this fixture does not independently test
every boundary discussed by robustness.

## Implementation approach

1. Verify the supplied hashes and engine identity. Record candidate/platform
   and scope baseline; inspect the existing webhook and SIGTERM functions.
2. Inside the named webhook function, bind a temporary listener to
   127.0.0.1:0. Preserve the IPv4 TcpSocket, explicit reuse false and explicit
   ephemeral loopback bind. Connect it to the listener under HELPER_DEADLINE,
   accept the peer under HELPER_DEADLINE and drop the listener.
3. Derive addr from the connected client's local_addr(). Retain that client
   and accepted peer as untouched locals through all observations. Update
   the fixture comment with the SIGTERM macOS limitation. Preserve the
   bounded real probe to addr and exact ConnectionRefused equality; update
   only diagnostics that describe the obsolete unconnected fixture.
4. Preserve the HTTPS URL using addr, PUT assertion, pre-mail metric baseline,
   SMTP success, first exact count, delta-one wait and last exact count.
   Replace final drop(reservation) with drop((reservation, peer)). Add no
   contender check, half-close, background owner, platform branch or sleep.
5. Run the focused test (exactly one pass), complete webhook and binary
   suites. Inspect the scope diff and current functions to establish that
   no #191 changed refused target remains an unconnected reservation.
6. Commit the scoped repair unsigned, then run every local house gate on
   that committed candidate and retain completed exit receipts. Final
   independent verification/review and CI macOS execution are later owners;
   design-baseline results cannot close implementation tasks.

## Data model and contracts

Test-only state: a temporary listener, explicitly bound connected client,
accepted peer and client SocketAddr. Lifecycle:
bind -> connect -> accept -> drop listener -> probe client endpoint -> PUT ->
baseline metric -> SMTP -> exact count -> delta-one exhaustion observation ->
exact count -> release pair. Connect, accept and probe use existing deadlines.
Normal final drop and RAII on unwind own cleanup; there is no keeper task.

No public data model or wire contract changes. SMTP acceptance/storage and
webhook best-effort loss remain separate contracts. Retention establishes
endpoint lifetime; the probe establishes setup refusal. Neither prevents a
later reuse-enabled macOS listener from claiming the endpoint or proves the
stage of every retry failure. Observation order does not impose a strict
SMTP latency bound or prove when the exhaustion counter first incremented.

## Quickstart and verification

Run offline with cached tools, install nothing, and keep routing unchanged.
Use the canonical pinned scripts after verifying their public shims (0050).
The authoritative inventory is [house rules](../../docs/house-rules.md#gates--part-of-every-change).

```bash
export CARGO_NET_OFFLINE=true
(cd .forge/epic192/issue191 && sha256sum --check --strict SHA256SUMS)
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo test --locked --test webhook refused_connection_never_blocks_the_smtp_path -- --exact
cargo test --locked --test webhook
cargo test --locked --test binary
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
```

Compare the engine digest with bundles/self/tooling.json and require no
policy-sweep warning. Coverage must finish with zero missed lines, branches
and functions and exit zero; a summary followed by timeout is insufficient.
Record platform, command, candidate commit/tree, output and completed exit.

Prove native-tls absence fail-closed with
`cargo tree --workspace --locked --offline --invert native-tls`: require a
nonzero exit AND the exact missing-package diagnostic line, accepting Cargo's
single-quote or backtick spelling for native-tls. Success means it is present;
any different failure does not prove absence. The intake contains the exact
Python validator for these two diagnostic strings.

The chief commits only spec.md, plan.md and tasks.md, then runs
`bash bundles/self/scripts/design-check.sh check .`. HEAD must touch exactly
those three clean committed files in one spec directory; no further commit
follows. Chief gate receipts stay ignored and describe existing Linux code,
not the future repair. Implementation repeats the required proofs afterward.
CI owns actual macOS and the charter's external gates; the operator owns live
settings, sealing and publication. No network or external inspection is needed.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| macOS reuse-enabled listener overlaps the endpoint. | Retain both ends and exact setup probe; disclose that parallel Tokio listeners can still claim it. Full exclusivity and each-attempt refusal are not promised. CI owns actual macOS execution. |
| Target accidentally uses listener/peer address. | Derive only from connected CLIENT local_addr; use the same addr in probe and URL. Connection success fails the exact oracle. |
| Early drop or half-close abandons reservation. | Local ownership, untouched streams and explicit pair drop after final count; source inspection confirms lifetime. |
| Setup fails or consumes resources. | Ephemeral loopback listener and two streams with bounded connect/accept. Failure stays visible; no probabilistic retries, longer deadlines or new framework. |
| Minimal duplication drifts. | Keep the passing binary fixture unchanged and compare its arrangement during review; accept a few duplicated lines. |
| Test results are overstated. | Exact storage counts and existing delta-one observation only; no strict latency, durable webhook, all-retry refusal or universal race-closure claim. |
| Scope reopens completed #191 work. | Baseline diff protects binary, helper, API and Dockerfile bytes and all other webhook functions. |
| Earlier receipts hide unfinished work. | Unchecked current tasks and fresh committed-candidate logs; historical sections explicitly superseded. Design Linux gates do not certify the repair or macOS. |
