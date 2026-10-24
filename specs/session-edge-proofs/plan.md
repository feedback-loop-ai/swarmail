# Implementation plan: session edge proofs

Implements [spec.md](spec.md); ordered proof obligations are in [tasks.md](tasks.md).

## Residual execution posture

Correct only the existing three-document family on input candidate `11134f1`.
The actual reviewed Swarmail base is `45d68df`, followed by accepted engine-pin
commit `238372d`; bind each later execution to its own current HEAD/tree.
Historical implementation `e7d1d8142c3e24b2a817822969ab0dae6619680a`
following design `150c84a` and base `cc30ea2` explains inherited AC1–AC6.
It is not the current candidate binding. No new implementation or fixture
repair is authorized here; retain failures and return to triage before repair.

## Technical context

One Rust crate using the existing locked Tokio/Axum stack, serde_json,
rusqlite and tempfile dev fixture. SMTP/POP3 tests use the real server and
raw TCP clients in `tests/common/mod.rs`. The #177 base supplies POP3's
private task-local `ListenerProbe`, one-shot accept error, backoff-pending
watch and held retry readiness. `AppState` and `api::serve` already support
sharing that listener's flag with a real HTTP server.

No plan template exists at `.specify/templates/plan-template.md` in this
checkout. Following the repository's prior plan precedent, this document
contains the required technical context, constitution check, project
structure, research, data model, contracts, quickstart and risks. Research,
model and contract material stays here; the design commit has exactly three
artifacts in this one directory.

## Constitution check

All assertions pin existing accepted behavior. There is no semantic change
and no ADR to declare. Preserve synchronous insert before DATA acceptance,
subscribe-before-check, oldest-only pruning, ingest extraction and
`envelope_from`. Real SQLite trigger refusal is compatible with 0047's raw
writer contract; never open a second cooperating server on a database alias.
Keep #177's actual timer, owner-local observation and task-completion fixes.

Preserve 0020 placement: public wire tests in `tests/`, branch-stable positive
library assertions inline and measured. Only a concrete failure-only arm
would warrant the narrow declared `src/pop3/health_tests.rs` alternative
after re-triage authorizes new work; this residual permits no such move.
All seams remain cfg(test), privately owned and exercised. The exact
coverage gate still measures all production code and inline tests.

A failing behavioral witness requiring production repair returns to triage
with evidence before any repair; a conflicting accepted ruling additionally
requires an explicit proposed amendment and operator decision. Full4 remains
selected under the existing delivery policy; this document dispatches none.

## Project structure and files touched

| File | Residual edit |
|---|---|
| `specs/session-edge-proofs/spec.md` | Correct current/historical identities, synthesize the current panel, define residual record acceptance while preserving AC1–AC6. |
| `specs/session-edge-proofs/plan.md` | Correct candidate-local quickstart, add the single tracked provenance note, bound implementation and verification scope. |
| `specs/session-edge-proofs/tasks.md` | Correct current execution binding and add paired record checks while retaining T2–T7 witness mappings and delivery obligations. |

Read-only preservation references: `tests/smtp_edges.rs` (AC1, AC3–AC5),
`tests/pop3.rs` (AC6), `tests/common/mod.rs` (raw replies and helpers),
`src/pop3.rs`'s inline health proof (AC2), `tests/persistence.rs`, runtime
modules, tooling.json and its patches, scripts, bundles and accepted decisions.
No harness move or test framework is part of this residual.

## Research and evidence baseline

The following gaps describe the historical pre-implementation base, not the
current candidate. The inherited implementation closes them with the witnesses mapped in
[tasks.md](tasks.md); that source inspection does not establish passing gates.

At `cc30ea2`, `sequencing_errors_need_mail_then_rcpt` checks only a 503 prefix
and stops. The DATA arm restores a recipient-free envelope but consumes a
complete envelope before chaos/commit outcomes. `auth_during_a_transaction_is_refused`
uses malformed base64; production checks the envelope before decoding.
PLAIN emits `334 ` and cancellation leaves identity unset. POP3 resolves
marked/gone slots before fetching current content; existing partial-QUIT
and RSET tests pin RETR/DELE but omit TOP on committed-gone slots.

