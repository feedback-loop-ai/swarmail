# Plan — prove and ship the fidelity closures (run 11 prove-and-ship pass)

- **Feature slug:** `fidelity-closures-prove-ship` · **Companion:**
  [spec.md](spec.md) (the WHAT/WHY and the red lines) ·
  [tasks.md](tasks.md) (the ordered proof list)
- **Base:** HEAD `24df038`. This plan contains **no production change**:
  the implement seat's edits are committed, the do-not-redo rule binds, and
  the remaining phases are verify → review → ship. Every file touch below
  is either a read (proof) or a review finding's repair.

## Technical context

Swarmail is a single Rust crate: production code under `src/`, real-protocol
integration suites under `tests/` (no mocks, 0004), the delivery
constitution in `bundles/self/` compiled by brokkr, and the journal in
`.brokkr/state/forge.db`. The pass's subject is the five fidelity closures
landed at `24df038` across seven production files (`src/extract.rs`,
`src/smtp.rs`, `src/persist.rs`, `src/store.rs`, `src/compat.rs`,
`src/threads.rs`, `src/ui.rs` + `src/static/ui.js`), the suites that prove
them (`tests/api.rs`, `tests/persistence.rs`, `tests/pop3.rs`,
`tests/compat.rs`, `tests/ui.rs`, `tests/mcp.rs`), and the docs the record
owns (`README.md`, `CONTRIBUTING.md`, `deny.toml`,
`docs/decisions/README.md`, `src/static/{openapi.json,llms.txt}`).

