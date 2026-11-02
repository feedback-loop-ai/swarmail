# Plan: implicit TLS (SMTPS) on the SMTP listener

Spec: [spec.md](spec.md). Decision: [0116](../../docs/decisions/0116-implicit-tls-smtp.md)
(proposed). This is a fresh design. No review has returned it.

## Technical context

- **Language and runtime:** a single Rust crate on tokio. TLS is rustls
  with the ring provider through `tokio-rustls`. PEM loading uses
  `rustls-pemfile`, and tests get a per-run certificate from `rcgen`
  (`TlsConfig::generate_self_signed`). Every one of these is already in
  `Cargo.lock`, so no package is added.
- **Where TLS lives today:**
  - `src/lib.rs::run_on` parses the pair into
    `Option<Arc<rustls::ServerConfig>>` before anything binds.
  - `src/smtp.rs::serve(listener, store, chaos, cfg, tls, degraded,
    shutdown)` admits connections and refuses over the cap with the
    plaintext 421 (0027 §3). It spawns `handle_conn(stream, store, chaos,
    cfg, tls)`.
  - `handle_conn` runs the chaos `Connect` check on the plain socket, then
    `session(.., TlsOffer::None | Offered(acceptor))`.
  - The STARTTLS arm of `session` runs
    `tokio::time::timeout(cfg.tls_handshake_timeout, acc.accept(raw))`.
- **Config:** `ServeArgs` (clap, one struct literal in
  `tests/config_cli.rs`) maps into `Config` via `From<&ServeArgs>`.
  `SmtpConfig` implements `Default`, and every literal of it ends in
  `..Default::default()`, so a new field there touches no literal.

## Constitution check (house rules)

| Rule | How this plan meets it |
|---|---|
| Frozen 0001–0003 | No change to `Store`, the park, `insert_async`, the `250` placement, `await` or pruning. The transport changes only before `session()` starts. AC1, AC4 and the existing suites prove it. |
| 0004 / 0112 / 0005 | Session code is reused verbatim. AC1, AC2 and AC3 prove it over implicit TLS. |
| 0027 §3 cap refusal | `serve()`'s cap branch is left byte-identical. AC8 pins the exact bytes. |
| 0045 §8, code under a proposed decision | 0116 amends nothing. Every accepted ruling holds, so the code may land while 0116 is proposed. |
| Tests real-protocol, no mocks (0004/0020 rule 3) | The new tests live in `tests/starttls.rs` against `run_on` servers with a real rustls client. The serve-level refusal is an inline unit test (0020 rule 1). |
| 0020 rule 5 | Chaos at probability 100. A per-run certificate. No sleep used as sync: real deadlines, wire reads and the `await` long-poll do the synchronizing. |
| Exact coverage, no `coverage(off)`, no new harness file | Every new line is listed under "Coverage map" below with the test that executes it. |
| Dependencies / `deny.toml` | None added. |
| Operator-held set (0056 addendum 2026-11-01) | No `.github/`, `scripts/`, `.brokkr/`, toolchain, `Cargo.lock` or `deny.toml` change. No decision is marked accepted. |
| Docs | Guide section, help text, doc comments and the decision index row. |

## Implementation approach

### 1. Config (`src/config.rs`, `src/smtp.rs`)

- `ServeArgs::smtp_implicit_tls: bool`:
  `#[arg(long, env = "SWARMAIL_SMTP_IMPLICIT_TLS", value_parser = clap::builder::BoolishValueParser::new(), ...)]`
  with `action = ArgAction::SetTrue`. The implementer pins clap's real env
  behaviour for a SetTrue flag with AC10's four cases. If clap's default
  `FalseyValueParser` already gives `true`→on and `false`/`0`→off, keep the
  default and drop the explicit parser. The test decides; the attribute
  follows it.
- Help text: "Speak TLS from the first byte on the SMTP listener (implicit
  TLS, RFC 8314). Requires --tls-cert/--tls-key. STARTTLS is not offered."
- `SmtpConfig::implicit_tls: bool`, default `false`, with a doc comment
  citing 0116. It is mapped in `From<&ServeArgs>` beside `route_domain`.
- B8 wording fixes on `--tls-cert`, `--tls-handshake-timeout-ms`,
  `SmtpConfig::tls_handshake_timeout` and the `src/smtp.rs` module doc.

### 2. One owner for the pair requirement (`src/smtp.rs`, `src/lib.rs`)

- `pub(crate) const IMPLICIT_TLS_NEEDS_PAIR: &str = "--smtp-implicit-tls requires --tls-cert and --tls-key";`
  and `pub(crate) fn require_tls_for_implicit(cfg: &SmtpConfig, tls_configured: bool) -> io::Result<()>`.
  The function returns `Err(io::Error::new(InvalidInput, IMPLICIT_TLS_NEEDS_PAIR))`
  when `cfg.implicit_tls && !tls_configured`.
