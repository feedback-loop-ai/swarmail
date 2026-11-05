# Plan: offline DKIM verification (#254)

Spec: [spec.md](spec.md) · Tasks: [tasks.md](tasks.md) · Decision:
[0125](../../docs/decisions/0125-offline-dkim-verification.md) (proposed).

## Technical context

- **Language and crate.** Rust 2024, a single crate. The evaluator lives in
  `src/deliverability.rs`, beside the `html`, `links`, `spam`, `reach` and
  `dns` collectors.
- **Evaluator.** `evaluate(raw, &Inputs)` runs on the blocking pool under one
  of four permits, and its reports are cached in 128 Weak-identity slots
  (0085).
- **DNS.** `Lookups` (0120 §3) is the synchronous, recording TXT resolver
  over the frozen `Arc<Effective>`. Outside tests it is unused today, which
  is why `collect` still carries an
  `expect(unused_variables, reason = "#254/#255 verifiers resolve through it")`.
  `Txt` and `resolve_txt` carry matching `expect(dead_code)` attributes.
  This story removes all three.
- **The new dependency.** `mail-auth 0.14.0`, checked in the offline
  registry at `~/.cargo/registry/src/index.crates.io-*/mail-auth-0.14.0`.
  - Public API used: `AuthenticatedMessage::parse_with_opts`,
    `MessageAuthenticator::new(ResolverConfig, ResolverOpts)`,
    `verify_dkim(Parameters::new(..).with_cache(..))`, `DnsCache`,
    `ResolverCache`, `TxtRecord`, `DnsError`, `DkimOutput::{result, signature, is_atps}`,
    `DkimResult` and `dkim::{Signature, DkimError, DomainKey, Canonicalization}`.
  - Used by tests only: `DkimSigner`, `crypto::{SigningKey, RsaKey, Ed25519Key}`
    and `AuthenticatedMessage::canonicalized_dkim_headers`.
  - Crate behaviour the design depends on:
    - `verify_dkim` reads `SystemTime::now()` for `x=`;
    - the header's `l` and `x` fields use 0 to mean "absent";
    - the crate does not check `From` in `h=`;
    - the crate hashes `min(l, body)`;
    - outputs list `DkimError`-rejected fields first;
    - the crate asks for ATPS and `_report` names through the same cache.
- **Constraints.** 100% line, branch and function coverage, exact; no mocks
  in the wire tests (0004); the sleep and wall-clock disciplines of 0020 §5.

## Constitution check (house rules)

| rule | how this plan meets it |
|---|---|
| 0001 accept == stored | `dkim::collect` is called only from `collect`, which only the evaluator calls; nothing touches the SMTP path or `Store::insert` |
| 0002 subscribe-before-check | the wait loop is untouched |
| 0003 oldest-only pruning | no retention change |
| 0005 extraction at ingest | stored bytes are read, never rewritten; no persisted verdict |
| 0020 test placement | every DKIM branch is reachable by a fixture, so all tests are inline `#[cfg(test)]` in `src/deliverability/dkim.rs`; no `_tests.rs` harness unless a concrete cold path appears; the wire proof is in `tests/deliverability.rs` |
| dependency policy | one exact pin, minimal features, `cargo deny` with `deny.toml` unchanged; a one-sentence reason in `Cargo.toml`; the transitive list in 0125 |
| PR hold | `Cargo.lock` gains packages, so the PR is held for the operator (0056 addendum) |
| no `coverage(off)` | none; the `expect` on `now_or_never` and on output/field agreement replace unreachable arms |
| decisions | 0125 is drafted `proposed`; nothing is accepted |

## Project structure (files touched)

