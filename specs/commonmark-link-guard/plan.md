# Plan — CommonMark tracked-link guard

## Technical context

The guard is Rust test code in the standalone, unpublished `evals/` workspace.
It already uses Git-backed temporary fixtures, `tempfile` and `serde_json`.
No dependency is needed. Root gates do not build or measure this workspace;
explicit eval commands are required. Only a spec template exists under
`.specify/templates/`; this plan supplies the requested plan-template sections
directly. Research, data model, contracts and quickstart are inline; no extra
design artifacts are required.

## Constitution check

This compatible enforcement repair preserves the operator's #243 ruling,
0055's workspace boundary and 0050's pinned proofs. Accepted 0001 synchronous
accept/store, 0002 subscribe-before-check, 0003 oldest-only pruning and 0005
ingest extraction are untouched. No new decision is warranted. Preserve the
exact root coverage denominator and 100% lines/branches/functions requirement,
no exclusions, existing routing and protected review. Commit unsigned without
pushing; operator acceptance/sealing is not delegated to this seat.

## Project structure and files touched

| Path | Purpose |
|---|---|
| `specs/commonmark-link-guard/spec.md` | Chief's WHAT/WHY, synthesis and acceptance criteria |
| `specs/commonmark-link-guard/plan.md` | Chief's implementation and proof approach |
| `specs/commonmark-link-guard/tasks.md` | Chief's ordered work and test pairs |
| `evals/tests/boundary.rs` | Downstream scanner repair, small fixture helper if justified, regressions |
| `scripts/data/html-entities.json` | Read-only existing named-reference data; preserve attribution and bytes |
| `.forge/` / `target/` | Ignored command receipts and build outputs, never staged |

The chief commits exactly the three design files. Later implementation changes
only the named test file and necessary design progress/evidence text. Do not
move code into `src/`, add a workspace/module framework or modify bundles.

## Research and implementation approach

Inspection confirms `without_code` currently pairs inline backticks over the
whole document before links are recognized. `markdown_destinations` recognizes
definitions only at bare physical line starts with up to three spaces.
`packet_destination` removes backslash escapes and splits fragments before
considering Markdown references. `check_tracked_markdown` already provides
Git discovery, tracked-only inventory, raw-source diagnostics and lexical
classification; preserve these boundaries.

1. Add all four named regression families first, with isolated temporary Git
   roots and actual indexed Markdown. Start from the existing `fixture_git`
   and `tracked_inventory_refuses_existing_packets_and_recovers_after_repair`
   pattern. Test command counts, assertion failures and setup success before
   any behavior edits. A compact shared helper can write/index a document,
   check refusal details and replace it with an allowed citation. It must
   not mutate global environment/current directory or a shared index.
2. Establish one block-context view for definitions and inline parsing. Track
   physical source ranges, eligible inline block identity, quote/list nesting,
   list content columns and fence/code state. Honor spaces/tabs as indentation
   columns where CommonMark requires them; keep source offsets in bytes.
   Recognize marker validity and continuation indentation structurally, with
   lazy paragraph continuation distinguished from a new definition block.
   Blank logical lines, container exits/item changes and block starts bound
   inline spans. Recognize supported block boundaries such as headings and
   thematic breaks; a soft newline alone does not terminate a paragraph.
   Do not strip arbitrary markers until a bracket appears.
3. Delete document-global inline backtick masking. In each eligible inline
   block, scan code spans and link brackets with explicit precedence. Use
   matching-length delimiter runs; an unmatched opener is literal. An active
   code span suppresses apparent links; an admitted inline link's destination
   and title are consumed together without interpreting their backticks as
   code. Parse reference definitions from the same block view before inline
   treatment of their destination bytes. Keep `destination`/`inline_end`
   behavior for existing syntax, but restrict their input/lookahead to the
   eligible scope. Reset bracket state at block boundaries. Preserve the
   existing direct-definition policy, including previously recognized bare
   definitions, rather than adding a reference-resolution database.
4. Preserve raw ranges throughout. Prefer original ranges per logical content
   line with source-offset mapping where container prefixes interrupt logical
   content; never treat stripped offsets as source offsets. Destinations
   retain original spelling and their physical start offset. Fence/indent
   suppression must use the same container context as definition recognition.
   Keep parsing iterative with advancing cursors; index delimiter runs within
   a block or otherwise avoid a full remaining-document search per unmatched
   opener. Bound entity lookahead by grammar limits and snapshot name length.
5. Add a local single-pass Markdown escape/reference decoder at classification
   entry. It scans original bytes, consumes punctuation escapes or valid
   references, and never rescans emitted characters. Parse the existing JSON
   snapshot with existing `serde_json` into an immutable once-initialized
   lookup (for example `OnceLock`), selecting semicolon-terminated keys and
   their `characters` strings. No hand-selected table or new data copy.
   Implement AC4's numeric grammar/scalar policy locally; production HTML
   decoding has different semantics and is not imported or changed.
