# Implementation plan: listener recovery

Implements [the spec](spec.md) and proposed
[0036](../../docs/decisions/0036-smtp-trickle-lifetime.md). This design handoff
does not implement the feature or accept the decision.

## Technical context

Single Rust crate; Tokio owns real TCP/TLS sessions in per-listener
`JoinSet`s. SMTP and POP3 already share `bound_io`; `run_on` starts both
and currently passes SMTP's idle value to POP3. Existing atomic degraded
flags feed the HTTP health handler. No dependency or persistent data
change is necessary. Follow the repository plan structure: technical
context, constitution, project structure, research, data model, contracts,
quickstart and risks; no extra design artifacts are needed.

## Constitution check

Accept still means stored: no new timer, cancellation, parse/commit reorder
or storage-boundary work. Subscribe-before-check, oldest-only pruning and
ingest extraction remain intact. Existing timeout/error and shutdown paths
are retained. This is a semantic clarification of accepted 0027, so 0036
stays proposed until the operator accepts it; historical text receives a
dated addendum, never a silent rewrite. 0034/0035 are untouched.

0020 governs placement by cold-path presence, and 0035 still checks the
actual build and expansion. Deterministic tests stay inline; public-wire
tests stay in integration suites. A concrete cold diagnostic may require
an owner-adjacent `*_tests.rs` declared under `#[cfg(test)]`; no mass moves,
feature-flagged seams, coverage exclusions or denominator workaround.
Real sockets, fresh stores/per-test inboxes, chaos 0/100, and explicit
rendezvous or deadlines replace guessed sleeps and shared global state.

## Project structure and files touched

| Path | Implementation work |
|---|---|
| `src/smtp.rs` | Relocate reap, observe accept polling outcomes, update stale timeout rustdoc; inline recovery/admission regressions and only necessary test hooks. DATA/commit code stays unchanged. |
| `src/pop3.rs` | Same reap/health rule, public `DEFAULT_IDLE_TIMEOUT` of 600 s; inline regressions. Existing serve signature stays. |
| `src/config.rs` | Public `Config::pop3_idle_timeout: Duration`, initialize from POP3 constant in `From<&ServeArgs>`; correct help; equality tests. No CLI argument. |
| `src/lib.rs` | Pass POP3's own field in `run_on`; retain existing cap and shutdown wiring. |
| `tests/smtp_edges.rs`, `tests/pop3.rs` | Replace retry-through-refusal recovery checks with first-connect assertions; retain full-cap refusals; POP3-only timeout override and independent-wiring regression. |
| `tests/common/mod.rs`, `tests/starttls.rs`, `tests/stdio.rs`, `tests/persistence.rs` | Add POP3 default to existing `Config` literals; retain TLS and persistence behavior. Audit other literals with `rg`. |
| `tests/config_cli.rs` | Default conversion and rendered-help contract assertions using the existing suite. |
| `tests/lifecycle.rs`, `tests/api.rs` | Reuse health precedence, drain and commit regressions; extend only if the current assertions do not cover AC4/AC5. |
| `README.md` | Accurate idle, trickle, health-recovery and drain descriptions; remove stale STARTTLS/no-timeout claim. |
| `docs/decisions/0027-living-listeners-bounded-sessions.md` | Dated clarification that identifies superseded clauses and links to proposed 0036; retain historical ruling. |
| `docs/decisions/README.md` | Add proposed 0036 row during implementation. |
| `tests/listener_contract_docs.rs` | Small documentation contract regression for authoritative README/addendum/decision text, including #80 red/green. No framework dependency. |
| `specs/listener-recovery/{spec,plan,tasks}.md`, `docs/decisions/0036-smtp-trickle-lifetime.md` | Exactly the four design artifacts; later task evidence/clarifications stay in their owning documents. |

## Research and decisions

The baseline has one reap before accept, success-only flag clearing,
SMTP's 300-second field wired into POP3, and complete-line DATA reads that
each renew the operation timeout. Existing cap regressions retry refusals
and hide #77; POP3's shortened timeout test changes the SMTP field and
hides #79. Both config help and SMTP rustdoc retain pre-0027 claims.

The panel synthesis and alternative costs are normative in spec.md.
Specifically: move one reap, clear on polling outcomes, split the idle
field, keep HTTP admission closure, and document lifetime exposure. Do not
carry over the robustness position's assertion that EOF is task completion
or its claimed strict DATA cutoff from a zero-duration timeout.

## Data model and algorithm

No stored model changes. Runtime state stays the existing `JoinSet`, flag
pair and stop watch; add only `Config::pop3_idle_timeout` and its default.
`Config` currently has no `Default` implementation: test actual
`Config::from(&ServeArgs)` rather than inventing a nonexistent default API.

For each accept loop, replace the real `listener.accept().await` inside
the existing selected future with a `std::future::poll_fn` wrapper around
`listener.poll_accept(cx)`. On `Pending`, clear the flag and return Pending
with Tokio's registered waker intact. Keep the successful-result clear and
error-result set/backoff in their existing owning arms. Injection remains
before the real poll. Never clear before checking an injected or real
error; no backoff-expiry store. No errno taxonomy or shared supervisor.

Move the entire `try_join_next` loop, including its existing test counter,
from the loop head to after the successful accepted-result match and before
`sessions.len()`. Keep the bounded full-cap write and shutdown drain.
Revise comments explaining the old placement. No separate semaphore or
live counter is added.

