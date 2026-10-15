# House rules — the swarmail realm

This realm is a single Rust crate. Production code is Rust under `src/`; the
integration tests under `tests/` speak the real protocol against real servers.
The delivery constitution lives in `bundles/self/` (`bundle.json` +
`policy.json`) and is compiled by brokkr (`brokkr compile --bundle
bundles/self`); the journal is `.brokkr/state/forge.db` (append-only, gitignored).
The constitution is a bundle directory and not the repo root so the run's
own journal writes and git objects stay outside the pinned manifest — a
root-level bundle pins its whole directory and could never survive a
resume.

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

## Gates — part of every change

Leave all of these clean before reporting success:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh     # the floor may rise, never fall
cargo deny check licenses         # permissive-only tree
```

Tests are part of every change. Extend the suite that proves the code —
`tests/burst.rs`, `tests/p2.rs`, `tests/rate.rs` or the unit tests — and run
the suite against real servers; no mocks in the loop.

The coverage floor lives in `scripts/coverage-gate.sh` — currently **100% of
lines, exact** (missed == 0, not a rounded 99.95). Raising it is an ordinary
change; lowering it is refused. Attribute-based coverage exclusions
(`coverage(off)`) are forbidden — production code cannot shrink the
denominator. Every new line of production code ships with a test that
executes it; a feature without its tests is an incomplete feature.

### Regression-test placement (decision 0020)

Decision [0020](decisions/0020-concurrency-tests-and-the-coverage-gate.md)
(*proposed*, awaiting the operator) settles where a concurrency regression
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
   module does not declare under `#[cfg(test)]` — the one silent way to
   shrink the measured denominator, closed structurally (decision 0020).

## Dependencies

`deny.toml` allowlists permissive licenses only (MIT, Apache-2.0, BSD, ISC,
Zlib, Unicode-3.0, CC-BY-4.0 for data noted inline). Anything outside the
allowlist fails `cargo deny check licenses`; a new dependency needs a
permissive license and a reason a human can defend in one sentence.

## Commits and history

Commits are GPG-signed with the operator's key (`commit.gpgsign` is set
repo-locally). Write messages in the repository's style: imperative mood, a
body that says why, not what the diff shows. History is append-only on
`main`; force-push only ever repairs signed history the operator asked to
re-sign.

## Delivery through brokkr

Every feature lands through a brokkr run: `brokkr run` drives
intake → design → implement → verify → review → ship (decision 0009,
*proposed*: the realm runs the speckit dialect, a contrarian design
panel argues before any code, a chief commits the spec, a deterministic
check rules the artifacts, and review splits into correctness +
security members); the review gate is constitutionally protected and
security-hold is a hard stop. `verify` and `ship` are hermetic exec
seats — a shell proves what a model claims. The work axis (intake,
design, implement) is pinned to `spark-glm/GLM-5.3-Flash-EXL3`
(`glm-flash`) only; the review panel is one independent model route per
member — `qwen-max`, `opus`, `astra`, `fable`, each falling back to
`glm-flash` (decision 0013). Adding a route is a decision document, not
an edit.
