# Playwright and pytest fixtures over REST, versioned with the server

Decision: 0110 — [Client fixture packages](../../docs/decisions/0110-client-fixture-packages.md)

Issue #162, epic #164 (an agent surface measured by an eval harness), P2.
Base `main` a3cff51, which carries the merged MCP work of #152, #153 and
#154. Lane: zero-Codex `claude-careful` (accepted 0098/0099; residual rework
per accepted 0102).

## Feature and rationale

A program under test sends mail; the test wants the code or the link that
mail carries, and nothing else. swarmail already owns every guarantee that
makes this safe: accept means stored (0001), a wait subscribes before it
scans (0002), extraction happens at ingest (0005) and the SMTP AUTH username
is the inbox (0004). Today a Playwright or pytest author re-derives that
over raw HTTP in every project, usually wrongly: a sleep loop, a regex over
the body, the first mail in the list.

This feature ships two thin clients of those guarantees, published from this
repository and carrying the server's version:

- `clients/playwright/` — an npm package exposing a Playwright `test.extend`
  fixture `inbox` with `waitForCode` and `waitForLink`;
- `clients/pytest/` — a PyPI package exposing a pytest plugin whose fixture
  `swarmail_inbox` has `wait_for_code` and `wait_for_link`.

Each fixture gives a fresh inbox per test execution, waits on the server's
long-poll, and selects only from the stored `codes` and `links` fields the
REST `Email` already serializes (`src/static/openapi.json`, components
`Email`). No Rust production behaviour changes, no REST surface is added
(proposed 0094 keeps the focused waits MCP-only), and no Cargo dependency
moves. Each package's example suite runs in CI against the binary built
from the candidate; that run is the issue's acceptance criterion.

The chief read both run-local positions and the resolved intake framing for
run `issue-162-publish-playwright-and-8ed94189`. The framing's AC7 stop
condition does not fire: the REST view serializes `codes` and `links`.

## Panel adjudication

