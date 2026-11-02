# Benchmarks

[← README](../../README.md) · [Guide index](README.md)

| Measurement | Result | How |
|---|---|---|
| Parse + extract | **1.28 µs/mail** (~780k/s/core) | `cargo bench` · criterion median |
| + store insert | **1.58 µs/mail** (~630k/s/core) | `cargo bench` · criterion median |
| + store insert at the default cap, steady state | **~2.1 µs/mail** (~470k/s/core) — memory-only, no `--data-file` | `cargo bench` · criterion median (`ingest/store/insert@cap-steady`) |
| End-to-end SMTP ingest | **5000/5000 exact**, zero loss | `cargo test --release --test rate -- --ignored --nocapture` (100 concurrent fresh sessions; session-latency-bound — scales with cores) |

Reproduce with `cargo bench` (results land in `target/criterion`). The
cap-steady row is the shipped default configuration
(`SWARMAIL_MAX_PER_INBOX` = 100 000): a store prefilled to exactly the cap,
every measured insert evicting exactly one oldest mail — the shape the
accept path runs in production. The uncapped rows above describe an
unbounded store, and before the deque eviction landed, the same cap-steady
bench measured ~13.7 µs/mail at this cap: the front-drain shift, not the
parse, was the cost. The capped query paths publish in the same run
(`ingest/threads/view@cap`, `ingest/store/count@cap`,
`ingest/store/count@cap-filtered`).

These are in-process ingest microbenchmarks. They are not network
throughput, wait latency or agent results, and they compare nothing.
The fair comparison against Mailpit and MailHog has its own method and
driver, and so far no results: see [docs/evals](../evals/README.md)
for what is built, what is still missing and why. This README and this guide state no
comparative or agent numbers until admitted evidence exists to cite.
