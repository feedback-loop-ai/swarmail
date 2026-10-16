# Run 10b (successor re-entry) — intake framing: the parse/bounds/loopback family's protected re-entry (issues #49 #48)

Successor to [`framing-10b.md`](framing-10b.md) (which framed the re-entry at
HEAD `81409d4`). That framing is **confirmed, not re-argued**: the goal, the
confirm-don't-reargue posture and the carried weighting stand. This copy
records the successor state — run 10d's review verified the run-10 repair,
closed the hold, and the operator landed its residual at **HEAD `95d2823`**
— and updates the delta claim and gate expectations to that tree.

## Goal (one sentence)

Re-run the protected phases — implement (confirmation) → verify → review →
ship — on the tree at `95d2823` (HEAD: the `a4ff22c` security repair, the
`81409d4` signal-death repair, and the `95d2823` residual repair:
`search_needle` refuses an uncompilable needle with a 400 instead of
unwinding, the Origin-guard comment states the true layer order, and 0030
§2 drops the false h2/`:authority` claim), confirming the committed
artifact contract (including the amended §2) and putting the repair line
before review without re-arguing the ruled shape or touching code unless
a defect is found (any edit re-runs the full gate set).

## What this re-entry is (and is not)

- **Intake and design confirm, not re-argue.** The ruled spec
  (`specs/off-worker-parses-loopback-posture/`, committed at `7550de3`)
  and the re-entry contract (`specs/protected-reentry/`: `6f9a4bf`,
  synthesized `b2c95f3`, third panel recorded `c190493`) stand; decision
  0030 is `proposed` and indexed; HEAD is `95d2823` and the tree is
  clean. Nothing is re-opened.
- **Implement is a confirmation seat.** The work exists. It confirms
  tree == spec == repair and touches nothing unless a defect is found —
  the smallest local fix beside its named pin, then the full gate set.
- **Verify, review, ship run as usual.** Verify is hermetic (network
  denied); review judges HEAD weighted against the ruled spec, the
  amended 0030 and the re-entry contract — verifying the `a4ff22c`
  repairs and the `95d2823` residual, **not re-litigating the
  pre-repair tree and not re-litigating run 10d's own LOW/INFO record**
  (those findings are the record of what changed and why); new findings
  rule as always, and a security-hold is a hard stop. Landing is the
  operator's (PR, both duplicate CI runs green, squash). Seats cannot
  edit `agents/`, `adapters/`, `bundles/` (operator notes to
  `.brokkr/state/run10b-operator-edits.md`); the state home is
  `.brokkr/state/` (0083).

## Intake's confirmation of the artifact contract (checked this session)

