# Plan — decision-record repair

## Technical context

A single Rust crate, Markdown decision records, and the existing Rust
integration suite `tests/store_contract_docs.rs`. No new dependency, runtime
API, schema or storage migration. Source and workflow are read-only grounding.
No plan template is present under `.specify/templates/` (only a spec template);
this plan supplies the required sections directly.

## Constitution check

Preserve synchronous accept==stored, subscribe-before-check, oldest-only
pruning, ingest extraction and all accepted runtime rulings. The only semantic
amendment is the operator-selected existing 0051; record it without inventing
another decision or self-acceptance. Respect the explicit implementation path
allowlist in the spec. Keep historical bodies, pinned proof ownership, locked
gates, exact coverage, permissive dependency policy and protected review.
Commit unsigned under 0056. No push, merge, tag, publish or installation.

Triage selected engine for normative release-policy risk; this design does not
rerun triage, dispatch another panel or downgrade that classification. The
engine owns the configured implementation route and full review. The task's
Luna xhigh pin applies to direct codex-fast implementation, not authorization
to substitute models in this escalated sequence. Configured review retains
Qwen Max correctness, Opus security, official DeepSeek Flash guarantees and
DashScope GLM 5.3 tests with no fallback or DeepSeek Pro.

## Research and evidence

Both panel positions, both triage framings, the decision index and affected
records were inspected. The supplied proposal hash matches the operator's
pin. The tracked 0051 is still proposed; its supplied correction already owns
the deliberate release-policy choice. Its absent `issues/184-source.json`
reference is historical provenance, not evidence freshly obtained here.

`tests/store_contract_docs.rs` has two AMENDMENTS entries for 0051 ->
0019/0045 using PROPOSED_1019. `amendment_problems` checks the successor
header and predecessor marker; the forged-acceptance test reads live 0051.
Adoption requires fixture/expected-state maintenance, not a checker bypass.

`src/smtp.rs` checks AUTH disabled, envelope and prior identity before
credentials. `src/pop3.rs` clears pending USER before validation.
`src/api.rs::the_served_openapi_publishes_the_await_and_assert_domains`
already pins the unsigned inert assert limit, exact count, zero and page
exceptions against `src/static/openapi.json`. 0013/0057 establish review
history, 0021 establishes watcher ownership and 0056 controls signing.

## Project structure and file ownership

| Path under repository root | Implementation |
|---|---|
| docs/decisions/0051-release-records.md | Supplied proposal baseline; current accepted status, original draft provenance, dated adoption appendix |
| docs/decisions/README.md | Only 0051's status/adoption entry |
| docs/decisions/0019-release-tag-integrity.md | Append accepted scoped 0051 amendment marker |
| docs/decisions/0045-delivery-repairs.md | Append accepted scoped 0051 marker and preserve 0056 authority |
| docs/decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md | Append factual AUTH/USER correction |
| docs/decisions/0030-loopback-plane-bounded-results.md | Append both maximum-claim corrections and assert exception |
| docs/decisions/0007-hermetic-gates-single-model.md | Append missing 0013 historical marker |
| docs/decisions/0023-off-worker-commits.md | Append 0021 watcher provenance pointer |
| tests/store_contract_docs.rs | Maintain finite adoption assertions and independent negative fixtures |
| specs/decision-record-repair/{spec,plan,tasks}.md | Chief's committed design only |

## Data model and contracts

There is no new data model. A decision keeps its current status header,
historical body and dated addenda; the index reflects that current status.
A current accepted amendment requires both the successor declaration and a
dated accepted predecessor backlink. A retained proposed backlink is history,
not evidence of adoption. Use the checker's existing marker syntax, for example
`*(Amended/clarified by [0051](0051-release-records.md) (accepted 2026-10-24:`.
Explain that this is acceptance recorded from the supplied task on that date.

0051's publication constraint applies only to successful joint tag releases.
Preparation remains independently versioned and main publication separate.
The unchanged runtime contracts and precise corrections are AC5–AC7; there
are no endpoint, numeric parser, release guard or watcher changes.

## Implementation approach

