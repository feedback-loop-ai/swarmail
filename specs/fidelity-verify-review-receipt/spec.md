# Spec — verify and review the fidelity batch at `47fa0e2`: the record's completion and the spine held (issues #39, #54, #55, #56, #57)

- **Run:** run 11 verify-and-review (`brokkr-state-run11-feature-txt-6bd0870c`)
  · **Phase:** design — the chief architect synthesizing the robustness and
  simplicity positions · **Feature slug:** `fidelity-verify-review-receipt`
- **Issues:** [#39](https://github.com/feedback-loop-ai/swarmail/issues/39)
  (extraction fidelity, P1), [#54](https://github.com/feedback-loop-ai/swarmail/issues/54)
  (restored state matches live state), [#55](https://github.com/feedback-loop-ai/swarmail/issues/55)
  (Mailpit/MailHog compat fidelity), [#56](https://github.com/feedback-loop-ai/swarmail/issues/56)
  (thread normalization + web UI), [#57](https://github.com/feedback-loop-ai/swarmail/issues/57)
  (documentation and comment drift).
- **Base:** HEAD `47fa0e2` ("Correct the receipt's refusal stories and five
  record sentences to the code"), working tree clean — verified first-hand
  this session, including a full `cargo test --workspace --locked` run:
  **470 passed, 0 failed, 1 ignored** (the opt-in `tests/rate.rs` smoke).
  The two predecessor repairs `e245fae` and `791f83d` and the operator
  repairs `81409d4` / `95d2823` are in history and present.
- **Positions:** `.brokkr/state/design/positions/simplicity.md` and
  `.brokkr/state/design/positions/robustness.md` (run-local, uncommitted).
  Both verified the spine line-level at this HEAD;
  their verdicts were `pass`.
- **Design of record:** [specs/fidelity-closures/spec.md](../fidelity-closures/spec.md)
  (committed at `2cfe08c`, amended at `24df038`) — this pass tightens one
  criterion sentence in it (AC-55.2's count discipline).
- **Receipt of record:** [specs/fidelity-prove-ship-receipt/spec.md](../fidelity-prove-ship-receipt/spec.md)
  — the spine, rulings 1–6 and red lines 1–16 stand, except the three
  sentences this pass corrects in §The amendment set.
- **Decisions:** no decision is minted by this pass. [0031](../../docs/decisions/0031-sender-honest-stored-bodies.md),
  [0032](../../docs/decisions/0032-emptied-inboxes-are-removed.md) and
  [0033](../../docs/decisions/0033-one-server-per-data-file.md) stay
  **proposed** and indexed — implemented by the tree, awaiting the
  operator; acceptance is not this run's work.

## The problem

The fourth design pass of this feature is the first whose framing is a
*confirmation*: prove the committed batch satisfies its recorded criteria
(V1–V4) with all six gates green, repair only from a red criterion or a
review finding, never re-litigate the recorded repairs, ship.

The two positions answer the confirmation question — *what does design owe
a finished tree?* — and they disagree exactly once:

- **The simplicity seat: the tree owes nothing; the record owes its own
  repair's completion.** The operator's six-LOW repair `47fa0e2` provably
  did not land two of the findings its commit message claims: the diff
  touches the receipt's 0033 cost sentence only, while the message says
  the receipt's "cap/DATA refusal stories" were corrected — and the
  receipt still claims an over-cap inbox refuses with a `552` tail and a
  4096-line DATA cap, three paragraphs below its own "no artifact claims
  a refusal the tree does not make" sentence. The thread-key locus clause
  was likewise fixed in `llms.txt` and left standing in the receipt.
  Completing a repair that its own message claims is not re-litigation;
  the record currently contradicts the repair receipt for this batch.
- **The robustness seat: the spine holds — bind it, close the one
  data-file-two-servers hole (0033, ranked above all polish), keep the
  cert-expiry correction, and pin the five per-closure contract
  sentences and six named tests.** It also contributes five quiet breaks
  the framing ignores and corrects the tree's cert story.

The sharpest fact, verified first-hand this session: `git show 47fa0e2`
shows the receipt spec touched only at the 0033 sentence (:99–103), while
the commit message states the cap and DATA stories were corrected. The
receipt indicts itself — its :105 sentence forbids exactly what its :164,
:176 and :141 sentences do. And the drift metastasizes: the robustness
position, written at this same HEAD, repeats the stale :141 clause as fact
("thread keys derived once at ingest"). A reviewer reading the record
inherits its lies.

## The positions, ruled

1. **The tree owes nothing; the record owes three sentences.** Adopted
   from simplicity. The amendment set (§below) is the whole executable
   delta of this pass: three sentences in the receipt of record, two
   count-neutral assertion tightenings in existing tests, one criterion
   alignment in the design of record. No production line, no new test, no
   new file, no schema, no route, no dependency, no await. **Cost of
   refusing the completion set:** review re-finds the receipt's false
   sentences against a repair whose own message claims them fixed — the
   machine pays one more loop for the same five edits, and the ship would
   carry a record that lies about itself. The completion is the cheap
   path and the honest one.
2. **0033 is not landed in this run.** Ruled against robustness, on the
   framing's own charter: findings are repaired only from a red criterion
   or a review finding, and no recorded criterion (A–E, V1–V4) demands a
   second-writer refusal — 0033 is *proposed*, its acceptance is the
   operator's, and the prior receipt's Ruling 3 already priced exactly
   this refusal ("the failure mode stays unnamed in the record" is no
   longer true: the receipt names it, the index carries it). Landing
   `PRAGMA locking_mode = EXCLUSIVE` now would also rework the vandal
   corruption tests — real design work a confirmation pass must not
   smuggle. The hole stays named in the record, open until the operator
   accepts 0033 and lands refusal + test + sentence in one commit.
   **Cost of the losing side:** until that acceptance, a second server on
   one data file diverges semantically (a live peer's stale memory vs the
   file, per-process seq, prune plans from partial views — the hazards
   0015 §4 names) while SQLite serializes the bytes; nothing corrupts and
   no commit is lost. For a test sink this is a misconfiguration input,
   not a hostile one, and the boundary fails loudly everywhere else.
3. **The spine is bound, and every test robustness demands is already in
   the tree.** Verified first-hand this session — the demanded pins exist:
   the hostile-entity bounded-work pin (`an_ampersand_run_decodes_in_bounded_work`,
   src/extract.rs:195), the wire-fidelity pin (`tests/api.rs:339`, `&amp;`
   and `&#38;` over real SMTP), the restart-order pin
   (`a_reopened_store_restores_mail_order_and_counters`, src/store/persist_tests.rs:17,
   beside the reopen-twice no-duplicate-seq pins), the ledger pin
   (`exhausted` at quiescence, tests/webhook.rs:588) and the `IDs`
   empty-selector refusal (tests/compat.rs, the 0011 leg), the 404 pin
   (tests/ui.rs:597). The sixth (a second-server refusal) is moot until
   0033 lands (ruling 2). Simplicity's count-neutrality therefore costs
   robustness nothing: **no new test is minted**, the recorded 470 stays
   true, and no 0020 placement question opens.
4. **The cert-expiry correction stands as record; no dependency is
   bought.** The receipt's Ruling 4 already carries it —
   `TlsConfig::server_config()` validates structure, not time; an expired
   supplied pair binds and serves and the failure surfaces at each client
   handshake; no doc claims startup time validation (verified by absence:
   no `not_after` check in `src/tls.rs`). This pass refuses the
   `x509-parser` dependency and keeps the correction verbatim in the new
   receipt. **Cost:** an expired supplied cert stays client-visible, not
   startup-refused — loud at the client, silent at the server — until a
   decision buys the check with a real operator case.
5. **The `html_to_text` residual is accepted, not repaired.** Both seats
   converge: `src/compat.rs:560` (`snippet`) and `:801` (`plain_response`)
   run mail_parser `html_to_text` per request on the async workers for
   html-only mail, and the 11d security member's recorded verdict is
   *shippable* — the cost class is the same as the pre-0022 per-request
   text clone that lived there for the shim's whole life, bounded by
   0022's body cap, on list-view pages that are not extraction inputs.
   The follow-up is named, not done: move the page's summary build to the
   blocking pool, or cap the rendered prefix at the snippet's own 200
   chars — either is a decision-shaped change to an output-equivalence
   surface, owed its own equivalence proof, not smuggled into a
   confirmation pass. **Cost:** a pathological html-only body pays an
   unbounded-by-us linear render on an async worker per request.
6. **The five quiet breaks are named in this receipt, not edited into
   README.** Adopted from robustness at one sentence each (§Named
   residuals): the blocking-pool memory ceiling, the 250-to-wire window,
   WAL growth under burst, quiescence-exact ledger, doc drift as a
   robustness defect. The #57 surface is closed — widening it in the same
   breath would be the re-litigation the framing forbids; the receipt is
   where this run's record lives. The bench measure robustness wants for
   the memory ceiling rides the existing bench family, not this pass.
7. **The thread-key locus is sourced from the corrected record, not from
   either position.** Robustness's #56 paragraph says "derived once at
   ingest"; that clause is **rejected** — 0018 §5 says the view is
   derived per request, and `group_threads` runs inside `threads::view`
   (src/api.rs:1014, src/feed.rs:54), with `to_lowercase()` and the
   recorded ß/ss full-folding residual. The *precedence* content of
   robustness's sentence is adopted (chains first, subject fallback).
   This is the metastasis simplicity named: the chief's receipt must not
   synthesize from a position that repeats the old one's lie.

## The amendment set (applied to the owning artifacts by the implement seat)

Five edits, all priced, all count-neutral (470 stands). A1–A3 correct the
receipt of record; A4–A5 tighten assertions inside existing tests; A4
also aligns the design of record's criterion wording.

1. **Receipt of record, the cap-race bullet (currently :164–169).** The
   bullet tells the truth about 0003: an over-cap insert *accepts and
   evicts oldest* — the store never refuses on count (`pop_front` in
   commit order, the multi-evict catch-up loop; proven by
   `insert_prunes_oldest_beyond_cap`, src/store.rs:864). The honest
   losses that remain: an over-cap restored inbox is served until its
   first insert (0003's letter), and a burst sees its oldest mail vanish
   under a slow waiter — the waiter's timeout is honest
   (`Timeout { matched }`), the cap means "the newest N survive". Drop
   the `552`-tail claim: `552` exists only for the DATA byte allowance.
2. **Receipt of record, the hostile-DATA bullet (currently :176–179).**
   DATA is bounded by the message-byte allowance: a line over it is
   drained unbuffered and the whole mail is refused `552 5.3.4 Message
   too big` at the terminator, nothing stored (src/smtp.rs, the DATA
   arm). `MAX_COMMAND_LINE = 4096` is named as what it is — the
   *command-line* byte cap (SMTP and POP3 alike), answered `500 5.5.2
   Line too long` with the line drained unbuffered; it never gates DATA.
   The rest of the bullet (linear extraction, the 40-byte window,
   rendered-text-only codes) is true and stays.
3. **Receipt of record, the #56 pin clause (currently :141).** "thread
   keys fold at ingest" → "thread keys derive per view request (0018 §5)
   with the fixed precedence (References/In-Reply-To/Message-ID chains
   first; the normalized subject — Re:/Fwd: stripped,
   Unicode-lowercased; full case folding a recorded limitation — as the
   fallback)", matching the `llms.txt` sentence `47fa0e2` already fixed.
4. **tests/compat.rs, the envelope leg of
   `an_envelope_only_recipient_surfaces_in_every_compat_shape` (~:1256).**
   `total >= 1` becomes `total == 1` plus `items.len() == 1` — the
   fixture delivers exactly one mail to a fresh server, so 0010's
   exact-count discipline closes its one loophole ("at least one" is a
   fresh-server assertion that survives a dropped mail). The design of
   record's AC-55.2 (`total >= 1`, specs/fidelity-closures/spec.md:478)
   is tightened to match.
5. **tests/ui.rs, `the_live_shell_guards_paints_with_a_token` (~:696).**
   Pin the guard clause itself, not its identifiers:
   `script.contains("seen !== paintSeq")` — the early return at
   `src/static/ui.js:81`. Today the test stays green if the guard is
   deleted and only the counter remains; the assertion closes it free.

Both test edits are assertions inside existing tests: no new test, no new
file, no 0020 question, no count movement, no new cold path, no sleep, no
wall clock. If any seat prefers a *new* test for either: refused — the
assertion is the whole repair.

## Named residuals and accepted costs (this receipt's honest-loss list)

Carried from the receipt of record's spine (which stands except for the
two amended bullets) plus the quiet breaks robustness contributes — one
sentence each, zero code:

- **The blocking pool is a hidden queue.** `offload` is `spawn_blocking`:
  when the pool is saturated, awaiting accepts queue, each holding its
  session's raw bytes — the real memory ceiling is sessions ×
  max_message_size, and the 256-session cap *is* the backpressure.
  Raising `max_sessions` without naming that number is raising an
  unbounded-looking buffer.
- **The 250-to-wire window.** The 250 is written after the commit, but
  the write is one `idle_timeout`-bounded I/O against a possibly vanished
  peer — the duplicate-on-retry of at-least-once. Name it so nobody
  "fixes" it by acking early.
- **WAL growth under a sustained burst.** `synchronous = FULL` inside WAL
  keeps commits durable and cheap, but a long burst outpaces the
  automatic checkpoint and the `-wal` file grows next to the data file;
  crash-mid-checkpoint is SQLite's ordinary recovery case. One sentence
  of operator awareness; no knob.
- **Ledger exactness is quiescence-exact, not live-exact.** A burst in
  flight makes a `/metrics` read a moving picture; tests assert at
  quiescence (0010), and a dashboard treating rows as live-exact is a
  test-shaped lie.
- **Doc drift is a robustness defect.** Docs may not promise what the
  code refuses to do — the disease class of #54's `COALESCE(MAX(seq),0)`
  and of this receipt's own :164/:176/:141, and the reason the amendment
  set exists.
- **The `html_to_text` residual** (ruling 5) and **the 0033 hole**
  (ruling 2) stay open with their follow-ups named; **expired supplied
  certs are client-visible** (ruling 4); **at-least-once, honest seq
  gaps, no durable outbox, best-effort webhook loss observably in the
  ledger** — all as the receipt of record prices them.

## Acceptance Criteria

The framing's V1–V4 verbatim, then the amendment set as this pass's own
red-able claims. Each can go red (its named proof fails, or the shape is
absent) and is green only when it holds on the committed tree.

- **V1 — Landed posture.** The tree is clean at `47fa0e2`; the operator
  repairs `81409d4` (signal streams registered by an eager poll before
  anything binds) and `95d2823` (fallible `search_needle` → 400) and the
  review repairs `e245fae`, `791f83d`, `47fa0e2` are in history and their
  changes present in the tree. Red = a dirty tree, a missing repair, or a
  seat edit that re-litigates one of them. Proof: `git log` + the repair
  tests passing by name. *(Read for this pass: the tree is clean at the
  completion commit — `47fa0e2` plus exactly the amendment set, whose
  whole executable delta is the two assertion tightenings.)*
- **V2 — All six gates green.**
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace --locked
  brokkr compile --bundle bundles/self
  TMPDIR=/operator-scratch/cov-tmp bash scripts/coverage-gate.sh
  cargo deny check licenses
  ```
  Red = any command fails, any test failure, or coverage below exact
  100/100/100 (missed == 0). `cargo test --workspace --locked` must show
  0 failures with the recorded 470 passing (a different total needs
  explanation, not silent acceptance). The `TMPDIR` is **mandatory**: the
  gate dies on `/tmp` tmpfs; never run the disk-bound coverage gate
  against tmpfs.
- **V3 — Per-issue criteria hold on the tree.** The criteria are the
  recorded A/B/C/D/E sets; re-prove them on the committed tree with real
  servers, no mocks: `cargo test --test api` and `cargo test --lib
  extract` (#39); `cargo test --test persistence --test pop3 --lib
  persist` (#54); `cargo test --test compat` (#55); `cargo test --test
  ui --lib threads` (#56); doc inspection at HEAD (#57, no new tests).
- **V4 — Frozen guarantees intact.** 0001: no await entered the
  insert/persist path — `Store::insert` stays synchronous, an SMTP `250`
  still means queryable. 0003: restore-order work changed determinism
  only; eviction is still oldest-only and never mid-delivery. 0002:
  untouched. Proof: the suites above plus code inspection of the persist
  path.

### The amendment set (this pass's red-able claims)

- **AC-A.1** The receipt of record's cap-race bullet states accept-and-
  evict and no longer claims a `552` refusal on count. *Proof:* the
  bullet text at the cap-race anchor reads "accepts and evicts oldest"
  with no `552`-tail claim; `grep -n "552" specs/fidelity-prove-ship-receipt/spec.md`
  shows the remaining `552` mentions belong to the DATA byte allowance
  and the storage-commit refusal only.
- **AC-A.2** The hostile-DATA bullet names the message-byte allowance and
  `552 5.3.4` at the terminator, and names `MAX_COMMAND_LINE = 4096` as
  the command-line cap answered `500 5.5.2`. *Proof:* bullet text at the
  hostile-DATA anchor; `grep -n "MAX_COMMAND_LINE" src/smtp.rs` (the
  constant, the `500 5.5.2` doc, the DATA allowance) matches the
  sentence.
- **AC-A.3** The receipt's #56 pin clause says "derive per view request
  (0018 §5)" with the fixed precedence and the recorded full-folding
  limitation. *Proof:* the clause text; `grep -n "fold at ingest"
  specs/ src/ docs/` is empty across the tree.
- **AC-A.4** `tests/compat.rs`'s envelope leg asserts `total == 1` and
  `items.len() == 1`, and the design of record's AC-55.2 says `total ==
  1`. *Proof:* `cargo test --test compat
  an_envelope_only_recipient_surfaces_in_every_compat_shape` green; the
  criterion text at specs/fidelity-closures/spec.md (AC-55.2).
- **AC-A.5** `the_live_shell_guards_paints_with_a_token` fails if the
  guard clause is removed from `src/static/ui.js` and passes with it.
  *Proof:* `cargo test --test ui the_live_shell_guards_paints_with_a_token`
  green on the tree; red-able by deleting the `seen !== paintSeq` early
  return in `src/static/ui.js:81` (a fixture flip, not a committed
  state).
- **AC-A.6** The completion commit's whole diff is the amendment set:
  three sentences in the receipt of record, one criterion sentence in the
  design of record, two assertions in existing tests — no other file.
  *Proof:* `git show --stat` on the completion commit — exactly four
  files: `specs/fidelity-prove-ship-receipt/spec.md`,
  `specs/fidelity-closures/spec.md`, `tests/compat.rs`, `tests/ui.rs`.
  This receipt's trio rides its own design commit beside it.

## Costs accepted

- The record moves one commit past `47fa0e2` mid-arc — priced by precedent
  (the record-honesty pass's Ruling 2, applied since): the owning artifact
  is where the answer lives, and git history preserves the prior state.
- 0033 stays open (ruling 2's cost); the `html_to_text` residual stays
  open (ruling 5's cost); expired supplied certs stay client-visible
  (ruling 4's cost).
- The receipt of record carries amended sentences rather than a v2
  document: three stale sentences, not three stale artifacts.

## What this pass refuses

- Any production line, any new test, any new seam, any dependency,
  lockfile or `deny.toml` change, any schema/route/await change.
- A doc linter or record-truth gate: the disease is three stale sentences;
  the cure is three sentence edits. The last receipt's refusal of a
  doc-linter carries — a gate that diffs prose against code is a new
  permanent cost bought to avoid two `edit` calls.
- Landing 0033's refusal in this run (ruling 2), minting a decision
  (none is owed — nothing here touches a frozen guarantee, a public
  contract or a ruling), and re-litigating any recorded repair.
