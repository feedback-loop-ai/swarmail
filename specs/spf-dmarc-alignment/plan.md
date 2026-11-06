# Plan: SPF and DMARC alignment (issue #255)

Specification: [spec.md](spec.md). Decision: [0126](../../docs/decisions/0126-spf-dmarc-alignment.md)
(proposed; one operator hold).

## Technical context

- **Language and crate:** Rust, the single `swarmail` crate. mail-auth
  `=0.14.0` (`aws-lc-rs`) as #254 pinned it; no new crate or feature.
- **Where it runs:** the lazy report evaluator (`evaluate_at` → `collect`),
  synchronous, inside 0085's four job permits and 128-slot cache. Crate
  futures are driven with `now_or_never()` as #254 does.
- **DNS:** 0120's declared policy through `Lookups`, the only source. The
  `AUTHENTICATOR` with no name servers (0125 §2).
- **Storage:** SQLite data file via `persist.rs`; two nullable added columns.
- **Testing:** inline `#[cfg(test)]` modules (measured, decision 0020 rule 1);
  real-protocol suites in `tests/`. No mocks on the wire. Exact 100% line,
  branch and function coverage.

## Constitution check

| rule | status |
|---|---|
| 0001 accept == stored | kept: the facts ride the one synchronous insert; `Store::insert` unchanged |
| 0002 await subscribes first | kept: no await code changes |
| 0003 oldest-only pruning | kept: no pruning code changes |
| 0005 extraction at ingest | kept: capture at ingest; evaluation lazy as 0085 rules |
| 0085 §1 report shape | kept: `{rule_set_id, findings, dns}` |
| 0085 §8 ambient time | **amended only if accepted**: the SPF `LookupLimit` timer (0126 §8, hold) |
| 0089 §5 facts in Inputs equality | met by the facts' determinants (0126 §6) |
| 0120 (proposed) | integration note: `spf_client_ip`; non-TXT answers |
| 0125 (proposed) | integration note: DMARC consumes #254's verdicts |
| dependency policy | no change; `Cargo.lock` byte-identical |
| coverage | exact; no `coverage(off)`; every new line tested inline |
| merge holds (0056 addendum) | no `.github/`, `scripts/`, `.brokkr/`, lockfile or `deny.toml` change; only the 0045 §8 hold |

## Project structure (files touched)

| file | change |
|---|---|
| `src/model.rs` | `Email.smtp_client_ip`, `Email.smtp_helo`, `#[serde(skip)]`, documented |
| `src/smtp.rs` | pass the peer into `session`; track the HELO name; clear on `STARTTLS`; set both fields after `build_email`; inline tests |
| `src/persist.rs` | two `ADDED_COLUMNS`, the write and restore mapping, refusal of a bad address; inline tests |
| `src/store.rs`, `src/api.rs`, `src/compat.rs`, `src/webhook.rs`, `src/threads.rs`, `src/deliverability/concurrency_tests.rs` | `Email` literals gain `None` fields |
| `src/deliverability/dns.rs` | `Policy.spf_client_ip`, its parse, canonical form and view; the existence rule for DMARC's A query |
| `src/deliverability/dkim.rs` | return the parsed message, the per-field verdicts and the crate outputs to the caller instead of dropping them; share `AUTHENTICATOR`, `owner_of`, `name_of` |
| `src/deliverability/auth.rs` (new) | SPF and DMARC adapters, evaluation, rule table, facts; inline tests |
| `src/deliverability.rs` | `mod auth`; `collect` runs DKIM → SPF → DMARC → facts → `bulk`; `Inputs.auth` becomes a `#[cfg(test)] Option` override; delete `policy_id`/`observation_id`; `RULE_SET_ID` = `swarmail-html/7` |
| `src/mcp.rs`, `src/static/openapi.json` | the optional `spf_client_ip` policy member; rule-id lists if enumerated |
| `src/static/llms.txt`, `docs/guide/deliverability.md`, `docs/index.html` (if it lists ids) | the member, ids, A-R table, unsupported mechanisms |
| `tests/deliverability.rs`, `tests/bulk_sender.rs`, `tests/persistence.rs`, `tests/mcp_revisions/*`, `tests/webhook.rs`, fixtures pinning `swarmail-html/6` | wire proofs; literals; the version |
| `docs/decisions/README.md`, `0120-…`, `0125-…`, `0126-…` | index row; dated integration notes; implementation record |