Expose the POP3 default constant for public config-literal callers. Set
`Config::pop3_idle_timeout` in CLI conversion and every literal; pass it in
`run_on`. Replace `c.smtp.idle_timeout = ...` in the POP3 expiry regression
with `c.pop3_idle_timeout = ...`. Do not split the cap or change SMTP's
DATA timer. Document the public-struct source compatibility cost.

## Contracts and test design

**Admission:** test real serve loops with a small cap. A per-test,
`#[cfg(test)]` observer may capture the `AbortHandle` returned by
`JoinSet::spawn` without removing the entry. Use its `is_finished()` state
to establish runtime task completion, and an observed pending-accept
rendezvous to ensure completion occurs while accept is parked. Do not use
a session's final notification or EOF as equivalent to `is_finished()`.
Then offer exactly one new TCP connection and check its first greeting.
Cover one freed slot, a completed full burst and genuinely full refusal for
both protocols. The seam must not reap, alter the count or admit a client
itself. Keep branch-stable tests inline; any required conditional timeout
diagnostic belongs in a justified adjacent harness under 0020. Existing
integration tests use QUIT plus EOF and one connection as smoke coverage;
the synchronized inline/harness test supplies the deterministic red proof.

**Health:** extend both inline injection tests: observe true, allow the
actual backoff to finish, observe false before any connect, then exchange
real protocol commands. Use per-runtime injection and an acknowledged
poll-boundary observer for immediate-error continuity; a periodic HTTP
sampler cannot prove the absence of a transient false store. Test pending,
successful and error outcomes plus shutdown during backoff. Existing health
router tests pin HTTP status and stopping precedence. For drain, choose
the documentation alternative; do not add an unsynchronized assertion that
a TCP connect fails the instant stop is signalled.

**Defaults/wiring:** pin SmtpConfig's 300 s and CLI-converted Config's 600 s
using equality. Assert each override leaves the other field unchanged.
Drive `run_on` with a short POP3 timeout and a long SMTP timeout: POP3 must
expire before the existing helper deadline, and SMTP must still answer a
command at that observation. Use the reciprocal setup to show SMTP expiry
does not terminate POP3. This exercises actual argument wiring, not merely
unused constants. Retain the separate SMTP mid-DATA zero-store proof.

**Documentation:** read README, 0036 and the clearly delimited dated 0027
clarification from `tests/listener_contract_docs.rs`. Require the explicit
no-lifetime posture, the complete-line/within-read distinction, and costs
specified in AC8. Reject the former unqualified assertion in current
README; in 0027 inspect its controlling clarification while allowing the
historical sentence to remain. Require conditional stopping language and
the independent idle values. For negative proof, carry this test to the
reviewed docs or restore those docs temporarily in an isolated worktree:
it must fail because the current contract is missing/wrong. Do not claim a
runtime red/green change for #80. Avoid a new documentation-check framework.

Collect a red/green ledger under ignored `.brokkr/state/` for each child.
Use the current pre-change baseline (record its SHA) and isolate the known
old behavior with a minimal revert/mutant where the new field or seam is
needed to compile. A legacy timeout wiring mutation must fail an assertion,
not compilation; old reap placement and success-only recovery must fail
their targeted assertions. Restore all mutants before the full gates.

## Quickstart and verification

Work on a feature branch, then implement the ordered tasks. Run targeted
suites as their changes land and all required gates before delivery:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --locked --lib
cargo test --workspace --locked --test smtp_edges --test pop3 --test config_cli --test listener_contract_docs
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
cargo deny check licenses
```

Coverage must be exact 100/100/100, without a TMPDIR override. Preserve the
helper deadline, host mawk compatibility and existing ANSI discipline.
A design-stage baseline pass is not implementation proof.

The design seat stages only the three spec files and 0036, signs an
imperative commit explaining why, and never pushes. Use the configured key
or the intake's fallback key; disclose any permitted unsigned fallback in
the message. If `.git` is read-only, preserve the four reviewable files and
report the blockage rather than claim a commit. Subsequent implementation,
verification, review and ship follow the framed branch/PR/17-check workflow;
this design seat neither ships nor closes issues.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| A complete-line trickler exhausts slots or retains DATA memory indefinitely. | Explicit proposed residual: up to 256 SMTP slots and about 12.5 GiB payload before overhead. No availability-hardening claim; operator can return 0036 for a separately proven bound. |
| Pending accept is mistaken for proof of descriptor recovery. | Define health as observed accept state, not an fd probe. Immediate errors stay red; later failures can set red again. |
| EOF-based regression flakes or passes the stale-count bug. | Observe actual task completion and parked accept via an exercised test-only seam; one new connection, no refusal retry. |
| Reap relocation retains completed entries during idleness. | Retention is bounded by the cap; next successful accept reaps and shutdown drains. Session resources have already been dropped. |
| A config constant passes tests while production uses SMTP's field. | End-to-end asymmetric overrides through `run_on`, default conversion equality, and a wrong-wiring mutant. |
| Added public field breaks Rust struct literals. | Update all repository callers; document the required field/default for embedders. CLI flags stay stable. |
| Historical and current timeout claims contradict each other. | Explicit dated controlling addendum and proposed link; documentation test checks current authoritative text without erasing history. |
| Test seam or cold diagnostics evade exact coverage. | Apply 0020 by cold-path presence; all hooks have installers and remain cfg(test)-only. 0035 and gate scripts stay unchanged. |
| Commit unavailable in a restricted workspace. | Preserve exact artifact set; report `.git` write failure in result notes. No alternate Git metadata or claim of committed delivery. |
