# Testing & coverage

[← README](../../README.md) · [Guide index](README.md)

**Every integration test runs against real servers speaking the real
protocol** — no mocks in the loop. The bar for a human contribution is the
bar the machine is already held to. **The gate list is
[`docs/house-rules.md` § Gates](../house-rules.md#gates--part-of-every-change)** — the one
inventory, `--locked` builds, the coverage selftest, the publish-guard proof
and the CI-only proofs included; this guide keeps no copy of it. CI
([`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)) runs it as these
jobs:

| CI job | What it runs |
|---|---|
| `quality` | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, the publish guard's proof, the packaged-licenses and WHATWG attribution proof, the shim digests, the design check's selftest and the seal tool's keyless proof |
| `test` (ubuntu + macOS) | `cargo test --workspace --locked`, then from `evals/` its fmt, clippy and `cargo test --locked` against the candidate's own `swarmail` binary, then the ignored port-80 bridge proof; on Linux also the Playwright and pytest fixtures' example suites against the built binary |
| `coverage` | the coverage gate's selftest, the exact gate itself and the delivery-proof test; uploads `target/coverage/` |
| `deny` | `cargo deny check licenses` and the fail-closed `native-tls` absence proof |
| `dependency-audit` | the RustSec advisory audit (network-backed, CI-only) |
| `brokkr-compile` | applies the checksummed engine patches and proves review aggregation from source, admits the published brokkr by its recorded digest and version, and compiles every recipe directory under `.brokkr/recipes/` by glob (today `self`, `codex`, `codex-careful`, `codex-fast`, `mixed-opus-terra`, `claude-careful` and `eval`), refusing an empty shelf, a directory without a `bundle.json` and a policy-sweep warning |
| `chart-verify` | `scripts/chart-verify.sh`: its selftest, the chart-version bump check against history, and lint + both profiles + contract assertions |
| `docker-smoke` | real Docker: the built image as its own uid keeps accepted mail across a graceful stop and a hard kill on a named volume, `--rm` leaks no anonymous volume, and no `target/` reaches the build context |
| `release-rate-5k` (tags only) | `cargo test --release --locked --test rate -- --ignored --nocapture`, the 5000/5000 proof |
| `docker`, `chart-publish` | publication only, after the jobs above succeed (main and `v*` tags for the image, `v*` tags for the chart) |

The coverage gate is not a percentage to trend upwards. It demands literal
integer equality — `covered == count` — over **lines, branches and
functions**: exactly 100/100/100, every missed count zero, on every run
(the counts themselves grow with the code, so none is quoted here). It
refuses `#[coverage(off)]` outright so production code cannot shrink its
own denominator, and writes the full lcov report and summary under
`target/coverage/` — kept when red, so the burn-down starts from evidence,
not from a number. There is no threshold to lower.

```bash
cargo test                                                    # the suite (seconds)
cargo test --release --test rate -- --ignored --nocapture    # the 5k guarantee
cargo bench                                                   # ingest path → target/criterion
bash scripts/coverage-gate.sh                                 # the floor gate
```

| Suite | What it proves |
|---|---|
| `tests/mcp_revisions.rs` | revision negotiation, headers, annotations, batch refusals, binary lifecycle/concurrency and immediate SMTP visibility |
| `tests/p2.rs` | the P2 end-to-end: MCP JSON-RPC, the seed API's parse+extraction pipeline, webhook delivery with secret and retry targets |
| `tests/smtp_edges.rs` | every protocol verb, refusal, AUTH form, DATA limit, dot-unstuffing, group addresses, chaos on connect/MAIL FROM/RCPT/DATA; the 0034 boundaries — mixed terminators, path and parameter matrices, greeting resets, validation before chaos, strict PLAIN and one AUTH per session |
| `tests/rate.rs` | the 5k losslessness guarantee + burst |
| `tests/burst.rs` | 1000 mails / 50 conns exact-count, per-inbox isolation, cap eviction, chaos rejection + recovery |
| `tests/bulk_sender.rs` | Source-citing bulk rules, original header grammar, genuine text bodies, untrusted auth unknowns and inclusive analysis limits (proposed 0089) |
| `tests/deliverability.rs` | Lazy typed report, REST/MCP parity, threshold assertions/waits, UI states, persistence, pruning and served contract documentation (proposed 0085) |
| `tests/link_reachability.rs` | The opt-in link reachability probe (proposed 0121): off and lazy by default, outcomes and the address policy against loopback listeners only, and the request, target and permit bounds |
| `tests/spam_rules.rs` | The six built-in spam signals over real SMTP, REST and MCP: a failing and a passing message per rule, the `swarmail-html/5` identity, no weight or total, determinism, laziness and the fail-closed 4 MiB text bound (proposed 0122) |
| `tests/api.rs` | every REST route, happy + error branches |
| `tests/config_cli.rs` | the CLI/env config surface, the independent SMTP/POP3 idle defaults, the rendered `serve --help` |
| `tests/listen.rs` | every listener's bind: both address families, the bind error reported as-is, and (Linux) a burst of 600 fresh SMTP connections all greeted, past the platform default backlog of 128 |
| `tests/listener_contract_docs.rs` | the listener contract as documented: README, the controlling 0027 clarification and 0036 state the per-I/O bounds, the unbounded complete-line trickle residual and its costs, and the conditional `stopping` answer |
| `tests/mcp.rs` | every tool, JSON-RPC protocol errors, `isError` results |
| `tests/agent_setup_docs.rs` | the agent setup guide and skill exercised: every fence of the guide is run, content-checked (the AGENTS.md block) or marked not-executed, each JSON/TOML/YAML wiring written verbatim into a fresh project and driven through `initialize`, `tools/list` and a seed → wait → extract round trip on its declared transport (transport only), and the taught loop and inbox route driven over real SMTP with the AUTH name the block's `smtp://` URL carries, including the already-mailed case (no clear), the wrong-route controls, the route-domain clause and the timed-out answer's `near_misses`; the tool table equal to the live `tools/list`, the AGENTS.md block's loop steps and failure classes checked against the eval taxonomy, and the 0001/0002/0005 sentences pinned |
| `tests/focused_waits.rs` | the focused code/link waits over real SMTP and MCP: each filter on each tool by counterexample, ingest occurrence metadata for SMTP and seed, post-park deliveries that never complete a wait, and local applications refusing stale codes and look-alike links (proposed 0094) |
| `tests/inbox_routing.rs` | recipient inbox routing and `swarmail_create_inbox` over real SMTP and MCP: three concurrent agents behind one AUTH login never see each other's mail, the rule's case-fold, whole-label, 550 and 452 shapes and the RSET and chaos clears, routing ahead of the park (a wait on a never-mailed minted inbox; the in-flight row), the tool's modern projection and distinct concurrent mints, and the unconfigured server unchanged (proposed 0112) |
| `tests/mailslurper.rs` | the MailSlurper service-API shim on its opt-in listener over real SMTP and raw HTTP/1.1 (proposed 0118): the consumer's recorded GET and DELETE with their exact headers, off by default and absent from the HTTP plane, the loopback and off-loopback guard, each item key pinned byte for byte, paging at 50 with the empty-page omission, closed queries, every destructive refusal with a before/after count over both inboxes, the 503 on a failed commit, listing at once after each `250`, a data-file restart, and the guide's contract |
| `tests/ui.rs` | every page, escaped (injection-proof) rendering |
| `tests/webhook.rs` | delivery with secret, pathless target, the retry budget, https over a real TLS server, the CA client cache bound |
| `tests/stdio.rs` | the stdio JSON-RPC bridge over in-process duplex pipes |
| `tests/starttls.rs` | STARTTLS: byte-identical plaintext listener, real rustls upgrade, RFC 3207 restart (identity and envelope reset), refused certs, stalled handshakes, malformed PEM refusal, gen-cert file handling |
| `tests/lifecycle.rs` | graceful stop: serve futures return, ports release, open feeds and stalled webhook deliveries cannot hold stop hostage, interrupted SMTP/POP3 transactions apply no partial changes, parked REST/MCP awaits answer shutdown inside the stop budget, three live seed requests observed under a held SQLite write lock refuse with 503 and store nothing, and fan-out in flight at drain is counted |
| `src/lifecycle_tests.rs` | `src/lifecycle_tests.rs::stalled_body_selects_drain_deadline` retains an unfinished real HTTP body and asserts the selected drain-deadline arm emitted its receipt; `src/lifecycle_tests.rs::graceful_http_completion_emits_no_deadline` is the negative control requiring no deadline receipt |
| `src/api.rs` | `tests::a_healthz_inside_its_handler_at_the_stop_is_answered_503_stopping` proves the handler-entered `503 stopping` response; the drain deadline ends the serve wait without promising every detached socket is closed |
| `src/store/persist_tests.rs` | `an_in_flight_commit_neither_holds_stop_nor_half_lands` proves the admitted-commit stop obligation beside the gate that holds it |
| `tests/binary.rs` | the shipped binary: SIGINT and SIGTERM → clean exit, SIGINT with seeds queued on a held write lock exits inside the budget and stores nothing, a second signal during the drain exits 70, a mail committed before SIGTERM is queryable after restart, mcp EOF → 0 |
| `tests/pop3.rs` | the POP3 surface: every verb happy + refused, multi-line replies, deletion-on-QUIT, its own idle bound wired independently of SMTP's |
| `tests/compat.rs` | the shims: Mailpit + MailHog envelopes, searches, deletes, 404/400 branches |
| `tests/persistence.rs` | restart restores exact state; a racing clear never loses 250-answered mail |
| `tests/sqlite_locks.rs` | a refused second server reaching the data file through an alias leaves the owner's SQLite locks intact — observed from a separate raw-SQLite child process (0047 §1) |
| `tests/store_contract_docs.rs` | the store, maildrop and compat contracts as documented — README, llms.txt, the chart comment and the served schema, their numbers built from the running constants — and every decision index row agreeing with its record's status |
| `tests/delivery_constitution.rs` | structural pins on `.brokkr/recipes/self/` (0050): the review-residual stops, the design sequence ending in the pinned check, the verify seat's pinned gates and shim check; and the documented install examples and CI's chart-history and docker-smoke wiring (#141) |
| `tests/delivery_recipes.rs` | Codex recipe discovery, triage routing and single/double/full review, inherited shell gates, work-seat tools, exact routes and independent review without Pro or author-model fallback (0057); [recipe tables and launch commands](../../.brokkr/recipes/README.md) |
| `tests/mail_compare.rs` | the fair wire comparison driver (proposed 0091) against the real binary it launches: method and source-pin refusals, hand-computed quantiles, exact throughput and present/future/empty wait reconciliation, a bounded 1,000-session run, closed-environment, private-output and reaped-group facts read from `/proc`, admission refusals, and every `scripts/compare-mail.sh` mode refusing without launching an agent |
| `tests/tooling_patches.rs` | the actual tooling-patch helper against real Git repositories: source identity, clean state, strict records, safe paths, all checksums before mutation and ordered patch application |
| unit | extraction, filters, chaos gating, model plumbing, POP3 line protocol, compat shapes |

For triage-first bug delivery, use the [Codex recipe guide](../../.brokkr/recipes/README.md).
Normal `brokkr tui` reads the canonical `.brokkr/state/forge.db`; isolated deliveries
name that shared journal explicitly. Each lane needs its own Git store,
branch, results, build and coverage outputs, and private cache overlays.
The prepared eight-lane layout reserves 16 CPUs per lane; throttle heavy
gates when host capacity requires it. Reserve overlapping files before launch.

Keep a live lane's committed recipe baseline immutable. A candidate gate repair
uses separate writable candidate scripts and must execute the repaired proof;
editing the pinned baseline invalidates the run. Compilation, local dispatch
fixtures and provider smoke prove different boundaries. Delivery still needs
committed-candidate hermetic verification, protected independent review and
the genuine ledger close-out. PR branches are not sealed (decision 0056,
2026-11-01 addendum): the operator's assistant squash-merges a delivery PR once
every house gate and required CI check is green, unless it is held for the
operator; agent runs never push or merge.
