# A hard eval tier: three traps frontier agents can miss

- Date: 2026-11-02.
- Status: designed (chief synthesis 2026-11-02); revised 2026-11-02 after the
  first review returned the run to design (accepted 0102, 2026-10-31 addendum).
  The revision is § Review return below; the earlier text stays as history
  where it is superseded and is marked so. Revised again 2026-11-02 after the
  second review returned the run to design (§ Review return 2): the backlog's
  request line no longer reads any clock.
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
| `otp-backlog` | `stale_code` | `gave_up` | Twelve near-identical code mails for one sign-in, delivered in a seeded order with the current generation neither first nor last. Taking the newest arrival, `emails[0]`, or the last in the list submits a superseded code. | Every mail prints `Requested N minutes before this email was sent.` (revised on the second return, F1′), and the smallest N is the most recent request; the goal says the most recently requested code is the valid one. Only content identifies the generation; arrival order and list position are the trap. |
| `recipient-crowd` | `wrong_message` | `gave_up` | Five confirmation mails in one inbox with identical subjects, every body greeting the agent's own address. Four are for look-alike recipients; one look-alike address contains the agent's address as a substring, so the MCP `to` substring filter still returns it. Trusting greeting, subject, filter or position presents a foreign token. | MCP returns the parsed `to` array; the goal names the exact address. Exact comparison of the recipient address finds the one right mail. |
| `digest-lookalike` | `wrong_link` | `gave_up` | An HTML-only digest whose every link is a tracking wrapper with a `u=` destination. Two anchors carry the exact requested title; one destination host is the publisher, the other is the publisher's host with a tracker suffix appended, so a substring check on the host also passes the decoy. Choosing by anchor text, by first match, or by "host contains" opens the decoy. | The destination is readable in the href without following anything, the goal names the publisher host, and the anchors are readable in the `html` the wait view returns (amendment 1 below). Exact host comparison finds the one right link. |

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
   `Requested at …`, derived from the fixture time and the generation, is
   adopted over robustness's `[n/12]` counter: it reads like a real resend
   mail and still gives a total order. The renderer and the existing
   `otp`/`stale-otp` mails are untouched. (The derivation's direction was
   corrected on review return: see § Review return, F1. The second return
   replaced the absolute stamp with a line relative to the mail's own send,
   `Requested N minutes before this email was sent.`: see § Review return 2,
   F1′.)
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
the code, the line `Requested N minutes before this email was sent.` and the
sentence `It expires in 10 minutes.`; no mail says it replaces another.
(Revised 2026-11-02 on the second review's return, F1′; the absolute
`Requested at YYYY-MM-DD HH:MM:SS UTC` form is superseded.) The request line
of generation `g` is relative to the mail itself:

    N(g) = (12 − g) × 4        for g in 1..=12

