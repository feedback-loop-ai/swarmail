# pytest-swarmail

A pytest plugin for [swarmail](../../README.md): a fresh inbox for every test,
and `wait_for_code` / `wait_for_link` that wake on the server's long-poll
instead of sleeping. A thin REST client — no dependencies beyond Python 3.9+
(`urllib`); pytest is the plugin host.

> **Status.** Versioned with the server (this package's version is
> `Cargo.toml`'s) and exercised in CI by `examples/`. **Not yet published** to
> PyPI: publishing is a separate operator act (proposed decision
> [0113](../../docs/decisions/0113-client-fixture-packages.md) §5). Python
> coverage of this package is not measured, and the packaged form — including
> the `pytest11` entry point — is unproved.

## Use

Point `SWARMAIL_URL` at the REST plane (default `http://127.0.0.1:8025`) and
use `swarmail_inbox.name` as the program under test's SMTP AUTH username:

```python
def test_sign_up_mails_a_code(swarmail_inbox):
    sign_up(f"{swarmail_inbox.name}@example.test", smtp_user=swarmail_inbox.name)
    assert swarmail_inbox.wait_for_code().code.isdigit()
```

- `swarmail_inbox.name` — `t` plus a random UUID's 32 hex digits, new for
  every test execution. The fixture never clears an inbox and never touches
  another; leftover mail is the evidence a failed test needs.
- `swarmail_inbox.wait_for_code(**options)` → `.code`, `.email`. The newest mail
  that carries any code decides. One distinct code in it is returned; several
  raise `SwarmailAmbiguous` (with `.candidates`). Older mail is never promoted.
- `swarmail_inbox.wait_for_link(match=None, **options)` → `.link`, `.email`.
  Every link in the filtered inbox, kept where `match(url)` is true, distinct.
  One is returned; several raise `SwarmailAmbiguous` — a mail with a verify
  link and an unsubscribe link needs a `match`.
- options: `timeout_ms` (default 5000, the server's own default; longer waits
  are sliced at the server's 60 000 ms cap), and the server's filters `to`,
  `from_`, `subject`, `since_ms` (forwarded verbatim; `received_ms` is
  inclusive, so pass `received_ms + 1` to skip a mail).
- Errors, all from `pytest_swarmail`: `SwarmailTimeout`, `SwarmailAmbiguous`,
  `SwarmailHttpError` (`.status`, `.body`; status 0 is a transport failure).
  Any non-200 other than the timeout fails at once, with no retry: the target
  is a local test server, and a 503 or refused connection is the test's
  failure.

Selection reads only the `codes` and `links` the server stored at ingest
(decision 0005) — no MIME decoding, no HTML parsing, no regex over a body.

## The examples

`examples/test_example.py` is six tests against a running `swarmail serve`;
`clients/playwright/examples/example.spec.ts` is the same six. The first sends
over real SMTP (`SWARMAIL_SMTP_ADDR`, default `127.0.0.1:1025`) with the inbox
name as the AUTH username; the rest seed through REST. CI runs them as the
"pytest fixture example suite" step of the `test` job:

```bash
cargo build --locked && target/debug/swarmail serve &
python3 -m venv .venv && .venv/bin/python -m pip install --no-deps --only-binary :all: -r clients/pytest/requirements-ci.txt
PYTHONPATH=clients/pytest .venv/bin/python -m pytest -p pytest_swarmail.plugin clients/pytest/examples
```

`requirements-ci.txt` is the CI-only pin of pytest and its own transitive
closure; it is not a dependency of the published package.
