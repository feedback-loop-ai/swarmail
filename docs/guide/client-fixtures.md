# Client fixtures — Playwright and pytest

[← README](../../README.md) · [Guide index](README.md)

Two thin REST clients live in [`clients/`](../../clients/), versioned with the
server (each package's version is `Cargo.toml`'s, checked under `cargo test`):
a Playwright fixture ([`clients/playwright`](../../clients/playwright/README.md),
`inbox.waitForCode()` / `inbox.waitForLink()`) and a pytest plugin
([`clients/pytest`](../../clients/pytest/README.md), `swarmail_inbox.wait_for_code()`
/ `.wait_for_link()`). Each gives every test its own inbox — the name is the
SMTP AUTH username — and waits on the server's `await` long-poll, then selects
only from the stored `codes` and `links`. Two distinct codes in the newest mail
that carries any, or two matching links anywhere in the inbox, raise an
explicit ambiguity error carrying the candidates; the fixtures never pick the
first. `SWARMAIL_URL` (default `http://127.0.0.1:8025`) is the only
configuration. Both run six example tests against the built binary in CI. Not
yet done: **publishing** to npm and PyPI is a separate operator act (proposed
decision [0113](../decisions/0113-client-fixture-packages.md)), TypeScript
and Python coverage is not measured, and the packaged forms are unproved.
