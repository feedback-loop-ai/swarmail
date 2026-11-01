# 0113 — Client fixture packages: versioned with the server, CI-only test pins

- **Status:** proposed
- **Date:** 2026-10-31
- **Scope:** Epic #164, issue #162 (Playwright and pytest fixtures over REST).
- **Specification:** [Client fixture packages](../../specs/client-fixture-packages/spec.md).
- **Plan and proofs:** [plan](../../specs/client-fixture-packages/plan.md),
  [tasks](../../specs/client-fixture-packages/tasks.md).
- **Preserves:** 0001–0005, 0010, 0018, 0030, 0044, 0048, 0049; proposed
  0094 stays MCP-only and unchanged. Amends nothing.
- **Number:** drafted as 0110, the next free after 0109 on `main` a3cff51;
  renumbered 0113 at integration, because #159 holds 0110, #156 0111 and
  #157 0112.

## Context and authority

The house dependency policy (`docs/house-rules.md` § Dependencies) governs
the Cargo tree through `deny.toml`. Issue #162 asks for a Playwright
fixture and a pytest fixture published from this repository, which brings
two package ecosystems the policy does not mention, and the run's charter
says no new dependency lands without an operator ruling. The intake seat
asked the operator three questions: the npm/PyPI dependencies and their
pins, whether new CI jobs become required contexts, and publishing. This
record proposes the answer to the first, scopes the second away and defers
the third. Only the operator accepts it. Compatible work may proceed under
accepted 0045 §8: nothing here contradicts an accepted ruling.

## Proposed ruling

1. **Layout and version lock.** Client packages live under
   `clients/<framework>/`, outside the crate's `include` list. Each
   package's version equals `Cargo.toml`'s `package.version`, proved by a
   test under `cargo test --workspace --locked`. A server release and its
   clients are one release train; a client is never released at a version
   the server does not carry.
2. **Zero runtime dependencies.** A published client depends on its
   language runtime and declares the issue-named framework only as a
   peer (`@playwright/test`) or as the plugin host (`pytest`). It installs
   nothing else on a consumer.
3. **CI-only test-time pins.** Running an example suite in CI installs the
   issue-named framework at an exact version recorded in a committed file
   (`clients/playwright/package.json` devDependencies,
   `clients/pytest/requirements-ci.txt`), with install scripts disabled
   and binary distributions only. The set is that framework and its own
   transitive closure, nothing more. Adding any other package is a
   reviewed widening with a one-sentence reason, as for the Cargo tree.
   `deny.toml` is unaffected because it governs the Cargo tree; the
   frameworks' licences (Apache-2.0, MIT) sit inside the house allowlist.
   No lockfile is required while none can be generated offline; a wrong
   pin is a red CI step, never a silent pass.
4. **Selection is the server's.** The clients read only stored `codes` and
   `links` (0005) and re-state proposed 0094 §5 (newest eligible mail for
   codes) and §6 (whole filtered snapshot for links). A change to those
   semantics in the server changes both clients in the same release. No
   REST route is added; focused waits stay MCP-only under 0094.
5. **Publishing is a separate act.** This record authorizes no publish.
   Publishing to npm or PyPI needs the operator's explicit approval, a
   registry-name check, licence files in each artefact and a proof that
   the packaged form works (`npm pack`, the `pytest11` entry point), none
   of which this change supplies. Issue #162's "published" stays open until
   that change lands.
6. **CI placement.** The example suites run as Linux steps of the
   required `test` job, so they block merges on day one without a branch
   protection edit. Moving them to their own job is an ordinary change
   plus the operator's required-context edit; this record does not ask
   for one.

## Rejected alternatives

- A new REST focused-wait endpoint, or the clients calling `POST /mcp`:
  the issue says REST, and 0094 keeps focused waits MCP-only.
- Client-side regex over bodies: a 0005 violation and a hostile-body bug.
- Session-scoped inboxes, `delete-all` in fixture code, webhook-based
  waits, TLS bypass flags: each breaks an isolation or guarantee the
  server already provides for free.
- A committed lockfile this run: unobtainable offline; owed by the
  publishing change together with its packaging proof.

## Costs and what stays unmeasured

- The selection rule exists in three places; the version lock and paired
  examples hold them together.
- TypeScript and Python coverage are not measured by any gate; the Rust
  gate is unaffected. Results report that as not measured, never as a pass.
- The example suites' first real execution is CI; the verify seat cannot
  run them.
- No paid trials, push, merge, publish, tag, issue closure or decision
  acceptance follow from this record.

## Operator ruling (2026-10-31)

The operator approved, in chat, the CI-only test dependencies this record
pins: `@playwright/test` 1.55.0 for the Playwright fixture, and `pytest`
8.4.2 with `iniconfig` 2.1.0, `packaging` 25.0, `pluggy` 1.6.0 and
`pygments` 2.19.2 for the pytest fixture. They are test-time only and live
under `clients/`; the swarmail crate gains no dependency. This approves those
dependencies; the record's status is unchanged.
