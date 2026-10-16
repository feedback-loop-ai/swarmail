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
- **Base:** `a4ff22c` (HEAD — the operator's repair, atop f9cde28 ←
  7550de3 ← 66a45bb). Every line cited below was re-verified against this
  tree in this session; nothing here is taken from the positions' prose.
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

Both positions independently confirm the artifact contract at a4ff22c —
the specs committed and unchanged, 0030 proposed and amended in place,
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
  claim; the 451/parse-arm distinction; **no re-litigation** of the
  pre-repair findings (Origin-only guard, wide kind domain, false
  `maximum`, gauge "never existed" claims); new findings rule as always.
- **No new decision, no new spec for the family.** `docs/decisions/`
  ends at 0030; the re-entry's corrections fold at its acceptance. The
  only committed artifact of this run's design phase is this spec trio.
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
   sentence, the 451/parse-arm distinction, and the no-re-litigation
   rule for the pre-repair findings.
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

1. At ship, `git diff a4ff22c HEAD --stat` names only defect-driven
   repairs, each with its named pin in the same commit.
2. `git diff a4ff22c HEAD -- Cargo.toml` is empty unless a defect
   demanded a new dependency — which would be a decision document, not an
   edit. `docs/decisions/` still ends at 0030 (no fork of the record).
3. No new bench, no new seam, no `coverage(off)`, no confirmation-test
   twin, no hardening knob exists that AC2's pins did not already prove.
4. The two declined renames are still declined: `origin_guard` keeps its
   name and `origin_is_loopback_of` stays the thin scheme-strip — recorded
   here as the price of a zero-delta re-entry, payable only when a defect
   opens the file anyway.
