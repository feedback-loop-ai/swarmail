# Run 10b — intake framing: the parse/bounds/loopback family's protected re-entry (issues #49 #48)

## Goal (one sentence)

Re-run the protected phases — implement (confirmation) → verify → review →
ship — on the repaired tree at `81409d4` (HEAD: the `a4ff22c` security
repair plus the `81409d4` signal-death repair), with the re-entry design
committed at `6f9a4bf` and synthesized at `b2c95f3`
(`specs/protected-reentry/`): confirm the committed
spec artifact set still holds (including 0030's amended §2) and put the
never-reviewed security repair before a fresh review panel, without
re-arguing the ruled shape and without touching the code unless a defect
is found (any edit re-runs the full gate set).

## What this re-entry is (and is not)

- **Intake and design confirm, not re-argue.** The ruled spec
  (`specs/off-worker-parses-loopback-posture/`, committed at `7550de3`)
  and the re-entry contract (`specs/protected-reentry/`, committed at
  `6f9a4bf`, synthesized into its final shape at `b2c95f3`) stand;
  decision 0030 is `proposed` and indexed; HEAD is `81409d4`. This
  framing's confirmation checks are recorded below; nothing is re-opened.
- **Implement is a confirmation seat.** The work exists
  (f9cde28 → a4ff22c, plus the `81409d4` signal repair). Implement
  confirms tree == spec == repair and
  touches nothing unless a defect is found — the smallest local fix
  beside its wire pin, then the full gate set.
- **Verify, review, ship run as usual.** Verify is hermetic (network
  denied); review judges HEAD weighted against the ruled spec, the
  amended 0030 and the re-entry contract, verifying the run-10 repairs
  and never re-litigating the pre-repair tree; landing is the operator's
  (PR, both duplicate CI runs green, squash). Seats cannot edit
  `agents/`, `adapters/`, `bundles/` (operator notes to
  `.brokkr/state/run10b-operator-edits.md`); the state home is
  `.brokkr/state/` (0083).

## Intake's confirmation of the artifact contract (checked this session)

- `specs/protected-reentry/` carries `spec.md`, `plan.md`, `tasks.md`
  (design commit `6f9a4bf`, synthesized at `b2c95f3` — whose message
  records the two sharpenings and rides this framing copy); HEAD is
  `81409d4` and the tree is clean.
- The signal-death repair is on the tree: `src/main.rs` constructs and
  polls both interrupt streams on the main task **before** `run_on`
  (`81409d4`) — registration happens at first poll and the fast-path
  binds complete without a yield, so a spawned watcher can no longer
  stay unpolled past the HTTP bind. `git diff a4ff22c HEAD --stat`
  names exactly `specs/protected-reentry/` and `src/main.rs`.
- The two-leg guard is in the tree: `authority_is_loopback_of`
  (`src/api.rs:350`) serves both the Host leg (refused before the
  router, every method and route) and the Origin leg (`src/api.rs:317`).
- `docs/decisions/0030-loopback-plane-bounded-results.md` line 3 reads
  `- **Status:** proposed`; the index row exists
  (`docs/decisions/README.md:39`); §2 derives the Host leg from the
  same-origin/no-`Origin` fact (0030:93–106) and carries the provenance
  paragraph on the earlier draft's wrong premise (0030:127–132).
- `scripts/design-check.sh` and `scripts/coverage-gate.sh` are present
  for the downstream seats.

## Frozen-guarantee proximity (0001–0003)

- Sits near **0001** and **0002** but weakens neither: the parse leg runs
  *before* the mutation boundary and the 250 is still written only after
  the commit outcome is known (0030 §Preserves); the awaits-gauge repairs
  touch the claims, never the subscribe-before-check ordering. The
  re-entry itself makes **no semantic change**, so it needs no decision
  document — that is exactly why it is `resolved` and not `blocked`.
- **0003** is untouched — no pruning path is in scope.
- No frozen guarantee is amended; 0030 (`proposed`, operator-mandated
  re-entry) is the authority on the ruled shape.

## Acceptance criteria (checkable claims)

1. **The artifact contract still holds.** `git diff 7550de3 HEAD --
   specs/off-worker-parses-loopback-posture/` is empty; 0030 is
   `proposed`, indexed, §2 amended with its provenance paragraph;
   `src/persist.rs` at zero diff since `66a45bb` and `src/store.rs` at
   only the `offload` visibility change. (Design phase re-runs
   `scripts/design-check.sh` — `designed` with no findings.)
2. **The repairs are proven by their existing pins, re-run, not
   rewritten.** `cargo test --test api` — the three rebind legs
   (`cross_origin_and_malformed_spellings_are_refused_before_the_router`,
   `a_rebound_host_refuses_even_without_an_origin`,
   `an_off_loopback_bind_leaves_the_guard_inert`) go green;
   `cargo test --test compat` pins `kind=subject` as a bare 400;
   `cargo test --lib` holds the awaits-gauge observable-row pins;
   `cargo test --test config_cli` pins the three loopback defaults; and
   `cargo test --test binary serve_answers_healthz_and_stops_on_sigint`
   goes green at HEAD — the `81409d4` repair's own pin. A red pin is a
   defect, never papered with a sibling twin.
3. **All house gates are green at HEAD**, including the exact coverage
   gate (100/100/100, missed == 0 — the floor may rise, never fall):
   commands below.
4. **The wire stays honest.** The piped `smtp=`/`http=` serve startup
   line parses in any capture — ANSI only when the target stream can
   render it (0029 §4; do not regress the ANSI guard) — in serve and mcp
   both.
5. **The re-entry's delta is exactly the ruled artifacts plus the named
   repairs.** `git diff a4ff22c HEAD --stat` names only
   `specs/protected-reentry/` (the design synthesis, `b2c95f3`) and
   `src/main.rs` (the signal repair, `81409d4`); any further delta is a
   defect-driven repair with its named pin in the same commit;
   `docs/decisions/` still ends at 0030; no new bench, seam,
   `coverage(off)` or hardening knob.
6. **Review rules on HEAD with the carried weighting.** The four-sentence
   guard ledger, the tenant-boundary scoping, the bound-port invariant
   and the 451/parse-arm distinction are review inputs; the run-10
   findings are verified repaired at `a4ff22c` and not re-litigated; new
   findings rule as always — a security-hold is a hard stop.

## Verify gate

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
#   targeted: cargo test --test api --test compat --test lib \
#                  --test config_cli --test binary
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh            # exact 100/100/100 — floor may rise, never fall
cargo deny check licenses
```

Verify seat additionally: network denied; the piped serve startup line
must parse in any capture (0029 §4 — do not regress the ANSI guard);
SIGBUS → disk-backed `TMPDIR`. The SIGINT test
`tests/binary.rs::serve_answers_healthz_and_stops_on_sigint` (real
process, `tests/binary.rs:204`) is **diagnosed and repaired at
`81409d4`**: the streams register on their FIRST POLL and the
fast-path binds complete without a yield, so a spawned watcher could
stay unpolled until after the HTTP bind — a signal landing once the
port answered a connect killed the process at the default disposition
(`ExitStatus(unix_wait_status(2))`, a signal death). Verify confirms
the pin green at HEAD and does not re-litigate the diagnosis; if it
still fails, record the exit status verbatim and treat it as a defect.

## Blocked check

Nothing in the re-entry contradicts a frozen guarantee: it changes no
semantics, amends no decision, and the one decision in play (0030) is
already committed `proposed` — the lawful path the run mandates.
**Resolved.**
