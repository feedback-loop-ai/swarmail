# Implementation plan — issue #154 operator continuation

Implements [spec.md](spec.md) under proposed
[0097](../../docs/decisions/0097-focused-review-repair.md) and the committed
[intake](intake.md). The plan template is absent from `.specify/templates`;
this plan supplies the dialect's technical context, constitution check,
structure, research, data model, contracts, quickstart and risks.

## Technical context

One production Rust crate (serde/serde_json, rusqlite, Tokio, Axum) with the
standalone `evals/` crate outside the root workspace. The candidate is
`d7647a6` (tree `ed588b3…`); its code equals implementation head `1979c8e`.
The chief commits exactly `spec.md`, `plan.md` and `tasks.md` in this
directory. Position files and `.brokkr/state/` stay uncommitted. The roster is
the zero-Codex claude-careful recipe (0099): Sonnet 5.5 implements; Fable
correctness, Flash security and guarantees, Spark GLM-5.3-Flash tests and Opus
independent review. No seat uses Codex.

## Constitution check

- 0001–0005 preserved: no production change is planned; a demonstrated-gap fix
  must keep commit-before-`250` inside `record_insert`, subscribe-before-capture
  in `Store::wait_focused`, oldest-only pruning and ingest extraction.
- 0020 placement: any new witness is inline and measured unless it needs a
  cold path; none is foreseen.
- 0045 §8: 0097 is proposed; this work contradicts no accepted ruling.
- 0043 §4 OPTIONS semantics and 0063/0071/0076 strict eval v2 stay as carried
  on `1979c8e`.
- 0050 §1: the verify seat runs the pinned bundle scripts; the candidate shims
  are checked against `shims.sha256`, never executed as proof.
- 0056: commits unsigned, never pushed; sealing is the operator's.
- 0081: the HIGH review-integrity residual is carried as a fact.

## Research — what is already true at HEAD

| Finding | State on `d7647a6` | Witness |
| --- | --- | --- |
| C1/G1 MEDIUM, derived readers admit positional occurrences and tagged sources | Fixed: map-only `LinkMetadata` over a closed `Members` set decoded from raw entries; string-only `LinkSource` (`src/model.rs`) | `persist::tests::metadata_null_empty_present_and_corrupt_are_distinct`; `tests/persistence.rs::a_corrupt_occurrence_column_refuses_to_serve` |
| G2 LOW, blanket "unchanged" prose | Fixed on README, served llms text, served OpenAPI | `mcp_revisions::focused::served_prose_states_additive_discovery` |
| C2 MEDIUM, eval conflict markers | Resolved against `4806ab8` (strict v2); no markers under `evals/`, `src/`, `tests/` | locked `--no-run` build; spawn-free scoring tests |
| #153 `393247c` port; `8d205d7`/`7bdb1e1`/`4227aef` cherry-picks | Present | `mcp::tests::every_tool_has_exactly_one_object_output_schema`; modern refusal/silence tests |
| #152 `f0b2b14` OPTIONS/shared-limiter port | Present | `api::tests::the_router_owns_one_tool_rate_across_connections_revisions_and_members` |
| C3/G3 HIGH security-relevant residual | Carried, not ours to fix | 0081 must-fail witness |

Conclusion: no implementation gap is demonstrated by static reading. The
implementer re-reads each witness against the code and fixes only what a
failing gate or a concrete red test demonstrates.

## Data model

Unchanged. An occurrence is `{url: string, anchor_text: string|null (present),
rel: string|null (present), source: "text"|"html"}`; column 19 holds `NULL`
(unknown history), `[]` (known empty) or a JSON array of such objects; any
other shape refuses restore with a column-19 Text conversion failure. No
migration, backfill or schema version.

## Contracts

Unchanged: the fifteen-tool MCP catalogue (thirteen preserved March/default
descriptors plus two appended focused waits), object-root output schemas,
OPTIONS 405/`Allow: POST`/no CORS under 0043 §4, REST OpenAPI projections,
and the focused wait result/schema contract of 0094. The implement result
contract gains no new keys: typed facts go under `inputs`.