so the current generation (12) reads `Requested 0 minutes before this email
was sent.` and generation 1 reads 44. No part of any backlog body reads the
fixture time, the run-level `--fixture-time` or any clock: each body is a
function of its code and its generation only. It is therefore true in every
product: a one-trial `run`, every cell of `matrix` and `baseline` however
long after the run's start that cell begins, and any pinned `--fixture-time`,
past or present. All twelve codes are minted at the trial's own prepare and
sent at once, so the current mail's two claims (requested when sent, expires
in ten minutes) are what the application enforces (`OTP_TTL`, 600 s from
mint). Every request is within the goal's "past hour", at most 44 minutes
before its own send, and N strictly decreases as the generation rises, so
the smallest N is the most recent request. The `Date` header is not part of
the trap: it stays the fixture time as for every other task, the wait view
exposes no `Date`, and no backlog text refers to it. Generations 1–9 are,
by their own line, past their ten minutes, and 10–11 are not. The
application reports all eleven older generations as superseded, which it
checks before expiry, and both are rejections.
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
real trial path and through `check` on the retained state. The crowd
control asks the real server for the agent's mails with `to` on the wire and
takes the look-alike the server's substring filter returns (revised
2026-11-02, F5). Layout invariants
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
- **AC7 — Layout invariants on every seed.** In process, for 64 distinct public seeds: `otp-backlog` plans twelve OTP deliveries whose current generation is neither first nor last, with twelve distinct codes, where (revised 2026-11-02 on the second return, F1′, superseding the first return's absolute-stamp clause) every mail's text body (the part after the header block, CRLF read as LF) equals exactly `Your verification code is {code}.\n\nRequested {N} minutes before this email was sent.\nIt expires in 10 minutes.\n` for its own code and `N = (12 − g) × 4`, so each mail has exactly one request line, N ≤ 60, N strictly decreases as the generation rises, and generation 12's N is 0; no backlog body contains `Requested at`, `UTC` or the fixture date; and all of this holds for the same seed planned under each of three pinned fixture times, `2026-10-24T12:00:00Z`, `2026-10-24T00:30:00Z` and `2020-01-01T00:00:00Z` (years before the trial, as a late matrix cell's master time can be), so the body cannot depend on the fixture time; `recipient-crowd` plans five confirmation deliveries with five distinct recipients, exactly one to the agent, that one neither first nor last, every body greeting the agent's address, and one decoy address containing the agent's address; `digest-lookalike` plans one HTML-only mail with twelve anchors, exactly two titled `Async drop finally lands on stable`, exactly one `DigestRight` token whose destination host is `news.fixture.eval`, and the decoy's host `news.fixture.eval.click-tracker.net`. The same seed reproduces the same layout; two seeds differ in at least one layout; secrets differ across two constructions of the same seed. Red: any seed violating an invariant.
- **AC8 — The backlog's resend is closed and harmless.** In `otp-backlog`, `POST /otp/resend` answers 429, records an unaccepted event with no category, sends no mail, and the twelve deliveries are the only OTP deliveries in the snapshot; a trial that hits the 429 and then verifies the current code passes; one that verifies a superseded code fails `stale_code`. Every other scenario's resend behaves as before. Red: a thirteenth OTP delivery, a categorized resend event, or a changed `stale-otp`/`otp` resend.
- **AC9 — The scorer's inputs do not grow.** `score`'s signature stays `(&Task, &Snapshot)`, the snapshot schema stays 2 with its field set unchanged, and no hard verdict consults anything else. Red: the boundary test on the signature and the snapshot field inventory.
- **AC10 — Documentation is bidirectional.** Every catalogue id appears in `evals/README.md` and `evals/run-plan.md`; every hard id named in either appears in the catalogue; both name `--tier hard`, `6068b687`, the operator-owned paid hard run and 16 default-tier plus 3 hard tasks; the README trap table lists the three new action/class rows; the decision index has exactly one `0115` row marked proposed; the agent eval harness spec's tier list names `hard` with a dated amendment. Revised 2026-11-02 (F2–F4), and again on the second return (F1′ and its siblings): the README's backlog row and the run-plan state that no request line depends on the fixture time and that the current code was requested when its mail was sent; both state that the window in which the current code can expire before the deadline is as long as the prepare-to-start gap, at the deadline's end, and is `stale_code` by the scorer's fallback; neither says "request times precede the send", "Requested at" or "before the agent begins"; the README says the wait view exposes no `Date` header and no longer says the store keeps none; proposed 0115 names accepted 0076 §5's seven-trap count as historical; the agent eval harness spec's AC3 amendment also states that the AC4 and AC5 controls run all 19; the agent eval harness spec's AC3 carries a dated amendment naming 19 catalogue rows in four tiers with the default product still the 16; `specs/baseline-wrappers/spec.md` carries a dated note that the approved product is 16 tasks of a 19-task catalogue; `evals/hard-tier-intake.md` opens with a dated status banner naming the three shipped ids and pointing at this spec and 0115; the run-plan states that default-tier rows compare with the 2026-11-01 baseline by task id and per-task `definition_sha256`, not by the catalogue hash. Red: the docs test.
- **AC11 — No reach beyond `evals/`.** No root production file, `Cargo.lock`, `evals/Cargo.lock`, `deny.toml`, `.github/`, `scripts/` or `.brokkr/` file changes; outside `evals/` only `docs/decisions/README.md`, `docs/decisions/0115-hard-eval-tier.md` (while proposed), `specs/agent-eval-harness/spec.md`, `specs/baseline-wrappers/spec.md` (revised 2026-11-02, F3: one dated note) and this design's own files change; no accepted decision changes text or status. Red: the diff.
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
   string. Only the format widened. *The "plus" direction this amendment kept
   was itself the defect the first review found; the date form stands, the
   derivation is now R3's `fixture_time − (12 − g) × 4 min` (§ Review return,
   F1).* *Superseded 2026-11-02 by § Review return 2: the line is now relative
   (`Requested N minutes before this email was sent.`) and carries no date.*

