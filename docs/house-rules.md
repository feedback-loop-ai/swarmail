# House rules — the swarmail realm

This realm is a single Rust crate. Production code is Rust under `src/`; the
integration tests under `tests/` speak the real protocol against real servers.
Everything brokkr reads or writes lives in one folder, `.brokkr/`
(decision 0100, brokkr's decision 0083): the recipes on its shelf
`.brokkr/recipes/<name>/`, the agents, adapters, dialects and realm map
beside them, and the folder's own `bundle.json`, which extends
`claude-careful`. The delivery constitution is the shelf recipe
`.brokkr/recipes/self/` (`bundle.json` + `policy.json`), compiled with
`brokkr compile --bundle .brokkr/recipes/self`. Runtime state stands under
`.brokkr/state/` and is ignored by `.brokkr/.gitignore`: the journal
`.brokkr/state/forge.db` (append-only), results, ledgers and run worktrees.
Earlier journals (`.brokkr/state/forge.db` before 2026-10-23, then
`.forge/forge.db` under 0057) remain historical evidence and must not be
fabricated. The constitution is a shelf directory and not the repo root so
the run's own journal writes and git objects stay outside the pinned
manifest — a root-level bundle pins its whole directory and could never
survive a resume.

## The frozen guarantees

These are the reason swarmail exists. A change that weakens one is refused in
review, no matter how green the tests:

1. **Accept == stored.** `Store::insert` is synchronous; an SMTP `250` means
   the mail is queryable. There is no async gap where a message can vanish.
2. **`await` subscribes before it checks.** The long-poll registers its
   watcher first, then scans, so a mail delivered between check and
   subscribe can never be missed.
3. **Pruning is oldest-only.** The per-inbox cap evicts the oldest mail and
   never touches a delivery in flight.

Semantic changes to these carry a decision document under `docs/decisions/`
with status `proposed`; only the operator accepts one. The decisions there
are the authority on why the code is shaped the way it is — intake seats read
the index and the decisions their task touches before framing anything.

**Code under a proposed decision** (accepted decision 0045 §8). A proposal
is a status, not operator acceptance. Compatible implementation — work that
preserves every accepted ruling — may land while its decision is still
proposed. A change that contradicts an accepted ruling waits for operator
acceptance of the decision that amends it; that holds for amendments to every
one of decisions 0001–0005, not only the frozen first three. Weakening a
frozen guarantee is refused regardless of green tests, even with a proposal
beside it. The operator's acceptance of earlier proposals (for example the
2026-10-19 acceptance of 0034–0050 and 0055) is not standing permission for
future ones: each proposal is accepted, or not, on its own.

## Gates — part of every change

