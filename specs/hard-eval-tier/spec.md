# A hard eval tier: three traps frontier agents can miss

- Date: 2026-11-02.
- Status: designed (chief synthesis); implementation, verify and review follow.
- Scope: Epic #164, the operator's 2026-11-01 ruling to build a harder tier
  before the next paid run. Base: main `f815d48`.
- Route: `claude-careful` (accepted 0098/0099), zero Codex; review residuals
  under accepted 0102.

Decision: 0115 — [A hard eval tier and its named traps](../../docs/decisions/0115-hard-eval-tier.md),
status proposed; acceptance is the operator's.

Authority relied on, not amended: accepted
[0055](../../docs/decisions/0055-agent-eval-harness.md) (closed failure
taxonomy, oracle/null proofs, app-state scoring, standalone `evals/` crate),
accepted [0071](../../docs/decisions/0071-eval-trap-scoring.md) (named traps
precede recovered goals; v2 catalogue/snapshot/result), accepted
[0076](../../docs/decisions/0076-sequential-eval-trials.md) (sequential
trials), accepted [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
§5 (no wall-clock validity assertions, no sleep as synchronization) and
accepted [0045](../../docs/decisions/0045-delivery-repairs.md) §8 (compatible
work may land beside a proposed decision).

## Feature and rationale

The 2026-11-01 baseline ([docs/evals/baseline-2026-11-01.md](../../docs/evals/baseline-2026-11-01.md))
scored 395 of 400 paid trials on the 16-task v2 catalogue. Four of five models
were perfect and the only failures were one model's early `gave_up` exits. The
suite is saturated: it cannot rank the epic's remaining work. This change adds
a `hard` tier whose tasks a careful agent passes and a careless one fails for a
reason the closed taxonomy already names.

Three hard tasks ship, each grounded in one real agent mistake:

| id | failure_class | idle_class | The mistake it provokes | Why a careful agent still passes |
|---|---|---|---|---|
| `otp-backlog` | `stale_code` | `gave_up` | Twelve near-identical code mails for one sign-in, delivered in a seeded order with the current generation neither first nor last. Taking the newest arrival, `emails[0]`, or the last in the list submits a superseded code. | Every mail prints `Requested at HH:MM:SS UTC`; the goal says the most recently requested code is the valid one. Only content identifies the generation; arrival order and list position are the trap. |
| `recipient-crowd` | `wrong_message` | `gave_up` | Five confirmation mails in one inbox with identical subjects, every body greeting the agent's own address. Four are for look-alike recipients; one look-alike address contains the agent's address as a substring, so the MCP `to` substring filter still returns it. Trusting greeting, subject, filter or position presents a foreign token. | MCP returns the parsed `to` array; the goal names the exact address. Exact comparison of the recipient address finds the one right mail. |
| `digest-lookalike` | `wrong_link` | `gave_up` | An HTML-only digest whose every link is a tracking wrapper with a `u=` destination. Two anchors carry the exact requested title; one destination host is the publisher, the other is the publisher's host with a tracker suffix appended, so a substring check on the host also passes the decoy. Choosing by anchor text, by first match, or by "host contains" opens the decoy. | The destination is readable in the href without following anything, the goal names the publisher host, and MCP returns `anchors` with url and text. Exact host comparison finds the one right link. |

The tier covers the three agent-caused classes with a misdirection worth
measuring: `stale_code`, `wrong_message`, `wrong_link`. The two idle classes
are already measured by the default tier.

### What the panel argued, and the rulings

Both positions (run-local, `.brokkr/state/design/positions/`) agreed that the
change stays inside `evals/` (0055), that no frozen guarantee is near, that
secrets stay on the OS RNG while the layout is seeded, that `--tier` must be
exclusive with `--tasks` and exit 2 with no rows, that versions stay at 2, that
no budget-class or wall-clock task is admissible (0020 §5), and that the
plus-alias task is dropped because it rests on proposed 0112. They differed on
five points; each is ruled here.

1. **Three tasks (simplicity) over four (robustness).** Robustness's
   `invite-reschedule` needs a new action string, a second invite record per
   token, SEQUENCE rendering and a changed `InviteAccepted` meaning, and its
   `chain-confirm` needs two new fixture routes and mid-trial sending, yet
   neither carries a misdirection: a goal that says "accept the rescheduled
   time" or "then read the second mail" is an instruction the baseline shows
   frontier models follow every time. A hard task every model passes is the
   design failure the operator named. **Cost of the ruling:** no hard task
   exercises the multi-email or attachment shape, and the tier spans three of
   seven classes. Both shapes are recorded as follow-ups to be chosen with the
   paid hard run's data, not before it.
2. **Named traps under a proposed decision (simplicity) over app lockouts
   scored by the fallback (robustness).** Robustness avoids amending 0071 by
   making the first wrong attempt terminal in the app. Accepted 0071 chose
   "truthful recovery over lockouts or fail-on-any-rejection" and its spec
   excluded application lockouts; a lockout contradicts an accepted preference
   to avoid proposing an amendment. The fallback is also weaker: an unrelated
   earlier refusal (a mistyped token is `tool_misuse`) would outrank the trap,
   and the first wrong attempt is fatal only if the app forbids recovery. So
   the three tasks get exact named-trap rows, each reusing an action/class
   pair 0071 already names (`superseded_code`/`stale_code`,
   `foreign_token`/`wrong_message`, `digest_decoy`/`wrong_link`), appended to
   0071 §1's list by proposed 0115. 0071 says future traps need explicit
   policy review; 0115 is that review. The implementation lands beside the
   proposal as compatible work under 0045 §8: no existing verdict moves, no
   accepted scenario's behaviour changes, the taxonomy stays closed and no
   action string is invented. **Cost of the ruling:** if the operator reads
   0071 §1 as a closed list, the three `trap_class` arms, and with them the
   disqualification criterion AC6, wait for 0115's acceptance; everything else
   in this design is independent of that reading and lands regardless.
3. **Content discriminator for the backlog (both, differently).** The store
   exposes `received_ms` and a UUIDv7 id; no `Date` header reaches MCP. Both
   positions reject the intake's "newest by Date". Simplicity's body line
   `Requested at HH:MM:SS UTC`, derived from the fixture time plus the
   generation, is adopted over robustness's `[n/12]` counter: it reads like a
   real resend mail and still gives a total order. The renderer and the
   existing `otp`/`stale-otp` mails are untouched.
4. **Inert tracking wrapper (simplicity) over a `/t` redirect route
   (robustness).** `/a/{token}?u=<destination>` already resolves to the
   existing link handler, which ignores the query, so the existing
   `DigestRight`/`DigestWrong` roles, actions and check apply unchanged and
   nothing is ever fetched. Robustness's SSRF and hang analysis becomes moot
   because no route reads `u=`. **Cost:** no redirect realism; the trap is
   reading the destination before clicking, which is the mistake being
   measured.
5. **Proof shape.** Robustness asked for eight seeds × three agents per task
   in a separate test target; simplicity for in-process invariants plus the
   default seed. Ruled: the only seed-dependent material is the layout, so
   the layout invariants are proved in process over 64 seeds, every hard task
   runs oracle and null through the real command path on the default seed via
   the existing per-task sweep and on four fixed seeds explicitly, and the
   scripted mistake agents run through the real trial path. **Cost:** fewer
   end-to-end seeds than robustness wanted; the invariant sweep covers what
   they would have varied.

Two things neither position saw are ruled here as well:

- **`POST /otp/resend` would bypass the backlog trap.** The existing route
  mints a fresh current code mid-trial. For `otp-backlog` it answers
  `429 "the resend limit for this sign-in has been reached"`, recorded as an
  unaccepted event with no category, so it neither passes nor classifies
  anything; the goal discloses the limit. Every other scenario's resend is
  unchanged.
- **The operator's paid hard run must go through `baseline`, not bare
  `matrix`.** The approved declarations carry a placeholder argv[0] that only
  `baseline` resolves to the wrapper executable and private config. So
  `baseline` gains the same `--tier` selector; its default product stays the
  16 default-tier tasks (32 controls, 480 paid slots) exactly as approved on
  2026-10-26, and `--tier hard` is a separate, operator-selected product
  (3 tasks, 6 controls, 90 paid slots) with the same controls-first
  discipline.

## Requirements

### R1. The tier value

- `Tier` gains `Hard`, serialized `hard`. The 16 existing tasks keep their
  ids, tiers and every field; the catalogue appends the three hard tasks
  after `inbox-isolation`, in the order of the table above.
- Catalogue, snapshot and result schemas stay at 2 (0071 §4, 0076): the
  change is additive and no field moves. The full catalogue hash changes as
  a consequence; `6068b687…` stays the recorded hash of the 2026-11-01
  baseline, and the default-tier subset of the new catalogue must still
  serialize to exactly the bytes that hash to it.
- `tasks` output is unchanged in shape; the hard entries print with
  `"tier":"hard"`. An unknown tier string still refuses on deserialization.

### R2. Selection

- `matrix --tier <straight|traps|multi_step|hard>` resolves to the catalogue
  ids of that tier, in catalogue order, then follows the unchanged matrix
  path. The spellings are the catalogue's serialized tier names; there is one
  spelling, not a second clap-cased one.
- `--tier` and `--tasks` are mutually exclusive and one of them is required;
  either violation, and an unknown tier value, is a usage error: exit 2 and
  no result row. A resolved selection that is empty is refused the same way
  (`matrix_empty_selection`), so a green run can never be an empty run.
- `--tier` is selection sugar: the matrix plan records the resolved ids as
  it does today, no artifact or schema version moves, and `check --matrix`
  replay is unchanged.
- `baseline --tier <tier>` selects the same way. Without `--tier`, the
  baseline product is the default-tier tasks, byte-for-byte the 16 the
  operator approved; `baseline --tier hard` is the hard product. The dry run
  prints the selected tasks and cardinalities.

### R3. The three scenarios

Every hard task is `agent_count: 1`, `idle_class: gave_up`, the default
deadline, a new `Scenario` variant mapped to an existing `Flow`, and an
existing `Check`. Each goal discloses its disqualifying action (0071 §4).
Secrets (codes, tokens) come from the fixture's OS RNG as today; the layout
(positions, send order) comes from `FixtureInputs::select`, keyed by task id,
so the same public seed reproduces the same trap and different seeds produce
different ones.

**`otp-backlog`** — `Scenario::OtpBacklog`, `Flow::Otp`, `Check::OtpLoggedIn`.
The fixture issues twelve OTP generations for the agent at prepare, through
the existing generation counter, so generation 12 is current and generations
1–11 are superseded. Every mail has the same subject and the same template:
the code and the line `Requested at HH:MM:SS UTC`, where the time is the
fixture time plus `generation × 4` minutes; no mail says it replaces another.
The twelve are sent in a seeded permutation in which the current generation
is neither the first nor the last delivery. Submitting any superseded code
records the existing `superseded_code` refusal; the new trap row makes it
disqualifying. `POST /otp/resend` answers 429 as ruled above. The backlog of
twelve is far below the server's default per-inbox cap of 100,000 and the
fixture mints no further mail, so oldest-only pruning (0003) cannot touch it.

**`recipient-crowd`** — `Scenario::SignupCrowd`, `Flow::Signup`,
`Check::Confirmed`. Beside the agent, four Decoy accounts share the agent's
inbox and SMTP identity, named from the agent's local part and domain:
`{local}.team@`, `{local}-old@`, `{local}2@` and `old.{local}@` (the last
contains the agent's full address as a substring). All five confirmation
mails have the subject `Confirm your account` and greet the agent's own
address; only the envelope and `To` recipient differ. The agent's mail lands
at a seeded position that is neither first nor last of the five. Presenting
a decoy's token as the agent, or as the decoy, records the existing
`foreign_token` refusal; the trap row and 0071's known-Decoy claimant rule
extend to this scenario.

**`digest-lookalike`** — `Scenario::DigestLookalike`, `Flow::Digest`,
`Check::DigestSelected`. One HTML-only digest from the publisher
`news.fixture.eval`, stating that host in its heading. Twelve anchors, each
`href="{app}/a/{token}?u=https://<host>/<slug>"`; the query is inert. Ten
anchors have decoy titles and the publisher host. Two anchors have exactly
the title `Async drop finally lands on stable` and the same slug: the right
one's host is `news.fixture.eval` (`DigestRight`), the decoy's host is
`news.fixture.eval.click-tracker.net` (`DigestWrong`). The order of the two
identical-title anchors, and their positions among the twelve, are seeded.
Opening any decoy records the existing `digest_decoy` refusal; the trap row
makes it disqualifying.

### R4. Scoring

Three `trap_class` rows, exact under 0071 §3's matching (rejected event,
exact action, category, flow and a provisioned Agent claimant; the
`SignupCrowd` Decoy-claimant case joins the existing `SignupSibling` one):

| Task / scenario | Action | Flow | Class |
|---|---|---|---|
| otp-backlog / OtpBacklog | superseded_code | Otp | stale_code |
| recipient-crowd / SignupCrowd | foreign_token | Signup (any flow for a known Decoy claimant) | wrong_message |
| digest-lookalike / DigestLookalike | digest_decoy | Digest | wrong_link |

No new `FailureClass`, action string, `Check`, `Flow` or snapshot field. The
scorer stays `score(task, snapshot)`; nothing reads a mailbox, transcript or
agent account. The refused resend's category-less event is invisible to both
the trap scan and the first-refusal fallback.

### R5. Proofs, free and offline

Oracle and null run through the real command path for every catalogue task,
hard tasks included, and the hard tasks also run on four fixed seeds. A
scripted mistake agent per hard task commits exactly the named trap and then
recovers the goal; the trial still fails with the task's class, through the
real trial path and through `check` on the retained state. Layout invariants
hold on every one of 64 seeds in process. No paid or networked agent runs;
no dependency is added to either lockfile.

### R6. Documentation

`evals/README.md` and `evals/run-plan.md` state: 16 default-tier tasks plus
3 hard, the `--tier hard` commands for `matrix` and `baseline`, that
`6068b687…` is the historical 2026-11-01 hash and the new catalogue hashes
differently, the trap table above (trap, class, why a careful agent passes,
first-wrong-attempt consequence), the 0071 trap table gaining three rows, and
that the paid hard run is the operator's after merge. The agent eval harness
spec's `tier` list gains a dated amendment naming `hard`. Proposed 0115 is
added to the decision index as proposed. No accepted decision changes status
or text.

## Acceptance Criteria

Each criterion names the red test that fails without the change.

- **AC1 — The sixteen are untouched.** Serializing `Catalogue { schema_version: 2, tasks: catalogue().filter(tier != hard) }` yields bytes whose SHA-256 is `6068b687475db4be0000fa325de9f2b610b98470526085010ebd648e04862a94`, and the full catalogue's bytes equal the regenerated `evals/tests/data/catalogue-v2.json` with a newly pinned full hash. Red: any edit to an existing task, or a hard task inserted before `inbox-isolation`, changes the subset bytes.
- **AC2 — `hard` is a tier.** `Tier` deserializes `hard`, refuses `Hard`, `harder` and unknown strings, and `tasks` prints exactly three entries with `"tier":"hard"`, in the documented order, each with `agent_count` 1, `idle_class` `gave_up`, the default deadline and a goal that discloses its disqualifying action. Red: the strict-enum test and the catalogue assertions.
- **AC3 — `matrix --tier` selects fail-closed.** `matrix --tier hard` with the oracle emits rows only for the three hard ids in catalogue order; `--tier` with `--tasks`, neither flag, an unknown tier value, and an empty resolved selection each exit 2 with zero rows. The resolved selection is a plain id list in the matrix plan. Red: a non-hard row, a row from a refused invocation, or a plan recording a selector instead of ids.
- **AC4 — `baseline --tier` keeps the approved product.** `baseline --dry-run` without `--tier` plans 16 tasks, 32 control rows and 480 paid rows exactly as before; `baseline --tier hard --dry-run` plans the three hard tasks, 6 controls and 90 paid rows with the same deadline, concurrency, steps and files. Red: the plan counts.
- **AC5 — Oracle passes and null fails with its idle class on every task.** Through the real CLI, oracle passes all 19 catalogue tasks and null fails all 19 with each task's declared idle class and no completed goal; for the three hard ids the same holds on four fixed public seeds, and the oracle's argv and environment carry no seed and no admin secret. Red: any failing oracle, any null class other than `gave_up` for a hard task, or a seed on which either differs.
- **AC6 — The named trap disqualifies even after recovery.** For each hard task a scripted agent commits exactly the named trap and then truly completes the goal: the fixture records the trap event and the accepted goal event, the goal fact is true in the frozen snapshot, every delivery is `delivered`, the child exits zero, yet the live row is `fail` with the task's class and `check` on the retained state agrees. In the scoring tables a trap-only snapshot scores `fail`/class, a goal-only snapshot scores `pass`, and both event orders fail. Red: a pass after a trap, a wrong class, or the idle class.
- **AC7 — Layout invariants on every seed.** In process, for 64 distinct public seeds: `otp-backlog` plans twelve OTP deliveries whose current generation is neither first nor last, with twelve distinct codes and one `Requested at` line per mail in strictly increasing generation time; `recipient-crowd` plans five confirmation deliveries with five distinct recipients, exactly one to the agent, that one neither first nor last, every body greeting the agent's address, and one decoy address containing the agent's address; `digest-lookalike` plans one HTML-only mail with twelve anchors, exactly two titled `Async drop finally lands on stable`, exactly one `DigestRight` token whose destination host is `news.fixture.eval`, and the decoy's host `news.fixture.eval.click-tracker.net`. The same seed reproduces the same layout; two seeds differ in at least one layout; secrets differ across two constructions of the same seed. Red: any seed violating an invariant.
- **AC8 — The backlog's resend is closed and harmless.** In `otp-backlog`, `POST /otp/resend` answers 429, records an unaccepted event with no category, sends no mail, and the twelve deliveries are the only OTP deliveries in the snapshot; a trial that hits the 429 and then verifies the current code passes; one that verifies a superseded code fails `stale_code`. Every other scenario's resend behaves as before. Red: a thirteenth OTP delivery, a categorized resend event, or a changed `stale-otp`/`otp` resend.
- **AC9 — The scorer's inputs do not grow.** `score`'s signature stays `(&Task, &Snapshot)`, the snapshot schema stays 2 with its field set unchanged, and no hard verdict consults anything else. Red: the boundary test on the signature and the snapshot field inventory.
- **AC10 — Documentation is bidirectional.** Every catalogue id appears in `evals/README.md` and `evals/run-plan.md`; every hard id named in either appears in the catalogue; both name `--tier hard`, `6068b687`, the operator-owned paid hard run and 16 default-tier plus 3 hard tasks; the README trap table lists the three new action/class rows; the decision index has exactly one `0115` row marked proposed; the agent eval harness spec's tier list names `hard` with a dated amendment. Red: the docs test.
- **AC11 — No reach beyond `evals/`.** No root production file, `Cargo.lock`, `evals/Cargo.lock`, `deny.toml`, `.github/`, `scripts/` or `.brokkr/` file changes; outside `evals/` only `docs/decisions/README.md`, `specs/agent-eval-harness/spec.md` and this design's own files change; no accepted decision changes. Red: the diff.
- **AC12 — Every house gate is green.** The whole inventory in [house-rules § Gates](../../docs/house-rules.md#gates--part-of-every-change), locked, including the evals fmt, clippy and tests against the candidate's own `swarmail`, the coverage gate and its selftest, publish-guard and seal proofs, bundle compile and `cargo deny`. Red: any gate.

## Non-goals and deferrals

Paid or networked trials (operator-owned, after merge). Version bumps. Server,
Store, await, pruning or MCP changes. A budget-class or tight-deadline task
(0020 §5). `slow-since` and any task needing wall-clock validity. The
multi-email (`chain-confirm`) and attachment (`invite-reschedule`) shapes,
deferred until the paid hard run shows which classes still saturate. The
plus-alias task, blocked on proposed 0112. Mail-format quirks beyond
HTML-only (quoted-printable, base64, encoded subjects), which need a root
hostile-DATA proof before any task relies on them. A webhook, redirect or
TLS component in any hard task. Changing the default-tier baseline product.
A `tasks --tier` filter: the catalogue already prints `tier`.

## Implementation amendments (2026-11-02, implement seat)

Two facts the design took from the plan's reading of the tree did not hold
when probed against the candidate `swarmail`; the implementation follows the
tree, and no acceptance criterion is weakened.

1. **The wait view has no `anchors`.** The design said MCP returns `anchors`
   with url and text. The negotiated protocol (`2025-03-26`) answers
   `swarmail_wait_for_email` in the legacy view (`html`, `text`, `links`, `to`,
   `received_ms`, no `anchors`, and `fields` is refused as an unknown
   argument). `digest-lookalike`'s trap is unchanged: the destination is in the
   `href`, readable without following anything. The oracle reads the anchors
   from `html` (`oracle::html_anchors`) and compares the destination host
   exactly (`oracle::wrapper_destination_host`). The README states this.
2. **The `Requested at` line carries the date.** The design specified
   `Requested at HH:MM:SS UTC` from fixture time plus `generation × 4` minutes.
   A time of day wraps at midnight (a fixture time of 23:30 puts generations
   8–12 on the next day, so string order would stop being generation order). The
   line is `Requested at YYYY-MM-DD HH:MM:SS UTC`, still exactly one per mail,
   still strictly increasing with the generation (AC7), and comparable as a
   string. Only the format widened.