## Review return (2026-11-02, after the first review; decision 0102)

The first review of the implementation (head `21a196a`) found the design
defective on one point and clean on security and on every frozen guarantee.
This section answers each finding; plan.md § Review-return changes says which
change answers which, and tasks.md T15–T20 give each finding the implementer
must act on its own task and proving test. No acceptance criterion is
weakened: AC7, AC10 and AC11 are tightened or widened as marked above, and
the goal texts, catalogue bytes, hashes, tiers, CLI, products and scoring are
unchanged.

### F1 (medium, spec defect; independent, security and tests, with a guarantees note) — the request times were impossible

*Superseded in part 2026-11-02 by § Review return 2, F1′. The subtraction
below fixed a one-trial `run` but not `matrix` or `baseline`, and the
absolute stamp it kept is replaced by a relative line. The rejections below
still stand: `issued = requested(g)`, the 300 s deadline cap, and any change
to the goal, the `Date` or the fixture time.*

R3 fixed `Requested at` as fixture time **plus** `generation × 4` minutes.
Every mail's `Date` is the fixture time and the default fixture time is the
trial's start, so every mail claimed a request 4–48 minutes after it was sent
and after the trial began, against the goal's "Over the past hour" and the
mail's "It expires in 10 minutes". The oracle still passed because it sorts
the same strings, and the 64-seed test pinned the flawed formula, so no proof
could go red. A careful agent that checks the clock would have met impossible
timestamps and been pushed toward `gave_up` or `tool_misuse`, diluting the
`stale_code` signal the paid run exists to rank.

**Ruling.** Both positions agree the fix is a subtraction; they differ on the
constant and on what else must move.

- *Adopted (simplicity): `requested(g) = fixture_time − (12 − g) × 4 min`.*
  The current generation is requested at the send time, the only value under
  which the current mail's three claims — request time, `Date` and "expires
  in 10 minutes" — all agree with the application, because all twelve codes
  are minted at prepare, a moment after the fixture time, and `OTP_TTL` runs
  600 s from mint. Review's suggested `13 − g` removes the impossible clock
  but has the current mail promise expiry at T + 6 min while the application
  accepts the code until about T + 10 min: a smaller contradiction of the
  same kind.
- *Rejected (robustness): mint each generation with `issued = requested(g)`
  and cap the hard product's deadline below the remaining validity (300 s).*
  Pairing `issued` with the request time needs an `Instant` up to 48 minutes
  before the clock seam's now; `Instant` subtraction underflows on a host
  booted less than that long ago, which CI runners routinely are, and a
  guarded fallback would make expiry depend on uptime. The deadline cap
  changes the one field the hard product was ruled to share with the approved
  product (AC4), `Plan`'s deadline validation and the run-plan's "600000 ms
  for both", to close a window that under the adopted formula is the
  prepare-to-start gap only. **Cost of the ruling:** by their own line,
  generations 1–11 are past their ten minutes while the application would say
  "superseded", which it checks first; both are rejections, and an agent
  learns which only by submitting one, which is the trap. And a model that
  submits the *current* code after the code's TTL but before the deadline —
  possible only in the seconds between its mint at prepare and the agent's
  start — is scored `stale_code` by the fallback, not by the named trap. This
  is pre-existing for `otp` and `stale-otp`, is bounded as stated, and the
  run-plan records the bound.
