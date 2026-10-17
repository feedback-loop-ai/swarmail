# Plan — land-gate-and-red-tests: verify, review and land the batch; prove the counter family alive before its refusal deltas

- **Run:** run 12fb — implement-seat brief for the landing of
  [`spec.md`](spec.md). Design authority:
  [`specs/gate-and-red-tests/spec.md`](../gate-and-red-tests/spec.md)
  (AC-1–4 and AC-7–16 in scope; its AC-5/AC-6 are run 12g's).
- **Base:** `aba0be2` on `run12b-landing` (13 commits over `origin/main` @
  `5671e0c`). Do not rebase, do not rewrite, do not force-push — add at
  most the one pre-ruled commit and merge.

## Technical context

- **The tree under review is built and operator-verified**: all six gates
  green at `aba0be2`, coverage exact 100/100/100 (evidence
  `target/coverage/run.c6Znd4`), selftest green with the hardening arms.
  This run's verify seat re-proves all of it independently — the operator's
  run is the claim; the seat's run is the evidence.
- **The verify seat is hermetic** (decision 0007; the network is denied).
  Nothing added may fetch a toolchain, an advisory DB or a crate. The
  coverage gate must pass with **no `TMPDIR` override** — the batch's
  whole point is that it no longer needs one; if it still does, that is a
  finding that stops the landing, never a fix to apply.
- **The host awk is mawk** — no `?` quantifiers, no gawk-only ERE in
  anything the gate runs. Nothing in this plan adds awk at all; the
  sharpening is Rust-side, in `tests/`.
- **Commits are GPG-signed** (repo-local `commit.gpgsign`; the operator's
  key `5DD13676…`). If pinentry fails, stop and report — never commit
  unsigned. Note the session env may carry a `GIT_CONFIG_*` override that
  disables signing; commit without it so the repo-local rule holds.
- **The merge is `--squash --delete-branch`**, then
  `git fetch origin -q && git checkout main -q && git reset --hard
  origin/main -q`. Main receives exactly one new commit; the branch and
  its commits remain in the PR record. Append-only history is
  preserved (nothing is rewritten on `main`).

## Constitution check

- **0001 (accept == stored):** the batch adds no await between commit and
  reply; `insert_async` only moves the identical mutation onto the
  blocking pool (0023). Verify by hunk: no change inside `Store::insert`'s
  commit-before-reply path.
- **0002 (subscribe before check):** the counter's `raise` sits at the
  park site after validation (0025 §5) — it observes the ordering, never
  changes it. The delta-zero refusal arms are 0002's proof; the B5
  prologue extends the proof to the happy path. Verify by hunk: the raise
  is additive at the existing single registration site
  (`src/api.rs:255`).
- **0003 (oldest-only pruning):** untouched — no hunk on the eviction
  path.
- **0005 (extraction at ingest):** `RefScanStats` adds two integer adds on
  the existing ingest parse; no query-time work, no second parse.
- **The exact gate:** the floor is 100/100/100, exact; `count_dimensions`
  is byte-for-byte main's; no `coverage(off)`; the staleness guarantee
  survives the explicit-directory arm because the export (`:527`)
  precedes `cargo llvm-cov clean --workspace` (`:566`).
- **0020 disciplines for the sharpening:** the prologue is a wire-shaped
  test in `tests/` (rule 3 — it needs no crate-private seam); the aliveness
  await uses a real deadline (`timeout_ms` well past poll+send+commit),
  never a sleep (rule 5); the mail is delivered **before** the await, so
  the wait resolves on its first scan deterministically — no timing
  window, no mid-band chaos; per-test inboxes on the wire (0004), no
  shared global mutable state.
- **Gates:** all six run exactly as the house rules state them, plus the
  selftest, before the PR and again at the tip.

## Project structure — every file the landing touches

| Path | Change | Why |
|---|---|---|
| `tests/api.rs` | one prologue inside `the_wait_refuses_over_the_cap_registers_no_waiter_and_accepts_the_boundary` | B5: prove the family alive before the refusal delta is read |
| `tests/mcp.rs` | one prologue inside each of `the_wait_registers_no_waiter_when_staging_refuses` and `the_staging_refuses_over_the_cap_and_honors_the_boundary` | same |
| `specs/land-gate-and-red-tests/{spec,plan,tasks}.md` | added | this spec (committed by the chief) |
| everything else in the diff | **none** | the batch lands as verified |

No new files under `src/` or `tests/`; no edits to `scripts/`,
`bundles/`, `docs/decisions/`, `Cargo.toml` or `Cargo.lock`.

## Approach — the load-bearing details

1. **Verification is three activities, not one.** (a) The six gates, plus
   selftest, no `TMPDIR` override; (b) the mutants — chaos read-clone
   lost update, removed extract reference window, webhook budget
   `0..4 → 0..5` — each applied, observed red, reverted, never committed,
   reported by name; (c) each change re-read against the design
   authority's acceptance criteria, scoped per the spec header. The
   mutant runs and the gate outputs are the run's evidence.
2. **The review is scoped by hunk.** Diff against `origin/main`, judged
   on what the batch adds. The declaration-bypass class is 12g's; the
   gate body's imprecise pre-export comment is noted, not edited; the
   frozen-guarantee hunks are named (constitution check above). Bar:
   `has_security_residual=false`, no residual above medium. Anything
   beyond the pre-ruled sharpening stops the landing and is reported.
3. **The B5 sharpening, exact shape.** In each of the three arms, before
   the refusal delta is read:
   - deliver one mail to a per-test throwaway inbox via SMTP (a fresh
     name per test, 0004);
   - complete one legal await on it through the arm's own plane (REST
     `GET /api/v1/inboxes/<alive>/await?count=1&timeout_ms=10000` in
     `tests/api.rs`; MCP `tools/call` `swarmail_wait_for_email` with a
     matching filter in both `tests/mcp.rs` arms) — it resolves
     immediately because the mail is already there, and 0002's
     register-then-scan discipline makes an immediately-resolving wait a
     counted one;
   - assert that inbox's `swarmail_awaits_registered_total` delta is
     exactly **+1** (`unwrap_or(0) == 1` is red when the family is
     absent, renamed or never raised).
   Then the arm's existing delta-zero assert on its refusal inbox means
   what it claims. Land as **one signed commit** whose body names the
   sharpening and cites #102, e.g. *"Prove the awaits-registered family
   is alive before its refusal deltas are read (#102)"*.
4. **The PR body carries the record** (the framing mandates it): #102's
   signal-based tests, the #101 first-half hardening, the superseded
   recognizer specs and run 12g's scope, the commit-count reconciliation
   (14 = the framing's 12 + the batch commit `aba0be2` + the sharpening),
   the mutant list, and any named findings. Title exactly:
   *"Make the tests able to fail and the gate live off its own disk
   space (#101, #102)"*.
5. **No issue is closed and nothing is cite-closed** — the operator
   cites; #102's citation-close happens after 12g lands or at operator
   discretion.

## Contracts

- **Verification contract:** the six gate commands and the selftest run
  with `export PATH="$HOME/.cargo/bin:$PATH"`, from the repo root, on
  this host; the coverage gate with no `TMPDIR` override; the no-litter
  count around one gate run plus one selftest run. Any red is a finding
  that stops the landing.
- **Review contract:** scope = the diff vs `origin/main`, judged on what
  the batch adds; scoping rule per the spec; bar per the framing.
- **PR contract:** branch `run12b-landing` → `main`; exact title; body
  per §4; `--squash --delete-branch`; post-merge reset; merge SHA
  reported. Do not rebase or force-push the branch.

## Quickstart — the implement seat's sequence

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git checkout run12b-landing && git log --oneline -1   # aba0be2

cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh            # exact 100/100/100, NO TMPDIR override
bash scripts/coverage-gate.sh selftest
cargo deny check licenses
find /tmp -maxdepth 1 -name 'swarmail-*' | wc -l   # before; repeat after

# the three mutants, one at a time: apply, watch red, revert (never commit)
#   src/chaos.rs read-clone lost update · src/extract.rs window removal ·
#   src/webhook.rs 0..4 -> 0..5

# the B5 sharpening: edit tests/api.rs + tests/mcp.rs per the Approach,
# commit signed, then re-run the six gates at the tip

gh pr create --base main --head run12b-landing --title "Make the tests able to fail and the gate live off its own disk space (#101, #102)" --body-file <body>
gh pr merge --squash --delete-branch
git fetch origin -q && git checkout main -q && git reset --hard origin/main -q
git log --oneline -1   # report this SHA
```

## Risks

- **The sharpening re-opens a verified tree.** *Mitigation:* it is
  test-only, one named signed commit, and the six gates re-run at the tip
  (AC-9); the PR body names it, so the deviation is in the open. Anything
  else found stops the landing (AC-7) — the landing never absorbs an
  un-ruled change.
- **The aliveness prologue could flake on a loaded runner.** *Mitigation:*
  there is no timing window — the mail is delivered before the await, so
  the wait resolves on its first scan; the await carries a real deadline
  well past poll+send+commit; no sleeps, no mid-band chaos (0020 rule 5).
  If it still proves flaky, stop and report — never revert silently.
- **CI's required checks disagree with the seat's gates.** *Mitigation:*
  the same six commands ran at the tip; a divergence is reported, not
  chased with a rebase — the branch is never rewritten.
- **Signing fails (pinentry).** *Mitigation:* stop and report; no
  unsigned commit (standing rule). Commit without the session's
  `GIT_CONFIG_*` override so the repo-local `commit.gpgsign` holds.
- **Scope-creep pressure** — the pre-export comment, the authority-spec
  status note, the #101 second half, a floor knob, a "tiny" recognizer
  arm. *Mitigation:* each is refused by name in the spec; anything found
  becomes a finding in the PR body, not an edit.
- **The no-litter baseline is polluted by pre-existing `/tmp` litter.**
  *Mitigation:* the assertion is the before/after **delta**, not an
  absolute zero of the directory.
- **The squash merge collapses 14 signed commits into one.** *Accepted:*
  the framing mandates `--squash`; the branch and its signatures remain
  in the PR record; `main` stays append-only.
- **mawk regressions in the gate.** *Mitigation:* nothing gate-side is
  edited at all; if a future need arises, the B7 sentence binds (no `?`
  quantifiers, no gawk-only ERE).
