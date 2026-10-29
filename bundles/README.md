# Delivery recipes

The Codex recipes compose the realm's existing `self` constitution.
`codex-careful` extends `codex`, which extends `self`. The policy, protected
review phase, offline verification and ship gates remain the realm's pinned
proof chain. Both inherit the complete design seat so its final validator
keeps `self` as its script origin; only that base seat's three model-office
references change to the Codex design crew. Model reviewers read the change
after deterministic verification; they do not replace the shell verifier.

Use `codex-fast` for bug intake. Triage runs first on Claude Opus 5.5 (`opus`)
at medium effort, with no fallback, and records its evidence and reason before
choosing the path:

```text
+----------------+----------------------+------------------------------+
| Triage result  | Work path            | Independent review           |
+----------------+----------------------+------------------------------+
| chore          | Implement -> verify  | DeepSeek Flash               |
| feature        | Implement -> verify  | Qwen Max + official Flash       |
| design/engine  | Design -> implement  | Flash (3) + GLM + Fable      |
| escalate       | Stop                 | Operator ruling needed       |
+----------------+----------------------+------------------------------+
```

Here `chore` means an isolated, understood bug and `feature` means a broader
understood fix within existing contracts; these are the engine's route keys,
not permission to add an unrequested feature. Small lanes use triage's framing
instead of a spec package. Security, concurrency, persistence, gate/policy
and normative contract work default to the full design and five-role review.
Every lane runs the complete deterministic verification and ship gates.
Missing or unknown strategies have no selector default and cannot select a
cheap lane. Implementation can return oversized scope once to triage; the
reviewers can also expose a scope defect on any lane, with bounded
reclassification and unchanged hard stops for security, high or unknown
residuals. Shipping refuses a missing, empty or mismatched ledger instead
of reporting completion with discrepancy notes.

`codex` and `codex-careful` explicitly select the full design/review path;
they are useful when the operator has already ruled that path necessary:

```text
+---------------------+-----------------------+-----------------------+
| Seat                | codex                 | codex-careful         |
+---------------------+-----------------------+-----------------------+
| Intake              | Sol / medium          | Sol / high            |
| Design: simplicity  | Sol / high            | Sol / high            |
| Design: robustness  | Astra / high          | Astra / high          |
| Design: chief       | Astra / high          | Astra / high          |
| Design: check       | Offline exec gate     | Offline exec gate     |
| Implement           | Sol / high            | Sol / xhigh           |
| Verify              | Pinned offline gates  | Pinned offline gates  |
| Review: correctness | Fable 5.1 / high      | Official Flash / high |
| Review: security    | Official Flash / high | Official Flash / high |
| Review: guarantees  | absent                | Official Flash / high |
| Review: tests       | GLM Flash / N/A       | GLM Flash / N/A       |
| Review: independent | absent                | Fable 5.1 / high      |
| Review: deadline    | 14400 s, 3 attempts   | 14400 s, 3 attempts   |
| Ship                | Pinned exec gate      | Pinned exec gate      |
+---------------------+-----------------------+-----------------------+
```

```text
+-----------+-----------------------+----------------------------+
| Model key | Concrete model        | Provider                   |
+-----------+-----------------------+----------------------------+
| sol       | gpt-6.1-sol           | Codex CLI                  |
| luna      | gpt-6-luna            | Codex CLI                  |
| astra     | gpt-6-astra           | Codex CLI                  |
| qwen-max  | dashscope/qwen3.8-max | DashScope through dsh      |
| opus      | claude-opus-5-5       | Claude CLI                 |
| fable     | claude-fable-5-1      | Claude CLI                 |
| sonnet    | claude-sonnet-5-5     | Claude CLI                 |
| haiku55   | claude-haiku-5-5      | Claude CLI                 |
| flash     | deepseek-flash        | Official DeepSeek platform |
+-----------+-----------------------+----------------------------+
```

