# Plan — tracked ruling links

## Technical context

This change is tracked documentation plus test-only Rust in the independent,
unpublished `evals/` workspace. Use existing Rust/std, Git and the provisioned
shell tools. No production source, dependency, lockfile, workspace membership,
license allowlist or bundle routing change is needed. Git is an explicit test
prerequisite; an archive without its own index is not a valid checker context.

The named `.specify/templates/plan-template.md` is absent and no alternative
plan-template file is present. This plan uses the requested technical context,
constitution check, structure, research, data model, contracts and quickstart
sections directly; no missing template blocks the specified deliverables.

## Constitution check

Preserve synchronous insert/SMTP acceptance, subscribe-before-check, oldest-only
pruning and ingest extraction (0001–0005). The new parser is test-only and must
not be reused on the mail path. Follow 0050's pinned proofs, 0056's unsigned
commits and the current candidate recipes. Copy authority without accepting any
proposal. The operator explicitly permits 0055's link-only repair. No semantic
contract amendment or new ADR is warranted. Root coverage stays exact 100% of
lines, branches and functions; no exclusions or denominator changes.

## Research and resolved choices

The current boundary checks four documents and relies on filesystem existence.
The packet explains how delivery-local evidence masked the clean-checkout
failure. Intake and both panels agree on the tracked home and inventory scope.
The chief chooses one checker in the boundary suite, with explicit eval commands,
not a root script/CI integration. The spec records the syntax decision and costs.
No further external research is needed or authorized.

## Project structure and touched files

| Path | Implementation change |
|---|---|
| `docs/rulings/epic-192.md` | New verbatim copy of verified authority |
| `evals/tests/boundary.rs` | Private inventory/destination/check helpers, focused fixtures and repository-wide refusal test; retain existing tests |
| `evals/README.md` | Repair citation; state checker scope, Git prerequisite and explicit boundary command |
| `docs/decisions/0055-agent-eval-harness.md` | Exactly one link-destination replacement |
| `docs/decisions/0075-eval-privacy.md` | Repair citation without status/contract edits |
| `specs/agent-eval-harness/{spec,plan}.md` | Repair ruling citations |
| `specs/eval-privacy-port/{evidence,framing,spec}.md` | Repair ruling citations |
| Historical specs discovered by Git inventory | Remove unavailable panel/intake hyperlinks or replace with truthful tracked synthesis citations |
| `specs/tracked-ruling-links/{spec,plan,tasks}.md` | Chief design and subsequent task/proof status as appropriate |

Known historical owners include `concurrency-tests-coverage-gate`,
`living-listeners-bounded-shutdown`, `one-ordered-mutation-boundary`,
`p1-persistence-closure`, `persistence-failure-contract`, `store-boundary`,
`total-shutdown` and `retain-cleanup-drains`. Validate a tracked replacement
such as `docs/tasks/issue-199-retain-cleanup-drains.md` before using it.
The Git inventory determines completeness; these examples are not exemptions.
Do not add optional checklists, metadata sidecars or extra chief-commit files.

## Data model and contracts

No persistent data/API changes. Internally separate three small operations:

1. `tracked_markdown(root)` obtains a checked `git ls-files -z` inventory from
   the explicit root and returns Markdown paths or an error. Include `.md`
   case-insensitively; preserve NUL path boundaries. Require a nonempty set.
2. A private scanner returns original destination plus source location for the
   spec's bounded grammar. Mask code spans/fences before scanning; maintain
   balanced/escaped destination handling and reference definitions. Do not
   describe splitting on `](` as a complete parser.
3. A lexical classifier separates external/fragment references from local
   paths, strips suffixes, percent-decodes once and resolves components. Retain
   an above-root condition so excessive `..` cannot bypass `.forge` refusal.
   The checker returns violations/errors with source and destination context.

