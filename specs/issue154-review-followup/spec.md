# Issue #154 operator continuation — re-prove the committed repair

Date: 2026-10-29. Epic #164, issue #154; operator continuation of concluded
run follow-up-epic-164-issue-154-on--2df22b72 on the zero-Codex claude-careful
recipe (accepted 0099, with 0098). Status: design draft, not implementation
completion, verification or a protected verdict.

Decision: 0097 — [Strict restore and full-story focused review repair](../../docs/decisions/0097-focused-review-repair.md)
(proposed; already committed and indexed, cited here as the governing
proposal, not drafted anew).

## What this run is

The repair for the protected panel's findings is already committed. This run
starts from candidate `d7647a6dd3f17d917d8bdf45ca31ff4025c498f8`, tree
`ed588b3dd16cb11514f97fa25473ce17c3addd6d`, whose only delta over the
implementation head `1979c8e` (tree `f9201e2…`) is the committed
[intake](intake.md). The previous run stopped on `IMPL-BLOCKED`
(`.forge/evidence/issue154-repair-followup-stop/implement-event.json`, SHA-256
`544a04f9f99350ddabfaf44266d4461bdfa81b726616cf3c9e853f547f491ae6`) with
`local_gates_clean: true` and every fix committed: the implementer chose
`blocked` so as not to claim story completion while the coordinated eval proof
was unavailable, and that hard stop meant engine verify and the five-member
protected review never ran on this tree.

The feature this run delivers is therefore a **re-proof of a committed tree**:
bind fresh gate receipts to the final candidate, send it through engine verify
and the five protected members, fix only a demonstrated gap, and carry every
proof the realm cannot produce as an explicitly outstanding obligation.

## Why

- **Verify and review are owed to this tree.** The commission says engine
  candidate-bound verification and five-member protected review are needed and
  that what can be verified now should be verified with outstanding
  obligations honestly retained. A second `blocked` reproduces the previous
  stop and yields no new evidence.
- **No prior verdict transfers.** The panel at
  `.forge/evidence/issue154-focused-review-stop/panel-event.json` (SHA-256
  `b1ebc081…891a`) ruled on `5dc3bc94…`; its historical verify and verdicts
  apply only there.
- **The code already enforces the promised shape.** Confirmed by the chief's
  static read of HEAD: `LinkMetadata` restores only through a map-only reader
  over a closed `deny_unknown_fields` member set decoded from the raw entries,
  and `LinkSource` restores only from the strings `text`/`html`
  (`src/model.rs`); the persistence bridge surfaces serde failure as a
  column-19 Text conversion failure through `Persist::load` and startup
  (`src/persist.rs`). The witnesses named in
  `specs/focused-review-repair/implementation.md` exist at HEAD; the eval
  conflict markers are absent; 0097 is indexed beside 0094.

## Panel synthesis

The simplicity and robustness positions (run-local, uncommitted) agree on
almost everything; the chief rules where they do not.

**Adopted from simplicity.** No new production code, test file, script,
decision, dependency or renumbering is commissioned; the spec artifacts live
beside the committed intake under one directory; implementation is a re-proof
plus an honest record; in-tree host-wide eval coordination, `trial_scope`
fixture runs, the #152 `9317cd1`/`70e85fc`/`7f2e2129`/`ab80743` and #153
`df8f610` imports, and reader refactors are cut.

**Adopted from robustness.** Its prohibitions are the design's invariants, so
that "fix only a demonstrated gap" cannot be satisfied by a smaller diff that
breaks a trust boundary: no derived occurrence reader, no
`serde_json::Value`-normalizing validator (it collapses duplicate keys), no
lenient `from_json` fallback for the occurrence column, no occurrence cap, no
query-time re-extraction, no post-`250` metadata write, no background prune,
no durable webhook outbox on the accept path, no poll-based focused wait, no
removal of the terminal scan, no disabled TLS verifier, no private lock
presented as coordination, no transferred verdict. Its seven failure modes
are recorded in the plan with the existing answer or the accepted loss.

