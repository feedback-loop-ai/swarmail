# 0115 — Trial separation and observed concurrency

- **Status:** proposed — only the operator accepts.
- **Date:** 2026-10-24 (design on record, branch
  `epic192/issue199-current-0058` at 518c52d, where it was numbered 0062);
  carried onto main a3cff51 on 2026-10-31 for issue #221.
- **Issue:** #221 (split from #199 by operator ruling, 2026-10-24), Epic #192.
- **Design:** [spec](../../specs/trial-isolation/spec.md),
  [plan](../../specs/trial-isolation/plan.md),
  [tasks](../../specs/trial-isolation/tasks.md).
- **Amends if accepted:** [0055](0055-agent-eval-harness.md) §§6–9 and
  accepted [0076](0076-sequential-eval-trials.md) §§1, 2, 5 and 6 — solely
  the execution-boundary/support and result-observation contracts below.
- **Preserves:** 0001–0005, 0055's isolation requirement, app-state
  authority, task catalogue, failure taxonomy/precedence and within-trial
  unconfined limits; accepted 0071, 0073, 0075 and 0076 until the operator
  accepts this record; 0098/0099 review routing; root coverage and
  dependency policy.
- **Number:** 0115 follows the highest record on main, 0109 (accepted
  2026-10-31). 0062 is occupied by *RPC edge contracts*; 0106–0108 are
  unreferenced gaps and are not reused.

## Current integration — 2026-10-31, carried onto main a3cff51

The ruling below is the 2026-10-24 text of the design on record, with its
number and links repointed. Main has moved since it was written; this dated
section governs where the two disagree, following the 0076 precedent of
carrying accepted history and integrating by dated addendum.

1. **Accepted 0076 is what keeps concurrency off.** 0076 §1 admits one
   trial per uid, §2 refuses `--concurrency` other than 1, and §5 requires
   integer `concurrency: 1` on every row. Under accepted 0045 §8 no production
   path may overlap trials, widen those values or read a "proof record" that
   unlocks overlap while this record is proposed. Concurrency returns only by
   operator acceptance of this record followed by the live proof named in the
   spec, never by a file the runner trusts.
2. **The schema paragraph (§4 below) is stale as written.** Rows are already
   schema 2 with required `concurrency` (accepted 0071 and 0076), and schema 3
   is occupied by comparison rows. If accepted, the closed `execution` object
   lands as a new row version chosen then, with the same fail-closed reading
   of older rows; nothing of it is activated by this record alone.
3. **Fresh HOME (§3 below) is shipped.** 0076 §4 gives every child a closed
   `RuntimeDirs` map; the boundary's private `/tmp` carries it without new
   code. §3 needs no implementation beyond keeping that map inside the
   boundary.
4. **Paths and seats.** Run-local evidence lives under `.brokkr/state/`, not
   `.forge/`. The review panel is the accepted 0098/0099 one for this lane.
   The #194/#196 reservation language describes branches that have since
   landed (0073, 0075); the obligation that survives is "do not refactor the
   cancellation or environment blocks incidentally".
5. **The capability probe is advisory, the launch is authoritative.** The
   branch recorded `EPERM`; the 2026-10-31 seat observed `ENOSPC` from
   `unshare -Urn true` while already inside a user namespace
   (`/proc/self/uid_map` reads `1000 0 1`) with
   `kernel.apparmor_restrict_unprivileged_userns=1`. These are different facts
   and one may be transient. The runner records the exact operation and errno
   of its own failed launch and never caches a verdict.
6. **This run prepares; it does not activate.** The design is implemented
   behind a hidden `trial --isolated` flag that refuses by name wherever the
   boundary is unavailable, never falls back to running unisolated, and leaves
   the public `--concurrency` refusal untouched. The hostile live proof (spec
   AC-L1..L4) must pass on a host or CI that permits unprivileged user
   namespaces before any activation is even proposed. Provider egress (spec
   AC-L4) is an open question for the operator, below.

## Context and evidence limits

The local issue snapshot alleges same-uid prompt discovery followed by
victim MCP mail retrieval and fixture confirmation, making an idle victim
pass. The two previous runs supplied no live isolation result: one stopped
at chief CLI transport, one at evidence access. No live leak was established
by the design on record; the spec of this run adds an always-on test that
establishes it on every host (spec AC-S1).

The existing cooperative proof does not test this attack. Private HOME,
random scratch names, modes under one uid, or fixture-only bearer checks
do not close all file/process/network routes: both trials are the same
principal to the kernel, and process lifetime is not trial lifetime. The
issue's documentation-only alternative is what accepted 0076 shipped; it is
honest about the limit and is not isolation.

Driver egress and production boundary feasibility remain unproven.

## Proposed ruling

