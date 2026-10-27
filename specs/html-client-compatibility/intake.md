# Intake — epic #251 issue #259: HTML client compatibility

Run: `epic-251-issue-259-check-html-fo-22774bce`; phase: intake; 2026-10-26.
Disposition: resolved framing. Implementation belongs to subsequent seats.

## Goal

Add a versioned, repository-backed HTML compatibility rule set to #252's lazy
cached deliverability report, identifying client compatibility hazards,
accessibility omissions, image-only content, excessive size and tracking pixels.

## Authority and inspected baseline

The supplied issue259 README and operator-rulings both pass their directory's
SHA256SUMS (checked from that directory); the prior-runs section is empty.
The operator's #251 ruling requires lazy cached analysis from stored bytes,
never SMTP analysis, and built-in deterministic rules without an external
scorer. Its authentication-crate direction concerns later authentication work;
it does not justify a new HTML dependency automatically. No external service,
journal or sibling worktree was consulted.

Starting HEAD: `7f27fe6`. Local history records the shipped report implementation
at `9f5f142` and subsequent deadline correction at `a9d0681`. Read the house
rules, decision index, 0001–0003, 0005, 0031 and proposed 0085; spot-checked
`src/deliverability.rs` (`Inputs`, `Collector::push/finish`, `collect`, `evaluate`,
`Service::with_inputs/report`), `Store::qualify/wait_for_report`, the real-server
`tests/deliverability.rs` suite, pinned verifier and current recipe definitions.
The current report is `swarmail-demo/1`, with a pure collector, typed locations,
canonical sorting/deduplication, Weak identity/config/version cache, four jobs,
128 slots, 256 findings and 65,536 serialized report bytes. Extend these owners;
no second analyzer surface, cache, predicate implementation or routing change.

0085 remains proposed. Reserve **0090** as explicitly assigned by this task,
even though the historical operator packet's ledger stops earlier. Design must
record new rule/parser/size/location semantics in proposed 0090 and its index
row; no seat accepts it or edits an accepted ruling. Additive analysis is
compatible with accepted ingest extraction: do not derive or replace stored
links/codes/bodies or broaden 0031's display carve-out. No missing workspace
input currently prevents framing.

## Checkable acceptance criteria

1. **Tracked data and version:** repository data declares the enabled rules,
   stable rule IDs, severity, fix guidance, client applicability, relevant
   property/element/value selectors and numeric thresholds. Each entry names
   its source; the data records its version and the basis/date of the captured
   source information. Tests assert catalogue validity and that every declared
   rule has a passing and failing fixture. Runtime reads no network source.
   Report `rule_set_id` advances from `swarmail-demo/1`; rule/parser/location/
   hint/order changes advance the version as required by 0085. Equal bytes,
   effective config and rule version produce byte-identical report JSON on
   cold/warm reads and after restart; changed versions cannot reuse old results.
2. **Client compatibility:** fixtures pin findings for unsupported CSS
   properties (and values where the data distinguishes them) and unsupported
   HTML elements for the major clients explicitly named by the catalogue.
   Each enabled rule's failing fixture produces its exact rule ID, severity,
   typed source location and nonempty actionable fix hint; its passing fixture
   produces no finding for that rule. Design records the supported client
   matrix and conservative parsing limits; no claim of exhaustive rendering
   simulation or untested client behavior.
3. **Missing alt and image-only content:** distinct paired fixtures prove the
   alt-text rule and image-only-mail rule. Design pins absent versus empty alt,
   decorative-image treatment, what counts as visible text, and the scope of
   MIME body candidates, with positive/negative witnesses for those choices.
   Plain-text-only mail and attachment HTML do not acquire body-HTML findings.
   Multi-part, encoded and malformed input witnesses pin the chosen parser
   behavior and source-relative locations without rewriting stored fields.
4. **Oversized messages:** a sourced rule (including Gmail clipping) declares
   the byte unit, measured content scope and exact comparator/threshold in data
   and proposed 0090. Fixtures immediately below, at and above each threshold
   assert whether the size finding appears. A reportable size warning remains
   distinct from an evaluator resource-limit failure; failures cannot be
   returned as empty, clean or silently truncated reports.
5. **Tracking-pixel disclosure:** paired fixtures prove a disclosed potential
   tracking-pixel finding and an ordinary-image pass. Data/design state the
   deterministic heuristic, severity and limitations, with boundary witnesses
   for the dimensions/visibility/source forms the heuristic supports. Reports
   disclose the detected signal and a fix hint; evaluation never fetches an
   image or link and does not claim proof of sender intent.
