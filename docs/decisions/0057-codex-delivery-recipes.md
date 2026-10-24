# 0057 — Codex authors; independent providers review

- **Status:** accepted (operator directive in chat, 2026-10-23: “set up
  recipes that do most of the work with codex”; “review tests - use
  dashscope GLM 5.3”; “use deepseek flash, never use pro”; “you can use
  it from the deepseek platform”; “try 8 concurrent delivery runs”; and
  the subsequent directive for triage before small bugs, with single or
  double review instead of requiring the full panel for every bug).
- **Date:** 2026-10-23
- **Amends:** [0007](0007-hermetic-gates-single-model.md)'s work-route
  restriction, [0009](0009-agent-council-hybrid.md)'s mandatory design path
  for small bug fixes, and [0013](0013-four-model-review-panel.md)'s fixed
  review size and route mapping for explicitly selected Codex recipes.
  Preserves deterministic gates, protected review, worst-verdict panel
  aggregation and the security hold.
- **Purpose:** prepare the delivery strategies before repairing epic #192.

## Ruling

1. `bundles/codex` composes from `bundles/self`; `bundles/codex-careful`
   composes from `codex`. Both use Codex Sol for intake and implementation.
   The shared design sequence uses Sol for simplicity and Astra for
   robustness and chief synthesis, all at high effort. Its deterministic
   final check stays declared in `self`: replacing a complete sequence
   changes its script origin, so the recipes inherit that sequence intact.
   This shared design also applies when `self` is selected directly.
   `codex` uses medium intake and high implementation effort;
   `codex-careful` uses high intake and xhigh implementation effort.
2. The full review path keeps four read-only lenses:
   correctness on `dashscope/qwen3.8-max`, security on Claude Opus,
   guarantees on the official DeepSeek platform's `deepseek-flash`, and
   tests on `dashscope/glm-5.3`. DashScope uses provider defaults; this
   host's exact custom-model metadata rejects an explicit effort flag.
   DeepSeek Flash and Claude Opus use high effort. The adapter records
   DashScope as an effortless route, and the legacy correctness office drops
   its unusable explicit effort while retaining its original model chain.
   Each office has a single-model chain.
   Provider failure stops rather than silently replacing an independent
   reviewer with the authoring model. DeepSeek Pro is never a candidate.
   `self`'s existing review mapping and GLM intake/implementation remain
   available for historical compatibility.
3. Offices are overlays of the existing role charters. Codex work offices
   declare workspace hands: the native shell stays read-only while the
   engine's workspace tool performs writes under the realm boundary.
   Cargo's overlay cache masks credentials and the Rust toolchain is
   mounted read-only. Work may use the network; verify and ship retain
   inherited no-network hands and fixed proof scripts. No model replaces
   the hermetic verifier or shipper.
4. `deepseek-official` is an uncontracted route, credentialed by the name
   `DEEPSEEK_API_KEY`; DashScope keeps `DASHSCOPE_API_KEY`. Claude and
   Codex use their existing CLI authentication. No credential value enters
   a tracked file, recipe, command argument or report.
5. Up to eight independent delivery runs may execute concurrently, each
   with its own branch, worktree, journal, run-local evidence and writable
   build output. Batches reserve overlapping files before launch; integration
   reconciles their commits and reruns required gates. Eight is the requested
   target, not evidence that all eight complete runs have been proved.
   No run may push or merge.
6. `codex-fast` adds triage before work. The engine's `chore` strategy means
   an isolated, understood bug with a narrow regression: implement directly,
   run all shell gates, then one independent DeepSeek Flash reviewer checks
   correctness, tests, guarantees and security. `feature` means a broader
   understood fix: implement and verify directly, then Qwen Max correctness
   and Claude Opus security review. `design` and `engine` retain the shared
   design council and all four independent review lenses. `escalate` stops.
   No missing or unknown classification selects a default lane.
7. Triage records the evidence, affected files, acceptance criteria and
   classification reason. Security-sensitive behavior, concurrency,
   persistence, gate/policy repairs and normative contract changes default
   to the full lane. A small diff or a low issue severity does not itself
   prove a small risk. Work that outgrows its class must return to triage
   through a bounded escalation rather than silently enlarging the fast
   lane. Full lanes require committed specs; direct bug lanes use the
   recorded triage framing. Every lane still executes the complete house
   gates before its protected review and ship gate.
8. Reviewers report scope defects explicitly on every lane. A known scope
   defect cannot take a clean or safe-residual shortcut to shipping; the
   bounded return to triage applies to the one-, two- and four-seat paths.
   Unknown clean scope/fixes facts stop. A ship close-out discrepancy is an
   execution refusal, never a successful `shipped` result with warning notes.

## Tooling and evidence