Leave all of these clean before reporting success:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked --bin swarmail    # the candidate's binary the eval suite runs against
cd evals && cargo fmt --all -- --check
cd evals && cargo clippy --all-targets --locked -- -D warnings
cd evals && SWARMAIL_BIN=<the candidate's swarmail> cargo test --locked
brokkr compile --bundle .brokkr/recipes/self   # the brokkr pinned in .brokkr/recipes/self/tooling.json
bash scripts/coverage-gate.sh          # the floor may rise, never fall
bash scripts/coverage-gate.sh selftest # the gate's verdict is untrusted without it
bash scripts/publish-guard-test.sh     # the publish guard's proof
bash scripts/seal-branch-test.sh       # the seal tool's keyless proof (0056)
cargo deny check licenses              # permissive-only tree
```

### Required status checks on main

The operator's assistant read main branch protection through the GitHub API on 2026-10-24. That dated observation reports strict required checks, enforce_admins enabled, linear history required, and main CI green at dc1822a. The nine status check contexts required before a pull request merges to main are:

- format, clippy, publish guard
- test (ubuntu-latest)
- test (macos-latest)
- coverage
- deny
- RustSec dependency audit
- helm chart
- brokkr bundle (pinned)
- docker smoke

These are the displayed status context names. The test context expands to both listed operating systems. This is the operator-supplied observation for 2026-10-24, not a continuous read of GitHub settings.

`evals/` is a standalone, unpublished workspace outside the root coverage
denominator (decision 0055), so none of the root commands reach it; CI and the
verify seat therefore run its fmt, clippy and tests explicitly, `--locked`,
against the candidate's own root `swarmail` binary (resolved from Cargo's
artifact record of `cargo build --locked --bin swarmail`, never the harness's
build-when-unset fallback) — CI on ubuntu and macOS, inside the `test` jobs
(issue #201). The root `cargo deny` does not read `evals/Cargo.lock`; the
harness's boundary test pins every foreign package there instead.

This is the one gate inventory: CI and the verify seat both run all of it,
plus a fail-closed proof that `native-tls` is absent from the tree. The absence
proof runs inside CI's deny job and is not a separate status context. The
required-on-main contexts above include the CI-owned Helm chart, Docker smoke
and RustSec jobs. CI also owns the chart-bump check against the event's
baseline and its history/selftest proofs (issue #141), the Docker image smoke
(persistence across stop and kill, no leaked anonymous volume, no target/
in the build context), the network-backed RustSec advisory audit, and the
test jobs' real port-80 bridge exchange (the ignored
portless_bridge_pings_port_eighty, run with the Linux unprivileged port floor
lowered and as root on macOS — decision 0047 §4). The tag-only
release-rate-5k proof gates both publish jobs. A tag publishes the image
before the chart: the chart job needs the image job's success and preflights
the image it names (proposed 0051). The verify seat runs offline and installs
nothing. Main and tag image publication, tag-only chart publication and the
tag-only rate proof are separate from the required pull-request contexts.

Tests are part of every change. Extend the suite that proves the code —
`tests/burst.rs`, `tests/p2.rs`, `tests/rate.rs` or the unit tests — and run
the suite against real servers; no mocks in the loop.

The coverage floor lives in `.brokkr/recipes/self/scripts/coverage-gate.sh` (run it
through `scripts/coverage-gate.sh`) — currently **100% of
lines, branches and functions, exact** (missed == 0, not a rounded 99.95).
The full lcov report and summary land under `target/coverage/`. Raising it is an ordinary
change; lowering it is refused. Attribute-based coverage exclusions
(`coverage(off)`) are forbidden — production code cannot shrink the
denominator. Every new line of production code ships with a test that
executes it; a feature without its tests is an incomplete feature.

### Regression-test placement (decision 0020)

Decision [0020](decisions/0020-concurrency-tests-and-the-coverage-gate.md)
(*accepted*, operator, 2026-10-16) settles where a concurrency regression
test lives; the axis is **cold-path presence**, not seam access:

1. **Inline `#[cfg(test)]` in `src/<module>.rs` is the default and stays
   measured.** Inline modules are part of the production report — every line
   and branch must be taken on every run, which is the exact gate's vacuity
   detector working. A branch-stable deterministic interleaving test stays
   inline even when it drives a crate-private seam; seam access alone never
   forces a move.
2. **A test that needs a cold failure path** — a line that runs only while
   the bug is present or the schedule betrays (an exhausted retry budget, a
   "window never hit" diagnostic, a sleep-as-sync residual) — is unwritable
   inline. Its home is `src/<module>/<name>_tests.rs`, declared
   `#[cfg(test)] mod <name>_tests;` beside its owner: it runs under
   `cargo test`, sees `pub(crate)`, is excluded from the coverage report by
   its harness name, and still covers the production lines it executes. The
   harness is a refuge for the cold path, never a landfill for the
   inconvenient: an inline module moves only when a concrete test demands it.
3. **Public-API, wire-shaped races live in `tests/*.rs`** — real protocol
   against real servers, no mocks (0004). A `tests/` test must not need a
   crate-private seam; if it does, it is a rule-2 test.