6. **One report contract:** new findings flow only through the existing
   collector/canonicalizer, typed MIME-part/link coordinates and bounded
   report service. Unit tests pin sorting/deduplication, input/version identity,
   malformed/limit behavior and failure-not-clean semantics for new code.
   Real SMTP -> REST/MCP/UI tests in `tests/deliverability.rs` (and existing
   stdio suite where needed) prove identical new findings, escaped UI output,
   current MCP JSON text-block behavior, inclusive severity qualification,
   exact asserts and minimum-one waits. Preserve sanitized errors and existing
   report finding/JSON bounds; new failures cannot produce a partial count.
7. **Frozen guarantees and ingest:** existing owner-local and real-protocol
   regression tests continue proving synchronous queryability after DATA 250,
   zero report evaluation during SMTP/seed/restore, unchanged raw and extracted
   fields, subscribe-before-scan while qualifying reports, and oldest-only
   retention. No derived state enters persistence or changes Store mutation.
   New HTML analysis executes lazily offloaded through the existing worker
   admission and cache; tests prove reuse rather than reanalysis per adapter.
8. **Delivery and dependencies:** living documentation names the rule version,
   client/source matrix, heuristics and limits; proposed 0090/index describe
   new rulings. Default to existing dependencies (`mail-parser` is present);
   any new dependency has a specific design justification and permissive
   license under house policy. Keep candidate bundles/recipe routing intact.
   All local gates below must pass on the committed implementation, with zero
   missed lines, branches and functions and a passing coverage selftest.

Each rule's fixture pair is an assertion on presence/absence of its exact
finding, not merely a successful evaluation. Design assigns literal rule IDs,
source snapshots, client matrix, severities, threshold values and parser
semantics before implementation; intake does not invent unsupported claims.

Nearest frozen guarantees: **0001** (no report work on SMTP), **0002** (existing
wait owner retains subscribe-before-scan), **0003** (cache ownership cannot
change oldest-only pruning). Accepted **0005** and **0031** remain intact:
analysis is a separate derived product under #251/0085, never query-time
replacement of ingest-extracted links/codes. No frozen weakening is requested.

CI/operator ownership stays outside local acceptance: macOS execution,
network RustSec, Helm/chart/history, Docker, privileged port-80 and tag-only
release-rate proofs, live GitHub settings, namespaces, decision acceptance,
branch sealing and publication. No push, merge, tag, issue closure or acceptance.

## Verify commands

Extend the existing real-server suite and deterministic owner tests under 0020:

```bash
export CARGO_NET_OFFLINE=true
cargo test --locked --lib deliverability
cargo test --locked --test deliverability
cargo test --locked --test stdio
cargo test --locked --test burst
cargo test --locked --test p2
cargo test --locked --test rate
```

No new throughput claim is made, so a new benchmark is not an acceptance gate.
Full workspace testing covers the remaining protocol and persistence suites.
Run every house gate plus the pinned verifier's structural/selftest proofs:

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
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
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf '%s\n' "$out" >&2; exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2; exit 1
fi
```

The public script shims are integrity-checked, then canonical bundle scripts
execute under 0050. Use candidate-local `.forge/bin/brokkr` with the exact
recorded digest, run offline and install nothing. The intake gate receipt is
baseline evidence for framing only; implementation requires fresh verification.

## Intake baseline evidence

All 14 commands in the [gate receipt](../../docs/delivery/issue259-intake-gates.json)
passed on framing commit `fcd4f811d9f12d2d56bbe58862ec35686a9f37b7`, tree
`8e5e268d18be3df9bd3238093fd0cb8dd68e3b59`. Every recorded log hash was checked.
Exact coverage passed at 100% lines, branches and functions; the full coverage
selftest passed. Coverage evidence is `target/coverage/run.bqJrY0`; per-command
logs remain in `.forge/evidence/issue259-intake/`. The workspace client timed
out after 300 seconds, but the continued runner finished all commands and
wrote its final receipt; the timeout is not used as a gate verdict.

The receipt commit changes documentation/evidence only; tested source, gates
and bundle bytes remain unchanged. This resolves intake framing, not the HTML
feature. Design, implementation and fresh implementation verification remain.
