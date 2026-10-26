# Issue #243 intake — make documentation independent of delivery packets

Run: `epic-192-issue-243-tracked-docs--b8890266`.
Phase: intake; framing resolved. Implementation and completion gates remain outstanding.
Candidate base inspected: `f49d122`.

## Goal

Give the supplied operator rulings a tracked home, replace every tracked Markdown link into `.forge/`, and enforce that boundary so the eval boundary suite passes without delivery evidence packets.

## Authority and spot-checks

Read `.forge/epic192/issue243/README.md` and `operator-rulings.md` first.
Both pass `sha256sum --check SHA256SUMS` from that packet directory:

- README SHA-256: `518cfc062c7dabbf66e625ed57c0bb009fa9ef115c30419382bba4fba92dacf6`.
- Rulings SHA-256: `03a4460d368cc77d7c2c927cd40aff6bab7568fbee5a5ca5b8369f5643c54534`.

The initial checksum command ran from the repository root and resolved the
manifest's relative names there; the corrected packet-directory check passed.
No evidence was changed. The packet's prior-runs section contains no prior
outcomes. Its 2026-10-26 #243 ruling expressly directs the tracked home and
refusal check. Preserve the complete 2026-10-24, 2026-10-25 and 2026-10-26
rulings verbatim, including the section labeled epic #164; do not filter their
contents by epic or infer acceptance of further decisions.

Read the house rules, decision index, accepted 0055, 0050 and 0056, and
proposed 0075/0076. The operator specifically authorizes changing only the
broken link in accepted 0055. This is a citation repair under supplied authority,
not a semantic amendment. Keep 0075 and 0076 proposed and preserve 0055's
accepted status and historical text.

The named owner is `sequential_contracts_and_decision_links_are_honest` in
`evals/tests/boundary.rs`: it checks local links in four fixed documents using
filesystem existence. The supplied packet explains why that passes in delivery
worktrees and fails on a clean main checkout; intake does not re-derive the
reported failure. Spot-checks confirm the cited links in `evals/README.md`,
0055, 0075, `specs/agent-eval-harness/{spec,plan}.md` and
`specs/eval-privacy-port/{framing,evidence,spec}.md`. The packet also identifies
older spec links to run-local design/intake artifacts. Inventory all tracked
Markdown during implementation, rather than limiting repair to these examples.