**Ruling on the one real conflict: the implement result.** Simplicity says
return `complete`; robustness says a missing proof is an obligation, not a
pass. Both are right about different things, and the chief rules for
`complete` with the obligation carried in typed facts. The implement
vocabulary in `bundles/self/policy.json` is `complete` ("implementation
complete and committed", next: verify), `broken` (retry) and `blocked`
(hard stop). `complete` is a statement about the implementation, not the
story; the story's completion is claimed nowhere in this run. `blocked` is
reserved for a gap the implementer cannot commit around: a gate that fails and
cannot be fixed within the invariants above, or a demonstrated defect that
needs operator input. What the losing side costs: a reader of the journal who
equates `complete` with "fully proven" is misled unless they read the typed
inputs and the record, so both must say the story is open, and the review
members are told so in the record they read.

## Acceptance Criteria

Each criterion is testable on the final committed candidate. "Outstanding"
criteria are satisfied by an honest record of what is missing, never by a
substitute proof.

- **AC1 — Source identities bound.** The implementation record names the
  starting candidate (`d7647a6`, tree `ed588b3…`), the implementation head
  (`1979c8e`, tree `f9201e2…`), the implement-stop event digest
  `544a04f9…91ae6`, the panel digest `b1ebc081…891a`, and the final
  candidate HEAD and tree after any commit. Proof: `git rev-parse HEAD
  HEAD^{tree}` and `sha256sum` outputs quoted in the record match.
- **AC2 — C1/G1 restore defect stays fixed.** `persist::tests::
  metadata_null_empty_present_and_corrupt_are_distinct` and
  `tests/persistence.rs::a_corrupt_occurrence_column_refuses_to_serve` pass on
  the final tree and still refuse a four-element positional occurrence, a
  `{"html":null}` and `{"text":null}` source, a raw duplicate key and the
  missing/extra/wrong-type shapes, each with a column-19 Text cause, while
  valid populated bytes, SQL NULL and `[]` serve. Proof: both tests run under
  `cargo test --workspace --locked`; the reader in `src/model.rs` has no
  `visit_seq` arm and `LinkSource` deserializes only from a string.
- **AC3 — G2 prose stays correct.** `mcp_revisions::focused::
  served_prose_states_additive_discovery` passes: README, served llms text and
  served OpenAPI state thirteen preserved descriptors and results plus two
  appended focused descriptors, with REST-backed projections pinned to OpenAPI
  and focused schemas MCP-only and independently checked. Historical specs
  and captures are byte-unchanged (`git diff --stat 1979c8e HEAD -- specs/`
  shows only this directory's files and any implementation record).
- **AC4 — Reconciliation state recorded, not extended.** The record lists each
  #152/#153 source and the eval `4806ab8` resolution with its disposition
  exactly as on `1979c8e`, states that reviewed #152/#153 heads and current
  `main` remain unavailable, and imports nothing further. Proof: `git log
  1979c8e..HEAD` contains only the intake commit, the design commit and any
  record commit; `every_tool_has_exactly_one_object_output_schema` and
  `the_router_owns_one_tool_rate_across_connections_revisions_and_members`
  pass.
- **AC5 — Standalone eval build proven; execution honestly open.** `cargo test
  --manifest-path evals/Cargo.toml --locked --no-run` succeeds and the
  spawn-free scoring tests pass on the final tree. The trial-launching
  regressions and `focused_tools_complete_link_and_code_traps_without_guessing`
  are **not** run unless authentic host-wide coordination exists with recorded
  evidence; `trial_scope`, a scratch copy or a sequential command is not that
  evidence. The record and the implement result's `inputs` carry
  `coordinated_eval_proof: "unavailable"` and name #201 as unproved.
- **AC6 — Full gate inventory fresh on the final tree.** After checking
  `bundles/self/scripts/shims.sha256` and `.forge/bin/brokkr` against
  `bundles/self/tooling.json`, every house gate runs on the committed HEAD
  with exit 0: fmt, locked clippy, locked workspace tests (including the
  real-server `persistence`, `focused_waits` and `mcp_revisions` suites),
  pinned `brokkr compile` with no unruled policy-sweep warning, pinned
  coverage gate at exactly zero missed lines/branches/functions plus
  `selftest`, pinned `design-check.sh selftest`, `delivery-proof-test.sh`,
  `publish-guard-test.sh`, `seal-branch-test.sh`, `cargo deny check licenses`
  and the fail-closed `native-tls` absence proof. Receipts stay outside the
  tree (`target/receipts/`) with a SHA256SUMS listing and the HEAD/tree they
  bind to. A receipt that names any other HEAD is not a pass.
- **AC7 — Frozen guarantees untouched.** `git diff 1979c8e HEAD -- src/`
  is empty unless a demonstrated gap was fixed, in which case the fix lives in
  the owning module, keeps the insert transaction/commit-before-`250`,
  subscribe-before-capture with resubscription and one terminal scan,
  oldest-only pruning and ingest-time extraction, and ships with a red-on-old
  witness under decision 0020 placement. No `coverage(off)`, feature flag,
  allowlist change or new dependency.
- **AC8 — Residual and ownership carried exactly.** The implement result and
  the record carry `has_security_residual: true` and
  `max_residual_severity: "high"` for the inherited review-integrity /
  result-attribution residual (0081 and its must-fail witness), with no attack
  reproduction, no widened access, no engine re-pin, no invented verdict and
  no claim that an upstream merge changed the pinned engine. The protected
  security member owns its verdict and any hold; the engine owns verify and
  advancement.
- **AC9 — Honest implement result.** The implementer returns `complete` only
  when AC1–AC8 hold and every tracked change is committed unsigned; `inputs`
  carries `candidate_head`, `candidate_tree`, `local_gates_clean: true`,
  `coordinated_eval_proof: "unavailable"` (or the recorded coordination
  evidence), `has_security_residual` and `max_residual_severity`; `notes`
  states plainly that AC4/#201, #152/#153 reviewed heads, #151 paid baseline
  and actual clients, CI-owned platform/network/chart/Docker/port-80 and #201
  macOS evidence, and operator sealing remain outstanding. A failing gate that
  cannot be repaired within AC7 returns `blocked` with the output quoted.
- **AC10 — Decision numbers untouched.** No decision is accepted, renumbered,
  replaced or rewritten; the 0090/0091/0092/0096 collisions and the
  provisional 0094/0097 numbers stay recorded as operator-owned. Proof:
  `git diff 1979c8e HEAD -- docs/decisions/` is empty.

## Frozen guarantees

Near all three and changed by none: metadata joins the synchronous insert
transaction (0001); the focused wait subscribes before capture, including
resubscription after `Closed` and the single terminal scan (0002); pruning is
oldest-only with in-flight protection (0003); extraction stays at ingest
(0005); 0012/0014/0015 boundaries and 0020 placement hold. 0097 is proposed
and the work is compatible under accepted 0045 §8; nothing here needs an
accepted ruling amended.

## Out of scope

Host-wide eval coordination (owned by the #201 lineage and its own proposal),
new decisions, imports from #152/#153 beyond what `1979c8e` already carries,
reader refactors, performance thresholds, checklists (the recipe did not
request the full map), and any push, merge, publication, tag, issue closure,
paid trial or decision acceptance.