| Question | Ruling and rejected position | Cost retained |
| --- | --- | --- |
| How the client reads | Adopt simplicity: `await` is the wake-up (`limit=0`), `GET …/messages` is the read. Reject robustness's "await page plus admitted ids": `wait_for_report` truncates its page to `count` (`src/store.rs`), so an `await` page can never show the whole snapshot that 0094 §6 link ambiguity needs. | Two requests per wake instead of one. |
| Code selection | Both positions, and 0094 §5: the newest mail with a non-empty `codes` decides; one distinct code there selects, several are ambiguous; older mail is never promoted. The framing's AC4 is reworded to say "in the newest eligible mail" — a wait after a resend must return the newer code, not raise. | A test wanting the older of two codes filters with `since_ms` or the subject; the fixture does not offer "pick the older". |
| Link selection | Adopt simplicity and 0094 §6: every link across the whole filtered snapshot, filtered by the caller's predicate, distinct; one selects, several are ambiguous. Reject robustness's newest-mail rule for links: the two surfaces must agree about the same mail. | A welcome mail with a verify link and an unsubscribe link needs a predicate; without one it is ambiguous, by design. |
| Errors, retries, 503 | Adopt simplicity: a 408 after the deadline is the timeout; any other non-200, and any transport failure, raises at once with the server's status and body. Reject robustness's classified back-off: the fixture targets a local test server, where a 503 (stopping, 0018) or a refused connection is the test's failure, and a retry would mask it. | A server restart mid-wait fails the test instead of recovering. |
| Inbox lifecycle | Both: a random name per test execution, never shared, never `delete-all`. Reject robustness's teardown `DELETE`: names are unique, the default cap is 100 000 per inbox, and leftover mail is the evidence a failed test needs. | Inboxes accumulate on a long-lived server; the cap and `delete-all` are the operator's tools. |
| `since_ms` | Reconciled: forwarded verbatim when the caller passes it, never generated from the client clock. The documented source is a stored mail's `received_ms` (inclusive, so pass `received_ms + 1`). | None. |
| TLS and certificates | Reject robustness's `SWARMAIL_CA` and per-run CA: the REST plane is plain HTTP (`src/stdio.rs`: the local server has no TLS). | None. |
| Configuration | Adopt simplicity: `SWARMAIL_URL` (existing name, default `http://127.0.0.1:8025`) and nothing else in the fixtures. The pytest SMTP example alone reads `SWARMAIL_SMTP_ADDR` (default `127.0.0.1:1025`); it is example configuration, not fixture API. | No fixture-level default-timeout knob; the per-call option is the knob. |
| Default wait | Simplicity's 5000 ms, the server's own default, overridable per call. Reject 30 000 ms: a fixture default should match the surface it wraps. | Slow CI passes a larger `timeoutMs`/`timeout_ms` explicitly. |
| Seeding in the examples | REST `POST …/seed` in both suites (same parse and extraction path, decision 0010). Adopt robustness's "250 is the contract" once, where the stdlib allows it: one pytest example sends through real SMTP with AUTH username = inbox via `smtplib`. Node has no stdlib SMTP client, so Playwright seeds over REST only. | The Playwright suite does not exercise SMTP AUTH; the Rust suites and the pytest example do. |
| Shared conformance corpus | Reject robustness's JSON scenario file: two ~40-line clients pinned by the same six examples cost less than a third artefact and its loader in two languages. | Drift between the two clients is caught only by the paired examples. |
| CI shape | Adopt simplicity: Linux-only steps appended to the existing `test` job, one named step per suite. Reject robustness's one job per fixture: a new job is not a required context on `main` until the operator edits branch protection, so it would not block merges; `test (ubuntu-latest)` already does. | A fixture failure reddens `test (ubuntu-latest)`; the named step says which suite. Moving the steps into their own job later is a mechanical edit plus one README row and one operator action. |
| Dependencies | Zero runtime dependencies (global `fetch`; `urllib`). CI-only exact pins of the issue-named frameworks, committed in `clients/playwright/package.json` and `clients/pytest/requirements-ci.txt`, installed with scripts disabled and binaries only. No lockfile this run: none can be generated offline. The policy question is proposed 0110; see Acceptance Criteria. | A wrong pin is a red CI step, never a silent pass; the first real execution is CI. |
| TypeScript | The runtime is ESM JavaScript with a hand-written `index.d.ts`; the example spec is TypeScript, run by Playwright's own transform. A `.ts` runtime would need a build and a `typescript` devDependency nobody has ruled on, and raw `.ts` is not usable from `node_modules`. | `index.d.ts` is not type-checked against the runtime; recorded as not measured. |
| Version proof | Adopt simplicity: a `#[test]` in `tests/store_contract_docs.rs` compares `Cargo.toml`, `package.json` and `pyproject.toml`; the CI wiring is pinned in `tests/delivery_constitution.rs` beside its chart/smoke sibling. This replaces the framing's "checked script run in CI": the effect, a red check on mismatch, is met, and the proof also runs offline in the verify seat. | None. |

## Contract

### Identity

- `inbox.name` (`swarmail_inbox.name`) is `t` followed by a random UUID's
  32 hex digits, generated per test execution. Playwright's `test.extend`
  and pytest's function scope give a retry a fresh inbox.
- The program under test uses that name as its SMTP AUTH username (0004);
  the fixture never enumerates inboxes, never clears anything and never
  touches another inbox.

### Waiting

One algorithm, written once per language, push-based, no sleep:

```
wait(inbox, filters, timeout, pick):
  deadline = monotonic_now + timeout;  count = 1
  loop:
    GET /api/v1/inboxes/{inbox}/await?{filters}&count={count}&limit=0
        &timeout_ms={min(60000, max(0, deadline - now))}
      408 -> raise SwarmailTimeout;  other non-200 or transport failure -> raise with status and body
    list = GET /api/v1/inboxes/{inbox}/messages?{filters}&limit=250
    if list.total > len(list.emails): raise (snapshot incomplete; a per-test inbox never holds 250 matches)
    picked = pick(list.emails);  if picked: return picked
    count = list.total + 1
```

- `await` subscribes before it scans (0002); the client never reads before
  its first `await`, so a mail delivered between the list read and the next
  `await` wakes that `await` at once.
- `await` and the list share `Store::list`, so they agree on order (newest
  first). `limit=0` makes `await` wake-only, which the server documents.
- `timeout_ms` is sliced at the server's 60 000 cap; a final `timeout_ms=0`
  is one scan, then 408. `count` above 250 is the server's named 400,
  raised as is.
- Each HTTP call carries a socket timeout of the remaining time plus a
  margin so a hung server cannot hang the test.
- Filters `to`, `from` (`from_` in Python), `subject`, `since_ms` are
  forwarded verbatim; unknown or malformed values are refused by the server
  by name (0044 §3). `timeoutMs`/`timeout_ms` defaults to 5000.

### Selection