- *Adopted (robustness): prove the clock property, not just the ordering.*
  AC7 now pins the exact derivation against the captured fixture time, the
  `≤ Date` and `≥ fixture_time − 60 min` bounds, generation 12 equal to the
  fixture time, and string order equal to generation order both at a pinned
  noon and at a pinned 00:30 UTC where the backlog spans midnight. This is
  the assertion that would have caught the defect; it lives in the existing
  64-seed test, not in a new control: a scripted clock-reading agent would
  re-test the same arithmetic through a process boundary and need a
  wall-clock assertion, which 0020 §5 forbids.
- *Rejected (both): changing the goal text, the `Date` header or the fixture
  time.* The story was right and the numbers were wrong; a goal change moves
  the catalogue hash and the docs pins for nothing, a per-mail `Date` touches
  the renderer every task uses and adds a signal MCP never exposes, and an
  earlier fixture time shifts every mail's header to fix one body line.

### F2 (info; independent, guarantees) — proposed 0115, plan.md and this spec described a superseded artifact

0115's Alternatives said `Requested at HH:MM:SS UTC`; plan.md and this
spec's own table said the oracle reads MCP `anchors`, which implementation
amendment 1 had already corrected. All three are aligned in this revision:
0115 (still proposed; its status does not move) states the date form and the
backward derivation, and the oracle is described as parsing anchors from the
`html` the wait view returns.

### F3 (info; guarantees, correctness) — accepted specs and the intake record contradict the shipped tier

`specs/agent-eval-harness/spec.md` AC3 still says "Exactly the 16 rows …
three tiers"; `specs/baseline-wrappers/spec.md` says the catalogue has 16
tasks; `evals/hard-tier-intake.md`, committed unchanged by the dispatcher,
frames four tasks with three ids that do not exist. Each gets a one-line
dated note; the intake record is a phase record and is not rewritten. AC10
holds the three through the docs test and AC11 names the second spec in the
allowed set.

### F4 (info; correctness) — the default product's plan carries the 19-task hash

`catalogue_sha256` is the identity that `check --matrix` replay and the
baseline plan validate against the live catalogue; a per-product subset hash
there would break replay or move the provenance schema (0071 §4, 0076).
Rejected (robustness asked for the field to move). Comparability with the
2026-11-01 baseline rests on what every row already carries: the task id and
the per-task `definition_sha256`, which AC1 proves unchanged for the sixteen.
The run-plan states that in one sentence (AC10).

### F5 (info; tests) — the crowd control re-implemented the server's `to` filter

The scripted `wrong_message` agent filtered the five mails client-side and
never sent `to` on the wire, so the trap's story rested on server semantics
only the root suite proves. Adopted (robustness): the control passes
`to = <agent address>` to `swarmail_wait_for_email`, asserts the real server
returned the look-alike beside the agent's own mail, and only then presents
the look-alike's token. Test-side only; oracle and null are unchanged. R5
reads accordingly.

### Robustness proposals not taken, with the cost

- *Rename the digest hosts to RFC 2606 `.example` names.* `fixture.eval` is
  the harness-wide fixture domain, the security review confirmed nothing
  fetches a `u=` destination, and the rename moves the goal text and so the
  catalogue hash, snapshot and docs pins. Cost: the no-resolution property is
  policy, as it is for every other fixture host.
- *A prepare-time `inbox_len << cap` assertion and a `uses_chaos()` arm.* The
  cap is the server's default and no hard scenario arms chaos; both are
  already true, and asserting them adds production lines for a risk no one
  observed.

### Correctness infos 1, 2 and 5 — no change

The oracle's argv/env boundary is one runner path for every task; the
digest fingerprint's determinism is by construction of `select`; the
resolver's re-check of the `--tier`/`--tasks` conflict is unit-tested
redundancy. None warrants a change.

### What the implementer reads (first return)

The revised R3, R5, AC7, AC10 and AC11 above; plan.md § Review-return
changes for the file-level mapping; tasks.md T15–T20 for the ordered work
and its proving tests. T1–T13 stay done; the catalogue, goals and hashes do
not move, so the regenerated `catalogue-v2.json` and `6068b687…` stand.

