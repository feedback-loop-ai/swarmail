# Implementation plan — signing policy

Implements [spec.md](spec.md) and proposed
[0056](../../docs/decisions/0056-signing-policy.md).

## Technical context

One Rust crate, with existing Bash delivery gates and Git/GnuPG tooling.
Use Bash, Git and the installed `gpg`/`timeout` utilities; no Cargo dependency
or license allowlist change. Gate hosts must provide those tools; verify
installs nothing and refuses missing prerequisites rather than skipping.
The plan template is absent from `.specify/templates/`; the required
technical context, constitution check, structure, research, data model,
contracts and quickstart are supplied here.

## Constitution check

No `src/` changes; accept == stored, subscribe-before-check, oldest-only
pruning, and ingest-time extraction are unaffected. Exact coverage remains
mandatory. Follow the existing pinned-proof boundary documented by the
house rules and proposed 0050 without accepting that entire proposal.
The operator's explicit unsigned-commit ruling overrides the old signing
sentence immediately. Design retains a proposed semantic artifact for its
validator; implementation records the already-authorized accepted policy.
Only the operator invokes successful signing and optional push.

## Research and decisions resolved

The reference uses same-base `rebase --force-rebase --exec commit --amend -S`;
that conflicts with the literal no-rebase instruction. Use existing trees
and Git object creation instead, preserving the reference's CLI and integrity
contract. This is logical amendment of each commit, with no patch replay.
Retaining a successful reference test is not proof of a different mechanism.
The reported merge-scratch and lanetally PR #230 evidence remains attributed
to the reference; adapted success/interruption evidence remains operator-only.

The reference's text digest is insufficient: replacing parent lists with
the word “parents” loses topology, and `%at` omits author timezone. Read raw
objects into files and compare exact message/author records and ordered
parent mappings. Do not place NUL records or messages into shell variables.
Avoid a key allowlist that rejects valid selectors merely to compensate for
shell interpolation: pass the nonempty selector as a quoted single argv
value, never eval or a generated `sh -c`/`--exec` program.

The test cannot copy unpinned code into scratch and call it pinned. Choose
canonical bundle implementations for both operator tool and proof. This
adds a shim but avoids a new exception to the proof ownership policy.

## Project structure and files touched by implementation

| Path | Responsibility |
|---|---|
| `bundles/self/scripts/seal-branch.sh` | Canonical operator-only tool; adapt the reference contract and implement safe object amendment. |
| `scripts/seal-branch.sh` | Executable fixed dispatch shim. |
| `bundles/self/scripts/seal-branch-test.sh` | Canonical offline keyless scratch proof, locating the canonical tool beside itself. |
| `scripts/seal-branch-test.sh` | Public fixed test shim. |
| `bundles/self/scripts/shims.sha256` | Digests for both new shims, without weakening existing entries. |
| `bundles/self/scripts/verify-seat.sh` | Required resource inventory, pinned proof invocation and honest success summary. |
| `.github/workflows/ci.yml` | Existing quality-job proof invocation; confirm the actual workflow filename before editing. |
| `tests/delivery_constitution.rs`, `bundles/self/scripts/delivery-proof-test.sh` | Update only existing resource/wiring/fixture expectations required by the new proof; retain refusal controls. |
| `docs/house-rules.md` | Commits/history policy, signature taxonomy, gate inventory and shim list. |
| `agents/charters/implementer.md` | Unsigned, never push, no signing stop. Search other active charters and edit only contradictory instructions. |
| `docs/decisions/0056-signing-policy.md`, `docs/decisions/README.md` | Record accepted operator policy and index entry during implementation. Preserve prior decision statuses and history. |

AGENTS.md and CONTRIBUTING.md currently contain no seat-signing requirement;
their existing pointers and release-tag instructions need no gratuitous edit.
Historical specs and release records are evidence, not active charter text.

## Data model and amendment algorithm

Temporary, private files contain the captured branch name, old head OID,
resolved base OID, unique merge-base OID and parent-first list of commits in
`merge-base..old`. Refuse ambiguous multiple merge-bases, shallow/incomplete
history, replace/graft interpretations, unresolved operations, or an
unsupported commit format rather than inventing a topology. A temporary
old-OID/new-OID map drives ordered parent substitution; no persistent seal
manifest or backup-ref namespace is needed.

1. Validate options, repository state and base. Capture immutable OIDs;
   never fetch. Handle empty range before requiring signing tools or a key.
2. Compute signature counts. In check mode report availability and return
   without ref/index/worktree changes. Disable GPG agent autostart for the
   bounded availability probe; a missing configured key reports unavailable.
   An already-good range is idempotent under the reference trust policy.
