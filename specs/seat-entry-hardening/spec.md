# Harden self-delivery seat entry

Epic #192, issue #172. Design synthesis, 2026-10-24.

## Feature and rationale

Repair the existing design validator and verifier entry points so hostile Git
path bytes, shell startup, missing pinned proof resources and unclear decision
formatting cannot produce misleading delivery evidence. Record the correctness
charter the selected recipe actually resolves. This is delivery machinery work;
triage's `engine` classification and full design/four-reviewer route stand.

The intake is `.brokkr/state/tasks/epic-192-issue-172-harden-the-se-9f26fbd3.md`.
The synthesis reads both run-local positions in
`.brokkr/state/design/positions/{simplicity,robustness}.md`. Those files are
uncommitted evidence; the requirements below stand on their own.

## Authority and scope

This restores accepted [0007](../../docs/decisions/0007-hermetic-gates-single-model.md),
[0045 §§1–2, 8](../../docs/decisions/0045-delivery-repairs.md) and
[0050 §§1, 3, 5](../../docs/decisions/0050-delivery-gates.md). No new semantic
ruling is declared: proof ownership, HEAD identity, status authority and gate
requirements remain the accepted ones. Parser bugs are repaired within those
requirements; the issue expressly permits an actionable strict-format refusal.
No proposed ADR is needed for this bounded restoration.