4. **Seams stay `#[cfg(test)]`-only** on the owning module, `pub(crate)`
   where a sibling harness file needs them — never a feature flag, which
   would double every gate's build matrix. Every seam keeps at least one test
   that installs it: the dead-arm refusal follows execution, not file
   location, because harness execution of production lines still feeds the
   report.
5. **Writing disciplines, wherever the test lives:** chaos at the poles only
   (probability 0 or 100 — a mid-band rule is unreproducible by
   construction); no wall-clock validity assertions (generate the fixture
   per run, or drive the branch behind a generated fixture); no sleep as
   synchronization (a sleep a test depends on is a race wearing a timeout —
   use a seam or a real deadline); no shared global mutable state without a
   seam (fresh `Store::new` inline; per-test inboxes on the wire).
6. **Harness names are reserved under `src/`** (`tests.rs`, `*_tests.rs`,
   `*-tests.rs`): the coverage gate refuses any such file that its parent
   module does not declare under `#[cfg(test)]`. That declaration check is
   the structural half; it is not what closes the denominator. The refusals
   are by effect (decision 0035, refined by 0050 §2): every file the
   production build compiles must appear in the report, every exclusion that
   survives the compiler's expansion of a production unit *or of the
   library's test unit* refuses, each production unit's expansion must equal
   its no-coverage-cfg and release-profile expansions, and a second
   workspace member or any local path package refuses.

The gate scripts themselves are the pinned bundle's
(`.brokkr/recipes/self/scripts/`, decision 0050 §1): `scripts/coverage-gate.sh`,
`scripts/publish-guard*.sh`, `scripts/design-check.sh`, `scripts/seal-branch*.sh` and
`.specify/scripts/bash/check-prerequisites.sh` are fixed dispatch shims the
verify seat checks against `.brokkr/recipes/self/scripts/shims.sha256` and never
executes; it runs the pinned copies against the candidate.

## Dependencies

`deny.toml` allowlists permissive licenses only, and only those the locked
tree uses: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0,
and CDLA-Permissive-2.0 for one data dependency. (Zlib and CC-BY-4.0 were
dropped as unused allowances, #142; restoring either is a reviewed widening
like any other.) The CDLA-Permissive-2.0 entry covers
`webpki-roots`, Mozilla's root CA bundle shipped as data. It is the only
license under which rustls distributes the offline trust store, and it
carries no copyleft and no field-of-use restriction. Anything outside the
allowlist fails `cargo deny check licenses`; a new dependency needs a
permissive license and a reason a human can defend in one sentence.
One data file is not a dependency and so is outside that check: the
pinned WHATWG `entities.json` snapshot, `scripts/data/html-entities.json`,
is CC-BY-4.0 data the crate packages for the entity table's proof
(decision 0049 §2), attributed where it is recorded.

## Commits and history

Author commits are unsigned (decision
[0056](decisions/0056-signing-policy.md), *accepted*, operator, 2026-10-20,
with its 2026-11-01 addendum). Seats and the operator's assistant commit
unsigned (`commit.gpgsign=false` is set repo-locally), never push, and never
stop because signing is unavailable — nothing signs mid-run. Write messages
in the repository's style: imperative mood, a body that says why, not what
the diff shows.

