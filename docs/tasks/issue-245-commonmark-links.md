# Issue #245 intake — close tracked-link CommonMark gaps

Run: `epic-192-follow-up-issue-245-clo-52600d1d`.
Phase: intake; framing resolved. Scanner implementation and its red/green proofs remain outstanding.
Inspected candidate: `8958176`.

## Goal

Close the four reported CommonMark findings in the tracked Markdown link guard, preserving existing link classifications and the dependency boundary.

## Authority and bounded inspection

Read the issue245 README and operator rulings first; both pass the packet's
SHA256SUMS when checked from its directory. README digest:
`dba3a19c69a8b7ef6862363eceea04485d886715dc5224f76e4f271f4c0a12f7`;
rulings digest:
`b68560f6782247377e75e90698f95f33feef390cdbdb71ddd8c9a3cc0fbdfd9e`.
The first root-relative checksum invocation used the wrong working directory;
the corrected check passed without changing either source. The prior-runs
section is empty; no prior outcome is inferred.

Read the house rules, decision index, accepted 0050, 0055 and 0056.
0055 keeps evals an unpublished standalone workspace, independently verified;
0050 owns pinned proofs and exact coverage; 0056 requires unsigned commits.
The operator's dated #243 ruling requires refusing tracked links into delivery
state; the supplied macOS ruling assigns eval macOS evidence to #201.
No new semantic ruling or decision acceptance is required.

Spot-checked `without_code`, `markdown_destinations`, `destination`,
`packet_destination` and `check_tracked_markdown` in
`evals/tests/boundary.rs`, including existing scanner, classifier and temporary
Git-inventory controls. Commit `8958176` (#243/#246) introduced this guard.
R1 identifies two manifestations of code-span handling; R2 and G1 overlap on
reference-definition containers; G2 concerns character-reference decoding.
Retain the reported four-finding accounting without inventing a fourth
independent scanner subsystem.

No frozen guarantee 0001–0003 is adjacent to this test-only scanner repair:
synchronous accept/store, subscribe-before-check and oldest-only pruning
remain untouched. No application extraction/runtime change is authorized.
Keep HTML and arbitrary renderer extensions outside the existing bounded
grammar. A general Markdown renderer, new dependency, lockfile widening,
routing change, namespace work and paid trials are outside this task.

## Checkable acceptance criteria

1. Before changing scanner behavior, add regression tests for R1's paragraph
   boundary and destination-backtick cases, R2/G1's container reference
   definitions, and G2's character references. Each reported finding has a
   named test/fixture mapping and a recorded failing command/status against
   the pre-fix scanner, followed by the same test passing after repair.
   Exercise real Markdown documents tracked in a temporary Git repository
   through `check_tracked_markdown`, following the existing fixture pattern;
   helper-only assertions do not replace these guard-level proofs. No
   prohibited fixture is left in the repository's tracked Markdown.
2. R1: matching backticks in separate paragraphs do not mask a genuine link
   between them; CommonMark-valid inline code within one paragraph, including
   soft line breaks and matching delimiter lengths, still masks example
   links. Backticks occurring in a valid link destination are retained as
   destination text rather than consumed as code spans that conceal its
   .forge path. Pair prohibited links with genuine-code and allowed-link
   controls so both false negatives and false positives fail a test.
3. R2/G1: reference definitions inside CommonMark block quotes and list
   containers, including their valid continuation/indentation forms, yield
   destinations and refuse a normalized .forge target through the guard.
   Bare definitions already supported retain their outcome. Pair each
   container form with an allowed tracked citation and code examples that
   must remain inert. Retain source-file, source-line and original-target
   diagnostics despite removing container syntax for parsing.
4. G2: decode valid CommonMark named, decimal and hexadecimal character
   references in link destinations before URL/path classification. Prove
   entity-spelled dots/slashes can resolve into .forge for inline links and
   reference definitions. The # in a numeric reference is not prematurely
   treated as a fragment delimiter. Retain correct handling of genuine
   fragments/queries, external destinations, Markdown backslash escapes and
   single percent decoding; escaped ampersands and invalid references follow
   CommonMark rather than an extra recursive decode. Include allowed
   counterparts and assert original spelling in refusal diagnostics.
5. Existing boundary tests pass unchanged in meaning: inline links/images,
   angle destinations, titles, escaped/balanced delimiters, bare reference
   definitions, code spans/fences, path normalization, root clamping,
   percent escapes, external/anchor/query links, tracked-only inventory and
   fail-closed inventory/read/encoding errors retain their classifications.
   Add no dependency; root and eval manifests/locks and license allowances
   remain unchanged. Do not skip or ignore existing tests or weaken their
   assertions, the guard, gate floors or protected review.
6. The committed implementation passes the explicit eval checks and every
   local house gate below, exact zero-miss coverage and selftest included.
   Preserve the standalone eval boundary and candidate's existing bundles/
   routing. Record command statuses and candidate identity; a missing input
   is named by exact path and is not a pass. Commit unsigned; no push,
   merge, tag, publication, issue closure or decision acceptance.

## Verification commands and ownership

Run on the committed candidate, offline with preprovisioned tools; install
nothing. The candidate-local engine was hashed and matches the required
digest. Recheck before use:

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
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

The canonical bundle scripts implement the house inventory's public shims.
Verify checks the shim digests and executes the dispatched pinned copies,
never candidate wrappers (0050). Coverage requires missed == 0 for lines,
branches and functions; no exclusions or rounded substitute. Root coverage
does not measure evals; its independent suite is mandatory. Bundle compilation
must also emit no `lets the run go on without reading` warning. The actual
hermetic verify dispatch remains a later phase's obligation; intake does not
substitute a manual local run for that dispatch.

Run the native-tls absence proof in a separate shell:

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

For the test-first obligation, record exact newly added boundary test names
and run each filter with `cargo test --manifest-path evals/Cargo.toml --locked
--test boundary <test-name> -- --exact` before and after its scanner repair.
No benchmark is needed for this test-only parsing repair. Root tests retain
real-server protocol evidence; these Markdown/Git fixtures need no packet I/O.

CI owns Helm/chart-bump/history/selftests, Docker smoke, network RustSec,
privileged port-80 bridge and tag-only rate/publication proofs. #201 owns
macOS eval evidence. The operator owns live branch settings, sealing and
decision acceptance. These are not additional local acceptance prerequisites.
Do not read GitHub, CI, journals or sibling worktrees.

## Intake evidence

The framing is the deliverable of this phase. Intake changes no scanner or
tests and claims no issue implementation success. Local gate attempts on the
committed framing are recorded separately; all implementation red/green
controls, completion verification and independent review remain outstanding.