3. For a needed seal, require signing availability. Read commit fields and
   message bytes into temporary files. Create each signed commit with
   `git commit-tree -S` using its original tree, ordered mapped `-p` arguments,
   explicit original author environment and message file. Do not invoke hooks.
   Bound each signing command, separately from the availability probe. Do not
   trust `commit-tree` to preserve exotic headers or message termination:
   compare the resulting raw fields and refuse any discrepancy before moving
   the branch. Preserve supported encoding metadata explicitly; refuse other
   unsupported headers rather than silently dropping them.
4. Independently verify the full mapping against original objects: equal
   count, tree, message bytes, author record, ordered parents and merge-base;
   every new signature must read `G`. Recheck the branch and checkout state
   before publication. `git update-ref` compares the old OID atomically.
   Concurrent local branch changes refuse without resetting somebody else's
   work. No worktree/index operation is needed because the final tree matches.
5. A single EXIT/INT/TERM cleanup removes private temporary files. Before
   publication all failures leave the old branch in place; after atomic
   publication it already points to a verified seal. Unreachable new objects
   on failure are harmless Git garbage. Never promise SIGKILL cleanup.
6. Optional push is the final operation, with an explicit old-head lease and
   full refspec. A push refusal keeps the verified local branch, exits 1 and
   requires operator inspection. No automatic remote reconciliation.

## CLI and test contracts

The spec defines the options, report, no-op precedence and exits; do not
create a second incompatible contract here. Diagnostic errors use stderr
and `seal-branch:`; reports use stdout. `--check --push` never pushes.

The proof creates a temporary HOME/XDG root and mode-0700 empty GNUPGHOME,
clears inherited Git repository/config overrides, sets `GIT_CONFIG_COUNT=0`,
disables system/global config and hooks, and unsets inherited GPG/SSH agent
socket variables. Set local author identity, `commit.gpgsign=false`,
`tag.gpgsign=false`; exercise both absent and dummy configured keys.
Use real Git and real GPG failure, not an injected fake signer. Fixture
branches include a merge and an empty-at-base branch. A local bare origin
is only an observation target for the check-plus-push case. Guard/report
mutations are test-only disposable copies and must fail their matching
assertions; they never exercise successful signing.

## Quickstart and validation

Delivery prerequisite: fetch origin successfully, verify current main, then
create/check out `signing-policy`. Do not silently proceed from stale refs.
Commit with `git -c commit.gpgsign=false commit`; never push from a seat.
Preserve the intake framing as run-local evidence outside the spec directory
before the chief's final commit, so untracked framing does not invalidate
the design. Commit exactly spec.md, plan.md, tasks.md and proposed 0056;
run `bash bundles/self/scripts/design-check.sh check .` on that HEAD.

Implementation runs `bash scripts/seal-branch-test.sh`, then the complete
house inventory: `cargo fmt --all` followed by `cargo fmt --all -- --check`;
`cargo clippy --workspace --all-targets --locked -- -D warnings`;
`cargo test --workspace --locked`; the pinned `brokkr compile --bundle
bundles/self`; `bash scripts/coverage-gate.sh` and its `selftest`;
`bash scripts/publish-guard-test.sh`; `cargo deny check licenses`; and the
existing fail-closed native-tls absence proof. Preserve design-validator
selftest and delivery tamper proof in verify. The actual dispatched verify
bundle must contain the additions; compiling changed resources alone does
not establish execution by the currently pinned seat.

Operator-only usage after the final run: fetch explicitly, inspect with
`scripts/seal-branch.sh --base origin/main --check`, then invoke sealing
with the configured key (optionally `--key <keyid>` and `--push`). Seal before
updating the branch from main. No command in this paragraph authorizes a seat
to sign or push.

## Risks

- **Reference evidence does not cover changed signing code.** Record its
  provenance accurately. CI proves only keyless behavior; review the object
  map and comparisons, and leave adapted signing success/interruption testing
  to the operator. Do not claim the original merge test exercised this code.
- **Exact metadata or graph preservation fails.** Use raw-file comparisons,
  ordered parent mapping and immutable base OIDs; unsupported inputs refuse
  before atomic publication. Do not normalize silently.
- **Process interruption or concurrent branch mutation.** Build and verify
  objects before a compare-and-swap ref update. Never reset the worktree.
  SIGKILL may leave temporary files or unreachable objects, not half-replayed
  history. Concurrent worktree editing during operator sealing is unsupported;
  recheck tracked state before ref publication and document sole ownership.
- **Key hangs or malicious key spelling.** Bound the probe and each signer;
  pass data only as quoted argv. Missing tools are distinct from unavailable
  signing. No host agent is reachable from the keyless gate.
- **Stale refs and lease failure.** Fetch is an explicit operator prerequisite;
  freeze the selected base and lease on the old branch OID. No automatic retry.
- **Proof bypass or stale dispatched bundle.** Pin both canonical scripts,
  check both shims, run the existing tamper controls and inspect actual verify
  receipts. An old active bundle cannot attest to a newly added proof.
- **Unsigned work merged prematurely.** Policy assigns the sole seal step to
  the operator. No CI key or new automatic enforcement is introduced.