The environment carries two hard-won facts the plan must respect: the
coverage gate's instrumented build filled /tmp's tmpfs once (the
predecessor's death), so the gate runs only with the disk-backed
`TMPDIR=/operator-scratch/cov-tmp`; and commits in this environment
are unsigned (pinentry cannot prompt) — the operator's own implement commit
says so in its message, and this pass's commit follows that precedent.

## Constitution check

- **0001 (accept == stored): intact.** All #54 work is synchronous SQL
  inside `open`/`load`; the #39 decode and part-type match run inside the
  existing offloaded parse before the 250; no await entered the insert
  path (0031 §Preserves records it; both positions verified it).
- **0002 (subscribe before check): untouched.** D4 is client-side paint
  ordering; the server's register-then-scan path and 0021's watcher
  lifetime are out of the diff's direction of change; 0032's removal
  happens at the same condition and moment that already released the
  watcher.
- **0003 (oldest-only pruning): intact, now deterministic across
  restarts.** B2 fixed the determinism of the order eviction *reads*
  (`seq, rowid` + the `emails_seq` index), never the eviction choice.
- **Applied, not amended:** 0005 (ingest ruling preserved, 0031's
  carve-out is the operator's to accept), 0011/0029/0025/0018 §3, 0030
  §2/§3 (the loopback posture and page caps untouched), 0027 §7 and the
  `smtp=` startup line (`4daa332`) — not re-litigated.
- **Decision documents:** 0031 and 0032 remain **proposed**; the operator
  accepts post-review. This pass mints none (spec.md, Decisions bullet).

## Project structure

Nothing changes. The artifacts this pass commits are
`specs/fidelity-closures-prove-ship/{spec,plan,tasks}.md`; the design of
record stays `specs/fidelity-closures/`; `.brokkr/state/` stays gitignored
run-local evidence (the positions and the operator notes live there). If —
and only if — a demonstration fails, the repair touches the single owning
file named in spec.md's criterion (production file + its proving suite),
and a semantic repair adds exactly one `proposed` decision under
`docs/decisions/`.

## Verify approach (the proof, not a re-derivation)

Run the gates in this order; stop at the first red and route it to the
repair protocol:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
cargo deny check licenses
TMPDIR=/operator-scratch/cov-tmp bash scripts/coverage-gate.sh   # exact 100/100/100 — never tmpfs
```

Targeted filters for the per-issue demonstrations (real servers, per-test
inboxes, 0004):

- #39: `cargo test --test api smtp_html_hrefs_decode_character_references`
  and `cargo test --lib extract`
- #54: `cargo test --test persistence`, `cargo test --test pop3`,
  `cargo test --lib persist`
- #55: `cargo test --test compat`
- #56: `cargo test --test ui`, `cargo test --lib threads`
- #57: no tests — the decision index row-by-row comparison, the allowlist
  diff, the README/CONTRIBUTING/openapi greps and the comment reads ship
  as review evidence.

The frozen three are re-proved by the standing suites
(`tests/burst.rs`, `tests/persistence.rs`) without modification —
spec.md's AC-R.3 names the legs. Verify records the gate outputs; it does
not re-run the gate on tmpfs under any circumstance, and it does not
re-run the instrumented build more than the criteria need.

## Review hand-off

The review phase splits into its panel members; review owns three things
this pass prepares:

1. **The named item:** 0031 §3's display-rendering carve-out — the two
   fallback sites (`src/compat.rs:560`, `:801`) and the codes haystack.
   Review rules on the carve-out's letter; if it refuses, 0031 §3's own
   recorded posture applies (the fallback collapses to an empty view and
   the diff shrinks by two lines; the stored truth is identical).
2. **The refusal ledger:** spec.md's red lines 1–11 are review's checklist;
   a finding names a line to delete or a line to fix — it does not re-argue
   the design (the do-not-redo rule binds reviewers hardest).
3. **The proposed decisions:** 0031 and 0032 travel to the operator with
   the run's evidence; acceptance is the operator's alone.

Security-hold is a hard stop (the protected gate); correctness and security
each record their verdict in the journal.

## Repair protocol (when a demonstration fails)

A false criterion is repaired, not re-designed:

1. Identify the owning file from the criterion's proof site.
2. Make the minimal line-level change that makes the criterion true —
   inside that file, no new paths, no new dependency, no new route
   (spec.md red lines 2–3).
3. Extend or adjust the proving test in the same suite the criterion names;
   the test obeys 0020 (no sleep-as-sync, no wall-clock validity, chaos at
   the poles, fresh stores/inboxes; a cold failure path moves to
   `src/<module>/<name>_tests.rs`, parent-declared).
4. Re-run the gates affected; if the repair is semantic (a public contract
   or a standing ruling moves), draft exactly one `proposed` decision and
   reference it from the spec before committing.
5. Commit the repair with the run's message style; never push.

## Ship hand-off

`ship` is a hermetic exec seat (`scripts/ship-seat.sh`): it confirms the
gate evidence, the journal's verdict chain, and the operator-facing record
(the two proposed decisions + the operator notes file), and closes the
run. Ship writes no code; its only output is the verdict and, if the
operator asks for it, the release note under `docs/releases/`.

## Quickstart

```bash
# prove
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked && brokkr compile --bundle bundles/self
cargo deny check licenses
TMPDIR=/operator-scratch/cov-tmp bash scripts/coverage-gate.sh
# demonstrate per issue (spec.md's proof sites)
cargo test --test api smtp_html_hrefs_decode_character_references
cargo test --lib extract && cargo test --lib persist && cargo test --lib threads
cargo test --test persistence && cargo test --test pop3
cargo test --test compat && cargo test --test ui
# review reads specs/fidelity-closures-prove-ship/spec.md's red lines
# ship runs scripts/ship-seat.sh (hermetic)
```

## Risks

- **The coverage gate fills its tmpfs again.** *Mitigation:* absolute rule
  — the gate runs only with `TMPDIR=/operator-scratch/cov-tmp`
  (disk-backed, 290G free); never on tmpfs; run it once and record the
  output. Any "quick local re-run" without the TMPDIR is the predecessor's
  failure mode.
- **A demonstration fails at HEAD** (a criterion proven false by verify or
  review). *Mitigation:* the repair protocol above — minimal, in the owning
  file, test-covered, 0020-conformant; the smallest honest diff wins. The
  risk is not the repair but the temptation to redesign; the red lines
  forbid it.
- **Review re-argues the design** (the 0031 §3 carve-out is the likely
  flashpoint). *Mitigation:* the carve-out's collapse posture is recorded
  in 0031 itself (two-line fallback, identical stored truth), so a refusal
  has a priced landing; the do-not-redo rule makes "over-built" a line
  finding, not a redesign.
- **The operator refuses 0032's door** (emptied inboxes kept instead of
  removed). *Mitigation:* the issue itself offered both doors; the "kept"
  door is a small, test-covered revert of the removal arm plus
  `tests/pop3.rs` expectations — but it is the operator's call, made with
  the decision in front of them, not review's.
- **The srcdoc-inheritance defense is silently swapped for a `src=` iframe**
  in some future edit, leaving AC-56.5's test green while remote loads
  return. *Mitigation:* red line 8 names the mechanism; review owns any
  change to the preview's markup (`src/ui.rs:138`).
- **Legacy data files pay a one-time index build** on first open after the
  upgrade. *Mitigation:* accepted and priced in the design of record (one
  O(n log n) build, idempotent thereafter, riding the pre-bind open path);
  `the_restore_scan_reads_the_seq_index` pins the plan so a silent
  regression goes red.
- **Environment drift** (the harness's broken `GIT_CONFIG_KEY_0`
  injection, missing GPG agent). *Mitigation:* complete the env as
  `GIT_CONFIG_KEY_0=commit.gpgsign` (the injected value is `false`, the
  operator's implement commit is the unsigned precedent, and the commit
  message says so); never push; the position files stay uncommitted under
  `.brokkr/state/`.
- **Flaky-looking wire proofs** (SSE/paint-token, long-poll legs).
  *Mitigation:* the proofs are the asset-assert and real-deadline patterns
  already in the suites (0020: no sleep-as-sync, no wall-clock validity);
  a flake is investigated as a product bug first, never papered with a
  sleep.