1. Strengthen enforcement of the existing cross-trial requirement with one
   whole-trial boundary around the runner worker, Swarmail, fixture and all
   agents. The selected feasibility target is a Linux private user/mount/
   PID/network (and IPC) namespace worker with minimal immutable inputs,
   private state/proc/HOME, closed escape descriptors and dropped privileges.
   Distinct host uids are not mandatory if same-host-uid denial is
   demonstrated. Namespace flags alone are insufficient evidence.

2. Own-trial services and the external command's required provider transport
   must remain usable without exposing host/sibling services. A loopback-only
   prototype is not a completed repair. No unisolated or host-network fallback.
   An unavailable boundary refuses before arbitrary agent launch as
   harness_error. If this restricts formerly supported Unix execution or
   driver inputs, acceptance of that exact restriction is required before
   activation. No new launcher, privileged service, dependency/license
   expansion or reduced egress contract is implicitly authorized.

3. Fresh HOME applies to sequential and simultaneous trials. Initialize it
   at the single-threaded worker boundary, preserving the environment block's
   ownership (see Current integration 3).

4. Amend 0055 §9's fixed producer contract only after acceptance. Retain all
   business fields and confinement:"unconfined". Add the closed
   execution/boundary and controller-scoped concurrency object defined in the
   linked spec. Emit it on every new direct, matrix, refusal and artifact row.
   Strictly read historical rows as unknown observation, without fabricated
   migration. All producers, validation and report consumers change together.
   Sidecars before acceptance preserve proof but do not fulfill the final
   every-row contract (see Current integration 2 for the version).

5. Count actual worker lifetimes observed by one trusted controller, not
   requested parallelism or agents. Unknown outside-controller concurrency
   is explicit. Not-started is zero/null; incomplete observation is not
   an isolation-success claim. The proof observes two live trials and six
   cooperative agents as well as an active poacher against a live idle
   victim, with real SMTP/MCP/fixture paths and no sleep-based synchronization.

6. Retain the meaning of unconfined: the interim opaque driver and
   within-trial admin separation are not malicious-driver authentication or
   proof of model honesty. Cross-trial access denial is a separate bounded
   property. Keep existing app-state scoring and infrastructure precedence;
   do not repair false passes by assigning a different scorer to idle agents.

## Question for the operator — provider egress

A private network namespace has loopback only, so a real model driver cannot
reach its provider. Nothing in this record authorizes a fix; the chief's
recommendation, for the operator to accept, reject or replace, is a
controller-side CONNECT relay with an explicit host:port allowlist, reached
over one inherited socket, which keeps TLS end to end, needs no external
binary or DNS inside the boundary, and is provable by a negative test (the
relay refuses the host's fixture port and any sibling address). The
alternative, a user-mode network stack such as `pasta` or `slirp4netns`, adds
an external launcher and exposes host addresses by default. Until the
operator rules, `--isolated` refuses every `wrapper_profile`, so no paid
driver can silently score a provider outage as a model `gave_up`.

## Operator boundary and evidence-limited outcome

Compatible proof, tests and experiments may proceed under accepted authority.
Activating a new row version, overlapping trials, or a conflicting
platform/driver restriction must wait under 0045 §8. This proposal neither
accepts itself nor authorizes a weakened concurrent guarantee. Passing design
validation is not acceptance.

If feasibility fails, preserve the exact command/errno, platform/privilege
requirements, binary/source hashes, live attack observations if available,
remaining access routes and required provider transport. Refine this proposed
record with the minimum concrete operator choice: provision the demonstrated
missing boundary capability, authorize a separately specified support-contract
change, or leave the feature pending. Do not recommend serialization as
fulfilling concurrency, assert an untested mechanism works, or claim a
documentation limitation closes #221. No live-baseline result may be invented
to make the proposal more persuasive.

## Reconciled alternatives and consequences

Adopt simplicity's whole-trial unit and bounded single mechanism. Reject
robustness's mandatory distinct-host-uid provisioning/lease infrastructure:
the cost is stronger same-uid proc/descriptor/mount escape testing and the
possibility that the chosen candidate must stop. Adopt robustness's complete
file/process/network and failure checks plus every-row observation honesty.
Reject partial authentication, random-name secrecy and a global sequential
lock as isolation; they do not establish the required concurrent boundary.

An explicit new row version replaces the tempting unversioned field addition.
Cost: operator acceptance, consumer migration and mixed-version tests.
Explicit capability refusal replaces silent fallback. Cost: currently usable
but unsupported environments may refuse after an accepted activation, and
full driver egress may prevent activation at all.

No runtime change, reproduction, complete verification, five-role review,
model measurement or issue closure is claimed by this proposed record.
The plan records the design-seat preservation checks and outstanding gates.
