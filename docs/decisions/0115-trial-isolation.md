# 0115 — Trial separation and observed concurrency

- **Status:** proposed — only the operator accepts.
- **Date:** 2026-10-24 (design on record, branch
  `epic192/issue199-current-0058` at 518c52d, where it was numbered 0062);
  carried onto main a3cff51 on 2026-10-31 for issue #221; revised
  2026-11-01 (the boundary is bubblewrap; main a4f552d merged in).
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

## Revision 2026-11-01 — the boundary is bubblewrap

Operator direction of 2026-11-01, recorded here while the record stays
proposed. This dated section governs where it disagrees with the 2026-10-24
ruling and the 2026-10-31 integration below; everything it does not name
stands.

**Evidence.** The 2026-10-31 worker built the namespaces itself (`clone`
with namespace flags, `uid_map`, `pivot_root`). On this host it refuses
`boundary_unavailable:identity_map:EACCES`: `kernel.apparmor_restrict_unprivileged_userns=1`,
the Ubuntu-family default most operators will hit. bubblewrap 0.11.1
(`/usr/bin/bwrap`) traverses that restriction through its AppArmor profile
`bwrap-userns-restrict`. The dispatcher's probe outside any seat, with
`--unshare-user --unshare-pid --unshare-net --unshare-ipc
--die-with-parent`, gave uid 1000, an empty private `/tmp`, loopback only
and pid 2; processes inside cannot create further user namespaces. In the
delivery seat the same binary refuses with `ENOSPC` (the seat is already
inside a user namespace), which is the named refusal the seat must show.

**Ruling 1, restated.** The whole-trial boundary is built by invoking
`/usr/bin/bwrap` (bubblewrap, at least 0.11.1) from the trial's controller
with the fixed argv the spec records, not by a namespace worker the harness
builds; the raw `clone`/`unshare` worker is retired and deleted. The
boundary's contract is unchanged: one boundary around the runner worker,
Swarmail, the fixture and every agent; private `/tmp`, HOME and `/proc`;
escape descriptors closed; an unavailable boundary refuses by name before
any server or agent exists; no unisolated fallback. bwrap is the same kernel
boundary with a maintained, distribution-integrated launcher; it is not
claimed to be stronger than one, and its trust is named, not hidden.

**Locating and pinning.** The path is the constant `/usr/bin/bwrap`: no
`PATH` lookup, no environment or flag override, because an override is the
lever that becomes an unisolated fallback. The floor is `0.11.1`, the only
version the boundary has been observed to work on; every option used is
older, so the floor is a compatibility statement and not a security
attestation of a distribution's patch level. Lowering it is an ordinary
change that carries a live proof on the lower version. Both the version and
a bounded probe with the real argv are checked on every launch and never
cached. The closed refusal set is `boundary_unavailable:platform`
(non-Linux; **macOS refuses by name**, bwrap is Linux-only),
`boundary_unavailable:bwrap_absent`, `boundary_unavailable:bwrap_version`
and `boundary_unavailable:bwrap_probe`. The errno-name vocabulary of the
worker is withdrawn; bwrap's own stderr is forwarded beside the row.

**Host prerequisite, for the operator.** bubblewrap is a **binary host
prerequisite, not a crate dependency**: LGPL-2.0-or-later, executed as a
separate process, never linked, vendored, downloaded or placed in
`Cargo.lock` or `evals/Cargo.lock`; `deny.toml` does not change and no Rust
dependency is added without a separate operator ruling. Running an isolated
trial requires Linux, `/usr/bin/bwrap` at least 0.11.1 (on Ubuntu, the
`bubblewrap` package; 24.04 ships 0.9.0, which the floor refuses until a
proof lowers it), and a host policy that lets bwrap create user, PID,
network and IPC namespaces (Ubuntu's `bwrap-userns-restrict` profile, or
`kernel.apparmor_restrict_unprivileged_userns` lowered). Where any of these
is missing the trial refuses by name and does not run. Accepting this record
is accepting that prerequisite. Ruling 2's sentence "no new launcher … is
implicitly authorized" is revised to: bwrap is the one launcher this record
asks the operator to admit.

**Network.** `--unshare-net` gives each trial a private loopback; Swarmail
and the fixture listen there, and neither the host's nor a sibling's
services exist there. `--share-net` is rejected outright: it would put the
trial's own services on the host loopback. **Provider egress for real
agents remains the open operator question** below, reframed: the design
cannot give it without new machinery, and this record does not guess one.
Until the operator rules, `--isolated` refuses every `--wrapper-profile`, so
no paid driver can score a provider outage as a model `gave_up`.

**Boundary observation replaces the in-init self-check.** No production
self-check runs inside; a scripted observer agent in the live proof reports
the topology from inside (uid, `/proc`, interfaces, absent host paths,
tmpfs bound, closed environment, no stray descriptor, empty capabilities,
`NoNewPrivs`), and the hostile proof attacks it from a sibling. Nested user
namespaces are observed, not asserted: `--disable-userns` is not used until
a live run shows it traverses the profile. The boundary is an access
boundary, not a resource boundary: bwrap sets no cgroup limits unprivileged,
and a hostile agent can still consume host CPU and memory; sequential
admission bounds that today and activation must revisit it.

**CI.** Nothing in `.github/` changes in this run (held for the operator
under 0056's addendum). CI proves the always-on tests on ubuntu and the
`platform` refusal on macOS. Whether a hosted ubuntu runner has bubblewrap,
which version apt installs there, and whether the runner's policy permits
the namespaces are **unobserved**; the operator's follow-up, if wanted, is a
serialized ubuntu job that installs a pinned bubblewrap, checks the floor
and policy, runs the one proof command and fails closed. Until then the
only live proof is the dispatcher's command on this host.

**Seats and the live proof.** Delivery seats cannot run bwrap in their own
box; every boundary-dependent test detects that and fails with the named
`SKIP trial-isolation: boundary_unavailable:<step>` line, never a fake
pass, and the gate inventory never runs them. The hostile proof named in the
issue (a same-uid agent in trial B cannot read A's prompt, reach A's MCP or
fixture, or make A's idle agent pass) is one documented command, free (null
and oracle agents and scripted hostile agents only), deterministic and
bounded; the spec says which criteria only it proves (AC-L1..L3).

**Still proposed.** Trials stay sequential under accepted 0076 until that
proof passes and the operator accepts this record; concurrency returns only
behind both. Current integration 5 and 6 below are superseded by this
section's refusal set and probe; integration 1–4 stand. Nothing here accepts
itself.

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

*Reframed for bubblewrap, 2026-11-01.* `--unshare-net` leaves the trial a
private loopback and nothing else. bwrap's `--share-net` is rejected: it
keeps the host network namespace, so the trial's own Swarmail and fixture
would listen on the host loopback where sibling trials and host services
reach them. The two remaining routes each cost something this record does
not authorize: `pasta` or `slirp4netns` composed around bwrap is a second
binary prerequisite with a user-mode network stack inside the trust
boundary and host-address mapping to disable; the controller-side CONNECT
relay above is new relay code plus one descriptor that must survive the
controller's close-on-exec sweep. The operator chooses one, or leaves real
drivers outside the boundary; nothing is built or guessed here.

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
