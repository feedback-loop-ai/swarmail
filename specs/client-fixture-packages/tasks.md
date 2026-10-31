# Ordered tasks — Playwright and pytest fixtures

Implementation and proof obligations, not completed work. Check a task only
after its paired proof is recorded against the actual candidate. The chief
commits the design and proposed 0110; the deterministic check owns the design
verdict. Dependencies follow this order.

- [ ] **T1 — Index 0110 and reserve the layout (AC8, AC9).** Add the 0110
  row to `docs/decisions/README.md` (status proposed, awaiting the
  operator); add `clients/` to `.dockerignore` and the client build
  artefacts to `.gitignore`.
  **Proof:** `cargo test --locked --test store_contract_docs` green,
  including `every_decision_file_has_exactly_one_resolving_index_row`;
  `git status` shows no artefact after a local client run.
- [ ] **T2 — Playwright client (AC2, AC4, AC7).** Write
  `clients/playwright/client.js`: `newInboxName`, the wait loop of spec.md
  § Waiting with `limit=0`, the 60 000 ms slice, `count = total + 1`, the
  incomplete-snapshot refusal, socket timeouts; `pickCode` (newest mail
  with codes, distinct, one/several/none) and `pickLink` (all links,
  predicate, distinct); `SwarmailAmbiguous`, `SwarmailTimeout`,
  `SwarmailHttpError`. No framework import, no regex over bodies.
  **Proof:** a `node -e` drive against a locally built `swarmail serve`:
  one code returned, two codes in one mail → ambiguous, two matching links
  → ambiguous, timeout → `SwarmailTimeout` with 408 observed; recorded as
  partial evidence (not the example suite).
- [ ] **T3 — Playwright fixture, types and package (AC2, AC6).**
  `index.js` (`test.extend({ inbox })`, a fresh name per test), `index.d.ts`,
  `package.json` (version = `Cargo.toml`, `peerDependencies`, exact
  `devDependencies` pin, `exports`, `engines`, `files`, licence), README.
  **Proof:** `node --input-type=module -e 'import("./clients/playwright/client.js")'`
  loads; `package.json` parses and its `version` equals `Cargo.toml`'s
  (T8's test).
- [ ] **T4 — Playwright example suite (AC1–AC5).**
  `examples/playwright.config.ts` (no browser projects) and
  `examples/example.spec.ts` with the six examples, a local REST seed
  helper, and example 5 gating on `swarmail_awaits_active{inbox="…"} 1`
  under a deadline.
  **Proof:** CI-only — the `test (ubuntu-latest)` step log shows six
  passed; locally the file is recorded as unexecuted.
- [ ] **T5 — pytest client (AC3, AC4, AC7).** `pytest_swarmail/__init__.py`
  mirroring T2 with `urllib`, `from_` spelled for the keyword, the same
  errors and picks; no pytest import.
  **Proof:** a `python -c` drive against the local binary with the same
  four scenarios as T2; recorded as partial evidence.
- [ ] **T6 — pytest plugin, package and pins (AC3, AC6, AC9).**
  `pytest_swarmail/plugin.py` (`swarmail_inbox`, function scope),
  `pyproject.toml` (version = `Cargo.toml`, `dependencies = []`, the
  `pytest11` entry point, licence), `requirements-ci.txt` with exact pins
  for pytest and its transitive closure, README.
  **Proof:** `pyproject.toml` parses with `tomllib` and its version equals
  `Cargo.toml`'s (T8's test); `requirements-ci.txt` has only `name==x.y.z`
  lines (T8's test).
- [ ] **T7 — pytest example suite (AC1–AC5).** `examples/test_example.py`
  with the six examples; the first sends through `smtplib` with AUTH
  username = inbox to `SWARMAIL_SMTP_ADDR`; the rest seed over REST;
  example 5 gates on the awaits gauge.
  **Proof:** CI-only — the step log shows six passed; locally
  `python -m py_compile` on the file.
- [ ] **T8 — Version and wiring tests (AC6).** In
  `tests/store_contract_docs.rs`: a test that `Cargo.toml`'s version equals
  `clients/playwright/package.json`'s and `clients/pytest/pyproject.toml`'s,
  that `package.json` declares no `dependencies`, that `pyproject.toml`
  declares `dependencies = []`, and that every `requirements-ci.txt` line is
  an exact pin. In `tests/delivery_constitution.rs`: `ci.yml` carries both
  example-suite run lines, inside the `test` job, guarded by
  `runner.os == 'Linux'`, with `--ignore-scripts` and
  `--no-deps --only-binary :all:` present.
  **Proof:** both tests green under `cargo test --workspace --locked`; each
  goes red when the version or a needle is mutated locally (reverted).
- [ ] **T9 — CI steps (AC1).** Append to the `test` job: build `--locked`,
  start the server, poll `/healthz` under a deadline, install the pinned
  framework, run the suite, stop the server and dump its log on failure —
  one step per suite, Linux only, after the port-80 step.
  **Proof:** T8's needles; the job's `timeout-minutes: 30` is unchanged and
  the steps' expected cost is noted in the step comments.
- [ ] **T10 — Documentation (AC1, AC10).** README "Client fixtures"
  subsection (install, fixture names, options, errors, the `SWARMAIL_URL`
  knob, "published" still pending) and the `test` CI-row sentence; each
  package README.
  **Proof:** `the_readme_names_every_ci_job_and_every_root_suite` and the
  gate-inventory test stay green; the README states that the examples run
  in CI only and that TS/Python coverage is not measured.
- [ ] **T11 — House gates and honest result (AC7, AC8, AC10).** Run every
  gate in `docs/house-rules.md` § Gates `--locked`, plus the evals crate
  (`cargo test --manifest-path evals/Cargo.toml --locked`); confirm
  `git diff a3cff51 -- src/ Cargo.lock deny.toml` is empty.
  **Proof:** gate receipts in the result; the result names the CI-only
  proofs (both suites, packaging) and the unmeasured TS/Python coverage,
  and does not close #162.
