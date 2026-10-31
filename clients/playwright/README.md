# @swarmail/playwright

A Playwright fixture for [swarmail](../../README.md): a fresh inbox for every
test, and `waitForCode` / `waitForLink` that wake on the server's long-poll
instead of sleeping. A thin REST client — no runtime dependencies beyond Node
18+ (global `fetch`); `@playwright/test` is a peer.

> **Status.** Versioned with the server (this package's version is
> `Cargo.toml`'s) and exercised in CI by `examples/`. **Not yet published** to
> npm: publishing is a separate operator act (proposed decision
> [0110](../../docs/decisions/0110-client-fixture-packages.md) §5). TypeScript
> and JavaScript coverage of this package is not measured, and the packaged
> form (`npm pack`) is unproved.

## Use

Point `SWARMAIL_URL` at the REST plane (default `http://127.0.0.1:8025`) and
use the program under test's SMTP AUTH username = `inbox.name`:

```ts
import { test, expect } from '@swarmail/playwright';

test('sign-up mails a code', async ({ page, inbox }) => {
  await signUp(page, `${inbox.name}@example.test`, { smtpUser: inbox.name });
  const { code } = await inbox.waitForCode();
  expect(code).toMatch(/^\d{6}$/);
});
```

- `inbox.name` — `t` plus a random UUID's 32 hex digits, new for every test
  execution (a retry gets a fresh one). The fixture never clears an inbox and
  never touches another; leftover mail is the evidence a failed test needs.
- `inbox.waitForCode(options?)` → `{ code, email }`. The newest mail that
  carries any code decides. One distinct code in it is returned; several throw
  `SwarmailAmbiguous` (with `candidates`). Older mail is never promoted.
- `inbox.waitForLink(match?, options?)` → `{ link, email }`. Every link in the
  filtered inbox, kept where `match(url)` is true, distinct. One is returned;
  several throw `SwarmailAmbiguous` — a mail with a verify link and an
  unsubscribe link needs a `match`.
- `options`: `timeoutMs` (default 5000, the server's own default; longer waits
  are sliced at the server's 60 000 ms cap), and the server's filters `to`,
  `from`, `subject`, `sinceMs` (forwarded verbatim; `received_ms` is
  inclusive, so pass `received_ms + 1` to skip a mail).
- Errors: `SwarmailTimeout` (nothing selectable in time), `SwarmailAmbiguous`,
  `SwarmailHttpError` (`status`, `body`; `status` 0 is a transport failure).
  Any non-200 other than the timeout fails at once, with no retry: the target
  is a local test server, and a 503 or refused connection is the test's
  failure.

Selection reads only the `codes` and `links` the server stored at ingest
(decision 0005) — no MIME decoding, no HTML parsing, no regex over a body.

## The examples

`examples/example.spec.ts` is six tests against a running `swarmail serve`;
`clients/pytest/examples/test_example.py` is the same six. CI runs them as the
"Playwright fixture example suite" step of the `test` job:

```bash
cargo build --locked && target/debug/swarmail serve &
npm install --prefix clients/playwright --ignore-scripts --no-package-lock
clients/playwright/node_modules/.bin/playwright test -c clients/playwright/examples
```

The pinned `@playwright/test` is a CI-only `devDependency`. The examples seed
through REST `POST …/seed` (Node has no stdlib SMTP client); the pytest suite
also sends one mail over real SMTP.