## Review return 2 (2026-11-02, after the second review; decision 0102)

The second review (head `76e2e60`, verify-passed tree `cc2885a`) found the
first return's T15–T20 implemented as specified, security clean and every
frozen guarantee untouched. It returned the run with one medium spec defect,
raised by the independent member and confirmed by the tests member, plus one
low finding and several infos. This section answers each one. plan.md
§ Review-return-2 changes maps each finding to its files, and tasks.md
T21–T27 give the implementer a task and a proving test for every finding
that needs action. No acceptance criterion is weakened. AC7 is restated
around the new line and gains an assertion. AC10 replaces its request-time
phrases and adds the sibling corrections. The goal texts, catalogue bytes,
both hashes, tiers, CLI, products, deadlines and scoring do not move.

### F1′ (medium, spec defect; independent and tests) — matrix and baseline cells stamped request times from the run's start

`matrix` and `baseline` each resolve one master `FixtureInputs` per run
(`evals/src/main.rs:365`, `2086`). `assigned_cell` (`main.rs:716-746`) clones
it into every cell and replaces only the seed, and the runner forwards that
fixture time unchanged. The first return's `Requested at` line was
`fixture_time − (12 − g) × 4 min`. Every later cell therefore stamped its
request times from the run's start, but minted its codes and started their
ten-minute `OTP_TTL` at its own prepare. Cells run task-major, then
agent-major within a task. In `baseline --tier hard`, the operator's paid
product, the agents declared last would see a current code that its own
text says expired tens of minutes or hours earlier. Their mail would also
contradict the goal's "past hour" premise. That pushes careful agents
toward `gave_up` or `tool_misuse`. It also biases results by declaration
position, which confounds the ranking the tier exists to produce. Every
proof pinned `--fixture-time`, which the product never does, so no proof
could go red. A single `run` with a pinned past `--fixture-time` has the
same mismatch.

**Ruling: take the clock out of the line instead of giving the line a
better clock.** Simplicity is adopted. Robustness's anchor is rejected.