- HEAD is `95d2823` ("Refuse an uncompilable search needle instead of
  unwinding, and stop the record's two small lies"); the tree is clean.
  History reads `f9cde28` → `7550de3` → `a4ff22c` → `6f9a4bf` →
  `b2c95f3` → `81409d4` → `c190493` → `95d2823`.
- `git diff a4ff22c HEAD --stat` names exactly nine paths:
  `specs/protected-reentry/{spec,plan,tasks,framing-10b}.md`
  (design records, `6f9a4bf`/`b2c95f3`/`c190493`), `src/main.rs`
  (signal repair, `81409d4`), and `src/api.rs`, `src/compat.rs`,
  `docs/decisions/0030-*.md`, `benches/read_stall.rs`
  (the `95d2823` residual).
- The residual repair is on the tree: `search_needle` returns
  `Option<Regex>` (`src/compat.rs:409`), the call site refuses via
  `ok_or(ApiError(..))` (`src/compat.rs:390`), and the seam pin holds —
  an uncompilable 250 000-char needle yields `None`, never an unwrap
  (`src/compat.rs:847`).
- 0030 is `proposed` (line 3), indexed (`docs/decisions/README.md:39`),
  and §2 fails closed without an `h2` hop: a request without a `Host`
  is answered `403` (0030:105–110).
- `specs/protected-reentry/` carries `spec.md`, `plan.md`, `tasks.md`
  beside the prior framing; `scripts/design-check.sh` and
  `scripts/coverage-gate.sh` are present for the downstream seats.

## Frozen-guarantee proximity (0001–0003)

- Sits near **0001** and **0002** but weakens neither: the parse leg
  runs *before* the mutation boundary and the 250 is written only after
  the commit outcome is known (0030 §Preserves). The re-entry itself
  makes **no semantic change**, so it needs no decision document — that
  is exactly why this frames as `resolved` and not `blocked`.
- **0003** is untouched — no pruning path is in scope.
- No frozen guarantee is amended; 0030 (`proposed`, operator-mandated
  re-entry) remains the authority on the ruled shape.

## Acceptance criteria (checkable claims)

1. **The artifact contract still holds at `95d2823`.**
   `git diff 7550de3 HEAD -- specs/off-worker-parses-loopback-posture/`
   is empty; `git status --porcelain` is empty at HEAD; 0030 is
   `proposed`, indexed, §2 amended (no `h2`/`:authority` claim, Host
   leg fails closed); `src/persist.rs` is at zero diff since `66a45bb`
   and `src/store.rs` carries only the `offload` visibility change.
   (Design re-runs `scripts/design-check.sh` — `designed`, no findings.)
2. **The repairs are proven by their existing pins, re-run, not
   rewritten.** `cargo test --test api` — the three rebind legs
   (`cross_origin_and_malformed_spellings_are_refused_before_the_router`,
   `a_rebound_host_refuses_even_without_an_origin`,
   `an_off_loopback_bind_leaves_the_guard_inert`) go green;
   `cargo test --test compat` pins `kind=subject` as a bare 400 and the
   page-cap refusals; `cargo test --lib` holds the awaits-gauge
   observable-row pins and the search-needle pin
   (`src/compat.rs:847` — uncompilable needle ⇒ `None`, the call site
   answers 400 at `src/compat.rs:390`); `cargo test --test config_cli`
   pins the three loopback defaults; and
   `cargo test --test binary serve_answers_healthz_and_stops_on_sigint`
   goes green at HEAD — the `81409d4` repair's own pin. A red pin is a
   defect, never papered with a sibling twin.
3. **All house gates are green at HEAD**, including the exact coverage
   gate (100/100/100, missed == 0 — the floor may rise, never fall):
   commands below.
4. **The wire stays honest.** The piped `smtp=`/`http=` serve startup
   line parses in any capture — ANSI only when the target stream can
   render it (0029 §4; do not regress the ANSI guard) — in serve and
   mcp both.
5. **The re-entry's delta is exactly the ruled artifacts plus the named
   repairs.** The `a4ff22c..HEAD` delta names only
   `specs/protected-reentry/` (`6f9a4bf`, `b2c95f3`, `c190493`),
   `src/main.rs` (`81409d4`), and `src/api.rs`, `src/compat.rs`,
   `docs/decisions/0030-*.md`, `benches/read_stall.rs` (`95d2823`);
   any further delta is a defect-driven repair with its named pin in
   the same commit; `docs/decisions/` still ends at 0030; no new bench,
   seam, `coverage(off)` or hardening knob.
6. **Review rules on HEAD with the carried weighting.** The
   four-sentence guard ledger, the tenant-boundary scoping, the
   bound-port invariant and the 451/parse-arm distinction are review
   inputs; the run-10 findings are verified repaired at `a4ff22c`, the
   residual at `95d2823`, and neither the pre-repair tree nor run 10d's
   LOW/INFO record is re-litigated; new findings rule as always — a
   security-hold is a hard stop.

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
`tests/binary.rs::serve_answers_healthz_and_stops_on_sigint` is
**diagnosed and repaired at `81409d4`** (streams register on first
poll; the fast-path binds complete without a yield): verify confirms
the pin green at HEAD and does not re-litigate the diagnosis; if it
still fails, record the exit status verbatim and treat it as a defect.

## Blocked check

Nothing in the re-entry contradicts a frozen guarantee: it changes no
semantics, amends no decision, and the one decision in play (0030) is
already committed `proposed` — the lawful path the run mandates.
**Resolved.**