Standard means `codex`; High-risk means fast `design`/`engine`, with the same
five required role assignments as Careful. The correctness and guarantees
roles retain distinct offices, charters and results; independent Fable is a
separately required correctness reading. Five roles do not mean five model
families: three of them now share official DeepSeek Flash. See accepted
[0058](../docs/decisions/0058-codex-review-routing.md) for the operator-directed
review matrix, with the revised 2026-10-26 tests-only direction recorded in
proposed [0081](../docs/decisions/0081-glm-flash-seats.md) and the 2026-10-28
direction recorded in accepted
[0098](../docs/decisions/0098-review-panel-flash-and-deadline.md): the full-panel
correctness and guarantees members (office names
`codex-review-astra-correctness` and `codex-review-guarantees`, kept for
continuity) select dsh `flash` at high effort exactly as security does, and the
review seat of `codex`, `codex-careful` and `codex-fast` (inherited by
`mixed-opus-terra`) allows 14400 s instead of 3600 s. The panel shares that one
deadline; intake, triage, implementation and the legacy `self` review keep their
limits. The tests member
selects only `glm-flash`: dsh routes it to local
`spark-glm/GLM-5.3-Flash-EXL3`. Reasoning effort is not applicable, represented
by an explicit empty office effort map and absent compiled effort. Intake
stays on Sol with its existing workspace hands.

Every Codex reviewer has a single-model chain except dedicated fast-feature
correctness: `[qwen-max, sonnet]`. Only a settled pre-session dispatch failure
(no accepted session, checkpoint or watchdog kill) advances that chain. Valid
adverse verdicts and malformed facts cannot trigger substitution; exhaustion
refuses shipping. Flash security never substitutes. DeepSeek uses official
Flash only; Pro is forbidden. DashScope Qwen keeps provider-default effort;
Spark tests effort is not applicable; the other requested reviewers use high.
All reviewers retain read-only charters
and `class: gate`; only panel security or the combined chore reviewer owns hold.
The dsh adapter does not claim native tool confinement: those reviewers'
read-only conduct is a charter obligation, and the delivery gates detect tree
changes. A security hold remains a hard stop, and unknown residual safety
facts never authorize shipping.

The `codex-fast` implementation seat uses GPT-6 Luna at xhigh effort. Its
triage seat uses Claude Opus 5.5 (`opus`) at medium effort with no fallback;
strategy selection and reviewer routes are unchanged.

`claude-careful` is the zero-Codex careful lane recorded in accepted
[0099](../docs/decisions/0099-claude-careful-recipe.md), for use while Codex
credits are exhausted. It extends `codex-careful` and replaces the intake,
design, implement and review seats; policy, verify, ship, protected review
and security hold are careful's. No Codex model appears anywhere in its
composed route, and every chain is a single model:

```text
+---------------------+------------------------------------------+
| Seat                | claude-careful                           |
+---------------------+------------------------------------------+
| Intake              | Haiku 5.5 / high (workspace hands)       |
| Design: simplicity  | Opus 5.5 / high (workspace hands)        |
| Design: robustness  | Official Flash / high (dsh, no hands)    |
| Design: chief       | Fable 5.1 / high (workspace hands)       |
| Design: check       | Offline exec gate (self's validator)     |
| Implement           | Sonnet 5.5 / high, 14400 s, 3 attempts   |
| Verify              | Pinned offline gates                     |
| Review: correctness | Fable 5.1 / high                         |
| Review: security    | Official Flash / high                    |
| Review: guarantees  | Official Flash / high                    |
| Review: tests       | GLM Flash / N/A                          |
| Review: independent | Opus 5.5 / high                          |
| Review: deadline    | 14400 s, 3 attempts                      |
| Ship                | Pinned exec gate                         |
+---------------------+------------------------------------------+
```

The engine has no agent-only override of a sequence, so this recipe
redefines the whole design seat. Its `./scripts/design-check.sh` then
resolves against `bundles/claude-careful/`, the only root its exec box
binds, so that directory carries byte-identical copies of `self`'s
validator and its two speckit helpers; `tests/delivery_recipes.rs` refuses
any drift. The dsh robustness position has no hands (dsh declares none);
it writes its run-local position file through its own tools, under its
charter. GLM serves only the tests member, so each run uses one Spark seat.