## Project structure and files touched

- `specs/issue154-review-followup/{spec,plan,tasks}.md` — this design (chief).
- `specs/issue154-review-followup/implementation.md` — the implementer's
  record for this run (new; one owner, so
  `specs/focused-review-repair/implementation.md` stays as history and is not
  edited).
- `target/receipts/<HEAD>/` — gate receipts and `SHA256SUMS`, untracked.
- `src/`, `tests/`, `evals/`, `docs/decisions/`, `README.md` — untouched
  unless a demonstrated gap forces a minimal owner-local fix (AC7).

## Quickstart (implementer)

```bash
git rev-parse HEAD HEAD^{tree}
sha256sum --check --strict bundles/self/scripts/shims.sha256
sha256sum .forge/bin/brokkr   # must equal tooling.json binary_sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh && bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
cargo tree -i native-tls   # accept only the explicit no-package-match outcome
cargo test --manifest-path evals/Cargo.toml --locked --no-run
cargo test --manifest-path evals/Cargo.toml --locked --test scoring
```

Receipts go under `target/receipts/` (outside the tree) and name the HEAD and
tree they bind to. Commit the record unsigned; rerun the gates if the commit
changes the tree, so the receipts bind to the final HEAD.

## Failure modes the robustness position raised, and the standing answer

1. **Crash between accept and commit.** `insert` commits inside the mutation
   boundary and the DATA arm awaits it before `250`; a failed commit refuses
   with `451`. Accepted loss: a crash after commit and before the reply can
   leave stored mail a sender retries. No change.
2. **Hung webhook target.** Fan-out is off the accept path, bounded by
   timeout, attempts and admission caps, with `aborted`/`exhausted`/`dropped`
   accounting. Accepted loss: best-effort notifications. No change.
3. **Per-inbox cap racing a burst.** Pruning and metadata deletes are planned
   and committed in the same transaction. Accepted loss: the cap evicts older
   accepted mail. No change.
4. **Chaos mid-pipeline.** Poles only (0/100), explicit seams; a wait across
   a chaos refusal rescans the store. No change.
5. **Hostile DATA body.** Bounded framing, selector and output caps; the
   source-proportional occurrence and snapshot cost stays a documented
   availability residual, never hidden by truncation. No change.
6. **Certificate expiring under a test.** Generated fixtures per run; TLS
   verification stays on. No change.
7. **Restart mid-flight / many parallel sessions.** Strict load refuses before
   readiness on damaged metadata, paired with a valid populated control;
   per-test inboxes, fresh stores, unique data files, real deadlines. A
   private lock is not host-wide coordination. No change.

## Risks

| Risk | Mitigation |
| --- | --- |
| `complete` read as a story-completion claim | Typed `inputs` (`coordinated_eval_proof: "unavailable"`, residual facts) and a record that names every outstanding obligation; review members read that record before ruling. |
| AC4/#201 remain unproved after the run | Accepted and recorded; only authentic host-wide coordination can close them and it is not on this stack. No private lock, scratch copy or paid trial substitutes. |
| A gate fails on the final tree | Fix within AC7 in the owning module with a red-on-old witness, rerun the whole inventory, rebind receipts; if not repairable within the invariants, return `blocked` with the output. |
| Receipts drift from the final HEAD | Receipts name HEAD and tree; a record commit after gates triggers a rerun. Receipts stay untracked. |
| Upstream #152/#153 drift | Recorded as outstanding; nothing imported or invented. |
| Inherited HIGH residual stops the run before ship | Intended: the security member and the engine own it; the run ends with verify and five bound verdicts, not a ship. |
| Decision-number collisions | Nothing renumbered, accepted or rewritten; operator-owned. |
| Unbounded per-mail occurrence and snapshot cost | Documented availability cost accepted in 0094/0097; no cap invented. |
