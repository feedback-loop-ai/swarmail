# Implementation plan — SMTP and POP3 cleanup

Implements [spec.md](spec.md). No semantic amendment is proposed.

## Technical context

One Rust crate using existing Tokio, rustls and protocol helpers. No new
packages, production types, configuration or schema. The repository supplies
a spec template but no plan template; this plan supplies the required technical
context, constitution check, structure, research, data model, contracts and
quickstart directly. Implementation stays in the isolated candidate with its
own target outputs. The engine owns the shared journal; do not manufacture it.

## Constitution check

Preserve 0001–0005, 0027/0036 listener ownership and health transitions, and
0046 transport/identity rules. No timer crosses insertion, no altered pruning,
no query-time extraction and no watcher reordering. Apply 0020 to actual cold
paths, keep inline code measured and test seams cfg(test)-only. Preserve
0035/0050 compiler-based coverage, exact 100/100/100, pinned proof scripts and
shim verification. Keep all gates in the single house-rules inventory; no
copied inventory here may replace it. No dependency allowlist or bundle edits.
0056 requires unsigned author commits; 0057 and retained recipe commits govern
triage, providers, review and candidate-local engine identity.

## Research and resolved questions

Source inspection at the baseline finds `auth_plain_inbox` copying its authcid
into a Vec, LOGIN passing its decoded Vec by value, and `auth_inbox` taking
ownership before validation. The correct local boundary is a byte slice plus
an owned validated result. Source structure, existing hostile-input tests and
exact routing tests prove this change; global allocation counts do not.

`write_line` already bounds write_all and flush together. Remove only the
outer cap-path `bound_io`; keep accepted-socket ordering, reply string and
idle duration. Do not apply this edit to POP3's different write owner.

Both protocols already expose owner-scoped `ListenerProbe` observations:
`accept_error`, `backoff_pending`, `retry_ready` and `observed_listener`.
Setting the atomic fault alone cannot wake a parked accept: a second TCP
connection provides that wake. Held retry readiness establishes a stable
interval for serving the original session. Existing observed-listener helpers
may need a narrow fixture argument/return adjustment to retain the same Store;
no production interface change is needed.

The counted-error seam is thread-local in each owner. Current callers are two
SMTP tests, two POP3 tests, and the API 1000-error health test; re-run the caller
search at implementation time. A previous-value guard fixes their cleanup
without migrating existing tests to another injection mechanism. New survival
tests use the owner-scoped probe, not the thread-local count.

The sole STARTTLS sibling test has a handwritten elapsed-deadline assertion
inside its completion loop. Its comment and 0046 describe cold-path placement;
the inline POP3 twin suggests a candidate form but proves no measured result.
Use a Tokio timeout around the actual completion-predicate wait, with result
assertion after cleanup. Preserve the ten-second observation budget as the
initial fixture setting and a separate generous whole-scenario bound (for
example 30 seconds); adjust only from measured fixture behavior, never to turn
a failing mutation green. The fixture must still cause backpressure and
observe actual session completion before new admission.

## Project structure and files

| Path | Planned work and proof |
|---|---|
| `src/smtp.rs` | Borrowed AUTH helper and two callers; single cap-refusal timeout; scoped injector and local callers; stateful survival test; inline STARTTLS regression; narrow obsolete pub(super) helpers after caller audit. |
| `src/smtp/protocol_edges_tests.rs` | Relocate its sole test using the branch-stable form, then remove the empty harness and parent declaration. |
| `src/pop3.rs` | Scoped injector and local callers; authenticated snapshot survival proof using existing probes. |
| `src/api.rs` | Bind SMTP injection guard through the health test; ensure task cleanup before restoration and remove manual zero reset. |
| `src/store.rs` | Remove only the duplicate cfg(test) on persist_tests. |
| `tests/smtp_edges.rs`, `tests/starttls.rs`, `tests/pop3.rs` | Reuse real-protocol coverage; extend only a missing AC assertion without duplicating existing matrices. |
| `src/smtp/*_tests.rs`, `src/pop3/*_tests.rs` (conditional) | A narrowly declared sibling only for a demonstrated cold unwind/cancellation diagnostic; document why 0020 requires it. Never hide the requested STARTTLS move or unrelated inline tests there. |

No edits to store persistence logic, protocol configuration, public API,
accepted decision bodies, dependencies or coverage scripts are planned.

## Implementation approach

1. Verify the candidate engine digest and recipe ancestry before any provider
   or engine action. Inventory actual helper callers and preserve a clean
   baseline. Establish existing targeted test names before changes.
2. Settle STARTTLS relocation early because its measured placement is the
   highest design uncertainty. Relocate, replace the handwritten deadline loop
   with bounded completion observation, retain unread-client ownership and add
   finite setup/admission bounds. Keep generated TLS trust. Ensure test-owned
   server/flood tasks are stopped and joined after success or timeout. Keep
   abort-on-drop ownership for unwind/cancellation; an outer timeout alone
   does not cancel spawned tasks. If necessary drive the body in a caught
   outcome so async joins happen before rethrowing failure; do not introduce a
   general task-management library. Inspect exact coverage and run the
   unbounded-failed-close behavioral mutation. If an unavoidable cold path
   remains, return the concrete conflict; do not mark AC1 done.
3. Add one small cfg(test) guard per protocol. Save `Cell::replace(count)`;
   Drop restores the saved value. A `PhantomData<Rc<()>>` or equivalent makes
   thread affinity structural. Mark must_use, expose only what API tests need,
   bind each returned guard by name, and keep it outside Send spawned futures
   on the installing current-thread runtime. Normal task stop/abort plus join
   precedes restoration. Test all exit modes; use only lexical LIFO nesting.