PR branches are not sealed (0056's 2026-11-01 addendum). The operator's
assistant merges a delivery run's PR by squash once every house gate passes
on the final tree, after `main` is merged in, and every required CI check is
green. It holds for the operator, and never merges on green, a PR that
changes a decision's status to accepted; adds a package to `Cargo.lock` or
`evals/Cargo.lock` or changes `deny.toml`; or changes `.github/`,
`scripts/`, `.brokkr/` or the toolchain configuration. The operator may
approve a held PR by name. `scripts/seal-branch.sh` stays available, and
sealing is optional; when it is used, the rules above it still hold: never
rebase or squash when sealing, never onto a newer `main`.

Three kinds of signature are on record, and they mean different things:

- **Sealed branch authorship** — the operator's signature added at the end
  by `scripts/seal-branch.sh`: the operator vouches for every author commit
  of the PR branch. Required from 2026-10-20 to 2026-11-01, optional since.
  (Author commits signed at commit time before 2026-10-20 carry the same
  meaning and remain as recorded.)
- **GitHub landing** — a squash merge, or a merge commit created on GitHub,
  is signed by GitHub's key, not the operator's; the author commits remain
  on the PR branch.
- **Release tags** — signed by the operator when cutting a release
  (`CONTRIBUTING.md`, Releases); sealing never touches a tag.

- **Documented historical exceptions** — the squash messages of #107
  (`d9a66f4`), #108 (`745d6ad`) and #109 (`bacadbe`) record eleven author
  commits as "not GPG-signed" (pinentry could not prompt in the seat's
  environment); the squash messages of #34, #59, #62 and #66 record
  "Unsigned" author commits the same way; and ten direct commits of
  2026-10-13/14 sit on `main` unsigned themselves (`4f94b33` through
  `66fab48` — `git log --first-parent --format='%G? %h' main` lists them
  as `N`). They stay as recorded: `main` is not rewritten to re-sign them,
  and they set no precedent.

Those exceptions stay as recorded; none is a standing permission. History is append-only on `main`; force-push only ever
touches a PR branch — sealing it, or repairing signed history the operator
asked to re-sign.

## Delivery through brokkr

Every feature lands through a brokkr run. The default full route drives
intake → design → implement → verify → review → ship (decision 0009,
*accepted*: the realm runs the speckit dialect, a contrarian design
panel argues before any code, a chief commits the spec, a deterministic
check rules the artifacts, and review is a four-member panel —
correctness and security among them, decision 0013); the review gate is constitutionally protected and
security-hold is a hard stop. `verify` and `ship` are hermetic exec
seats — a shell proves what a model claims. Decision
[0057](decisions/0057-codex-delivery-recipes.md) and its accepted amendment
[0058](decisions/0058-codex-review-routing.md) define the operator-directed
Codex recipes. Standard uses Fable 5.1 correctness, official DeepSeek Flash
security and GLM Flash tests. Careful and the fast design/engine lanes require
five separate results: Flash correctness, Flash security, GLM Flash tests, Flash
guarantees and independent Fable. `codex-fast` uses GPT-6 Luna at xhigh for
implementation and triages first on Claude Opus 5.5 (`opus`) at medium effort
with no fallback: isolated bugs get one combined Flash
reviewer; broader fixes get Qwen Max correctness and Flash security, with
Sonnet allowed only after Qwen dispatch failure before a review result. Risky
work keeps the five-role panel. The revised 2026-10-26 operator ruling,
recorded in proposed
[0081](decisions/0081-glm-flash-seats.md), changes only tests review to local
Spark `glm-flash` through dsh, with reasoning effort not applicable; intake
keeps its current models and workspace hands. The 2026-10-28 operator
direction, recorded in accepted
[0098](decisions/0098-review-panel-flash-and-deadline.md), moves the
full-panel correctness and guarantees members from Codex Astra to official
DeepSeek Flash through dsh at high effort (security's route; offices and
results stay separate) and gives the `codex`, `codex-careful` and `codex-fast`
review seat a 14400 s deadline instead of 3600 s.
Accepted [0099](decisions/0099-claude-careful-recipe.md) records the
2026-10-28/29 direction for `claude-careful`, a zero-Codex careful lane:
Haiku 5.5 intake, an Opus 5.5 / official Flash / Fable 5.1 design council,
Sonnet 5.5 implementation, and Fable correctness, Flash security and
guarantees, GLM tests and Opus independent review, with careful's gates.
DeepSeek Pro is forbidden; other reviewers
have no fallback. Every lane retains all shell gates, protected review and the
hard security hold. The `self` bundle
retains GLM intake/implementation and its original `qwen-max`, `opus`,
`astra`, `fable` review chains from decision 0013; its shared design now
uses Codex. Adding a route is a decision document, not an undocumented
edit. Up to eight isolated delivery runs may execute concurrently under
0057; no run pushes or merges.