| path | change |
|---|---|
| `Cargo.toml` | the `mail-auth` line and its one-sentence reason |
| `Cargo.lock` | the locked additions (generated with `--locked` resolution, then checked in) |
| `src/deliverability/dkim.rs` | **new**: pre-scan, bounds, the cache adapter, the authenticator `LazyLock`, `canonical_len`, the verdict mapping, inline tests and test signing helpers |
| `src/deliverability.rs` | `mod dkim;`; `RULE_SET_ID` becomes `swarmail-html/6`; `evaluate` and `collect` take `receipt`; the call after `spam::collect`; drop the `expect(unused_variables)`; an honest header comment; literal-report tests updated |
| `src/deliverability/dns.rs` | drop the two `expect(dead_code)` attributes now that production calls `resolve_txt` and reads `Txt` |
| `src/deliverability/html-rules.json` | `rule_set_id` |
| `src/config.rs` | the `--dns-resolver real` refusal text and its test |
| `src/static/llms.txt`, `src/static/openapi.json`, `src/mcp.rs` | the id and refusal text where they appear |
| `docs/guide/deliverability.md`, `docs/guide/testing.md` | a "DKIM verification" section (rules, receipt time, residual, bounds, how to declare keys) and the version history |
| `docs/decisions/0125-offline-dkim-verification.md` | §9 completed with the real lockfile list and licences |
| `docs/decisions/README.md` | the 0125 index row |
| `tests/deliverability.rs` | the wire proof (AC11) and the signed two-process comparison (AC12); id updates |
| `tests/bulk_sender.rs`, `tests/spam_rules.rs`, `tests/mcp_revisions/structured.rs`, `tests/fixtures/**` | the mechanical id update only |
| `Dockerfile` | only if the Alpine build of `aws-lc-sys` needs a build tool (see Risks) |

## Research (decisions with their evidence)

1. **Backend.** mail-auth's `[features]` list has no `ring`. `aws-lc-rs`
   enables `dns-hickory` and hickory's TLS and DNSSEC on aws-lc. The aws-lc
   verifier uses `RSA_PKCS1_1024_8192_SHA256_FOR_LEGACY_USE_ONLY`, so it
   accepts RSA keys of 1,024 to 8,192 bits, which RFC 8301 requires.
   `rust-crypto` needs uncached crates and `rsa` 0.9.
2. **Offline injection.** `txt_lookup` reads `cache.get(name)` before
   `resolve_txt`, and a `Some` returns at once. With an adapter that never
   misses, the resolver is never called.
3. **Clock.** `verify_dkim` passes `SystemTime::now()` to the crate-private
   `verify_dkim_`, and its check is
   `x == 0 || (x > t && x > now)`. That gives the receipt-time rule and its
   residual in 0125 §8.
4. **Fields are crate-private.** `AuthenticatedMessage`'s fields are
   `pub(crate)`. That is why Swarmail calls the public `Signature::parse`
   per field to learn which output belongs to which field, and computes the
   canonical body length itself.
5. **`l=`.** The crate's body hasher truncates to `min(l, len)` and treats
   `l = 0` as "whole body". That gives `body_length_exceeds` and the
   literal-`l=0` refusal.
6. **Time in test fixtures.** `DkimSigner::sign` stamps `t=` from the wall
   clock. Fixtures that need explicit `t`/`x`/`l` use a test helper:
   - write the tag list with the chosen tags and an empty `b=`;
   - take `bh=` from a `DkimSigner` signature of the same body and `c=`;
   - parse, call `canonicalized_dkim_headers()`, sign with
     `SigningKey::sign`, and fill in `b=`.

   Expiry fixtures that only need "receipt before or after `x`" sign
   normally with `expiration(1 day)` and generate `received_ms` from the
   signature's own `t`/`x`.
7. **Keys for tests.**
   - The RFC 8463 Appendix A.1 private keys are published: an ed25519 seed
     and an RSA key, believed to be 1,024-bit; the seat confirms the size.
   - One fixed, test-only 2,048-bit RSA key is embedded as a constant.
   - No key is generated at run time, and no `generate` feature is
     enabled.

## Data model

- `MAX_DKIM_SIGNATURES: usize = 16` and `MAX_DKIM_SIGNED_HEADERS: usize = 128`.
- `struct Field<'a> { occurrence: u64, value: &'a [u8], tags: PreScan, parsed: Result<Signature, mail_auth::Error> }`,
  where `PreScan` is `{ duplicate: bool, literal_l_zero: bool, h_names: usize }`.
- `struct Adapter<'l> { lookups: RefCell<&'l mut Lookups>, invalid: RefCell<BTreeSet<String>> }`,
  which implements `ResolverCache<Box<str>, TxtRecord>` and `DnsCache`.
- `enum Rule`: 17 variants, each with `id()`, `severity()` and `hint()`
  as `const` strings. A test binds the list to the guide.
- `canonical_len(body: &[u8], c: Canonicalization) -> u64`.
- The report types are unchanged.

## Contracts

- Report JSON is unchanged in shape. `rule_set_id` is `swarmail-html/6`.
  `findings` may carry `dkim.*` entries at `header`/`dkim-signature`
  locations, and `dns.lookups` may be non-empty in production.
