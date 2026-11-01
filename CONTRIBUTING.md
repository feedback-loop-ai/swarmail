# Contributing to Swarmail

The short version: **every change leaves the gates clean, and the frozen
guarantees are not yours to trade.** Read
[`AGENTS.md`](AGENTS.md) → [`docs/house-rules.md`](docs/house-rules.md) →
[`docs/decisions/`](docs/decisions/).

## The gates

The gate list lives in one place:
[the Gates section of house rules](docs/house-rules.md#gates--part-of-every-change). Run every
command there: the locked builds and tests, the pinned brokkr compile, the
exact coverage gate and its selftest, the publish guard's proof, the seal
tool's keyless proof and cargo deny. That section also names the exact status
checks required before a pull request merges to main, plus the CI and verify
proof ownership. The native-tls absence proof runs in both CI and verify; the
Helm chart, Docker smoke and RustSec jobs are required main checks. Release
rate and publication jobs keep their main/tag conditions. This file keeps no
copy of the check list, so it cannot drift from house rules.

- Tests speak the real protocol against real servers — `tests/common/mod.rs`
  boots them; no mocks in the loop. Extend the suite that proves the code.
- The coverage floor (`scripts/coverage-gate.sh`) is exactly 100% of lines,
  branches **and** functions — every missed count zero — and may rise, never
  fall; `coverage(off)` attributes are forbidden.
- New dependencies need a permissive license (`deny.toml`) and a reason a
  human can defend in one sentence.

## Decisions

Semantic changes to the frozen guarantees (decisions 0001–0003) or to any
accepted ruling land as a **new** decision document with status
`proposed` under `docs/decisions/`; only the operator accepts one.
Corrections amend with a dated addendum — accepted decisions are never
edited.

## Delivery through brokkr

Features land through a brokkr run (`brokkr run`) — intake frames,
implement builds, verify proves, review refuses or cleans, ship commits.
The review gate is constitutionally protected; security-hold is a hard
stop. Direct commits to `main` are reserved for docs, decisions and
repairs, and still leave every gate clean.

Commit unsigned and never push from a seat. PR branches are not sealed:
a delivery run's PR merges by squash once every house gate and required
CI check is green, unless it is held for the operator (decision
[0056](docs/decisions/0056-signing-policy.md), 2026-11-01 addendum).

## Releases

Release notes live at `docs/releases/v<version>.md`; the tag is signed and
the GitHub Release renders the same text. See
[`docs/releases/v0.1.0.md`](docs/releases/v0.1.0.md) for the shape.

## License

Dual licensed MIT OR Apache-2.0 ([LICENSE-MIT](LICENSE-MIT) /
[LICENSE-APACHE](LICENSE-APACHE)); contributions are accepted under both.
