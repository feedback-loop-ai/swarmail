# House rules — the swarmail realm

This realm is a single Rust crate. Production code is Rust under `src/`; the
integration tests under `tests/` speak the real protocol against real servers.
The delivery constitution lives in `bundle.json` + `policy.json` and is
compiled by brokkr (`brokkr compile --bundle .`); the journal is
`.forge/forge.db` (append-only, gitignored).

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
brokkr compile --bundle .
bash scripts/coverage-gate.sh     # the floor may rise, never fall
cargo deny check licenses         # permissive-only tree
```

Tests are part of every change. Extend the suite that proves the code —
`tests/burst.rs`, `tests/p2.rs`, `tests/rate.rs` or the unit tests — and run
the suite against real servers; no mocks in the loop.

The coverage floor lives in `scripts/coverage-gate.sh`. Raising it is an
ordinary change; lowering it is refused. Attribute-based coverage exclusions
(`coverage(off)`) are forbidden — production code cannot shrink the
denominator.

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
intake → implement → verify → review → ship; the review gate is
constitutionally protected and security-hold is a hard stop. `verify` and
`ship` are hermetic exec seats — a shell proves what a model claims. The
model adapter is pinned to `spark-glm/GLM-5.3-Flash-EXL3` (`glm-flash`)
only; adding a route is a decision document, not an edit.