- The CLI refusal text changes as the spec states.
- No REST, MCP, OpenAPI schema or UI change beyond example ids.

## Quickstart

```bash
swarmail serve &
curl -X PUT localhost:8025/api/v1/inboxes/qa/dns \
  -d '{"txt":[{"name":"sel._domainkey.example.com","values":["v=DKIM1; k=ed25519; p=<base64>"]}]}'
# send a message signed d=example.com s=sel to qa@… over SMTP
curl localhost:8025/api/v1/messages/<id>/deliverability   # no dkim.* finding
curl 'localhost:8025/api/v1/inboxes/qa/await?no_findings_at_or_above=warning'
```

## Risks

| risk | mitigation |
|---|---|
| **The hickory construction.** Building `MessageAuthenticator` with an empty name-server list may be refused, may need a Tokio context, or may read system files | Preflight it first (T1). The evaluator runs inside `spawn_blocking`, so a runtime handle exists there, and unit tests use `#[tokio::test]`. If hickory refuses an empty list, record the evidence in the 0125 §9 ledger and keep the no-miss guarantee (AC9's `key_missing`-not-`TempError` proof) as the sole offline proof. Never point it at a real resolver. If the crate cannot answer from the adapter at all, **stop and report** (0125 §9) |
| **Field/output misalignment.** Swarmail's header scanner and mail-auth's could disagree on which fields are `DKIM-Signature` (odd folding, a missing colon) | The signature-equality `expect` turns disagreement into a failed analysis, never a wrong verdict. AC6 fixtures cover malformed fields between parsed fields. If a fixture finds a real disagreement, make it a tested `AnalysisError::Failed` branch |
| **A wrong record type for the `_report` selector.** A selector literally named `_report` collides with the RFC 6651 name shape, and the crate's own cache has the same ambiguity | Fail closed: such a key reads as a report record and gives `key_invalid`, never a pass. Documented in the guide's known limits |
| **Wall-clock residual** (0125 §8) | Named rule, pole fixture, operator question in 0125, guide text |
| **Crate upgrade drift** | Exact `=0.14.0` pin, one fixture per rule asserting the id, and a canonical-length oracle test |
| **aws-lc-sys build on Alpine/musl (Docker smoke) and macOS** | Run `cargo build --locked` locally. If Docker is available, run the image build; if the musl build needs `cmake` or `clang`, add it to the Dockerfile's `apk add` line and say so. CI's docker-smoke and macOS jobs are the final proof, which the seat reports as CI-owned |
| **Two rustls providers.** hickory turns on rustls' `aws_lc_rs`, so a `ClientConfig::builder()` without a provider would panic with an ambiguous provider | Every Swarmail config already uses `builder_with_provider(ring)`. The existing real-HTTPS webhook and STARTTLS tests must pass unchanged (AC13). Any panic there is fixed by pinning ring explicitly at that call site, never by removing a feature |
| **RustSec advisories on the new tree** (aws-lc-sys, hickory, rsa if pulled) | `cargo deny check licenses` runs locally. The RustSec job is CI-owned and network-backed. The seat runs `cargo deny check advisories` if an advisory DB is cached and reports the result. An advisory needing an ignore is a second widening, and the seat stops and reports it |
| **Report size** | ≤ 16 DKIM findings, and lookups bounded by the 8 KiB policy, so the 65,536-byte bound still holds. The existing over-bound test stands |
| **Many literal-report updates** | Mechanical; `grep -rn swarmail-html/5` must return nothing outside history notes in decisions and release notes |
| **Load** | Optional. If the seat proves anything under load, it uses at most `nproc/4` workers for at most 10 minutes with a named, timed command, for example `timeout 600 cargo test --locked --test deliverability -- --test-threads=$(( $(nproc)/4 ))`, and reports the load applied |

## Panel rulings carried into this plan

The spec's [panel adjudication](spec.md#panel-adjudication--2026-11-05) is
authoritative. In short, simplicity wins on:
- the backend;
- the key source inside the evaluator;
- receipt time;
- severities;
- the 16-signature bound;
- "no new surface".

Robustness wins on:
- `l=` length checks;
- the `from_unsigned`, `key_revoked` and `selector_invalid` rules;
- the duplicate-tag pre-scan;
- the `h=` cap;
- bounds as findings rather than `Failed`;
- the honest determinism exception;
- per-rule snapshot tests against crate drift.