The repository test and isolated fixtures invoke the same checker helpers.
Git failure, invalid inventory encoding, unreadable/missing tracked Markdown
and empty inventory must all be errors. Use unique temporary repositories
(existing `tempfile` is available), local Git configuration only, and no shared
mutable index. Fixtures cover both existent and nonexistent packet targets;
existence cannot influence the lexical verdict. Table tests cover syntax and
path cases; integration fixtures cover actual tracked discovery and diagnostics.
No reusable/public parser API or new crate is needed.

## Implementation approach

Verify source hashes again, copy the authority without annotations, and record
its digest/cmp proof while the source is available. Inventory tracked Markdown
and classify each affected citation as ruling authority or historical run-local
evidence. Repair links with document-relative paths. For 0055, use a single
literal destination replacement and compare the whole result to that exact
transformation of the base blob. Review replacement destinations against Git's
tracked files, not only filesystem existence. Preserve dated clarification text.

Add the checker and fixtures in the boundary owner. Keep the original
`sequential_contracts_and_decision_links_are_honest` assertions, including
existing link validity and decision statuses. Document the grammar and local
invocation in evals/README. Commit the completed implementation unsigned before
producing the isolated candidate proof; any later edits require a fresh proof
of the resulting commit.

## Quickstart and verification

Use the commands in [intake](../../docs/tasks/issue-243-tracked-rulings.md#verification-commands-and-ownership)
and the single [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change).
Set `CARGO_NET_OFFLINE=true` and prepend candidate `.forge/bin` to PATH. Verify
`.forge/bin/brokkr` against
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`
before compilation. Reject the policy-sweep warning named by intake even if
compilation exits zero. Use canonical pinned scripts after shim checks, including
coverage and selftest, publish/seal proofs, design selftest and delivery proof.
Run independent eval fmt, locked all-target Clippy and workspace tests explicitly.

For AC5, use `git archive HEAD` into a unique directory beneath ignored `target/`,
with pipefail enabled. Initialize its own Git repository and force-add exactly
the exported files before creating build output. Assert no `.forge/epic192`.
Run the exact boundary command from that tree, offline, optionally with an
external target directory. Record HEAD's source identity and each exit status.
Then add a tracked Markdown fixture outside the four existing documents, create
an ignored target under `.forge/issue243-control/` (not `.forge/epic192`), and
require the actual boundary command to fail for that fixture's diagnostic.
Remove the fixture from the isolated index/worktree and its temporary target,
reassert packet absence and require the same command to pass. Keep Git fixture
mutations sequential; never rename the live packet or use sibling worktrees.

Store run receipts under ignored `.forge/` as plain paths, never tracked links.
An unexecuted/failed gate remains outstanding; design-only receipts cannot prove
implementation criteria. CI/operator ownership is stated in the spec.

## Risks

- **Partial grammar:** bounded scanning can misclassify unusual syntax. Keep
  grammar explicit, test code masking, escaped/balanced destinations and reference
  definitions; do not claim HTML/general-renderer coverage or add a dependency.
- **Inventory masking:** current packets or inherited Git discovery could hide
  violations. Use explicit roots, checked NUL inventories, fail-closed errors
  and isolated repositories; prove refusal even for an existing ignored target.
- **False authority:** copying rulings or retargeting old positions could imply
  new acceptance. Copy bytes, preserve dates/statuses, review each citation's
  claim and remove unavailable hyperlinks honestly.
- **Accepted-record drift:** a broad rewrite could change 0055. Require exact
  whole-file equality to a single destination replacement of its base blob.
- **CI gap:** root tests omit evals until #201. Require explicit local eval
  commands now and assign CI integration to #201 without inventing its receipt.
- **Proof drift or damaged evidence:** test only exported committed source,
  preserve candidate identity and statuses, and leave the live packet intact.
- **Stale ledger:** the verbatim source's historical next-free-number statement
  is dated evidence, not permission to allocate a number. This repair allocates
  none and changes no decision status or index authority.
