# Delivery recipes

The Codex recipes compose the realm's existing `self` constitution.
`codex-careful` extends `codex`, which extends `self`. The policy, protected
review phase, offline verification and ship gates remain the realm's pinned
proof chain. Both inherit the complete design seat so its final validator
keeps `self` as its script origin; only that base seat's three model-office
references change to the Codex design crew. Model reviewers read the change
after deterministic verification; they do not replace the shell verifier.

Use `codex-fast` for bug intake. Triage runs first on Codex Sol at medium
effort and records its evidence and reason before choosing the path:

```text
+----------------+----------------------+------------------------------+
| Triage result  | Work path            | Independent review           |
+----------------+----------------------+------------------------------+
| chore          | Implement -> verify  | DeepSeek Flash               |
| feature        | Implement -> verify  | Qwen Max + Claude Opus       |
| design/engine  | Design -> implement  | Qwen + Opus + Flash + GLM5.3  |
| escalate       | Stop                 | Operator ruling needed       |
+----------------+----------------------+------------------------------+
```

Here `chore` means an isolated, understood bug and `feature` means a broader
understood fix within existing contracts; these are the engine's route keys,
not permission to add an unrequested feature. Small lanes use triage's framing
instead of a spec package. Security, concurrency, persistence, gate/policy
and normative contract work default to the full design and four-seat review.
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
| Review: correctness | DashScope Qwen Max    | DashScope Qwen Max    |
| Review: security    | Claude Opus / high    | Claude Opus / high    |
| Review: guarantees  | DeepSeek Flash / high | DeepSeek Flash / high |
| Review: tests       | DashScope GLM 5.3     | DashScope GLM 5.3     |
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
| flash     | deepseek-flash        | Official DeepSeek platform |
| glm53     | dashscope/glm-5.3     | DashScope through dsh      |
+-----------+-----------------------+----------------------------+
```

Every new office has a single-model chain: unavailable providers stop the
attempt rather than silently replacing an independent reviewer with the
author's model. DeepSeek uses Flash only; no Pro route or fallback is hired.
DashScope uses provider defaults: the installed custom-model metadata does
not advertise an explicit reasoning-effort control for either Qwen Max or
GLM 5.3. DeepSeek Flash and Claude Opus keep high effort. All four reviewers
retain their existing read-only charters and `class: gate`.
The dsh adapter does not claim native tool confinement: those reviewers'
read-only conduct is a charter obligation, and the delivery gates detect tree
changes. A security hold remains a hard stop, and unknown residual safety
facts never authorize shipping.

The `codex-fast` implementation seat uses GPT-6 Luna at xhigh effort. Its
triage seat stays on Sol / medium; strategy selection and reviewer routes are
unchanged.

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