Not touched: `Cargo.toml`, `Cargo.lock`, `evals/`, `deny.toml`, `.github/`,
`scripts/`, `.brokkr/`.

## Research (decided, with evidence)

- mail-auth 0.14.0 `[features]` gate neither `spf` nor `dmarc`.
- `check_host` asks TXT through `txt_cache`, A/AAAA/MX/PTR through their
  caches; every cache type is `Result<RecordSet<T>, Negative>` with public
  fields, so an adapter can answer every query. A non-TXT cache cannot express
  a temporary error, which is one reason 0120 is not extended.
- `LookupLimit` admits ten terms (starts at 1, admits while ≤ 10) and reads
  `Instant`. No void counter exists.
- `SpfRecord`'s fields are public, so `ra` can be cleared; `with_report`
  samples only when `ra` is `Some`.
- `%{t}` parses only in `exp=` text (`Variable::parse_exp`); explanation is
  fetched as `Macro`, which an SPF-only adapter cannot answer.
- `verify_dmarc` computes SPF alignment only for an SPF Pass or TempError and
  takes `&[DkimOutput]`; `DkimOutput` cannot be built outside the crate but an
  owned `Vec` can be filtered with `retain`.
- The tree walk returns early with no alignment when no record applies, so
  alignment facts are Unknown then.
- `parse_records` keeps the first record that parses, so record selection is
  Swarmail's.

## Data model

- `Email { …, smtp_client_ip: Option<IpAddr>, smtp_helo: Option<String> }`.
- SQLite `emails.smtp_client_ip TEXT NULL`, `emails.smtp_helo TEXT NULL`.
- `dns::Policy { txt, tempfail, spf_client_ip: Option<IpAddr> }`;
  `PolicyView` gains `#[serde(default, skip_serializing_if = "Option::is_none")]
  spf_client_ip: Option<String>`.
- `AuthEvidence { policy_present, valid_from, dkim_pass, dkim_aligned,
  spf_pass, spf_aligned }` (provenance strings removed); `Inputs.auth:
  Option<AuthEvidence>` under `#[cfg(test)]` only.

## Contracts