Codex work offices inherit the original intake, design and implementation
charters. Their declared workspace hands put writes through the engine's
boxed workspace tool under the realm's namespace boundary; the native Codex
shell stays read-only. The Cargo cache uses a private overlay with credential
files masked, and rustup is mounted read-only. Work hands permit network
access; verification remains offline. Provider credentials belong to the
host's existing login or secret configuration, never to a recipe or journal.

The tracked relative alias `recipes -> bundles` exposes these same directories
through the engine's default recipe shelf. It also identifies this checkout
as a recipe library, so modern engines discover the existing top-level
`agents/` and `adapters/` without moving or copying definitions. The canonical
files and scripts remain under `bundles/`.

Use the recorded brokkr build from `self/tooling.json`, with the required
Rust toolchains, cached locked dependencies and provider credentials already
available. The local recorded build is staged at `.forge/bin/brokkr`; put that
directory first on `PATH` rather than selecting another executable merely
because it reports the same version. Compile the recipes before delivery:

Before each isolated run, stage the digest-matched binary in that candidate's
own `.forge/bin/brokkr` and verify it against `bundles/self/tooling.json`.
Pass its absolute path to boxed checks; a sibling worktree's binary or the
launcher `PATH` is not visible inside the candidate. Treat a missing or
mismatched binary as a setup block and resolve it before provider dispatch.

The source identity includes the ordered checksummed tooling patches.
Apply them with `python3 scripts/apply-brokkr-patches.py <clean-source>`
before building the recorded source commit with its locked dependencies.
CI repeats that operation and the engine aggregation tests. The review patches preserve unknown member safety, scope and edit facts,
reject contradictory clean verdicts, and bind review and shipping to the
journal's verified commit and tree. Candidate checks explicitly include
untracked source regardless of Git's display settings. These are declared
dependency fixes with regression tests and checksummed source patches.

```bash
export PATH="$PWD/.forge/bin:$PATH"
brokkr compile --bundle bundles/codex
brokkr compile --bundle bundles/codex-careful
brokkr compile --bundle bundles/codex-fast
brokkr compile --bundle bundles/claude-careful
brokkr recipes show codex-fast
mkdir -p .forge
brokkr run --recipe codex-fast \
  --feature 'Implement one scoped story from epic #192'
```

Use `--recipe codex-careful` for higher intake and implementation reasoning
efforts; both recipes use the same Astra robustness and chief design seats.
The library's runtime state remains in gitignored `.forge/`. The realm map
selects `.forge/forge.db` as the shared local journal. From this checkout,
`brokkr tui` opens both imported history and new runs without extra arguments;
press `a` to include all older runs. Use the compatible recorded executable on
normal `PATH`. Import history through `brokkr export` / `brokkr import`, which
verify each event chain; keep the original source journal unchanged.

Launch isolated concurrent checkouts with `--db` pointing to this checkout's
absolute canonical journal path, so one TUI shows all eight runs. SQLite
coordinates journal writes; each run keeps its own run ID and attempt records.
Do not relocate or alias an open SQLite database or its WAL files. `.forge/` alone does not identify a recipe library for compilation:
keep the tracked `recipes` alias. Do not create an empty root `.brokkr/` as a
substitute configuration. Named launches use the default `recipes` alias;
`--recipes-dir bundles` is an equivalent explicit shelf.

The operator permits up to **eight concurrent delivery runs** on this host.
Give each run its own Git worktree, branch, result directory and
writable build targets. Share only read-only inputs or private cache overlays;
never share a writable Cargo target directory between runs. Tests bind port 0
or hold an occupied-port control rather than selecting a port and releasing
it. Group stories by file ownership, and reduce concurrency when memory, disk
or provider capacity requires it. Eight independent runs is the delivery limit,
not a promise of eight simultaneous heavy builds: each design and review
panel can launch several model calls of its own.

The route, triage and inheritance pins in `tests/delivery_recipes.rs` catch drift in
these assignments. Passing them alone does not establish executable delivery:
the engine must compile each composition, and a live run must execute the
inherited gates and protected review before the recipe is reported verified.