4. Add deterministic survival regressions. Start real sessions and retain
   their state, arm the owner-scoped fault, connect the poke client, wait for
   observed backoff Pending, and assert retry still false. Complete the
   original SMTP transaction/POP3 snapshot queries before readiness release.
   Retain Store access for exact data assertions; release and greet the queued
   or next client, observe health recovery, then stop and join the owner.
   Every peer await and observation has a finite failure bound. Cleanup must
   release latches and terminate/join tasks before asserting a saved failure.
5. Borrow AUTH bytes with `std::str::from_utf8`, unchanged validation and
   `to_owned` only on success. Remove the cap's outer timeout and retain its
   inner write_line deadline. Remove the extra store attribute. Inspect every
   changed branch against the accepted contract and reuse its focused proofs.
6. Collect mutation evidence, restore the correct tree, run all required gates,
   then pass honest local/CI status to verification and protected review.

## Data model and contracts

No persistent or public data model changes. Session identity remains owned
String state; the input slice cannot outlive its decoded buffer. Invalid AUTH
changes no identity. POP3 retains PASS-time snapshot numbering and fetches
from the store; the survival test seeds stable data without concurrent removal.

Test-only guard state is a saved usize and a thread-affinity marker. Drop has
no I/O, await or panic path. It restores a predecessor, not always zero; there
is no claim of isolation between concurrently overlapping independent
installations on one thread. A listener-local probe remains the owner for
held-backoff tests. Existing consumption and health semantics remain intact.

Existing failure boundaries remain: TCP accept/354 are not accepted mail;
commit-before-lost-reply can produce retries/duplicates; memory-only restart
loses its data; a 250 is not a retention lease against later lawful pruning.
Webhook fan-out remains bounded and nonblocking without durable replay.
Complete-line trickling and progressing POP3 replies can retain slots and
buffers indefinitely under existing policy. This cleanup changes none of them.

## Quickstart and proof receipts

First verify the exact executable
`/home/valentin/source/feedback-loop-ai/swarmail-epic192-delivery/NEXT-189-luna1/.forge/bin/brokkr`
against SHA-256
`e525babf643d6a1e905ce6cf108a7f624895a12adaa2677497d9062c60a4d86c`.
Missing/mismatched means stop before model attempts. Use it for compile and
required engine checks; never a sibling executable or PATH substitute.

Targeted starting commands (all use the candidate's own build output):

```bash
cargo test --locked --lib smtp::tests
cargo test --locked --lib pop3::tests
cargo test --locked --lib a_degraded_smtp_listener_answers_503_naming_it_on_the_wire
cargo test --locked --lib store::persist_tests
cargo test --locked --test smtp_edges --test starttls --test pop3
```

Run applicable new cold-path harnesses by their actual module names too.
For each of three mutations (unbounded failed TLS close, SMTP abort on accept
error, POP3 abort on accept error), record exact test, diff, expected failure,
actual runtime failure within the independent bound, and restored green run.
Mutate one behavior at a time; compile errors and test filters matching zero
tests are not proof. Remove mutants before final coverage/gates.

Execute the complete current
[house-rules gate inventory](../../docs/house-rules.md#gates--part-of-every-change),
including coverage selftest, publish/seal proofs, locked commands, licenses,
pinned compile and fail-closed native-tls absence. The verifier runs pinned
scripts and checks fixed shims; candidate-local invocations do not replace that
proof. Keep run-local logs and coverage report/summary paths with actual exit
statuses. Do not install tooling in verify or claim missing execution passed.

External evidence to name as pending until actually obtained: chart contract
and history/selftest, real-Docker persistence/context/volume smoke, network
RustSec advisory audit, privileged port-80 bridge exchange, and tag-only
release-rate-5k where applicable. The design author validates artifact shape,
relative links and HEAD identity; the sequence's pinned check rules design.
Implementation and issue closure remain separate obligations.

## Risks

| Risk | Mitigation and remaining cost |
|---|---|
| STARTTLS's inline body has a real cold branch or becomes vacuous. | Inspect measured lines/branches/functions, retain open peer and actual completion, require bounded behavioral red. A remaining genuine cold path blocks AC1; no gate exception. |
| Kernel buffers or scheduler load defeat the stall fixture. | Generated TLS, constrained receive buffer, substantial flood, finite independent deadline and actual task observation. Measure before adjusting fixture; never substitute sleeps or close the client early. |
| Timeout leaves child tasks detached. | Explicit ownership and abort-on-drop; capture outcomes and join before assertions on normal/error paths; prove cancellation cleanup after installation. |
| Guard restores another thread's state or a live old listener consumes new errors. | Non-Send guard on current-thread scope, named bindings, lexical nesting only, listener teardown/join before normal restoration and failure-safe ownership. |
| Survival test accidentally proves only recovery. | Pre-fault envelope/authenticated snapshot; held Pending barrier; same-connection commands/data before release; abort-session mutation must fail. |
| AUTH changes identity or exposes credentials. | Strict UTF-8, unchanged validator/framing and exact padded-inbox routing; refusal/no-effect matrix and TLS-epoch pins. Accept LOGIN's possible copy. |
| Both cap deadlines are removed or flush becomes unbounded. | Retain write_line owner and its write-plus-flush proof; real cap/no-store/admission test plus primitive expiry tests. |
| Cleanup expands into availability/storage policy. | Preserve named residuals, JoinSet ownership and existing timeout distinctions; one attribute is the only store edit. |
| Design evidence is mistaken for delivery proof. | Implementation tasks stay unchecked until executed; record actual local statuses and pending external CI; design commit contains exactly its three artifacts. |