Preserve [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
all frozen guarantees and accepted 0004–0005. Product code, release settings,
dependencies, coverage denominator/floor, engine pin, recipe routing and policy
are outside the implementation scope. If satisfying a requirement demands
changing proof ownership, acceptance authority, HEAD selection or an accepted
ruling, stop that dependent work and return for triage/operator resolution.
A proposal cannot authorize contradicting accepted policy.

Use [0057](../../docs/decisions/0057-codex-delivery-recipes.md)'s `codex-fast`
engine lane: Qwen Max correctness, Claude Opus security, official DeepSeek
Flash guarantees and DashScope GLM 5.3 tests, with singleton chains, no Pro,
no fallback, protected review, worst-verdict aggregation and security hold.
Chief authors only this design; the deterministic final check owns advancement.

## Synthesis and explicit rulings

1. **Adopt both positions' byte-preserving discovery.** Count actual NUL
   records and deduplicate complete slug strings. Choose a checked temporary
   file over an asynchronous process substitution. Its lifecycle and linear
   array comparisons cost some code and bounded quadratic comparisons; that
   is preferable to hidden Git failures, a second index or a new dependency.
2. **Adopt simplicity's existing runner, with robustness's execution proof.**
   Use non-login Bash in the existing `run` helper. Preserve failure meaning,
   cwd, stdin, offline behavior and pinned identity. A string-based runner
   remains useful for the compound native-tls probe; no dispatch framework is
   warranted. Safe quoting of the resolved engine path is part of preserving
   identity, not a new toolchain-attestation project.
3. **Adopt both positions' explicit preflight list.** Include the complete
   static helper chain, especially `delivery-proof-test.sh`. Reject a generated
   inventory or dependency crawler. The cost is maintaining a small list and
   its missing-resource tests when the pinned call chain changes.
4. **Rule for simplicity on status typography.** Keep the canonical
   `- **Status:** proposed` form and print it literally in refusals. Reject
   robustness's preferred acceptance of multiple Markdown spellings. The
   rejection of that option costs authors a formatting repair for alternative recorded
   forms; accepting those forms would cost extra grammar and acceptance arms
   without closing an additional issue criterion. Adopt robustness's token
   and ambiguity safeguards: prose, `proposedness`, Draft, accepted records
   and contradictory status headers must not pass as a proposed declaration.
   Detecting alternate headers to refuse ambiguity does not authorize them.
5. **Adopt provenance documentation from both positions.** Record the tracked
   resolution and recheck it at actual launch; do not create a shadow library
   or claim to have reconciled an absent historical copy. The cost is that
   candidate-local resolution does not prove historical or live-provider use.
6. **Adopt executable negative controls over a minimal textual patch.** Keep
   fixtures in existing owners and reject new production test bypasses,
   recursive verifier tests, generic parsers and unrelated product work.
   Robustness's product failure examples motivate retaining the existing
   real-protocol suites; they do not add SMTP/webhook/storage features here.

## Acceptance Criteria

### AC1 — Git path identity and discovery failure

The validator consumes `git diff-tree --no-commit-id --root -r -z --name-only
HEAD` as NUL-delimited records. Every distinct immediate `specs/<slug>/`
component counts before kebab-case validation; no whitespace normalization or
invalid-slug filtering can hide a second directory. Multiple files under one
slug count once. Exactly one valid slug at chief's HEAD, committed nonempty
artifacts, clean worktree, required headings/tasks and pinned prerequisites
remain necessary.

The existing selftest must execute real committed Git fixtures proving:

- A valid design plus `specs/evil<LF>name/file.md` in the same HEAD refuses
  with a two-directory diagnostic; the old validator demonstrably admits that
  same fixture, with its other prerequisites satisfied.
- A lone newline-bearing slug refuses kebab-case validation; ordinary two
  slugs and non-kebab names continue to refuse.
- One valid slug with repeated changed files passes, including a newline in
  a nonessential filename below that slug. No blanket filename ban is added.
- A failed Git producer, including valid-looking partial output followed by
  failure, refuses with the discovery-failure diagnostic. Temporary storage
  failures refuse, cleanup preserves existing selftest cleanup, and no stale
  stream or verdict is reused.

Retain zero-slug, unrelated-HEAD, linked-worktree, dirty/untracked artifact,
missing/empty artifact, missing heading/task and decision/prerequisite controls.

### AC2 — Non-login execution preserves the verifier contract

Every gate runs through non-login Bash. An execution fixture of the actual
seat proves candidate cwd, `CARGO_NET_OFFLINE=true`, stdin EOF, gate ordering,
captured failure diagnostics, JSON `fail`, and no gate after the deliberate
failure. A controlled startup sentinel has a positive login-shell control;
the repaired runner must not trigger login initialization. The fixture must
fail if `-l` is restored, not merely search the script text.

The engine invoked for compile is the resolved executable whose bytes passed
the pinned digest check, with its path quoted as data even with spaces or shell
metacharacters. Missing/mismatched digest refusals remain. No network install
or retry is introduced. Inspect the actual exec environment's `BASH_ENV`
handling before claiming its boundary safe; non-login execution alone does
not attest all inherited environment/compiler/cache configuration. Untrusted
startup injection requiring a boundary change is an escalation, not a waiver.

### AC3 — Complete pinned-resource preflight

Before the first gate, presence checks cover every static resource later
invoked or sourced in the proof chain: coverage gate and scanner source,
manifest and lock; design check and both speckit helpers; delivery proof;
publish guard and proof; seal tool and proof; shim digest manifest; tooling
identity record. All resolve from the dispatched pin directory. Generated
selftest helpers are derived from pinned code, not additional static inputs.

A table-driven actual-seat fixture removes each required entry in turn and
requires a `fail` result naming that entry and an untouched gate-start
sentinel. The complete-resource control reaches gate execution. Missing
resources never select a candidate copy or skip a proof. Preserve the separate
behavior for changed public shims: record integrity failure and still obtain
canonical coverage diagnostics. The existing uncovered-line plus exit-zero
candidate-gate tamper proof must still fail for both reasons.

### AC4 — Actionable status refusal without invented authority

An explicitly declared decision must remain one committed, clean linked record
with proposed status. Support the current bulleted `**Status:**` spelling,
case-insensitive status recognition and horizontal whitespace tolerance; the
status token must be exactly `proposed`, followed by end of line or whitespace
before explanatory text. Refusals print the literal expected form
`- **Status:** proposed` and explain that accepted records may be authorities,
not new declarations. A record with multiple status headers is ambiguous and
refuses, including a canonical proposed header alongside an alternate-form
accepted header. Header detection is anchored to metadata lines, not prose.

Execute validator fixtures for canonical proposed success (including an
explanatory suffix), unsupported colon placement/bullet forms with the exact
format diagnostic, accepted/missing/malformed/Draft/proposedness/prose-only
status refusals, duplicate/conflicting headers, and accepted-authority-link
success without an explicit declaration. Do not rewrite accepted decisions
or infer operator acceptance from typography.

### AC5 — Reproducible correctness-charter provenance

Document in `agents/README.md` the effective tracked library, overlay/base
inheritance, charter path, compile target and reproducible resolution command.
Compare engine output with the file digest during implementation validation
and again for the actual review launch. Record any divergence honestly and
resolve or escalate it before claiming the required reviewer was used.
Do not manufacture `.brokkr/agents` or silently change reviewer chains.

Chief's 2026-10-24 observation: `.brokkr/agents` is absent;
`.forge/bin/brokkr agents show codex-review-correctness` resolves singleton
`qwen-max` and `charters/review-correctness.md`. The overlay
`agents/codex-review-correctness.json` extends `agents/review-correctness.json`.
The resolved charter digest equals `agents/charters/review-correctness.md`:
`215d6e38a66cbaf16ed7b76f1742a68b48a8e42bb2deeb1e57ec53a099ddb2fa`.
Its compile target is `brokkr compile --bundle bundles/self`. The launched
candidate-local engine digest matches `bundles/self/tooling.json`:
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
This is current resolution evidence, not historical-host or live-review proof.

### AC6 — Existing deterministic and independent evidence remains mandatory

Implementation/verify must leave the entire
[house-rules gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
clean, with locked builds, exact non-rounded 100/100/100 coverage, coverage
selftest, publish/seal proofs, pinned compile, license check and fail-closed
native-tls absence proof. Also execute design selftest, delivery tamper proof
and actual engine design/verify dispatch under 0050. Fixture instrumentation
and structural tests supplement those executions; they cannot replace them.
Keep CI-only chart/history/selftest, Docker, advisory, tag-rate and port-80
proofs identified as CI evidence. Do not claim passed gates or reviewers from
this design draft, and do not close outstanding proof obligations with prose.