- `run_on` calls it right after the TLS `match`, before `listen(...)`, so
  nothing binds.
- `serve` calls it as its first statement, so any in-process caller fails
  closed.
- One branch is executed by two tests: AC9's `run_on` test and its inline
  `serve` test.

### 3. The handshake and the starting state (`src/smtp.rs`)

- Lift the STARTTLS deadline block into
  `async fn tls_handshake(acc: &TlsAcceptor, raw: TcpStream, deadline: Duration) -> io::Result<server::TlsStream<TcpStream>>`.
  Its three exits are success, a rustls error (`?`), and the deadline
  (`TimedOut`, "TLS handshake deadline elapsed"). The STARTTLS arm calls
  it, with behaviour unchanged. No test pins the old message text; the
  implementer re-greps to confirm.
- `handle_conn` decides the starting transport once:
  ```rust
  let (conn, offer) = match tls {
      Some(c) if cfg.implicit_tls => {
          let acc = TlsAcceptor::from(c);
          let s = tls_handshake(&acc, stream, cfg.tls_handshake_timeout).await?;
          (Conn::Tls(Box::new(s)), TlsOffer::Active)
      }
      Some(c) => (Conn::Plain(Some(stream)), TlsOffer::Offered(TlsAcceptor::from(c))),
      None => (Conn::Plain(Some(stream)), TlsOffer::None),
  };
  // then the existing chaos Connect check on `conn`, then BufReader + session + close
  ```
  This moves the chaos check after the transport is chosen, so in
  implicit mode it writes through TLS (B5). In the other modes the chaos
  check still runs before any byte, on the plain socket, so their bytes
  are unchanged. The `None`-with-implicit combination never reaches here,
  because `serve` refuses it (§2). The match therefore has no dead arm and
  no extra branch.
- The implicit stream is **not** routed through `take_plain` or
  `Conn::Plain(None)`. Those exist only for upgrading a live STARTTLS
  session; this fixes robustness's "starts reading EOF" hazard.
- A handshake error returns `Err` from `handle_conn` before any write.
  The `serve` task logs it at debug and drops the socket, and the
  listener is unaffected.
- `serve()`'s cap branch is **not** touched (0027 §3, B6).

### 4. Tests (`tests/starttls.rs`, `tests/common/mod.rs`, `tests/config_cli.rs`, inline in `src/smtp.rs` and `tests/binary.rs`)

- Add helpers to `tests/common/mod.rs`:
  - `implicit_tls(addr, server_name, root) -> TlsSmtp`, which connects TCP,
    immediately runs `TlsConnector::connect` and reads the `220` over TLS.
    It reuses `tls_connector(root)` and the existing TLS session reader
    type.
  - `implicit_tls_handshake_fails(addr, root)`, which asserts the client
    handshake errors within a bound.
- Add `start_implicit(name, SmtpConfig)` to `tests/starttls.rs`, wrapping
  `start_tls_cfg` with `implicit_tls: true`.
- Tests, one per acceptance criterion: AC1–AC8 in `tests/starttls.rs`;
  AC9's `run_on` case in `tests/starttls.rs` (beside
  `a_broken_tls_pair_refuses_to_serve`), its binary case in
  `tests/binary.rs` and its `serve` case inline in `src/smtp.rs`; AC10 in
  `tests/config_cli.rs`, where the ServeArgs literal gains
  `smtp_implicit_tls: false`.
- Kratos' `skip_ssl_verify=true` relaxes only the client. The tests use a
  verifying client that trusts the per-run root, which is the stricter
  proof of the server side. No permissive verifier is written.

### 5. Docs

- `docs/guide/smtp.md`: the "Implicit TLS (SMTPS)" subsection after the
  STARTTLS paragraph (B9). Amend the STARTTLS paragraph's `0` caveat so it
  also says that with implicit TLS every connection fails before the
  greeting.
- `docs/decisions/README.md`: the 0116 row, status `proposed`, with a note
  that the number is provisional (recheck against #148/#314 at
  integration).
- No release-notes file. The next release cut writes it.
- No chart change. `charts/swarmail/values.yaml`'s comment "TLS for the
  SMTP listener (STARTTLS; …)" stays true for the chart's own surface. The
  guide shows the `config:` env recipe.

### Coverage map (every new production line, and its test)

| Line or branch | Executed by |
|---|---|
| `require_tls_for_implicit` Err arm | AC9 `run_on` test; inline `serve` test |
| `require_tls_for_implicit` Ok arm | every server start |
| `tls_handshake` success | AC1 (implicit); existing STARTTLS upgrade tests |
| `tls_handshake` rustls error | AC6 (implicit); existing cert-refusal test (STARTTLS) |
| `tls_handshake` deadline | AC5 (implicit); existing stalled-STARTTLS test |
| `handle_conn` implicit arm | AC1–AC8 |
| `handle_conn` Offered / None arms | existing suites |
| chaos Connect in implicit mode | AC7 |
| `From<&ServeArgs>` new field | AC10 |