The existing real HTTP degradation test drives SMTP, while POP3's direct
handler case sets flags. AC2 must connect the actual POP3 accept failure to
the actual HTTP router. The owner-held readiness from #177 closes the
sampling window without repeated injections or deadline spinning.

These were inspected coverage gaps, not observed production regressions or
baseline-red claims. Do not fabricate red evidence. A failure in a new test
must first be classified as fixture error, placement/coverage error or a
concrete production defect before proceeding under the spec's scope rule.

## Data model and ownership

No persistent model changes. Test values are a socket/session, optional
authenticated inbox, open envelope sender/recipient vector, raw payload,
server-returned ID and exact inbox ID set. Header sender differs from MAIL
sender. Refused and accepted payloads/recipients differ. In AC4 the dedicated
store separates `default` from the later named inbox; AC5 compares inbox
inventory before any successful delivery could create an inbox.

POP3 uses its existing snapshot numbering, pending DELE marks and committed
`gone` slots; no lock, cached body or new state enum. The selective trigger
refuses one later ID so the earlier prefix genuinely commits.

Each fixture owns ports, server handles, shutdown senders, probe and database.
Subscribe to the backoff event before arming/triggering it. Keep the probe
inside one `LISTENER_PROBE.scope(...serve...)`. Use release-on-drop/owned
cleanup to remove holds and abort outstanding tasks on failure, and explicit
stop/join on success. If a cold diagnostic is needed to preserve joining on
failure, record it for re-triage under 0020 before changing a harness. EOF is not task completion.

## Contracts and implementation approach

Audit and execute the inherited implementation below; do not repeat these
changes. These are preserved contract/witness descriptions, not residual
edit targets. Any fixture or runtime defect returns to triage before repair.

1. Retain an untrimmed SMTP single-line reader delegating to existing
   `read_line_named`, without altering reply/reply_multi callers. Use it for
   exact prompt and fixed SMTP reply bytes. Drive DATA explicitly where an
   exact 354 and final acceptance/refusal matter. Parse the entire success
   prefix plus one nonempty ID, then fetch `/api/v1/messages/<id>` and its
   `/raw` immediately; assert 200, exact metadata/raw and inbox membership.
   Account for `SmtpConn::send` appending CRLF and dot transparency once.
2. Retain AC1 in its current sequencing test. For AC3 use two explicit tests:
   acknowledged HTTP DATA-chaos configuration at probability 100, delay 0;
   and a tempfile persistent server with `refusing_trigger(db, "insert")`.
   Assert empty pre-recovery membership, remove the cause, continue on the
   same socket with a distinct envelope and witness exactly the accepted ID.
   Reuse trigger remove/Drop; do not replace commit failure with a busy timer.
3. Retain AC4 with a dedicated store, literal prompt/cancel/NOOP, default mail,
   later valid AUTH and named mail. Retain AC5 with `base64_of` over known
   three-field credentials and real inventory comparisons; keep malformed
   input tests separate. Successful AUTH after AC5 is unnecessary duplication
   of AC4; original delivery and unchanged inventory prove its own contract.
4. Retain existing partial-QUIT and `rset_never_resurrects_a_committed_delete`
   tests for AC6. Keep marked TOP before QUIT and gone TOP before/after RSET,
   exact partial-QUIT error and a NOOP after every refusal. Preserve existing
   HTTP/disk witnesses and exact suffix content; the external-clear test
   remains a separate current-store-absence path.
5. Retain `pop3_degradation_reaches_http_healthz` beside the POP3 probe. Bind
   real POP3 and HTTP listeners, fresh store/shared POP3 flag, false SMTP
   flag and live shutdown watch. Complete POP3 greeting/USER/PASS/NOOP and healthy
   HTTP request first. Hold retry_ready false, arm accept_error, offer a
   connection, await backoff_pending, then GET health. Compare status 503
   and complete parsed JSON equality; do not pin member order or dynamic
   healthy uptime. Release the hold, close clients, signal shutdown and join.
   No new recovery cycle or API-side adapter is required.

Use bounded network operations and a bounded whole-scenario failure guard;
the limits detect hangs, not protocol latency. No sleeps, arbitrary failure
counts, retry-until-visible reads, global injection state or auto-reconnect.
No certificate fixture is needed for these plaintext loopback proofs.