- *Adopted (simplicity): `Requested N minutes before this email was sent.`
  with `N = (12 − g) × 4`.* The defect has one root: a trap signal written
  in absolute wall-clock time with no per-trial clock behind it. A line
  relative to the mail's own send has no clock input, so no master time,
  cell position or pinned past time can contradict it. The current mail
  says it was requested when it was sent, at that trial's prepare, and that
  it expires ten minutes later. That is exactly what `OTP_TTL` enforces
  from the same prepare. The trap is unchanged: arrival order and list
  position still mislead, and content is still the only generation signal.
  A careful agent takes the smallest N. Reading the scale backwards (the
  largest N) is a real content-reading mistake and still lands in
  `stale_code`. The goal text ("Each email states when its code was
  requested") stays true, so the catalogue bytes, `definition_sha256`,
  `6068b687…` and the full hash do not move. A bare sequence number
  ("request 7 of 12") is not a "when". It would move the goal, and with it
  every hash and docs pin, so it is rejected.
- *Rejected (robustness): one wall-clock anchor per trial, captured at
  prepare and used for the backlog's `Date`, every token's `issued` and the
  `Requested at` base, plus a two-cell real-process matrix proof with a
  pinned past `--fixture-time`.*
  - The fixture's clock seam is `Arc<dyn Fn() -> Instant>`
    (`evals/src/fixture.rs:51`): a monotonic instant with no calendar value.
    The anchor would need a second, wall-clock seam threaded through
    `Private::new` and every test that builds a fixture, and every line of
    that seam falls under the exact gate.
  - `otp-backlog` would become the one scenario whose `Date` departs from
    the recorded `--fixture-time`, so replaying that provenance input would
    no longer reproduce the mail's headers.
  - The product proof's pass condition would be "inside the trial's own
    start–end window", a timing claim that 0020 §5 makes awkward.

  The relative line fixes the same paths, and every pinned run, with one
  `format!`. **Cost of the ruling:** the backlog loses absolute-timestamp
  realism and the date/midnight reading the first return paid for. An agent
  that wants absolute times must combine N with the wait view's
  `received_ms`. That combination is coherent, because all twelve mails are
  sent at prepare. Robustness names this form as its own honest fallback.
- *Rejected (tests member's correction; independent option 2): rebase each
  cell's `fixture_time` at dispatch.* The matrix plan serializes each
  cell's fixture before the cell starts (0076). A dispatch-time value
  either rewrites a planned cell after the plan is fixed, which replay and
  the planned-versus-recorded checks refuse, or adds a field. Both change
  an accepted 0055 provenance input for all 19 tasks, the approved sixteen
  included, to fix one body line. That is an amendment to accepted rulings
  and waits for the operator (0045 §8). It would also leave every pinned
  past `run` showing stale stamps. **Cost:** later matrix cells still carry
  the master time in every task's `Date` header. That is pre-existing for
  all 19, the wait view does not expose it, and after this ruling no trap
  text refers to it.
- *Adopted from robustness: a proof that varies the input the product
  generates.* Robustness is right that a test which pins the product's free
  input proves nothing about it. AC7 now plans the same seed under three
  pinned fixture times, including `2020-01-01T00:00:00Z` (standing in for a
  late cell's stale master time). It asserts that each body equals the
  clock-free template built from its own code and generation. That is the
  executable form of "no clock coupling", checked where the body is
  decided. It goes red on `76e2e60`, which prints `Requested at`, and on any
  later edit that reads `inputs.time()` again. It makes no wall-clock
  validity assertion (0020 §5).

### Tests LOW 1 — no test executes the `Date` tie

Answered by deletion, not by a new assertion. Under F1′ the body refers to
no `Date` and no time, so R3 and AC7 drop the "at or before the mail's
`Date`" claim the finding asked to prove. Robustness also showed that
`Date == requested(12)` would have been green on the defect, because both
sides derived from the same stale input.

### Guarantees LOW 1 — accepted 0076 §5 still counts seven traps

Accepted text is not edited. Proposed 0115, the record of the trap-list
amendment, now names 0076 §5's "seven traps" in its "Amends upon
acceptance" line, as a count that becomes historical on acceptance; the
list itself stays 0071 §1's. This design commit drafts that sentence in
0115, whose status stays `proposed`, and AC10's docs test pins it (T24).

### Sibling infos — folded into text being rewritten anyway

- *Correctness info 1 and guarantees info 3 (README expiry sentence).* The
  current code can expire before the deadline only in a window as long as
  the prepare-to-start gap, at the deadline's end, not "before the agent
  begins". The README and run-plan say so, and the docs test pins it (T23).
- *Guarantees info 2.* The README says the wait view exposes no `Date`
  header. The store does keep the raw message, Date included (T23).
- *Guarantees info 4.* The agent eval harness spec's dated AC3 amendment
  also says the AC4/AC5 controls run all 19 (T25).
- *Guarantees info 5.* plan.md's file map now names `evals/src/mcp.rs`
  (`wait_to`, `wait_emails_to`). This revision makes that correction.
- *Guarantees info 6 (the "generations 1–7" comment).* The 00:30Z midnight
  case the comment described goes with the date, and the comment goes with
  it (T21).
- *Correctness info 2 (`crowd_mails` doc comment).* The comment now says
  that a server answering with the agent's mail alone makes the count-2
  wait time out (T26).
- *Tests info 2 (a client-side drop of `to` is red on two seeds in three).*
  **Not taken.** Robustness asked for a pinned crowd seed and an asserted
  position. The member who raised the finding said no change is required.
  The named server-side regression is red on every seed, and a pinned-seed
  trial would be a new real-process test for a helper regression. **Cost:**
  a client-side drop of `to` is caught on about two runs in three, not on
  every run.

### What the implementer reads (second return)

R3's backlog paragraph, AC7's backlog clause and AC10's revised sentence
above; plan.md § Review-return-2 changes for files; tasks.md T21–T27 for
the ordered work. T1–T20 stay done. The 0115 text (T24's text half) is
already in this design commit. The catalogue, goal texts and hashes do not
move, so `catalogue-v2.json` and `6068b687…` stand.