The rule table, the A-R table, the facts table and the declaration grammar in
[spec.md](spec.md#contract) are the contracts. The report shape is unchanged.

## Quickstart

```bash
curl -X PUT "$SWARMAIL_HTTP/api/v1/inboxes/t/dns" -d '{
  "txt":[
    {"name":"example.com","values":["v=spf1 ip4:127.0.0.1 ip6:::1 -all"]},
    {"name":"_dmarc.example.com","values":["v=DMARC1; p=reject"]}]}'
# send MAIL FROM:<a@example.com>, From: a@example.com over SMTP to t@…
# GET the report: no spf.* finding, no dmarc.* finding (DMARC passes via SPF)
# declare "spf_client_ip":"192.0.2.9" instead: spf.fail.v1 and dmarc.fail.reject.v1
```

## Implementation approach

1. **Capture first, evaluation later.** The model, SMTP and persistence
   changes land with their tests before any evaluator code, so the
   observation's guarantees (AC1–AC4) are proven on their own.
2. **Policy member.** `spf_client_ip` goes through 0120's one parser, its
   canonical form and its view; the API, the server file and the report share
   it. The existence rule (a name exists when it or a descendant is declared)
   is a pure function on `Policy`.
3. **DKIM hand-off.** `dkim::collect` keeps its findings and bounds exactly,
   and additionally returns what DMARC needs: the parsed
   `AuthenticatedMessage` (borrowing `raw`), the crate outputs and the
   verdict per output, paired by signature identity as #254 already does.
   Outputs are filtered with `Vec::retain` against #254's verdicts. When #254
   stops (boundary, bounds, unsigned), it says so, so the facts can be
   Unknown or computed from an empty slice.
4. **`auth.rs`.** One generic adapter over `RefCell<&mut Lookups>` with a
   record-kind parameter (SPF or DMARC), reusing `owner_of`/`name_of`, plus
   the non-TXT caches: SPF's answer negative and set a `Cell<bool>` mark;
   DMARC's A cache applies the existence rule. `collect` order: DKIM → SPF →
   DMARC → facts → `bulk` → html → spam. Findings are sorted at `finish`, so
   order changes no bytes.
5. **Facts.** `bulk` reads `auth_facts` from the evaluation, or the
   `#[cfg(test)]` override when set. The truth-table tests move to the
   override; the cache-identity tests that mutated `policy_id` now vary the
   policy snapshot and the mail.
6. **Version** last among code changes, with every pinned `swarmail-html/6`
   fixture updated in the same commit.
7. **Wire proofs, then docs and records** (integration notes on 0120 and
   0125, the 0126 implementation record, the README index row).

## Risks

| risk | mitigation |
|---|---|
| The operator refuses the timer hold (0126 §8) | The PR stays held under 0045 §8; the remedies (upstream API, or no SPF) are named in 0126 and need a new design return. Nothing is merged on green. |
| A hidden reachable read or resolver path in the crate | The adapter answers every cache; AC15 asserts `Some` from each and first-poll completion with `expect`. The implementer re-reads `spf/verify.rs`, `dmarc/verify.rs`, `dns/lookup.rs` and `sampling.rs` for 0.14.0 and records any new reachable read as a stop-and-report, not compatible work. |
| SPF or DMARC needs a feature or crate #254 did not add | Stop and report (brief); AC18 proves the lockfile unchanged. |
| Exact coverage on the new module (cold arms such as a cache that is never asked) | Every adapter arm gets an inline test that calls it directly; no `coverage(off)`; a genuinely cold failure path moves to an `auth/<name>_tests.rs` harness only under decision 0020 rule 2. |
| DKIM refactor changes #254's bytes | #254's existing tests stay green unchanged apart from the version; the hand-off is additive. |
| Lifetimes: `DkimOutput<'x>` borrows the parsed message | `auth` runs inside the scope that owns the parse (the DKIM step returns a struct borrowing `raw`), so no clone of the message is needed. |
| Report growth past 65,536 bytes for extreme declared worlds | Bounded names (spec work bound); past the byte bound the existing `AnalysisError::Failed`, proven by an existing bound test plus one SPF chain fixture. |
| Schema change of the policy breaks pinned MCP/OpenAPI fixtures | The member is optional and present only when declared, so existing outputs keep their shape; fixtures that enumerate the schema are updated in the same commit with their provenance notes. |
| Every report now carries DMARC and SPF findings, changing existing tests' expectations | `swarmail-html/7` makes the change explicit; tests that asserted only `bulk.*.not_evaluated` are updated to the new evidence, never loosened to "contains". |
| Swarmail's lenient SMTP (MAIL before any greeting) | `smtp_helo` is `None`; MAIL FROM SPF still runs with an empty `%{h}`; the HELO identity is not evaluated (AC6). |
| Double reporting (`dmarc.fail.*` and `bulk.*`) confuses users | The guide's A-R table explains both; the ids and hints are static. |
| A load proof is wanted | None is required. If one is run, it is bounded to `nproc/4` workers for at most 10 minutes with a named, timed command, and the applied load is reported. |
| No brokkr binary in the implementer's sandbox | The bundle-compile gate runs in verify; the implementer says so instead of reporting a broken gate. |