The installed 0.12.0 engine rejects the tracked adapter `mcp.carriage`
schema; version equality alone is not compatibility. Recipe validation
uses an identified build recorded in `bundles/self/tooling.json`: base source
commit, ordered patch paths and SHA-256 values, compiler and binary digest.
The recorded base engine incorrectly synthesized safe aggregate facts from
residual members with missing or malformed safety fields. Actual CLI probes
demonstrated shipping; the pinned patch preserves unknowns, witnessed
security and above-medium severity, so the existing unsafe rules can stop.
The patch includes upstream regressions and remains explicit until an
upstream source pin contains the fix.

`scripts/apply-brokkr-patches.py` requires the exact clean source checkout,
validates every patch path and digest before any mutation, and applies the
same buffered bytes in order. Local and CI builds use that helper; CI runs
the engine's aggregation regressions before building and compiling recipes.
This replaces an unavailable, unreproducible operator-local build with
declared source inputs. Verification still refuses an unrecorded executable;
there is no digest bypass or claim that patched source is the clean base.

The existing recipe library is exposed by the relative `recipes -> bundles`
symlink: default recipe lookup and library discovery see the same canonical
directories without copying proof scripts or policy. Explicitly name each
run's journal under gitignored `.forge/`. A state-only `.brokkr/` directory
would be interpreted without the library marker as adopter configuration,
so it is not used for configuration discovery. Legacy role framing and
positions may still be mirrored under `.brokkr/state/`; that is ignored
compatibility evidence. The self-contained realm map names only swarmail
and defaults its journal
to `.forge/forge.db`; the unrelated sibling realm and its missing dialect
are not prerequisites for delivering swarmail. The ship script renders ledger
output into `.forge/ledger/<run>.md`, avoiding engine-specific directory
defaults and a state-only `.brokkr` directory.

Compilation, route/effort and inheritance regression tests, the full house
gates and an actual delivery run provide different evidence. Compile success
is not a live-provider proof; live dispatch is not a green coverage gate.
Neither closes epic #192 findings without the corresponding story evidence.
Old journal rulings cannot be recreated from issue text.

## Operator clarification — default run visibility (2026-10-23)

The operator requested that `brokkr tui` show the runs without an explicit
binary or database argument. Use the compatible recorded engine on normal
`PATH` and the realm map's canonical `.forge/forge.db`. Isolated concurrent
checkouts write their distinct runs to that shared journal using its absolute
path; their Git branches, result directories and writable build targets
remain separate. The journal's SQLite coordination is shared, not the build
or candidate workspaces.

Historical runs are adopted locally through verified `export` / `import`,
retaining their original event bytes and run IDs. Keep the recovered source
snapshot and remote original unchanged. Importing history does not resume,
supersede or discharge its rulings. Do not relocate or alias an open SQLite
journal or its WAL files.

## Operator clarification — fast implementation model (2026-10-24)

The operator directed that the `codex-fast` implementation seat use GPT-6
Luna (`luna`) at `xhigh` effort. Triage stays on Sol / medium. This does not
change the chore, feature, design or engine selectors, the independent
reviewer assignments, the protected review, or any deterministic gate.

## Operator clarification — candidate-local engine preflight (2026-10-24)

Every isolated candidate must contain the digest-matched recorded engine at
its own `.forge/bin/brokkr` before a delivery run starts. The operator's
launch procedure verifies the digest from `bundles/self/tooling.json` and
passes the absolute path to boxed checks. A sibling worktree's binary or the
host `PATH` does not satisfy this requirement. Missing or mismatched binaries
are setup blocks to resolve before provider dispatch, not implementation
failures to retry.
## Operator-directed routing amendment — accepted (2026-10-24)

Decision [0058](0058-codex-review-routing.md) amends the reviewer matrix
without changing who implements, triage selectors, protected review, or
deterministic gates. Standard uses Fable 5.1 correctness, official DeepSeek
Flash security and Sonnet 5.5 tests. High-risk (`codex-fast` design/engine)
and Careful each require Astra correctness, Flash security, Sonnet tests, Astra
guarantees and independent Fable as five separate results. Fast chore keeps
one combined Flash reviewer. Fast feature uses Qwen Max correctness and Flash
security; Sonnet is allowed only after Qwen dispatch failure before a review
result. No other fallback is allowed, and DeepSeek Pro is forbidden.

## Operator clarification — fast triage model (2026-10-24)

The operator directed that only the `codex-fast` triage office use Claude
Opus 5.5 through the existing Claude model key `opus`, at medium effort,
with no triage fallback. This directive is the accepted amendment to 0057
and supersedes only “Triage stays on Sol / medium” in the earlier fast
implementation clarification. The historical text and 0057's accepted status
remain as recorded; no new decision number or proposal is introduced.

Fast implementation remains Luna / xhigh. Standard and Careful work routes,
shared design, the accepted 0058 review matrix and its sole fast-feature
Qwen-to-Sonnet dispatch fallback, legacy `self` routes, triage classifications
and bounded returns, protected review and all deterministic gates are unchanged.
No other acceptance follows from this directive.