## Project structure (files touched)

```
src/config.rs                 ServeArgs flag, From mapping, help text (B1, B8)
src/smtp.rs                   SmtpConfig field + doc, require_tls_for_implicit,
                              tls_handshake, handle_conn starting state, module doc,
                              inline serve refusal test
src/lib.rs                    run_on calls require_tls_for_implicit before binding
tests/common/mod.rs           implicit-TLS client helpers
tests/starttls.rs             AC1–AC8, AC9 (run_on)
tests/binary.rs               AC9 (binary exit + message)
tests/config_cli.rs           AC10; ServeArgs literal gains the field
docs/guide/smtp.md            Implicit TLS (SMTPS) section (B9)
docs/decisions/README.md      0116 index row
docs/decisions/0116-implicit-tls-smtp.md   (committed by design)
```

## Research notes

- RFC 8314 §3.3: on implicit TLS, the TLS handshake starts immediately
  after connect, and the SMTP greeting follows inside TLS. RFC 3207 §4.2
  forbids advertising STARTTLS on an established TLS session, which the
  existing `TlsOffer::Active` arm already implements.
- On a non-TLS first record, rustls fails the handshake and may queue a
  fatal alert, which tokio-rustls tries to flush as a last write. AC6
  therefore accepts "empty or a TLS alert record (0x15)" and forbids any
  SMTP reply. It does not insist on zero bytes, which would fail for
  reasons that have nothing to do with swarmail.

## Data model and contracts

- New public field `SmtpConfig::implicit_tls`. This is additive, and
  `Default` keeps every existing literal compiling.
- New CLI flag and env.
- No HTTP, MCP, POP3, store or persistence contract changes.

## Quickstart

```
$ swarmail gen-cert --domain localhost --out ./certs
$ swarmail serve --smtp-implicit-tls --tls-cert ./certs/cert.pem --tls-key ./certs/key.pem
# Kratos: smtps://user:pass@<host>:1025/?skip_ssl_verify=true&legacy_ssl=true
```

## Risks

| Risk | Mitigation |
|---|---|
| **The cap refusal writes plaintext onto a TLS port** (robustness's objection). | Kept on purpose: accepted 0027 §3 rules the refusal's wire bytes, and changing them needs an amendment (0045 §8). The bytes are a fixed public string, and the client fails its handshake and retries. AC8 pins the behaviour, and the guide and 0116 §5 document it. The silent-close variant is left to the operator. |
| **Fail-open: the mode is on but plaintext is served** (an embedder builds `Config` without a pair). | One shared check, called by both `run_on` (before binding) and `serve` (first statement). AC9 proves both paths and the binary. |
| **Chaos error leaked in cleartext before the handshake.** | The check moves after transport selection. AC7 has a raw TCP reader assert the error text never appears in cleartext. |
| **STARTTLS advertised inside TLS** (the session was seeded `Offered`). | The implicit arm seeds `TlsOffer::Active`. AC2 compares exact EHLO lines and the 554. |
| **The implicit stream starts reading EOF** (reusing `take_plain`). | The implicit arm builds `Conn::Tls` directly from the accepted socket and never touches `take_plain`. AC1 reads the greeting. |
| **Stalled or trickling handshakes pin slots.** | The handshake runs under `tls_handshake_timeout` inside the capped session task, never in the accept loop. AC5 proves the drop and that serving continues. The residual (one slot per deadline) is the same as STARTTLS today and is documented. |
| **A plaintext-refusal test that is flaky because of rustls's alert behaviour.** | AC6 accepts empty-or-alert and forbids only an SMTP reply. The long deadline proves the close came from rustls, not the timer. |
| **Wall-clock flakiness in AC5/AC8.** | Generous bounds, matching the existing stalled-STARTTLS test (deadline/5 probe, 5× deadline close). Synchronization comes from wire reads and real deadlines, never sleeps (0020 rule 5). The certificate is generated per run, with no validity-window assertion. |
| **The clap bool+env parse surprises** (`SWARMAIL_SMTP_IMPLICIT_TLS=false` read as on). | AC10 pins all four cases. The attribute follows the test (§1). |
| **Decision number collision** (#148's branch holds a `0116-*`; #314 may land first). | 0116 is recorded as provisional. Whichever lands second renumbers at integration, updating the spec link, the index row and the `Decision:` line. |
| **One mode per process** surprises an operator who needs both. | Documented in the guide and 0116's Consequences. A future additive listener stays possible. |
| **Coverage gate: a new line left unexecuted.** | The coverage map above. The implementer runs `scripts/coverage-gate.sh` and its selftest before reporting. No `coverage(off)` and no harness file. |