## Engine provenance

The committed [tooling.json](../../bundles/self/tooling.json) and its ordered,
checksum-pinned patches describe reproducible source inputs: Brokkr base
`731189f9097308c01ae4f2b4c9caea5f41224844`, then
`0001-preserve-review-residual-facts.patch`,
`0002-preserve-review-scope-and-fixes.patch`, and
`0003-bind-verified-candidate.patch` under `bundles/self/tooling-patches/`.
Tooling.json alone owns exact patch paths, order and checksums; do not copy
or change that manifest for this correction. Its recorded source-composite
SHA-256 is `c972dde7785915743fb8d7365ad5bb5efecafd29be78520911f0cb4d74a75e5b`,
compiler is `rustc 1.98.0`, and binary SHA-256 is
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
The Brokkr source base is distinct from Swarmail's reviewed base `45d68df`.

Tooling.json's provenance declaration reports a release `--locked --offline`
build, core/runtime tests, formatting, workspace all-targets clippy and four
bundle compiles. Those are attributed historical claims, not fresh residual
execution receipts. The named
`.forge/epic192/engine-main-entry/build-receipt.json` is gitignored run-local
build evidence: it is not tracked or committed. A tracked reference does not
make its contents durable or available in another checkout. The chief checked
ignore/untracked status, not receipt contents or the original build. This
residual does not independently rebuild/reconstruct the source composite or
prove bit-for-bit reproducibility across environments. Declared reproducible
inputs and executed reproducibility proof remain distinct.

## Quickstart and verification

Run from this candidate's repository root. Before any execution receipt,
resolve current HEAD/tree, prove ancestry through reviewed base `45d68df`
and engine-pin commit `238372d`, and record the actual diff and command exits.
Input `11134f1` and historical cc30ea2/150c84a/e7d1d81 do not replace the
current HEAD binding. Do not modify Git metadata or certify another tree to
bypass a failure. Check `recipes` remains the tracked mode-120000 `bundles`
symlink. Hash the candidate-local engine before invoking it; use its resolved
absolute path in boxed checks. Missing/mismatched binaries are setup blocks,
never permission to borrow a sibling binary or substitute PATH.

```bash
set -euo pipefail
git rev-parse HEAD HEAD^{tree}
git merge-base --is-ancestor 45d68df HEAD
git merge-base --is-ancestor 238372d HEAD
brokkr_candidate_bin="$(pwd -P)/.forge/bin/brokkr"
brokkr_pinned_digest="$(jq -er '.brokkr.binary_sha256' bundles/self/tooling.json)"
test "$brokkr_pinned_digest" = ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f
printf '%s  %s\n' "$brokkr_pinned_digest" "$brokkr_candidate_bin" | sha256sum --check --strict -
"$brokkr_candidate_bin" compile --bundle bundles/self
```

The root-relative equivalent is `.forge/bin/brokkr compile --bundle bundles/self`;
boxed checks receive the absolute candidate-local path resolved above.
Keep network disabled (`CARGO_NET_OFFLINE=true`) and install nothing.
Run targeted tests and record nonzero executed-test counts:

```bash
cargo test --locked --test smtp_edges
cargo test --locked --test pop3
cargo test --locked --test persistence
cargo test --locked --lib pop3_degradation_reaches_http_healthz
```

