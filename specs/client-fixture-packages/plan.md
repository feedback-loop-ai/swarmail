# Implementation plan — Playwright and pytest fixtures

Implements [spec.md](spec.md) under proposed
[0110](../../docs/decisions/0110-client-fixture-packages.md).

## Technical context

One Rust crate, unchanged. Two new client directories outside the crate's
explicit `include` list (`Cargo.toml`), so the `.crate` is unaffected. Node
18+ (global `fetch`, `crypto.randomUUID`) and Python 3.9+ (`urllib`,
`uuid`, `smtplib`) are the only runtimes; the GitHub runners preinstall
both. `.specify/templates/plan-template.md` is absent in this checkout; this
plan supplies the technical context, constitution check, structure,
research, data model, contracts, quickstart and risks here.

This is a design-only commit. It implements nothing and attests no gate.
The sequence's deterministic check rules these artifacts; implementation,
verify and the five-member protected review remain owed.

## Constitution check

- 0001–0005 untouched: no production line changes. The clients read stored
  fields only (0005), wake through `await` (0002) and name inboxes by the
  SMTP AUTH username (0004).
- 0094 (proposed) is honoured, not amended: no REST focused-wait route; the
  clients re-state §5 and §6 selection and are version-locked to the server.
- 0010: the examples seed through the same constructor the MCP seed uses.
- 0018/0030/0044: the client sends only documented query keys and keeps
  `timeout_ms`, `count` and `limit` inside their domains; refusals are the
  server's, surfaced verbatim.
- 0020: the Rust tests added are deterministic file-reading contract tests
  in existing `tests/*.rs` suites; no new root suite, so the README suite
  table (D09) is unchanged. The example suites gate on the awaits gauge,
  never a sleep (§5).
- Dependencies: `Cargo.lock` and `deny.toml` unchanged. npm/PyPI pins are
  CI-only and recorded in 0110 for the operator.
- 0056: commit unsigned, never push. 0045 §8: compatible work under a
  proposed decision.

## Research and decisions

- `GET /api/v1/inboxes/{inbox}/await`: `limit` 0–250 (0 answers an empty
  page), `count` 0–250, `timeout_ms` 0–60000, 408 on timeout, 503 when
  stopping, 400 named for domain and unknown keys. `total` is counted after
  the wait. `wait_for_report` truncates matches to `count`, which is why the
  list, not the await page, is the read.
- `GET /api/v1/inboxes/{inbox}/messages`: newest first, `limit` ≤ 250,
  `total` in `EmailList`; `since_ms` is inclusive (`src/store.rs`,
  `filter_since_ms_is_inclusive`).
- `Email` carries `links: string[]` and `codes: string[]` (at most the
  first 64 distinct codes, extraction order, decision 0049).
- `POST /api/v1/inboxes/{inbox}/seed` takes `to`, `from`, `subject`, `text`,
  `html`; `html` supersedes `text`; extraction runs at ingest.
- `/metrics` renders `swarmail_awaits_active{inbox="…"} n`; `/healthz`
  answers `{"status":"ok",…}` once every listener accepts.
- `swarmail serve` with defaults binds `127.0.0.1:1025/1110/8025`
  (`src/config.rs`); `SWARMAIL_URL` already names the REST base for the
  stdio bridge and keeps its meaning here.
- SMTP AUTH PLAIN/LOGIN is accept-any with username = inbox (README,
  `tests/smtp_edges.rs`).
- `tests/store_contract_docs.rs` already reads repo files and `ci.yml`;
  `tests/delivery_constitution.rs` pins CI wiring with string needles
  (`ci_runs_the_chart_history_check_and_the_docker_smoke`). Both patterns
  are reused as is.
- The `test` job restores `Swatinem/rust-cache`; npm and pip write outside
  `target/` and `~/.cargo`, so nothing they install enters the cache. A
  pull request's `cargo test` already runs arbitrary code in that job.

## Project structure

```
clients/
  playwright/
    package.json             name, version (= Cargo.toml), type: module, exports,
                             peerDependencies.@playwright/test, exact devDependencies pin,
                             engines.node, files, license
    client.js                SwarmailClient: newInboxName, waitForCode, waitForLink; errors
    index.js                 export const test = base.extend({ inbox })
    index.d.ts               the typed public surface
    examples/
      playwright.config.ts   testDir '.', no browser projects, retries 0
      example.spec.ts        the six examples; REST seed helper local to the file
    README.md                install, fixture use, options, errors
  pytest/
    pyproject.toml           name, version (= Cargo.toml), dependencies: [], pytest11 entry point
    pytest_swarmail/
      __init__.py            SwarmailClient and errors (no pytest import)
      plugin.py              the swarmail_inbox fixture
    requirements-ci.txt      exact pins: pytest and its transitive closure
    examples/
      test_example.py        the six examples; the first one sends via smtplib
    README.md
```

Touched elsewhere:

