# Spec — the parse/bounds/loopback family's protected re-entry (issues #49, #48)

- **Run:** run 10b (chief architect, synthesizing the simplicity and
  robustness positions) · **Feature slug:** `protected-reentry`
- **Issues:** [#49](https://github.com/feedback-loop-ai/swarmail/issues/49)
  and [#48](https://github.com/feedback-loop-ai/swarmail/issues/48) —
  already implemented (f9cde28) and repaired after run 10's review hold
  (a4ff22c). This re-entry re-runs the protected phases on the tree as it
  stands; it does not re-open the ruled shape.
- Decision: 0030 (proposed) —
  [0030-loopback-plane-bounded-results](../../docs/decisions/0030-loopback-plane-bounded-results.md)
  — the ruling under review. This spec amends nothing in it: §2 was
  already rewritten in place at a4ff22c with its provenance paragraph, and
  the re-entry's corrections ride 0030's acceptance pass, not a fork.
- **Base:** `a4ff22c` (the operator's repair, atop f9cde28 ← 7550de3 ←
  66a45bb), with this spec trio committed at `6f9a4bf` and its record
  carried through the synthesis commits (`b2c95f3`, `c190493`) — the
  re-entry design this run confirms — and the tree re-verified at HEAD
  `95d2823`, where the two defect-driven repairs (`81409d4`'s signal
  reorder, `95d2823`'s search-needle refusal) sit atop a4ff22c. Every
  line cited below was re-verified against that tree in this session;
  nothing here is taken from the positions' prose.
- **Predecessor spec:**
  [off-worker-parses-loopback-posture](../off-worker-parses-loopback-posture/spec.md)
  (committed at 7550de3, unchanged since) — the ruled shape. This is its
  re-entry contract, not a spec v2: where the two could disagree, the
  ruled spec wins and this one records the confirmation.
- **Frozen guarantees:** 0001/0002/0003 re-verified in code at HEAD
  (evidence in the plan's constitution check); 0004 and 0005 untouched by
  the whole family — `src/persist.rs` at zero diff, `src/store.rs` at the
  one-word `offload` visibility change plus its doc comment.

## The feature

The protected re-entry. Run 10's review hard-stopped the tree on a
security hold (the guard was Origin-only; the same-origin/no-`Origin`
read shape of a DNS-rebinding page rode straight through). The operator
repaired the findings at a4ff22c — the Host leg first in
`origin_guard` (`src/api.rs:307-342`), the shared
`authority_is_loopback_of` serving both legs (`:350-380`), `search_hog`
staging the dialect's own kind domain again, the record's false claims
deleted, 0030 §2 amended — and **the repair has never been read by a
review panel**. Run 10b re-runs implement, verify and review on the tree
as it stands: implement confirms (and repairs only defects), verify runs
the hermetic gates, review judges HEAD weighted against the ruled spec,
the amended 0030 and this contract. The feature is that contract itself —
including what nobody does: no redesign, no guard hardening, no new
artifact, no re-litigated finding.

## Why

1. **A security repair is unreviewed surface.** The Host leg is now the
   plane's single point of admission on every method and route; it went
   in *after* the panel that was supposed to read it had stopped. The
   re-entry is the review that repair is owed — not because the repair is
   suspected, but because the protocol's protection is precisely that a
   hard stop re-enters through review.
2. **The framing's rule binds the re-entry's direction.** The named HIGH
   and the MEDIUM/LOWs are repaired, each with its own test. Verify the
   repairs; do not re-litigate the pre-repair tree. New findings rule as
   always — the ruled shape is not a finding, and a repaired finding is
   not a new one.
3. **The artifact contract must hold, including the amended §2.** The
   design committed at 7550de3 must still be the tree; 0030 must still be
   `proposed`, indexed, and internally honest — §2 now derives the Host
   leg from the same-origin/no-`Origin` fact (0030:93-106) and carries
   the provenance paragraph naming the earlier draft's wrong premise on
   both counts (0030:127-132). A re-entry that finds the contract drifted
   is a different run than one that finds it held.
4. **A security re-entry's own failure mode is scope growth wearing the
   hold's respectability.** Every seat wants to *also* harden something.
   The contrarian cores of both positions are the load-bearing part of
   this spec, and they agree: the repair is the smallest shape that
   closes the named hole; the re-entry adds nothing the defects do not
   demand.

## The positions, ruled

Both positions independently confirm the artifact contract — re-executed
at `6f9a4bf` (the design commit), one commit past the a4ff22c they first
verified — the specs committed and unchanged, 0030 proposed and amended
in place,
`persist.rs` at zero diff, `store.rs` at the one-word change, zero new
`src/` files (the family's only added file is `benches/read_stall.rs`),
zero new dependencies (the `Cargo.toml` diff is the `[[bench]]` block),
no new env/flag, the repair inside the ruled shape at minimum size. Their
confirmations are adopted as the re-entry's floor. Where they conflict,
ruled here, not averaged:

| Conflict | Ruling | The loser's cost |
|---|---|---|
| The Origin-parse leniency note (robustness sentence 4: "the reasoning belongs in the record") vs. zero further edits to a record that was just amended (simplicity §5.1) | **Record it here, not in 0030.** The leniency reasoning is committed in this spec (§ The re-entry's ledger, below) and carried into review weighting; the sentence folds into 0030 §2 at the operator's acceptance pass (0028's landing precedent — amendments ride acceptance, corrections-in-place ride only a wrong premise, which this is not) | The decision text itself lacks the paragraph until acceptance. Mitigated: 0030 is `proposed`, so its text is still open at exactly the moment the sentence belongs there; meanwhile the reasoning is public, committed, and in review's inputs. |
| Zero-edit ideal (simplicity §5.5) vs. the defect path both positions name | **Zero edits is the goal, not a vow.** A defect found in the repair ships the smallest local fix beside its wire pin and re-runs the full gate set — the framing's price, budgeted, not a scope grant | The budget itself: every edit costs the six gates. The two cosmetic simplifications (`origin_guard`'s name lagging its body; `origin_is_loopback_of`'s ~6-line fold) stay **declined and recorded** — they wait for a defect that opens the file anyway. |
| Robustness's four-sentence ledger vs. simplicity's no-new-observables rule | **The ledger is sentences, not machinery — adopted as review weighting.** The machinery any sentence could be mistaken for (a refusal counter in `/metrics`, an allowlist knob, CSP/X-Frame-Options headers, an SMTP-plane extension) is **refused**: each is new vocabulary, a five-surface ripple, and permanent 100% coverage tax purchased by no acceptance line | Nothing — the refusal costs no sentence its content. What is lost is the *temptation* to make the ledger executable, and that is the point of ruling it now. |
| No confirmation-test twins (simplicity §5.4) vs. robustness's "verify the repairs" | **Verify by re-running the existing pins, not by writing siblings.** The rebind legs, the kind refusal and the gauge pins are named in AC2; implement re-runs them. A red pin is a defect (the path above); a green pin is the confirmation | A twin that "confirms" what a live pin proves is pure denominator under the exact gate — the loser here would be the 100/100/100 floor's honesty. |

**Corrected (position vs. code, code wins):**

- Robustness sentence 3's conditional — "any phrasing in the landing
  docs that says the loopback default *isolates parallel sessions*" —
  **does not bite at HEAD**: README:153-155 attributes parallel isolation
  to 0004's AUTH username and calls the loopback posture "the boundary"
  for the machine, never a tenant boundary. The correction is therefore
  carried as **review weighting for new claims** (AC4), not as a repair.
  The scoping sentence any future landing note must honor: *the loopback
  default defends against network-reachable clients — browsers and
  remote hosts; cross-session isolation remains 0004's per-test inboxes
  via AUTH plus fresh stores; a co-tenant process on the same loopback
  is inside the perimeter the operator drew.*
- Simplicity's §3 audit re-verified line-by-line and adopted as AC1's
  checks; robustness's §"confirmed in code" arms re-verified and adopted
  as AC2's pins. One asymmetry survived both: the `451 4.3.0 storage
  commit failed` arm at `smtp.rs:1045` is the commit-failure arm (0012),
  **not** a parse-panic arm — the parse-panic residual is 0030 §6's, its
  trigger named (an evidenced panic path becomes the 451 arm's own
  decision). Review weights that distinction; conflating them misstates
  which failures get a wire reply.

### Adopted at the re-entry panel (the positions written at `6f9a4bf`)

Both seats re-verified the contract at the design commit and passed.
Three items from their re-entry positions are adopted here — ruled, not
averaged — and none becomes code, a knob or a counter:

1. **The commit→250 window costs a duplicate on retry** (robustness,
   sharpening 1). The 250 is written only after `insert_async` resolves
   (`src/smtp.rs:1033-1050`), so a crash *after* the commit but
   *before* the 250 stores the mail and answers nothing: the peer sees
   a dead connection and retries — into a duplicate. The guarantee that
   makes `250` mean something is one-directional by design (`250 ⇒
   queryable`, never `stored ⇒ 250 seen`); dedup is the *sender's* job
   (message-id), the duplicate lands in the same inbox, and for a test
   realm that is the correct loss. At-least-once means exactly this. No
   server-side dedup and no idempotency arm — machinery purchased by no
   acceptance line.
2. **Durability is delegated, at its strongest setting** (robustness,
   sharpening 2). There is no `fsync`/`sync_all` anywhere in `src/`,
   and that is correct *because* `src/persist.rs:86-87` pins
   `PRAGMA journal_mode = WAL` and `PRAGMA synchronous = FULL`: the
   fsync happens inside every commit, under SQLite's journal. The
   sentence that must survive any persist edit: **accept==stored is
   only as durable as that pragma; anyone touching `persist.rs`
   re-derives 0001's disk leg or refuses the edit.**
3. **The intake framing copy rides the chief's commit** (simplicity §4,
   procedural). `specs/protected-reentry/framing-10b.md` was placed
   after the design commit and left untracked; the design-check probe
   (`scripts/design-check.sh:94`) counts untracked files in the spec
   dir and would read `fail` on it. The validation-family precedent —
   `specs/validation-family/` carries committed `framing.md` and
   `framing-9b.md` — answers it: the copy is committed in this
   synthesis commit and the checker is not edited (a design seat does
   not edit its own judge; plan risk 3). A run record, not a design
   artifact: the "no new artifact" rule is untouched.

Both sharpenings fold into 0030 at the operator's acceptance pass
(0028's precedent), riding the same channel as the ledger's leniency
and bound-port sentences; until then they are review weighting (AC4).
No decision document is drafted: the guarantee (0001) is unchanged —
these sentences record its one-directional shape and its disk leg, they
weaken nothing.

### Adopted at the third panel (the positions written at `81409d4`)

The design phase re-entered once more after the second operator repair
(`81409d4`, the signal-death fix). Both seats re-verified the artifact
contract at that HEAD and passed; neither proposes a change — the
re-entry still needs a review, not a redesign. One genuine conflict
remained for the synthesis to rule, and the losing side's cost is named:

| Conflict | Ruling | The loser's cost |
|---|---|---|
| Robustness: the record must carry two new sentences plus the rule the signal repair earned, and they must ride with the re-entry into review vs. simplicity: the only commit the design phase leaves behind is the framing copy ride | **Record them here — the `b2c95f3` shape, applied to the third panel.** Record sentences live in this contract, the one citable carrier the review weighting and the acceptance pass read; the intake framing copy rides the same commit. Two of robustness's four sentences (the duplicate-on-retry window and the pragma as 0001's disk leg, § Adopted at the re-entry panel items 1–2) are already carried — nothing new is owed for them | The re-entry's delta grows by the record lines themselves: one more synthesis commit, a dozen lines in a file no gate measures (`specs/` is outside the coverage report). What simplicity actually protects — zero new `src/` lines, no new observable, knob, test twin, dependency or decision — is untouched; the "zero commits" accounting gains exactly the synthesis commit AC5.1 already anticipates |

The third panel's record items, adopted as sentences and nothing else —
no code, no knob, no counter, no new test:

1. **A `250` is true when written; how long it stays true is the
   operator's cap** (the 250-then-pruned window — new on the record, and
   not a violation). `insert` applies and releases the boundary before
   the `250` is written, so a racing capped insert can evict the
   just-accepted mail before the client has finished reading its own
   reply. 0001 promises queryable at the boundary, not a retention SLA;
   0003 makes eviction deterministic (oldest-only) and counted
   (`emails_dropped`). The design refuses to "fix" this by widening the
   cap or by blocking the reply on not-yet-evictable — either invents
   accept-path semantics. The sentence belongs wherever the cap is
   documented (0003's ruling, the operator docs); it folds at the
   operator's acceptance pass like the other record sentences.
2. **A poisoned boundary dies loudly, never quietly retries.** The
   mutation lock's `.expect("store mutation lock poisoned")` is 0012 §7's
   fail-stop (as amended by 0015 §3: restart is the recovery) — this
   restates the posture rather than amending it: a boundary that has
   violated its own invariant must not pretend the store is healthy and
   keep answering recoverable `451`s. Confirmed: no *reachable* path
   poisons it — the `expect`s are stated invariants; a reachable one
   rules as a new finding, never a smuggled repair.
3. **Arm every crash/termination surface before the wire starts
   answering** — the rule the `81409d4` repair earned. The signal
   streams register on their *first poll*, and the fast-path binds
   completed without a yield, so a spawned watcher could stay unpolled
   until after the HTTP bind; a signal landing once the port answered a
   connect killed the process at the default disposition. The repair
   constructs and polls the streams on the main task before `run_on` —
   a reorder, not a machine: one file, net +10 lines, `std::future::poll_fn`,
   zero dependencies, zero new tests (the existing pin
   `serve_answers_healthz_and_stops_on_sigint`, `tests/binary.rs:204`,
   is the exact repro, and the five signal legs — `tests/binary.rs:207`,
   `:232`, `:263`, `:285`, `:509` — execute the registration lines on
   every workspace run, which is how the exact 100/100/100 gate holds
   with no new measured line). It is the same ordering discipline as
   commit-before-`250`: the state that must exist before the first
   observable act is made to exist, structurally. Any future listener,
   handler or stream that registers lazily on first poll reopens the
   class; the answer is the same reordering, never a sleep.

Everything else the third panel raised is already on the record or is
re-refused: the accepted losses (at-most-once webhooks, no global
backstop, a dribbling peer holding one session slot, no NotAfter
warning) are 0027/0029/0030 §6's landed postures; the dispatcher cannot
hold the store hostage (owned payloads built after the budget check,
per-sink `WEBHOOK_DELIVERY_CAP = 32`, the stop check first in the `Mail`
arm) is confirmed, not changed; and the refusal list stands — no guard
hardening, no confirmation-test twins, no spec v2, no `0031`, no
re-litigation of the pre-repair tree or the SIGINT diagnosis, no new
bench, seam, model route or gate, and the two declined renames (AC5.4)
stay declined.

### Adopted at the fourth panel (the positions written at `95d2823`)

The design re-entered once more after run 10d's review closed the hold
and the operator landed its residual (`95d2823`: `search_needle` refuses
an uncompilable needle with a 400 instead of unwinding, the
Origin-guard comment states the true layer order, 0030 §2 drops the
false h2/`:authority` claim). Both seats re-verified the artifact
contract at that HEAD and passed; neither proposes a change — the
re-entry still needs a review, not a redesign. The third repair is
audited as a **signature, not a machine**: one signature (`Regex` →
`Option<Regex>`, `src/compat.rs:409`), one caller mapping
(`ok_or(ApiError(..))`, `:390`), one inline pin (`:847`), one bench
line — no new file, seam, knob, observable, error vocabulary or
dependency; the engine's own compiled-size budget is the bound, rightly
not re-invented (no `SWARMAIL_REGEX_BUDGET`, no needle-length cap, no
needle cache, no `Result` alias — each a second rule where the regex
crate already enforces the truer one: the cost is the compiled
automaton, and only the engine measures that). The re-entry's delta is
still exactly the ruled artifacts plus three named, pinned repairs.

Four items are adopted here — ruled, not averaged — and none becomes
code, a knob, a counter or a new test:

1. **The 400 arm is executed but unasserted on the wire: named, priced,
   and handed to review — not adopted as a design addition.** The
   refusal is pinned at its unit (`src/compat.rs:847`: an uncompilable
   250 000-character needle yields `None`), but its *translation* —
   400, not unwind — has no wire test: no test sends an uncompilable
   needle (the largest `repeat(` fixture in `tests/` is 20 000 bytes,
   in POP3), and llvm-cov emits no branch region for the `ok_or` arm,
   so the exact gate cannot see it — a future edit restoring
   `.expect()` would pass every gate silently. The price is already
   computed: **one ~8-line wire test in `tests/compat.rs`**
   (`GET /api/v1/search?query=` + 250 000 `k`s → `400`), real protocol
   against a real server (0020 rule 3), no new file or seam.
   **Ruling:** the design phase does not add it. The framing prices
   this run per edit and hands new findings to review; the residual is
   run 10d review's own record and its diagnosis is closed, so a
   design-demanded pin would be the design phase inventing the demand —
   a quarter-megabyte URI fixture in every suite, purchased by nobody's
   finding. If review names it a finding, the priced test lands beside
   it through AC4.3's channel — full gates, nothing more. **The
   loser's cost:** until then the arm stays unasserted and
   regression-invisible to the gate; that risk is accepted for this
   run, witnessed by this sentence, and one review finding away from
   repair.
2. **The record's webhook summary is corrected against the code: retries
   exist, so "at-most-once" is false as written** (the third panel's
   sentence above; corrected here — code wins). `deliver` runs four
   attempts with exponential backoff (`src/webhook.rs:497-499`), so a
   POST whose success response is lost can deliver twice. The honest
   sentence (the fourth panel's, adopted): *webhooks are at-least-once
   within the attempt budget — a lost success-response can duplicate —
   with every loss counted (`exhausted`, `dropped`, `aborted`) and
   never replayed across a restart; deliveries are not part of
   accept == stored; the store is the truth, the webhook a
   notification.* **The loser's cost:** the imprecise summary teaches
   both wrong mental models at once — it hides the duplicate window
   and hides the counted-loss channels a tester would otherwise watch.
   The correction folds into 0030 at the operator's acceptance pass
   with the other record sentences.
3. **The serialized boundary is the throughput ceiling, and the tree
   does not batch** (the fourth residual sentence, new to the record).
   A thousand parallel sessions accept at commit speed, not at core
   count — the price of the one ordered mutation boundary (0014) for
   0001, and the correct price: every alternative (batching,
   per-inbox writers, an async gap) reorders or exposes pre-commit
   state, and both are guarantee-killers. Sentence only; folds at
   acceptance.
4. **The lazy-registration rule reads like a guarantee in review's
   inputs.** The `81409d4` rule (§ Adopted at the third panel, item 3)
   gains its reading instruction: review reads for un-armed death
   surfaces — a new signal, listener or spawn that registers lazily on
   first poll — the way it reads for 0001. Carried into AC4.1 and the
   plan's weighting; no new mechanism.

Procedural, resolved the house's way a third time:
`specs/protected-reentry/framing-10b-successor.md` — this run's
successor intake framing, written after `c190493` — is untracked, and
the design-check probe counts untracked paths in the spec dir
(`scripts/design-check.sh:94`). It rides this synthesis commit per the
validation-family precedent (adopted item 3's own shape); the checker
is untouched — a design seat does not edit its own judge (plan risk 3).
The `GIT_CONFIG_KEY_0` env break recurred a fourth time (operator note
§2) and did not bite: every probe in this synthesis ran under a working
env.

Re-refused a fourth time, and one nit declined: no guard hardening, no
confirmation-test twin, no spec v2, no `0031`, no new bench or
re-benchmark (`read_stall.rs`'s delta is the signature change; numbers
are report, never an assertion — 0023), no new seam, `coverage(off)`,
model route or gate; `ok_or` stays `ok_or` — the eager short-string
allocation is dwarfed by the fold's own regex build a line below, and
an `ok_or_else` closure is vocabulary, not clarity; the two declined
renames stay declined (`95d2823` opened neither file). Zero production
lines and zero tests remain the design phase's count, fourth attempt
running.

## The re-entry's ledger — the four sentences the guard's own surface carries

The Host leg is new production surface; from this re-entry it gets the
same treatment as everything else — each residual answered or recorded.
None of these four becomes code, a knob or a counter:

1. **Exotic-but-real loopback spellings eat a 403.** A client dialing a
   vanity name resolving to 127.0.0.1, a trailing-dot FQDN, or
   zero-padded octets is refused with its sentence. Fail-closed is the
   contract (the dangerous direction — over-admission — is the one the
   parser at `src/api.rs:350-380` does not have: unterminated brackets,
   wrong/missing port and abbreviated IPv4 all refuse); the refusal
   names the fix (`SWARMAIL_HTTP_LISTEN`, or an honest spelling). The
   same sentence covers the HTTP/1.0-shaped client that sends no `Host`
   at all — 0030 §2 records that such a client is not HTTP/1.1.
2. **The guard is keyed to the actually-bound port; any future bind
   change must carry that fact through.** Today the configured and bound
   ports are the same fixed literal (`src/lib.rs:190`), so the Host leg
   keys to the truth — but nothing in the type system witnesses it. This
   sentence is the witness: a future ephemeral/remapped bind either
   refuses everything (legible) or admits a non-plane spelling (silent,
   wrong). Recorded here for review weighting; folds into 0030 §2 at
   acceptance with the leniency note.
3. **The loopback default is a network boundary, not a tenant boundary**
   (the scoping sentence above). Weighted into review; landing notes are
   held to it.
4. **The Origin parse is lenient in the admitting direction, and that is
   fine — because it cannot widen.** `origin_is_loopback_of`
   (`src/api.rs:384-392`) splits the authority at the first `/`, so
   `Origin: http://localhost:8025/evil` is admitted as
   `localhost:8025`. No widening follows: admission only helps a client
   that already names the plane, and a non-browser that wants to spoof
   the plane simply omits `Origin` — the leg passes on absence. The
   threat model is a browser page, whose `Origin` is set honestly. A
   future hardening pass that "fixes" the leniency would be fixing
   nothing observable.

## The design

- **Implement is a confirmation seat.** Tree == ruled spec == repair.
  Its only lawful write is a defect repair (the ruled conflict #2 path):
  smallest local fix, its wire pin, the full gate set. A decision
  document is owed only if a defect touches a frozen guarantee or the
  posture itself.
- **Verify is hermetic and prices nothing new.** The six gates; SIGBUS →
  disk-backed `TMPDIR`; the piped `smtp=`/`http=` startup line must
  parse in any capture (0029 §4 — the ANSI guard is a wire-honesty item,
  not a convenience). Benches are not re-run: `read_stall.rs` exists,
  relabeled, and numbers are a verify/report activity, never an
  assertion.
- **Review judges HEAD** weighted against the ruled spec, the amended
  0030, and this contract: the four-sentence ledger above as the guard
  surface's checklist; the tenant-boundary scoping applied to every new
  claim; the 451/parse-arm distinction; the two 0001-durability
  sentences (§ Adopted at the re-entry panel); **no re-litigation** of
  the pre-repair findings (Origin-only guard, wide kind domain, false
  `maximum`, gauge "never existed" claims); new findings rule as always.
- **No new decision, no new spec for the family.** `docs/decisions/`
  ends at 0030; the re-entry's corrections fold at its acceptance. The
  only committed artifacts of this run's design phase are this spec
  trio and the intake framing copy that rides it (§ Adopted, item 3 —
  a run record, per the validation-family precedent).
- **Ship as usual:** landing is the operator's (PR, both duplicate CI
  runs green, squash). Seats sign false in this env (pinentry
  unreachable); the commit records it, as a4ff22c and c5f2a1f did.

## Acceptance Criteria

### AC1 — the artifact contract holds at the reviewed HEAD

1. `git diff 7550de3 HEAD -- specs/off-worker-parses-loopback-posture/`
   is empty — the ruled spec is unchanged since its design commit.
2. `docs/decisions/0030-loopback-plane-bounded-results.md` line 3 reads
   `- **Status:** proposed`, the file is indexed in
   `docs/decisions/README.md`, §2 derives the Host leg from the
   same-origin/no-`Origin` fact, and the provenance paragraph
   (0030:127-132) is present.
3. `git diff 66a45bb HEAD -- src/persist.rs` is empty, and
   `git diff 66a45bb HEAD -- src/store.rs` shows exactly the
   `pub(crate)` visibility change on `offload` plus its doc comment.
4. No new file under `src/` exists that the family did not ship:
   `git diff 66a45bb HEAD --stat -- src/ Cargo.toml` names only
   pre-existing files, and Cargo.toml's only delta is the `[[bench]]`
   block. No new env var or flag: `src/config.rs`'s diff is the three
   loopback literals and their doc comments.

### AC2 — the repairs are proven by their existing pins, re-run, not rewritten

1. `cargo test --test api` — the three rebind legs
   `cross_origin_and_malformed_spellings_are_refused_before_the_router`
   (`tests/api.rs:1295`),
   `a_rebound_host_refuses_even_without_an_origin`
   (`tests/api.rs:1349`), `an_off_loopback_bind_leaves_the_guard_inert`
   (`tests/api.rs:1431`) go green: the rebind dies on both shapes
   (attacker `Host` with no `Origin` refused; loopback spellings pass;
   off-loopback binds inert on both legs).
2. `cargo test --test compat` — `mailhog_v2_lists_and_searches_with_upstream_shapes`
   pins `kind=subject` as a bare 400 (`tests/compat.rs:536-540`): the
   engine's kind is not MailHog's.
3. `cargo test --lib` — the awaits-gauge inline pins
   (`src/api.rs:1616-1630`) hold: a live wait renders its
   `swarmail_awaits_active` row; a quiescent inbox renders none — the
   observable claim, not "never existed".
4. `cargo test --test config_cli` pins the three loopback defaults; every
   raw-HTTP leg rides `tests/common/mod.rs`'s honest `Host: {addr}`
   harnesses (the repair restoring behavior, not changing it).
5. No new test asserts what an existing pin already proves; a red pin is
   reported as a defect, never papered with a sibling.
6. `cargo test --lib` — the search-needle pin (`src/compat.rs:847`)
   holds: an uncompilable 250 000-character needle yields `None` (the
   `95d2823` repair's own pin), and the call site answers `400`
   (`:390`). The unit refusal is asserted; the wire translation of it
   is AC4.1's named residual, review's to take as a finding.

### AC3 — the gates are green at the reviewed HEAD, and the wire stays honest

1. `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets
   --locked -- -D warnings`; `cargo test --workspace --locked`;
   `brokkr compile --bundle bundles/self`; `bash scripts/coverage-gate.sh`
   (exact 100/100/100, missed == 0); `cargo deny check licenses` — all
   exit 0.
2. The piped serve startup line parses in any capture: `smtp=`/`http=`
   extracted by `tests/binary.rs`'s `startup_addrs` (`:50-54`), ANSI only
   when the target stream renders it (0029 §4).

### AC4 — review rules on HEAD with the carried corrections, and only on new findings

1. Each review member's inputs cite the weighting: the four-sentence
   ledger, the tenant-boundary scoping, the bound-port invariant
   sentence, the 451/parse-arm distinction, the at-least-once
   duplicate-on-retry window and the pragma-as-disk-leg sentence (§
   Adopted at the re-entry panel), the no-re-litigation rule for
   the pre-repair findings, and the fourth panel's items (§ Adopted at
   the fourth panel): the corrected webhook delivery sentence (retried
   within the budget; losses counted; never replayed across a restart),
   the serialized-boundary-as-throughput-ceiling sentence, the
   death-surface rule read the way 0001 is read, and the priced
   400-arm wire residual — review's to take as a finding, the ~8-line
   `tests/compat.rs` test pre-priced beside it.
2. Every claim review endorses is one the wire can observe (the
   awaits-gauge discipline): any landing-note sentence about the loopback
   default scopes it to network-reachable clients and leaves
   cross-session isolation to 0004.
3. A new finding lands through exactly one channel: a code repair beside
   its pin, a doc repair beside its gate run, a residual sentence (folded
   into 0030 at acceptance or recorded here), or the operator-note
   channel (`.brokkr/state/run10b-operator-edits.md`). No finding is
   carried only in the journal.

### AC5 — the re-entry's own delta is zero unless a defect demands otherwise

1. At ship, `git diff a4ff22c HEAD --stat` names only this contract's
   own spec artifacts (the design commit `6f9a4bf` and the synthesis
   commits — `b2c95f3`, `c190493`, and this one) and defect-driven
   repairs, each with its named pin in the same commit (to date:
   `81409d4`'s signal reorder pinned by
   `serve_answers_healthz_and_stops_on_sigint`; `95d2823`'s refusal
   pinned at `src/compat.rs:847`) — no production line outside a
   defect's fix.
2. `git diff a4ff22c HEAD -- Cargo.toml` is empty unless a defect
   demanded a new dependency — which would be a decision document, not an
   edit. `docs/decisions/` still ends at 0030 (no fork of the record).
3. No new bench, no new seam, no `coverage(off)`, no confirmation-test
   twin, no hardening knob exists that AC2's pins did not already prove.
4. The two declined renames are still declined: `origin_guard` keeps its
   name and `origin_is_loopback_of` stays the thin scheme-strip — recorded
   here as the price of a zero-delta re-entry, payable only when a defect
   opens the file anyway.