6. Pass the decoded value through the current scheme/authority test, genuine
   query/fragment split, one percent pass and root-clamped component walk.
   Do not resolve files, probe packet existence or change URL policy. Keep
   raw diagnostic values separate from the classification value.
7. Run the same regression filters green, the interaction/negative controls,
   all existing boundary tests, independent eval checks and local house gates.
   Review the diff for unchanged manifests/locks/runtime/routing and preserve
   red/green receipts. Commit implementation before downstream verification.

## Data model and contracts

No persistent schema, public API or executable changes. Test-local logical
lines/blocks carry original source ranges, container state and code status.
A found destination remains an original byte offset plus raw string; the
classification decoder returns a separate value. A refusal remains a named
file/physical line/raw target diagnostic. Inventory/discovery/read/encoding
failures remain errors, and classification remains independent of the target
filesystem. These internal representations may be small structs or tuples;
they do not require a general Markdown AST.

## Quickstart and validation

Run from repository root with provisioned tools, offline; install nothing.
Record exact command, exit status, output location and `git rev-parse HEAD`.
For red tests additionally record the scanner diff/source identity, since
new tests necessarily precede the repair commit. Use these proposed names
(or record a one-to-one final-name mapping in the implementation evidence):

```bash
cargo test --manifest-path evals/Cargo.toml --locked --test boundary tracked_code_spans_respect_blocks -- --exact
cargo test --manifest-path evals/Cargo.toml --locked --test boundary tracked_destinations_own_backticks -- --exact
cargo test --manifest-path evals/Cargo.toml --locked --test boundary tracked_container_definitions_are_checked -- --exact
cargo test --manifest-path evals/Cargo.toml --locked --test boundary tracked_character_references_are_decoded_once -- --exact
```

Run each before any scanner repair and after repair; require one executed test
per exact filter. Every refusal family gets allowed-citation/code controls.
Combined container/entity/backtick and UTF-8 source-offset cases run through
the guard too. Pure helper tests may supplement numeric/name boundaries.
Do not claim helper tests alone establish tracked-inventory behavior.

The sole gate inventory is the [house rules](../../docs/house-rules.md#gates--part-of-every-change).
Execute its canonical pinned implementations after checking the fixed shims,
as required by 0050; the following commands also retain intake's independent
eval and proof checks. They apply to the committed implementation; attempts
on the design commit establish no red/green implementation claim.

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

Also refuse bundle output containing `lets the run go on without reading`,
even if compilation returns zero. Run the fail-closed native-tls absence proof
in a separate shell that captures Cargo's nonzero status:

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

For the chief's artifacts, run `git diff --check`, commit exactly the three
files, then `bash bundles/self/scripts/design-check.sh check .`. Leave that
commit at HEAD and the artifacts clean. The sequence's pinned final check
owns the design verdict. Do not commit run-local positions, gate logs or the
engine result JSON. Do not substitute manual local checks for downstream
hermetic verify dispatch. External proof ownership is listed in the spec.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| Container state recreates a partial renderer | Keep only block context needed for reported forms and code boundaries; no AST or extension grammar. Review explicit transitions with contrasting fixtures. |
| Overbroad stripping makes code/prose look like definitions | Track list content indentation and quote nesting, including continuation/code controls; accept extra state over a regex patch. |
| Global code masking still erases destinations | Remove that path; let code spans and admitted links own bytes in their scope. Test both precedence directions. |
| Source offsets drift after prefix removal or Unicode | Preserve original ranges/mapping; assert physical start line and raw destination in Git-backed fixtures. |
| HTML decoder reuse silently changes semantics | Use the existing data only; local CommonMark numeric/escape rules, malformed and decode-once tests. Runtime extraction stays untouched. |
| Multiple decode stages activate escaped syntax | One combined Markdown pass, then existing URL stages; test escaped ampersands, emitted references/backslashes and percent interactions. |
| Unmatched delimiters cause repeated whole-document searches | Iterative scoped scans with delimiter-run indexing/bounded lookahead; long malformed-input controls without timing thresholds. |
| Root green hides eval failures | Explicit eval fmt/clippy/boundary/full suite; preserve all baseline assertions and report command failures honestly. |
| Fixture contamination or forbidden citations enter tracked docs | Fresh temporary Git roots and Rust fixture strings; run tracked inventory against committed design and implementation. |
| Gate/environment problems are misreported as success | Record current-candidate failures and exact missing input paths; no network installs, waivers or unrelated gate fixes. |

The accepted limitation remains a bounded convention checker, not universal
CommonMark/rendering conformance. No known failure in the three requested
forms is accepted as that limitation. Snapshot initialization adds modest
memory/startup work to the test process. No benchmark, runtime networking or
new global mutable test seam is justified.