Relevant history: `422dd9b` (#199/#241) added the sequential contract boundary
assertions and citation; `c13a507` (#200/#237) added provenance, and `aec4b46`
(#198/#231) added trap-scoring changes. Preserve the existing boundary assertions.
`evals/` remains its own unpublished workspace with an independent lockfile.

No frozen guarantee 0001–0003 is directly adjacent to this documentation/test
repair: synchronous accept/store, subscribe-before-check and oldest-only pruning
are unchanged. No application runtime, eval execution semantics, paid trial,
namespace implementation, dependency addition or license widening is required.

## Checkable acceptance criteria

1. A tracked `docs/rulings/epic-192.md` contains the complete supplied
   `operator-rulings.md` verbatim. A byte comparison against the checksum-verified
   source passes, and Git lists the destination as tracked. Do not rewrite,
   summarize, omit or add rulings. This copies existing authority; it accepts
   no proposal and authorizes no additional work.
2. Every tracked Markdown link whose local destination points into the
   repository's `.forge/` directory is removed or replaced with a resolving
   tracked citation. Ruling citations point to the tracked home (optionally its
   actual section anchor) or the decision recording that ruling. Older links to
   design/intake positions point to the tracked decision/spec recording their
   adopted substance, or have the unavailable hyperlink removed with truthful
   historical wording. Do not invent tracked copies of unavailable evidence or
   claim that the rulings file contains those positions. An inventory over
   `git ls-files` finds zero remaining prohibited link destinations.
3. The diff to accepted `docs/decisions/0055-agent-eval-harness.md` changes
   only the link destination currently naming the issue199 operator packet.
   Its label, prose, status, historical sections and semantics are byte-for-byte
   unchanged. 0075 and 0076 remain proposed, with no new decision acceptance or
   unrelated decision-index changes.
4. Add an executable regression check, in the eval boundary suite or a
   root-level check with an explicit verification command, that examines all
   tracked Markdown files, including docs and old specs, and refuses links into
   `.forge/` even when the referenced file exists locally. Cover inline and
   reference-style Markdown links and relative destinations that normalize into
   that directory. Ordinary prose/code mentions of run-local paths are allowed;
   the ban is on links. Fail with the source file and offending destination.
   Exercise both an allowed tracked citation and prohibited link controls through
   the actual checker. A temporary tracked Markdown fixture outside the four
   existing boundary documents with a link to an existing `.forge/` target must
   turn the check red; restoring it must turn the check green. Never leave the
   mutation staged or committed in the final candidate. If implemented outside
   `boundary`, design must name its exact command and add it to verification.
5. On the committed candidate's tracked-only tree, assert that
   `.forge/epic192` does not exist, then successfully run
   `cargo test --manifest-path evals/Cargo.toml --locked --test boundary`.
   Preserve every existing boundary assertion, the standalone unpublished
   workspace/lock boundary, and local citation validity. Do not recreate packet
   files, exempt the failing document, or skip/ignore the test to obtain green.
6. Commit the implementation unsigned and pass all local house gates below,
   exact coverage and selftest included, plus independent eval formatting,
   Clippy and tests. Keep the candidate's existing `bundles/` routing and pinned
   proof ownership. No push, merge, tag, publication, issue closure, sealing or
   decision acceptance is performed by delivery seats.

## Verification commands and ownership

These are implementation/verify obligations, not intake pass receipts. Run
on committed candidate changes, offline with preprovisioned tools; install
nothing. The candidate-local engine was independently hashed during intake and
matches the required digest. Check it again before use:

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
cmp .forge/epic192/issue243/operator-rulings.md docs/rulings/epic-192.md
git ls-files --error-unmatch docs/rulings/epic-192.md
cargo test --manifest-path evals/Cargo.toml --locked --test boundary
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
sha256sum --check --strict bundles/self/scripts/shims.sha256
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

The canonical bundle scripts above are the implementations of the house
inventory's public shims (`bash scripts/coverage-gate.sh`, its `selftest`,
`bash scripts/publish-guard-test.sh`, `bash scripts/seal-branch-test.sh`).
Verify checks shim digests and executes dispatched pinned canonical copies,
never candidate wrappers, under 0050. Coverage must have exactly zero missed
lines, branches and functions; no exclusions, lowered floor or rounded result.
Bundle compilation must also refuse `lets the run go on without reading` even
if compilation returns zero. Use the existing recipe and protected review;
intake neither changes routing nor chooses the next phase.

Run the fail-closed absence proof in its own shell, capturing the tree command's
status without `set -e` terminating before the expected absence diagnostic:

```bash
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf 'REFUSAL: native-tls present\n%s\n' "$out" >&2; exit 1
elif printf '%s\n' "$out" | grep -q 'did not match any packages'; then
  printf 'native-tls absent\n'
else
  printf 'REFUSAL: absence unproved (exit %s)\n%s\n' "$code" "$out" >&2; exit 1
fi
```

For criterion 5, export only the committed candidate, without touching or
renaming the authoritative evidence packet. Initialize an isolated index so a
checker using `git ls-files` sees precisely the exported tracked files. Run
from that tree so Git never falls back to the delivery worktree's index:

```bash
set -e
repo="$PWD"
mkdir -p "$repo/target"
proof=$(mktemp -d "$repo/target/issue243-clean.XXXXXX")
git archive --format=tar HEAD | tar -xf - -C "$proof"
git -C "$proof" init --quiet
git -C "$proof" add --all --force -- .
test ! -e "$proof/.forge/epic192"
(
  cd "$proof"
  export CARGO_NET_OFFLINE=true
  export CARGO_TARGET_DIR="$repo/target/issue243-clean-evals"
  cargo test --manifest-path evals/Cargo.toml --locked --test boundary
)
```

If the new checker is a separate root check, also run its specified command
inside this tracked-only tree. Keep the red/green mutation proof and clean-tree
command statuses as current-candidate evidence. No benchmark is needed for
this documentation and regression-check change; root tests retain real-server
protocol coverage and standalone eval tests retain their existing controls.

CI owns the house inventory's Helm/chart-bump/history/selftests, Docker smoke,
network RustSec, privileged port-80 bridge and tag-only release-rate/publication
proofs. Per the supplied ruling, #201 owns macOS eval evidence. The operator
owns live GitHub settings, sealing and decision acceptance. None is a new
local acceptance prerequisite; no network, namespaces, GitHub, CI, journal or
sibling-worktree inspection is authorized here. Record a missing local input
by its exact path; do not substitute a historical pass or fabricate evidence.

Intake resolves the framing only. Implementation, mutation controls, all
completion gates and independent review remain outstanding.
