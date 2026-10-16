# Tasks — the protected re-entry

Ordered. Every task is paired with the test that proves it; a task whose
proof cannot run is not done. Tasks 1–3 are the implement seat's
confirmation (zero edits expected), 4 is its defect path, 5 is verify,
6–8 are review's, 9 is the landing audit.

- [ ] 1. Confirm the artifact contract at the implementation seat's HEAD
  (the framing's own criterion): the ruled spec unchanged since its
  design commit, 0030 `proposed` and indexed with its amended §2.
  **Proof:** `git diff 7550de3 HEAD -- specs/off-worker-parses-loopback-posture/`
  is empty; `docs/decisions/0030-loopback-plane-bounded-results.md`
  carries `- **Status:** proposed` (line 3), its index row in
  `docs/decisions/README.md`, and the §2 provenance paragraph
  (0030:127-132). The re-entry's own framing copies
  (`specs/protected-reentry/framing-10b.md` and
  `framing-10b-successor.md`) are committed
  (validation-family precedent), so `scripts/design-check.sh` reads
  `designed` with no findings under a working git env.
- [ ] 2. Confirm the landed shape is the ruled shape at minimum size —
  the boundary pins held through the whole family including the repair.
  **Proof:** `git diff 66a45bb HEAD -- src/persist.rs` is empty;
  `git diff 66a45bb HEAD -- src/store.rs` shows exactly the
  `pub(crate)` visibility change plus its doc comment;
  `git diff 66a45bb HEAD --stat -- src/ Cargo.toml` names no new file
  and Cargo.toml's only delta is the `[[bench]]` block; `src/config.rs`'s
  diff is the three loopback literals and their doc comments (no new env
  or flag — `cargo test --test config_cli` pins them).
- [ ] 3. Re-run the repair's own pins — verify, never rewrite. **Proof:**
  `cargo test --test api` —
  `cross_origin_and_malformed_spellings_are_refused_before_the_router`
  (`tests/api.rs:1295`),
  `a_rebound_host_refuses_even_without_an_origin` (`:1349`),
  `an_off_loopback_bind_leaves_the_guard_inert` (`:1431`) — the rebind
  dies on both shapes and off-loopback binds are inert on both legs;
  `cargo test --test compat` —
  `mailhog_v2_lists_and_searches_with_upstream_shapes` pins `kind=subject`
  as a bare 400 (`tests/compat.rs:536-540`); `cargo test --lib` — the
  awaits-gauge inline pins (`src/api.rs:1616-1630`) render a row for a
  live wait and none for a quiescent inbox; every raw-HTTP leg rides
  `tests/common/mod.rs`'s honest `Host: {addr}` harnesses. A red pin is
  a defect (task 4), never a sibling test.
- [ ] 4. If — and only if — a confirmation goes red: ship the smallest
  local fix beside its wire pin and re-run the full gate set in the same
  batch; owe a decision document only if the defect touches a frozen
  guarantee or the 0030 posture. **Proof:** the pin that went red is
  green again with no sibling added (AC2.5's rule), and
  `git diff a4ff22c HEAD --stat` names exactly that repair and its pin —
  nothing else. No red pin found → this task records "no defect" and is
  still done.
- [ ] 5. Run the hermetic gate set as the verify seat, network denied.
  **Proof:** `cargo fmt --all -- --check`; `cargo clippy --workspace
  --all-targets --locked -- -D warnings`; `cargo test --workspace
  --locked`; `brokkr compile --bundle bundles/self`;
  `bash scripts/coverage-gate.sh` (exact 100/100/100, missed == 0;
  SIGBUS → disk-backed `TMPDIR`); `cargo deny check licenses` — all exit
  0, and the piped serve startup line parses in any capture
  (`tests/binary.rs` `startup_addrs`, `:50-54` — the 0029 §4 ANSI guard).
- [ ] 6. Review judges the tree at HEAD weighted against the ruled spec,
  the amended 0030, and the spec's weighting section — the four-sentence
  ledger, the bound-port invariant sentence, the tenant-boundary
  scoping, the 451/parse-arm distinction, the two 0001-durability
  sentences (the commit→250 duplicate window; the WAL/FULL pragma as
  0001's disk leg), the third panel's record sentences (a `250` is true
  when written — retention is the operator's cap; a poisoned boundary
  dies loudly, never quietly retries; arm crash surfaces before the
  wire answers — read for new signals, listeners and spawns the way
  0001 is read), the fourth panel's items (the corrected webhook
  delivery sentence; the serialized-boundary-as-throughput-ceiling
  sentence; the priced 400-arm wire residual — review's to take as a
  finding, the ~8-line `tests/compat.rs` test pre-priced beside it),
  and the no-re-litigation rule for the pre-repair
  findings. **Proof:** each review member's verdict
  names the weighting it applied; a re-opened pre-repair finding without
  a new observable is refused by the weighting itself.
- [ ] 7. Hold every claim review endorses to what the wire can observe
  (the awaits-gauge discipline). **Proof:** each landing-note sentence
  about the loopback default scopes it to network-reachable clients and
  leaves cross-session isolation to 0004 (`README.md:153-155` is the
  pattern); any claim without a proving pin is dropped, not reworded
  into vagueness.
- [ ] 8. Audit the re-entry's own delta at ship. **Proof:**
  `git diff a4ff22c HEAD --stat` names only this contract's spec
  artifacts (6f9a4bf and the synthesis commits) and defect-driven
  repairs, each with its named pin in the same commit (to date:
  `81409d4`'s signal reorder pinned by
  `serve_answers_healthz_and_stops_on_sigint`; `95d2823`'s refusal
  pinned at `src/compat.rs:847`);
  `git diff a4ff22c HEAD --
  Cargo.toml` is empty unless a defect demanded a dependency (a decision
  document, not an edit); `docs/decisions/` still ends at 0030 (no
  `0031`); no new bench, seam, `coverage(off)` or confirmation-test twin
  exists; the two declined renames (`origin_guard`'s name,
  `origin_is_loopback_of`'s fold) are still declined.
- [ ] 9. Record every seat-bound finding through its named channel, not
  the journal alone. **Proof:** `.brokkr/state/run10b-operator-edits.md`
  (gitignored, the sanctioned channel) carries the design-check
  env-vacuity note and anything else a seat could not lawfully edit;
  `scripts/design-check.sh` itself is untouched by this run (a design
  phase does not edit its own judge), verified by
  `git diff a4ff22c HEAD -- scripts/design-check.sh` being empty.
