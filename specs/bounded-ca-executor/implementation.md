# Bounded CA implementation proof

The selected 0078 policy is implemented without a dependency or a Store/SMTP
production change. The proposal remains proposed; accepted decision bodies and
bundle routing are unchanged. Two native workers and one process owner enforce
J = W = 2 and B = 67,108,864 logical charged bytes. There is one reserved handoff
per worker and no pending queue. GET sizes borrowed state before cloning, PUT
moves its extracted buffers, and delivery misses reserve before copying a key.
One envelope/reply owns the reservation until data transfer or disposal.

Server scope closure serializes against PUT/cache publication and the first
poll authorizing a delivery send. Stop closes the scope before its existing
stop → serve join → drained → dispatcher join sequence. Native workers have no
Tokio Handle and no server join/drain. Startup error, server drop, stopped scopes,
independent live scopes, runtime replacement, partial startup failure, ordinary
job panic and unreceived replies have direct proofs. Abandoned finite builds
retain their charge and can only advance successful actual-build counters.

Proof owners:

- Inline `ca_executor::tests`: exact tariff, wide-integer arithmetic oracle,
  overflow/boundaries/spare capacity, canonical PEM envelope, native construction,
  sticky partial-initialization failure and close/publication serialization.
- Declared `ca_executor/executor_tests.rs`: controlled pre-entry cancellation,
  closed handoff, held reservations, aggregate byte bounds, completed-unreceived
  replies and disposal/refund. Deadline failures stay in the cold harness (0020).
- `ca_construction_tests`: actual GET saturation/cancellation with a single Tokio
  blocking slot, persistent DATA 250, exact raw mail retrieval before release,
  unused watchdog and reopen; mixed GET/PUT/delivery admission and config churn;
  refusal-before-clone; native 503/extractor precedence and oversized state;
  four-attempt pressure exhaustion/recovery; late PUT/cache/send refusal;
  cancellation/build counters, winner/LRU behavior and held-builder stop accounting.
- `lifecycle_tests::ca_server_drop_and_startup_failure_close_only_their_scopes`:
  identity-keyed startup observation, actual startup failure and two live servers.
- `tests/webhook.rs`: privacy, complete-root/later-root verified TLS and >32-root
  fan-out. Fixtures expecting uncontended CA admission share a scoped test guard;
  other protocol suites remain concurrent. The >32-root fixture establishes each
  cold root through SMTP/TLS before the concurrent fan-out. Arbitrary cold bursts
  cannot demand success under no-queue/no-fairness admission; explicit pressure
  tests prove that loss policy instead. No test retries an admission to hide it.
- `tests/api.rs`: served GET/PUT 503 and existing GET 500, with three independent
  omission controls. README, served llms and OpenAPI name resource/refusal costs.

A source-only negative control restores slot zero's admitted CA work to Tokio's
shared blocking pool, keeping the second native slot so both actual GETs enter.
With one shared blocking slot the persistent SMTP isolation proof fails at its
three-second pre-release DATA deadline (exit 101); restoration passes (exit 0).
No watchdog release creates success. The executor source digest before/after
restoration is identical. Sanitized logs and hashes are in the ignored
`.forge/epic192/issue220/implement-gates/isolation-control.json` and paired logs.
There is no production switch or leftover control diff.

The first exact gate reported zero missed lines/branches/functions (14652/820/1593).
Final command statuses bind to the committed candidate in the ignored
`.forge/epic192/issue220/implement-gates/receipt.json`; final coverage has its own
fresh `target/coverage/run.*/` report. Checkboxes are implementation tracking;
only executed zero-exit receipts support the phase result. The final pass also
requires the coverage selftest, publish/seal proofs, permissive license check,
pinned engine/hash/shims, design/delivery selftests and fail-closed native-tls
absence. No old receipt supplies a current pass.

CI retains macOS, Helm/chart-history, Docker, network RustSec and privileged
port-80 evidence. The operator retains live settings and sealing. This seat
accepts no decision, clears no protected review, and pushes/publishes nothing.