- `waitForCode` / `wait_for_code`: the newest mail with a non-empty `codes`
  decides. Its distinct codes: one → return `{ code, email }`; several →
  raise `SwarmailAmbiguous` with `candidates`. No mail with codes → keep
  waiting.
- `waitForLink(match?)` / `wait_for_link(match=None)`: all `links` across
  the snapshot, kept where `match(url)` is true (no `match` keeps all),
  distinct; one → return `{ link, email }` with the newest mail carrying it;
  several → raise `SwarmailAmbiguous` with `candidates`; none → keep waiting.
- Both read only stored fields. Neither decodes MIME, parses HTML or runs
  a regex over bodies (0005).

### Packages

- `clients/playwright/`: `package.json` (name `@swarmail/playwright`, ESM,
  `exports`, `peerDependencies: @playwright/test`, exact `devDependencies`
  pin, `engines.node >= 18`), `client.js` (no framework import),
  `index.js` (the fixture), `index.d.ts`, `examples/example.spec.ts`,
  `examples/playwright.config.ts`.
- `clients/pytest/`: `pyproject.toml` (name `pytest-swarmail`, a
  `pytest11` entry point), `pytest_swarmail/__init__.py` (the client, no
  pytest import), `pytest_swarmail/plugin.py` (the fixture),
  `requirements-ci.txt`, `examples/test_example.py`.
- Each package version equals `Cargo.toml`'s `package.version`.
- Publishing to npm or PyPI is outward-facing and is not part of this run;
  issue #162's "published" stays open after it, and 0110 says so.

### Example suites, the same six tests in each language

1. One mail with one code and one link: both waits return them.
2. Two distinct codes in one mail: `wait_for_code` raises ambiguous.
3. Two matching links across two mails: `wait_for_link` raises ambiguous.
4. A code-less mail first, then the code mail: the code is returned.
5. The wait is parked before the mail is sent and still returns it (0002).
   The test gates on the server's `swarmail_awaits_active{inbox="…"}`
   gauge reaching 1 before seeding, never on a sleep (0020 §5).
6. A wait against an inbox that only another test's mail reaches times out
   with `SwarmailTimeout`.

The pytest suite's first test sends over real SMTP with AUTH username =
inbox through `smtplib`; every other seed is `POST …/seed`.

## Acceptance Criteria

- **AC1 (issue):** CI runs each example suite against `target/debug/swarmail`
  built from the candidate with `--locked`, as named Linux steps of the
  `test` job; both steps are green and each log shows at least one passing
  test. CI-only: the verify seat installs nothing.
- **AC2:** The Playwright fixture gives a per-test inbox whose name is the
  SMTP AUTH username (0004), and `waitForCode` / `waitForLink` over REST
  read the stored `codes` / `links` only (0005). Proof: example 1.
- **AC3:** The pytest fixture gives the same three behaviours with the same
  six example assertions. Proof: `examples/test_example.py`.
- **AC4:** Ambiguity is explicit, as 0094 §5 and §6 rule it: two distinct
  codes in the newest mail carrying any, or two matching links anywhere in
  the filtered snapshot, raise an ambiguous outcome carrying the candidates;
  the fixture never silently picks the first. Proof: examples 2 and 3.
- **AC5:** A wait started before the mail is sent returns that mail
  (0002), with the parked state proved through the awaits gauge, not a
  sleep. Proof: example 5.
- **AC6:** Each package version equals `Cargo.toml`'s version, and
  `ci.yml` carries both example-suite steps, proved under
  `cargo test --workspace --locked` (offline in the verify seat, red in CI
  on mismatch). This supersedes the framing's "checked script".
- **AC7:** No Rust production behaviour changes; no REST route is added.
  Proof: `git diff a3cff51 -- src/` is empty.
- **AC8:** Every house gate is clean `--locked`, `Cargo.lock` is unchanged,
  `deny.toml` is unchanged and `cargo deny check licenses` is clean.
- **AC9:** The published packages have zero runtime dependencies; CI-only
  test-time pins are exact, committed, installed with scripts disabled and
  binaries only, and limited to the issue-named frameworks and their own
  transitive closure. The policy is proposed 0110, awaiting the operator;
  implementation is compatible work under accepted 0045 §8.
- **AC10:** Honest reporting: TypeScript and Python coverage are reported as
  not measured; the example suites' first real execution is CI; packaging
  (`npm pack`, the `pytest11` entry point) is unproved and publishing stays
  open. No result claims otherwise.