1. Capture initial HEAD/diff and recheck proposal hash. Use the supplied file
   as baseline, retain its original status as provenance, update current
   header, and append adoption naming the exact authority/hash. Explicitly
   scope all old pending language to the draft. Update only its index row.
2. Append the two 0051 accepted markers in existing syntax, naming the limited
   equality/ordering amendment, preserved guards, preparation and signing.
   Keep old proposed pointers and explicitly supersede their pending status.
3. Append the four dated factual corrections. Review each against its source
   or accepted successor, without changing that source or successor.
4. Replace only the two 0051 expected marker states with accepted 2026-10-24.
   Keep the negative forged-acceptance test using a small proposed fixture
   through the existing read closure. Require failures for each missing
   accepted backlink despite retained historical proposed text, and preserve
   missing-declaration and index mismatch fixtures. Add focused correction/
   adoption assertions in this suite where needed; no second parser or test
   hierarchy. Tests need no gitignored proposal files to run in CI.
5. Audit final paths and historical preservation; review the diff of 0051
   against the hash-pinned input. Human review checks policy fidelity;
   finite tests cannot prove operator identity or arbitrary prose semantics.
6. Run targeted documentation proof and the complete house gate inventory
   against the final implementation. Keep exact commands, exit status and
   assessed tree in run-local receipts; failures are outstanding work. The
   hermetic verifier uses pinned implementations, never candidate shims.
   Retain independent full review and CI-only proof ownership.

## Quickstart and validation

```bash
sha256sum .forge/epic192/proposals/0051-release-records.md
cargo test --locked --test store_contract_docs
cargo test --locked --test pop3 a_failed_user_clears_the_pending_name
cargo test --locked --test pop3 pop3_user_controls_clear_pending_name
cargo test --locked --test smtp_edges auth_succeeds_once_per_session
cargo test --locked --test smtp_edges auth_control_names_refuse_without_identity_change
cargo test --locked --test api the_assert_never_refuses_on_the_page_cap
cargo test --locked --lib the_served_openapi_publishes_the_await_and_assert_domains
```

The behavior tests also run in the full workspace gate; they need not be
redundantly rerun after an unchanged passing full suite. The authoritative
complete gate inventory is [house rules](../../docs/house-rules.md#gates--part-of-every-change),
including fmt, locked clippy/workspace tests, digest-matched pinned brokkr
compile, exact coverage and selftest, publish/seal proofs and license check,
plus verify's fail-closed native-tls proof. CI owns chart/history/selftest,
Docker smoke, advisory audit, same-run tag rate proof and port-80 exchange.
Do not manufacture those receipts with an unauthorized release.

Chief commits exactly the three design files as the last commit, checks
whitespace/paths/cleanliness and may locally run the existing design checker.
Only the sequence's pinned final check rules design success. No house-gate
success or implemented feature is claimed by this design handoff.

## Risks

| Risk | Mitigation and retained cost |
|---|---|
| Historical proposed text looks current | Accepted header plus explicit dated adoption supersedes pending language; retain history at the cost of reader cross-reference |
| Acceptance attribution is invented | Name supplied operator task and exact hash; recording date 2026-10-24, no earlier acceptance timestamp |
| Joint-release equality spreads to preparation | State both domains together in adoption and predecessor markers; human diff review |
| Accepted status weakens provenance proof | Require both fresh markers and preserve missing-marker/declaration and forged-status negatives with local fixtures |
| Protocol correction creates new semantics | Ground narrow wording in code, accepted 0034/0046 and existing real-protocol tests |
| Assert uncapped becomes unlimited numeric input | Explicit parser-domain qualification and served-schema test; no schema/handler edits |
| Old signing/reviewer text regains authority | Link controlling 0056/0057; unsigned commits, configured four reviewers, no fallback |
| Scope expands to other epic findings | Nine-path implementation allowlist; no house-rule, 0050, release-note, attribution or registry edits |
| Stronger availability/durability claims creep in | Preserve registry refusal/partial-release/race limits; no new runtime or release guarantee |
| A green subset masks delivery gaps | Record all final-tree gates and pending CI/review evidence distinctly; no feature-complete claim while required work remains |