- `.github/workflows/ci.yml` — Linux-only steps at the end of the `test`
  job: build `--locked`, start `target/debug/swarmail serve` on the
  defaults, poll `/healthz` under a deadline, install the pinned framework,
  run the suite, stop the server — one step per suite, each with its own
  server lifetime, each guarded by a trap that kills the server and dumps
  its log on failure.
- `tests/store_contract_docs.rs` — the version-equality test.
- `tests/delivery_constitution.rs` — the CI wiring needles for both steps.
- `README.md` — a "Client fixtures" subsection beside the API material and
  a sentence in the `test` row of the CI table.
- `.gitignore` — `node_modules/`, `__pycache__/`, `test-results/`,
  `*.egg-info/`, `.pytest_cache/`.
- `.dockerignore` — `clients/`.
- `docs/decisions/README.md` — the 0110 index row (the inventory test
  requires it; the chief's commit carries only the design files).

## Data model

- `Inbox` — `name: string`.
- `WaitOptions` — `timeoutMs?`, `to?`, `from?`, `subject?`, `sinceMs?`
  (Python: `timeout_ms`, `to`, `from_`, `subject`, `since_ms`).
- `CodeMatch` — `code: string`, `email: Email` (the REST object, untouched).
- `LinkMatch` — `link: string`, `email: Email`.
- `SwarmailAmbiguous` — `candidates: string[]`, `inbox`, the filters.
- `SwarmailTimeout` — `inbox`, the filters, `timeoutMs`, `matched` from the
  last list (`total`).
- `SwarmailHttpError` — `status`, `body` for every other refusal.

## Contracts

- REST, unchanged: `await`, `messages`, `seed`, `metrics`, `healthz` as
  served by `src/static/openapi.json` at a3cff51.
- Public fixture surface: exactly the names in spec.md § Contract. Nothing
  else is exported; the seed helper stays inside the example files.
- Version lock: `package.json.version == pyproject.project.version ==
  Cargo.toml package.version`.

## Quickstart (what the examples do)

```bash
cargo build --locked && target/debug/swarmail serve &      # defaults: 127.0.0.1:8025 / :1025
until curl -fsS 127.0.0.1:8025/healthz; do sleep 0.2; done  # CI: under a deadline
npm --prefix clients/playwright install --no-save --no-package-lock --ignore-scripts @playwright/test@<pin>
npx --prefix clients/playwright playwright test -c clients/playwright/examples
pip install --no-deps --only-binary :all: -r clients/pytest/requirements-ci.txt
PYTHONPATH=clients/pytest python -m pytest -p pytest_swarmail.plugin clients/pytest/examples
```

## Risks

1. **The first real execution is CI.** The seats install nothing and this
   box has no npm, pip or pytest. Mitigation: `client.js` and
   `__init__.py` import no framework, so the implementer drives them with
   `node -e` / `python -c` against a locally built binary and records that
   as partial evidence; the result says plainly that the suites ran only
   in CI. A wrong framework pin is a red step, never a silent pass.
2. **Pins cannot be verified offline.** `@playwright/test`'s own chain is
   exact-pinned upstream; pytest's transitive closure (`iniconfig`,
   `packaging`, `pluggy`, `pygments` on Python ≥ 3.11) is listed by hand.
   Mitigation: `--no-deps --only-binary :all:` fails closed on a missing or
   source-only pin; the implementer names the newest versions it can state
   with confidence and CI proves them.
3. **The selection rule lives in three places** (server, TS, Python).
   Mitigation: the version lock ties the clients to the server release;
   the same six examples pin both clients; 0110 records that a change to
   0094 §5/§6 changes the clients in the same release.
4. **`index.d.ts` drifts from `client.js`.** Playwright strips types
   without checking. Mitigation: the surface is about fifteen lines and
   the TS example exercises every exported name at runtime; reported as not
   measured.
5. **Packaging is unproved.** pytest runs with `PYTHONPATH` and
   `-p pytest_swarmail.plugin`, so the `pytest11` entry point is never
   exercised; Playwright resolves the package through relative imports in
   the example config. Mitigation: 0110 §5 owes that proof to the
   publishing change, with licence files and registry-name checks;
   issue #162 stays open after this run.
6. **npm/pip in the rust-cache job.** Mitigation: `--ignore-scripts` and
   `--only-binary :all:` close the install-time path; nothing installed
   reaches the cached directories; the token is read-only on pull requests.
   If review refuses the placement, the fallback is one new job without
   rust-cache plus one README CI row, and nothing else in the design moves.
7. **Example 5's gauge gate.** `/metrics` must show
   `swarmail_awaits_active{inbox="<name>"} 1` before the seed; a
   mis-rendered inbox label would make the gate spin to the deadline.
   Mitigation: the gate itself has a deadline and names the gauge on
   failure; the Rust suites already pin the row's rendering (0065).
8. **More than 250 matching mails in one test.** The client raises instead
   of answering. Accepted: a per-test inbox that large is a test bug, and it
   fails loudly.
9. **Coverage not measured for TS and Python.** The Rust gate is unaffected
   because no production line changes; the result reports the gap as not
   measured, never as a pass.
