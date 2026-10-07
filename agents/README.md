# starter — scaffolded by `brokkr init`

This directory was scaffolded from inside a repository that reads
as a rust project (`Cargo.toml`). Everything here is ordinary text:
read it, edit it, commit it.

## What is here

- `bundle.json` — three model offices plus boxed exec verify and
ship gates, with each seat's results and limits.
- `policy.json` — the phase table; `review` is the protected phase.
- `adapters/claude.json` and `adapters/exec.json` — the model and
deterministic drivers, including their trust tiers.
- `agents/*.json` — one agent per model office: charter, model chain,
tool allowance, limits. `brokkr agents show <name>` reads one back.
- `agents/charters/*.md` — the three model-office charters.
- `scripts/*.sh` — deterministic verify and ship offices; verify
names this repository's own commands and runs without network.

## Tool grants

`adapters/claude.json` maps each tool the seats are granted below
to the `Bash(...)` expression the claude CLI reads — the stack's
own runners, and nothing broader:

    cargo → Bash(cargo:*)
    git → Bash(git:*)
    ls → Bash(ls:*)
    rg → Bash(rg:*)
    mkdir → Bash(mkdir:*)

Work-class seats (intake, implement) are granted the whole set, so
a seat may run exactly the commands its charter names:
cargo, git, ls, rg, mkdir.

The model-backed review gate is granted the read-only subset — the
test runner's tools and the tools that read — and never `mkdir`:
cargo, git, ls, rg. Verify and ship are boxed scripts with no model grant.

The grant is per BINARY, not per subcommand: the test runner's
binary also answers to its build and install subcommands, so it is
each gate's charter (prove it, fix nothing) and not the grant that
keeps a gate from building.

An allowance is ONE grant with the adapter's `tool_permissions.names`:
an allowance whose name the map cannot express refuses this
scaffold's own compile, so when you edit one, edit both.

`brokkr init` chose all of this from the files at the repository
root — it ran nothing to find out. Correct it here if it is wrong.

## Agent CLI and boundary

Scaffolded for `claude`, the first of claude, codex and dsh found on PATH.

## Specification dialect

Found neither `.specify/` (spec-kit) nor `openspec/config.yaml` (OpenSpec), so `realms.json` carries no dialect. An artifact phase will need one before it can run.
