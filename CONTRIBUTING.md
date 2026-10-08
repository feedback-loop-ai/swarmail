# Contributing to Swarmail

The short version: **every change leaves the gates clean, and the frozen
guarantees are not yours to trade.** Read
[`AGENTS.md`](AGENTS.md) → [`docs/house-rules.md`](docs/house-rules.md) →
[`docs/decisions/`](docs/decisions/).

## The gates

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
brokkr compile --bundle .
bash scripts/coverage-gate.sh
cargo deny check licenses
```

- Tests speak the real protocol against real servers — `tests/common.rs`
  boots them; no mocks in the loop. Extend the suite that proves the code.
- The coverage floor (`scripts/coverage-gate.sh`) is 100% of lines, exact,
  and may rise, never fall; `coverage(off)` attributes are forbidden.
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

## Releases

Release notes live at `docs/releases/v<version>.md`; the tag is signed and
the GitHub Release renders the same text. See
[`docs/releases/v0.1.0.md`](docs/releases/v0.1.0.md) for the shape.

## License

Dual licensed MIT OR Apache-2.0 ([LICENSE-MIT](LICENSE-MIT) /
[LICENSE-APACHE](LICENSE-APACHE)); contributions are accepted under both.
