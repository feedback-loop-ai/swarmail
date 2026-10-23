# Implementation plan: session edge proofs

Implements [spec.md](spec.md); ordered proof obligations are in [tasks.md](tasks.md).

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

Use 0020 placement: public wire tests in `tests/`, branch-stable positive
library assertions inline and measured. Only a concrete failure-only arm
warrants the narrow declared `src/pop3/health_tests.rs` alternative. Record
that arm and move only the test/helper needing it; no broad module migration.
All seams remain cfg(test), privately owned and exercised. The exact
coverage gate still measures all production code and inline tests.

A failing behavioral witness requiring production repair returns to triage
with evidence before any repair; a conflicting accepted ruling additionally
requires an explicit proposed amendment and operator decision. Full4 remains
selected under the existing delivery policy; this document dispatches none.

## Project structure and files touched

| File | Implementation scope |
|---|---|
| `tests/smtp_edges.rs` | Extend sequencing and valid transaction-AUTH tests; add focused DATA-chaos, SQLite DATA-refusal and PLAIN-cancellation flows (AC1, AC3–AC5). |
| `tests/pop3.rs` | Extend existing selective partial-QUIT/RSET flows with TOP/refusal/NOOP and exact surviving-content witnesses (AC6). |
| `tests/common/mod.rs` | Add at most the needed bounded, untrimmed single SMTP reply reader; reuse existing server, raw HTTP and SQLite trigger helpers. Preserve all #177 helper timeout ownership. |
| `src/pop3.rs` | Only cfg(test) health proof and minimal local helper/wiring using its private ListenerProbe (AC2); optional cfg(test) declaration for the cold-path alternative below. |
| `src/pop3/health_tests.rs` | Conditional, narrow alternative only when concrete failure-safe cleanup/diagnostics contain a cold arm. Do not create merely for private seam access. |

`tests/persistence.rs`, `src/smtp.rs`, store/persist/API runtime code,
existing POP3 progress/maildrop harnesses, bundle, locks, dependencies and
product documentation are read-only preservation references. No new test
framework or public fixture API is needed.

## Research and evidence baseline

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

These are inspected coverage gaps, not observed production regressions or
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
failure, use the narrow harness allowed above. EOF is not task completion.

## Contracts and implementation approach

1. Add an untrimmed SMTP single-line reader delegating to existing
   `read_line_named`, without altering reply/reply_multi callers. Use it for
   exact prompt and fixed SMTP reply bytes. Drive DATA explicitly where an
   exact 354 and final acceptance/refusal matter. Parse the entire success
   prefix plus one nonempty ID, then fetch `/api/v1/messages/<id>` and its
   `/raw` immediately; assert 200, exact metadata/raw and inbox membership.
   Account for `SmtpConn::send` appending CRLF and dot transparency once.
2. Extend AC1 in its current sequencing test. For AC3 use two explicit tests:
   acknowledged HTTP DATA-chaos configuration at probability 100, delay 0;
   and a tempfile persistent server with `refusing_trigger(db, "insert")`.
   Assert empty pre-recovery membership, remove the cause, continue on the
   same socket with a distinct envelope and witness exactly the accepted ID.
   Reuse trigger remove/Drop; do not replace commit failure with a busy timer.
3. Add AC4 with a dedicated store, literal prompt/cancel/NOOP, default mail,
   later valid AUTH and named mail. Repair AC5 with `base64_of` over known
   three-field credentials and real inventory comparisons; keep malformed
   input tests separate. Successful AUTH after AC5 is unnecessary duplication
   of AC4; original delivery and unchanged inventory prove its own contract.
4. Extend existing partial-QUIT and `rset_never_resurrects_a_committed_delete`
   tests for AC6. Add marked TOP before QUIT and gone TOP before/after RSET,
   exact partial-QUIT error and a NOOP after every refusal. Preserve existing
   HTTP/disk witnesses and exact suffix content; the external-clear test
   remains a separate current-store-absence path.
5. Add `pop3_degradation_reaches_http_healthz` beside the POP3 probe. Bind
   real POP3 and HTTP listeners, fresh store/shared POP3 flag, false SMTP
   flag and live shutdown watch. Complete POP3 greeting/NOOP and healthy
   HTTP request first. Hold retry_ready false, arm accept_error, offer a
   connection, await backoff_pending, then GET health. Compare status 503
   and complete parsed JSON equality; do not pin member order or dynamic
   healthy uptime. Release the hold, close clients, signal shutdown and join.
   No new recovery cycle or API-side adapter is required.

Use bounded network operations and a bounded whole-scenario failure guard;
the limits detect hangs, not protocol latency. No sleeps, arbitrary failure
counts, retry-until-visible reads, global injection state or auto-reconnect.
No certificate fixture is needed for these plaintext loopback proofs.

## Quickstart and verification

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
| #177 integration invalidates proof | Root-owned landing before final CI and rerun on integrated tree; retain event ownership changes. |
| Tests overclaim broader robustness | No new crash durability, exactly-once SMTP, webhook delivery, cap-concurrency or availability promise; existing tests retain those boundaries. |
