# Spec — prove and ship the fidelity batch at `791f83d`: the receipt, the record truth, and the spine (issues #39, #54, #55, #56, #57)

- **Run:** run 11 (`brokkr-state-run11-feature-txt-6578a517`) · **Phase:**
  design — the chief architect synthesizing the robustness and simplicity
  positions · **Feature slug:** `fidelity-prove-ship-receipt`
- **Issues:** [#39](https://github.com/feedback-loop-ai/swarmail/issues/39)
  (extraction fidelity, P1), [#54](https://github.com/feedback-loop-ai/swarmail/issues/54)
  (restored state matches live state), [#55](https://github.com/feedback-loop-ai/swarmail/issues/55)
  (Mailpit/MailHog compat fidelity), [#56](https://github.com/feedback-loop-ai/swarmail/issues/56)
  (thread normalization + web UI), [#57](https://github.com/feedback-loop-ai/swarmail/issues/57)
  (documentation and comment drift).
- **Base:** HEAD `791f83d` — the operator's five-residual repair ("Prove
  the html leg against realistic mail and align four record sentences"),
  on top of the record-honesty contract `4b482e3` and the review-hold
  repair `e245fae`. Verified first-hand this session at `791f83d`: the
  full suite reports **470 passed, 0 failed, 1 ignored** (`tests/rate.rs`'s
  opt-in throughput smoke), and the repair is exactly four files, +37/−6 —
  `src/extract.rs` (+27/−1: the inline-adjacent pin
  `inline_adjacent_codes_behave_as_the_renderer_renders` plus five lines
  of `///` on `extract_codes`), `README.md` (+2/−1),
  `docs/decisions/0031` (+6/−2), `src/static/openapi.json` (+2/−2) — no
  `Cargo.toml`, no lockfile, no `deny.toml`, no new file, no new route.
- **Design of record:** [specs/fidelity-closures/spec.md](../fidelity-closures/spec.md)
  (committed at `2cfe08c`, amended at `24df038`) — unchanged by this pass
  except the two record-truth edits in §The amendment set.
- **Contract of record:** [specs/fidelity-closures-prove-ship/spec.md](../fidelity-closures-prove-ship/spec.md)
  with its [plan](../fidelity-closures-prove-ship/plan.md) and
  [tasks](../fidelity-closures-prove-ship/tasks.md) — amended by this
  pass (the amendment set); red lines 1–14 stand verbatim, this pass adds
  15–16.
- **Decisions:** [0031](../../docs/decisions/0031-sender-honest-stored-bodies.md)
  and [0032](../../docs/decisions/0032-emptied-inboxes-are-removed.md)
  still **proposed**, implemented by the tree, awaiting the operator.
  [0033](../../docs/decisions/0033-one-server-per-data-file.md) is minted
  by this pass — **proposed and deliberately not implemented**: it rules
  the one failure mode no seat had named, and its landing is the
  operator's.

## The problem

The tree is finished — the phase machine re-ran design a third time, and
the framing is explicit: this run proves and ships the committed tree;
its edits are not redone or re-argued. The question this pass answers is
the one both predecessor passes answered, one repair later: **what does
design owe a repair it is forbidden to re-litigate?** The two positions
partly disagree on the answer, and their disagreement is the work:

- The simplicity seat holds the tree owes nothing and the record owes
  four edits: the repair made three committed sentences of this run stale
  or false (the 469 count, the unnamed new pin, the Base tree `e245fae`,
  and a null-surface sentence the operator's own 0031 amendment made
  false). It prices the synthesis as the receipt again, carrying the
  amendment set and nothing more.
- The robustness seat binds the spine — one wait, try-shaped bounds,
  one rule per path — and names what nobody had: **nothing refuses a
  second server process on one data file**. It also corrects its own
  prior claim (an expired supplied cert does *not* refuse at startup;
  expiry is client-visible) and refines its own hostile-body demand (pin
  linearity structurally; the bench family is the proof home, not the
  gate).

Both are right, and neither is complete. The receipt *is* the synthesis —
but a receipt that silently carries a false sentence is the #57 disease
inside the machine's own artifact, and a spine with an unnamed failure
mode is not bound. This pass mints the receipt, repairs the record
inside the owning artifacts (Ruling 2 of the record-honesty pass, applied
here), and files the newly named failure mode as a proposed decision
rather than a tree edit the phase machine has no seat to land.

## The positions, ruled

1. **The synthesis is the receipt; the tree owes nothing.** Adopted from
   simplicity. No fourth tree change; 470 is the suite verify proves; any
   new test is a new repair with its own record trail, not verify's
   initiative. *The letter of "no new spec, no new decision" is refused*:
   the same refusal in the last two passes was overruled for the same
   reason — the machine's receipt is a committed artifact, and a newly
   named failure mode earns a ruling (Ruling 3).
2. **The drift is repaired inside the owning artifact.** Adopted
   (simplicity carrying the record-honesty Ruling 2): the four edits land
   in `specs/fidelity-closures-prove-ship/`, `specs/fidelity-closures/`
   and `specs/fidelity-record-honesty/` in this commit, not as
   restatements; git history preserves the prior state. The amendment
   set is exactly §The amendment set — nothing else in those files
   changes.
3. **The two-servers hole is real, named, and filed — not landed.**
   Adopted from robustness, verified first-hand (`src/persist.rs` has no
   `locking_mode`; `:88` takes any `Connection`). The refusal is
   *specified* in [0033](../../docs/decisions/0033-one-server-per-data-file.md)
   (proposed) and **routed to the operator** — the arc's every repair
   has been an operator landing, and this run's remaining phases prove,
   rule and close; they do not edit. *The chief rules the mechanism on
   the full bill the positions did not price:* `PRAGMA
   locking_mode = EXCLUSIVE` breaks the 0001 proof family itself —
   `tests/common/mod.rs::hold_write_lock` (`BEGIN IMMEDIATE`, stalling a
   live commit at `tests/lifecycle.rs:447,568`), the mirror reads
   (`:505,520,621,849`), and the vandal (`src/persist.rs:341`) are all
   second connections to a live store. The sidecar `File::try_lock`
   (std, 1.89+) refuses exactly the second *server* and touches none of
   it. **Cost of ruling 0033 instead of landing it now:** until the
   operator accepts, a second server on one data file diverges
   semantically (deleted-or-pruned rows resurrected from a peer's stale
   memory on restart, per-process seq, prune plans from partial views —
   the hazards 0015 §4 names) while SQLite serializes the bytes — the decision
   states this honestly, and no artifact claims a refusal the tree does
   not make. **Cost of refusing robustness's "refuse now" outright:**
   the failure mode stays unnamed in the record, which is how it
   survived four design passes.
4. **The cert-expiry claim is corrected; no dependency is bought.**
   Adopted from robustness. The prior position's "an expired supplied
   cert refuses at startup" is refused: `TlsConfig::server_config()`
   validates structure, not time — an expired supplied pair binds and
   serves, and the failure surfaces at each client handshake
   (`src/tls.rs` names no time check; verified). This is now a record
   sentence (red line 16): no README or module doc may claim startup
   time validation, and the check arrives only through a decision that
   also buys its dependency. **Cost of refusing the `x509-parser`-class
   check until a real operator case exists:** an expired supplied cert
   serves until a client's handshake fails — loud at the client, silent
   at the server; acceptable for a loopback test sink, and revisitable
   by one defensible sentence when an operator case appears.
5. **The hostile-body invariant stays pinned structurally; the bench
   family is its proof home.** Adopted from robustness as red line 15:
   the 40-byte window, the single bounded scans, decode-on-the-match,
   and the 500,000-`&` output pin are the mechanism; no cap-sized
   (50 MiB) hostile-body wire test enters the always-on suite; if
   continuous proof is wanted it is a bench beside
   `benches/writer_stall.rs`, never a gate tax. This refines the prior
   position's 50 MiB wire-test demand — the same refusal direction the
   record-honesty pass's Ruling 3 took against wall-clock latency
   assertions.
6. **The five closure refusals are pinned as sentences.** The closures
   are implemented; what the committed spec must carry is the refusal
   each one made, so no later change quietly reopens it: #39 — decode
   character references on the match only, first-occurrence order while
   deduplicating, no harvesting from CSS/attributes, stored bodies are
   only what the sender sent (0031); #54 — `next_seq = MAX(seq)+1`,
   total `(seq, rowid)` restore, one emptied-inbox rule on every path
   (0032), honest seq gaps; #55 — the `IDs` alias is spelling-only,
   envelope + header union is the one recipient source, named `400`s;
   #56 — thread keys derive per view request (0018 §5) with the fixed
   precedence (References/In-Reply-To/Message-ID chains first; the
   normalized subject — `Re:`/`Fw:`/`Fwd:`/`Aw:` stripped,
   Unicode-lowercased; full case folding a recorded limitation — as the
   fallback for the chain-less mails), the UI orders by
   `received_ms DESC, id`; #57 — no doc sentence promises what
   the code refuses to do, including the two this pass binds (single
   server per file [0033, unlanded], cert expiry client-visible).

## The spine: the six failure modes and the seventh, with the honest losses

The framing's failure modes, each answered by the verified design and
priced with what it honestly loses (robustness's line-level account,
endorsed by this pass):

- **A crash between accept and commit.** Commit before visibility: the
  mirror transaction completes inside the mutation boundary before the
  memory phase and before any reply; a failed commit refuses whole (no
  inbox minted, no counter moved, no watcher woken); a session parked at
  its insert still lands its commit. *Lost:* at-least-once — an accepted
  mail whose reply never leaves lands in the restart copy; the client
  retries into a duplicate, and the honest seq gap marks the boundary.
- **A hanging webhook.** `POST_TIMEOUT = 5s`, four attempts, at most 32
  deliveries per target, drops counted in the ledger; the accept path
  never waits on a target. *Lost:* no durable outbox — a target down
  past its budget loses the attempt, observably (`dropped`), never
  silently.
- **A cap race under burst.** The eviction plan is computed from a peek
  inside the boundary and applied in the same transaction; an over-cap
  insert *accepts and evicts oldest* — the store never refuses on count
  (`pop_front` in commit order, the multi-evict catch-up loop; proven by
  `insert_prunes_oldest_beyond_cap`, src/store.rs:864). *Lost:* an
  over-cap restored inbox is served until its first insert (0003's
  letter), and a burst sees its oldest mail vanish under a slow waiter —
  the waiter's timeout is honest (`Timeout { matched }`), the cap means
  "the newest N survive".
- **Chaos mid-pipeline.** Chaos fires after DATA completes, envelope
  consumed; delays are bounded by `MAX_DELAY_MS` with probabilities at
  the poles exact; a chaos-dropped mail is indistinguishable from an
  unwritten one. *Lost:* chaos is wire-only — it cannot observe or
  perturb the commit itself, so the gate proves refusals, not commit
  interleavings.
- **Hostile DATA.** DATA is bounded by the message-byte allowance
  (`cfg.max_message_size`): a line over the remaining allowance is
  drained unbuffered and the whole mail is refused `552 5.3.4 Message
  too big` at the terminator, nothing stored (src/smtp.rs, the DATA
  arm). `MAX_COMMAND_LINE = 4096` is the *command-line* byte cap (SMTP
  and POP3 alike), answered `500 5.5.2 Line too long` with the line
  drained unbuffered — it never gates DATA. Extraction is linear by
  construction (the 40-byte window, single bounded scans, decode on the
  match only) and codes come from rendered text only — a code the
  renderer glues across an `</a>` close is refused, never invented.
  *Lost:* one linear parse on the shared blocking pool delays other
  mutations briefly; it cannot starve them (the pool's other workers
  and the mutation boundary hold).
- **A cert under test.** Certs are minted fresh per boot; a supplied
  pair is parsed before bind (structure only — red line 16); the CA
  client cap is an LRU of 32 for attacker-chosen roots. *Lost:* expiry
  is client-visible, not startup-refused — see Ruling 4.
- **The seventh, now named: two servers on one data file.** Nothing
  refuses it today; the bytes survive, the semantics diverge. Answered
  by [0033](../../docs/decisions/0033-one-server-per-data-file.md)
  (proposed, unlanded) — *lost, until the operator lands it:* the
  divergence itself.

## The semantic change

One decision is minted by this pass:
[0033 — one server per data file](../../docs/decisions/0033-one-server-per-data-file.md),
**proposed, not implemented by the tree**. It rules the data-file
concurrency contract (a ruling and a public contract in the task's
sense, hence a decision document), specifies the sidecar-lock refusal,
the in-process deterministic test, the exact contract sentences to land,
and the named alternative it refuses (EXCLUSIVE locking, with the 0001
proof-family bill). Acceptance — and the single landing commit of
refusal + test + sentence — is the operator's. No other ruling here
amends a standing decision; 0031 and 0032 are untouched.

## The design of record and the red lines

The contract's red lines 1–14 stand verbatim (its §Red lines plus the
record-honesty pass's additions: `REF_MAX` stays a named local; the pin
stays an inline output assertion; no parse timer; restore stays before
bind and outside `insert`; the openapi 503 states 0014's per-mutation
refusal). This pass adds two, both true of the tree today:

- **15. The hostile-body invariant is pinned structurally, and the bench
  family is its proof home.** The 40-byte window, the single bounded
  scans (`URL_RE`, `CODE_RE`), the decode-on-the-match rule, and
  `an_ampersand_run_decodes_in_bounded_work`'s 500,000-`&` output pin
  are the mechanism. No cap-sized (50 MiB) hostile-body wire test enters
  the always-on suite, and no wall-clock latency assertion returns; if
  continuous proof is wanted, it is a bench beside
  `benches/writer_stall.rs`, never a gate tax.
- **16. Supplied-cert time validity is client-visible, by record.**
  `TlsConfig::server_config()` validates structure, not time: an expired
  supplied pair binds and serves, and the failure surfaces at each
  client handshake. No README or module-doc sentence may claim the
  startup validates `not_after`; such a check arrives only through a
  decision that also buys its dependency — and that decision needs a
  real operator case first.

## The amendment set (applied to the owning artifacts in this commit)

The five-residual repair made three committed sentences of this run
stale or false. Each is repaired in its owning artifact, exactly and
only:

1. **The count: 469 → 470.** `specs/fidelity-closures-prove-ship/spec.md`
   AC-G.1 and `tasks.md` task 1; `specs/fidelity-record-honesty/spec.md`
   AC-1 — whose proof clause is rewritten to stay truthful (this pass's
   run at `791f83d` reports 470; the record-honesty pass's own run at
   `e245fae` reported 469, and the html-leg pin moved the count).
2. **The pin, named.** AC-39.4's proof list in both the design of record
   and the contract gains
   `inline_adjacent_codes_behave_as_the_renderer_renders`, with the
   narrowing clause: the renderer's recorded narrowings are pinned —
   `<b>`/`<span>` text extracts with its source whitespace; a code the
   renderer glues across an `</a>` close is refused, never invented.
3. **The Base tree.** The contract's Base names `791f83d` as the tree
   the remaining phases prove (470 tests there), with the pointer to
   this receipt for the amendment set; the `e245fae` sentence becomes
   past tense, not deleted.
4. **The null-surface clauses.** AC-39.6 (design of record) and AC-39.5
   (contract) state the surface truth the operator's 0031 amendment
   already recorded: the native REST and MCP planes keep the null; on
   the Mailpit and MailHog compat shapes an absent part is an **empty
   string** (`Text`/`HTML` map through `unwrap_or_default()` at
   `src/compat.rs:669–670`; MailHog's `Body` is one string,
   text-else-html, at `:757`) — a client there cannot distinguish "no
   part" from "empty part".

## Acceptance Criteria

### The gates (re-grounded at `791f83d`)

- **AC-G.1** `cargo fmt --all -- --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace --locked`,
  `brokkr compile --bundle bundles/self` all exit 0; the suite is 470
  tests. *Proof:* the commands themselves — run this session at
  `791f83d` (470 passed, 0 failed, 1 ignored).
- **AC-G.2** `TMPDIR=/operator-scratch/cov-tmp bash
  scripts/coverage-gate.sh` reports exact 100/100/100, missed == 0, no
  `coverage(off)`. The TMPDIR is absolute — the gate is never run on
  tmpfs, and never run twice without need (red line 10). *Proof:*
  verify's recorded output.
- **AC-G.3** `cargo deny check licenses` exits 0 with only the recorded
  CDLA-Permissive-2.0 exception. *Proof:* the command.
- **AC-G.4** The landed shape is the landed shape. *Proof:*
  `git show 791f83d --stat` is exactly four files, +37/−6 (`src/extract.rs`,
  `README.md`, `docs/decisions/0031-…`, `src/static/openapi.json`);
  `git diff --name-status --diff-filter=A 2cfe08c..24df038` is empty;
  `grep '^percent-encoding' Cargo.toml` shows the one declared
  dependency; `grep -rc 'coverage(off)' src/` is 0.

### The tree this pass binds

- **AC-T.1** The html-leg pin is real, inline, and an output assertion:
  `cargo test --lib extract inline_adjacent_codes_behave_as_the_renderer_renders`
  is green; the body asserts exact output (`<b>`/`<span>` extract with
  their source whitespace; `<a>go</a> 555555` renders glued and is
  honestly refused), no duration, no fixture generation (0020 rule 1).
  *Proof:* the test body.
- **AC-T.2** The count is 470, verified first-hand. *Proof:*
  `cargo test --workspace --locked` — 470 passed, 0 failed, 1 ignored
  (`tests/rate.rs`'s opt-in smoke); re-recorded by verify under AC-G.1.
- **AC-T.3** The shape claims hold: `REF_MAX` is a named local `const`
  in `parse_ref` (`src/extract.rs:78`); exactly three render sites
  (`src/compat.rs:560, :801`, `src/extract.rs:155`); one declared
  dependency. *Proof:* the greps.
- **AC-T.4** The frozen three are re-proved by the standing suites,
  unchanged: accept == stored (`tests/burst.rs::the_burst_guarantee`,
  `::a_cap_sized_data_is_stored_as_soon_as_the_250_leaves`),
  subscribe-before-check (`tests/burst.rs::await_waits_for_arrival`
  family), oldest-only pruning across a restart
  (`tests/persistence.rs` cap legs). *Proof:* those suites.

### The record truth (the amendment set)

- **AC-R.1** The contract of record says 470 in AC-G.1 and tasks.md
  task 1, names the pin in AC-39.4's proof list with the narrowing
  clause, and its Base names `791f83d` with the receipt pointer; the
  `e245fae` sentence is past tense. *Proof:* greps of the committed
  files.
- **AC-R.2** The design of record's AC-39.4 names the pin, and AC-39.6
  states the surface truth (nulls on REST/MCP, empty strings on the
  compat shapes). *Proof:* read of the committed file.
- **AC-R.3** The record-honesty spec's AC-1 states 470 with a truthful
  proof clause. *Proof:* read of the committed file.
- **AC-R.4** Nothing else in the three owning artifacts changes.
  *Proof:* `git show --stat HEAD` — the amendment edits are the only
  changes to those files.

### The spine (the rulings)

- **AC-S.1** [0033](../../docs/decisions/0033-one-server-per-data-file.md)
  exists, is status `proposed`, states the tree does **not** yet refuse a
  second connection, names the sidecar mechanism, the in-process test,
  the contract sentences, and the refused EXCLUSIVE alternative with the
  0001 proof-family bill. *Proof:* the file; `grep -n locking_mode
  src/persist.rs` is empty.
- **AC-S.2** No committed artifact claims the refusal is landed: the
  spec and the decision both say "proposed, not implemented", and the
  spec's failure-mode list carries the honest current-tree sentence.
  *Proof:* the two files.
- **AC-S.3** The corrected cert sentence is in the record and binds
  (red line 16); no README or module doc claims startup time validation.
  *Proof:* `grep -n 'not_after\|expire' src/tls.rs README.md`.
- **AC-S.4** Red lines 15–16 are stated in this spec and referenced from
  the contract's Base; the five closure refusal sentences (§The
  positions, ruled — item 6) are carried. *Proof:* the committed files.

### The phases that remain

- **AC-V.1** Review rules line- and sentence-level against the
  contract's red lines 1–14 plus this pass's 15–16; a security-hold is a
  hard stop (the protected gate). *Proof:* the review verdicts in the
  run journal (contract AC-R.2).
- **AC-V.2** Verify proves the amended contract's demonstrations — the
  gates (AC-G.1–G.4 with 470), the per-issue proof sites (AC-39…57),
  the frozen three — and records the outputs. *Proof:* the contract's
  tasks, executed as written; the only changed expectation is the count
  and the pin's name.
- **AC-V.3** This pass's commit contains exactly: the three synthesis
  artifacts (`specs/fidelity-prove-ship-receipt/{spec,plan,tasks}.md`),
  the decision document (`docs/decisions/0033-one-server-per-data-file.md`),
  and the amendment edits to the four owning spots — no `src/`, no
  `Cargo.toml`, no `deny.toml`, no README change, no index edit (0033's
  row lands with the operator's landing commit, per the 24df038
  precedent). *Proof:* `git show --stat HEAD`; `git status --porcelain
  -- specs/fidelity-prove-ship-receipt` empty for the deterministic
  check.

## Costs accepted

- Until 0033 lands, a second server on one data file diverges
  semantically; SQLite serializes the bytes, nothing corrupts, and the
  divergence is the recorded risk (Ruling 3).
- An expired supplied cert serves until a client handshake fails
  (Ruling 4).
- The seven honest losses of §The spine — at-least-once acceptance, no
  durable outbox, over-cap served until first insert, wire-only chaos,
  the shared-pool linear parse, client-visible expiry, the two-servers
  divergence — stand as the record of what the design bought and what it
  did not.
- The exact count is a tripwire that will drift again if any future
  repair adds a test; that is the tripwire working, twice proven cheap.
- This pass's commit is not GPG-signed — pinentry cannot prompt in this
  environment (`env.sh` pins `commit.gpgsign=false`); the commit body
  records it, as every unsigned commit in this arc has.

## What this pass refuses

- Any fourth tree change: no new production line, test, file, route,
  dependency, schema or knob. 470 is the suite verify proves.
- Re-deriving, re-arguing or "improving" any landed design
  (`24df038`, `e245fae`, `791f83d`) — the do-not-redo rule binds every
  successor seat.
- A 50 MiB hostile-body wire test in the gate, a Mailpit empty-string
  change-detector test, a doc-linter, a `locking_mode` pragma, or an
  `x509` dependency — each named, each refused with its cost recorded
  (Rulings 3–5, simplicity §6).
- Minting vocabulary beyond red lines 15–16 and the 0033 ruling; the
  design of record needs no more words — it needs its sentences to be
  true.