Update the filter if the concrete name changes; a zero-test run is not proof.
Then execute every command in the authoritative
[house gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
including all specified `--locked` flags and exact 100/100/100 coverage with
zero misses and its selftest. Keep gate output/failures as run-local evidence.
Publish-guard and keyless seal proofs, license check and fail-closed native-tls
absence are mandatory. For absence, capture `cargo tree --locked -i native-tls`
status/output: success means dependency present and fails; a failure passes
only when it explicitly says the package did not match any packages.

Verify is offline and installs nothing. It validates shims, checks the
launched brokkr digest against tooling.json, and executes pinned bundle
proofs, including design-check selftest and delivery tamper-boundary proof.
Unavailable tools or caches are recorded as blockers, never skipped passes.
Design preflight is `bash bundles/self/scripts/design-check.sh check .` plus
its selftest; only the sequence's pinned final check owns the design verdict.
Full house gates prove the candidate, not implementation of still-unchecked
tasks. Independent full4 reviews and hermetic verify remain downstream.

Root must land #177 before final integration/CI, preserving its owned-event
fixes, and rerun required proofs against the integrated candidate. CI retains
chart/history/selftest, Docker, network RustSec, real port-80 and tag-only
rate obligations in their proper contexts. No seat publishes or tags.

## Risks

Residual-specific risks and mitigations:

- A valid digest can still name the wrong candidate: resolve HEAD/tree and
  reviewed-base/pin ancestry separately from hashing the local executable.
- Historical SHAs or preflights can appear current: inspect every surviving
  occurrence in context; preserve dated labels rather than globally replacing.
- A tracked receipt reference can imply committed evidence: check both
  `git check-ignore` and `git ls-files` and retain the provenance limits above.
- Broader cleanup can change six sensitive criteria: compare AC1–AC6 and
  T2–T7 with input `11134f1` and restrict the diff to these three documents.
- Full gates/providers can be unavailable: retain actual failures as run-local
  evidence, never weaken checks, install into hermetic verify or substitute
  a reviewer. The engine route's four-review cost remains accepted.

Inherited protocol-proof risks (preservation guidance, not new edit scope):

| Risk | Mitigation and accepted cost |
|---|---|
| Correct error masks lost envelope or stale state | Same socket, distinct envelopes/payloads, exact accepted-ID/raw/provenance and membership; longer explicit traces. |
| Malformed AUTH hides precedence or side effects | Fresh valid encoding and exact inbox inventory before/after refusal; keep invalid-base64 cases separate. |
| Reply trimming hides prompt/stale bytes | Bounded raw SMTP line, exact replies, POP3 NOOP immediately after TOP refusal; small fixture addition. |
| Health recovers before HTTP observes it | Hold the actual owner's retry operand after observed backoff; no repeated-error cushion. |
| Cleanup leaks held tasks or triggers | Scoped guards, release/abort cleanup and joins; use narrow cold-path harness only when needed. |
| Coverage pressure tempts broad migration | Inline stable proofs, document concrete cold arm, cfg(test) declaration and unchanged exact gate; accept extra review. |
| Pruning or parallel tests erase witnesses | Dedicated store for default routing, per-test inboxes, sufficient cap, private temp DB and ephemeral ports. |
| SQLite refusal reaches wrong stage | Real insert trigger and DATA terminator, exact cause, distinct same-session recovery; preserve existing restart proofs. |
| Partial QUIT proves only pending marks | Selective later-ID refusal yields committed prefix; HTTP/disk absence and RSET/TOP prove no resurrection. |
| Scope expands into runtime repair | Preserve real failing evidence and return to triage/full panel; do not silently change the contract. |
| Candidate or tool identity is unavailable in another seat | Resolve Git objects and exact binary digest in that execution environment before certifying any tree; missing tools/caches/providers remain blockers, never substituted passes. |
| #177 integration invalidates proof | Root-owned landing before final CI and rerun on integrated tree; retain event ownership changes. |
| Tests overclaim broader robustness | No new crash durability, exactly-once SMTP, webhook delivery, cap-concurrency or availability promise; existing tests retain those boundaries. |

## Failure-model limits retained from robustness

A commit may survive a lost SMTP acknowledgement; retry can duplicate mail,
and cancelling a session does not roll back an admitted blocking commit.
Fresh HTTP reads prove immediate visibility, not crash durability; existing
restart and CI stop/kill proofs retain that responsibility. Webhooks remain
best-effort after storage with no durable replay or acceptance backpressure.
Oldest-only pruning may legitimately evict earlier accepted mail, so these
witnesses use isolated stores and sufficient capacity. Existing hostile-byte,
cap/burst and TLS proofs remain intact; the string DATA helper is not a
byte-fuzzer, and these plaintext scenarios add no certificate fixture.
Subscribe-before-check and ingest-time extraction remain preserved boundaries.
Failure Drop cleanup releases/aborts; only the successful path explicitly
joins, so no failure-path join receipt or global lifetime bound is claimed.
