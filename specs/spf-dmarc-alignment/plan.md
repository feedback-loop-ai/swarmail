# Plan: SPF and DMARC alignment (issue #255)

Specification: [spec.md](spec.md). Decision: [0126](../../docs/decisions/0126-spf-dmarc-alignment.md)
(proposed; three operator holds, five named items).

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
| 0085 §8 ambient time (accepted) | **amended only if accepted**: the SPF `LookupLimit` timer (0126 §8, hold 2) |
| 0089 §5 facts in Inputs equality (accepted) | **a reading, the operator's**: met by the facts' determinants (0126 §6, hold 3) |
| 0120 (accepted 2026-11-06) | **amended only if accepted**: §2 `spf_client_ip` (1a), §3 unrecorded non-TXT answers (1b), §4 NXDOMAIN/NODATA for `np=` (1c); §5's second byte-identity exception is hold 2 |
| 0125 (accepted 2026-11-06) | integration note only: DMARC consumes #254's verdicts; not amended |
| 0121 §5, 0122 §1.4 (accepted) | followed: `swarmail-html/7`, the next never-shipped id |
| dependency policy | no change; `Cargo.lock` byte-identical |
| coverage | exact; no `coverage(off)`; every new line tested inline |
| merge holds (0056 addendum) | no `.github/`, `scripts/`, `.brokkr/`, lockfile or `deny.toml` change; the 0045 §8 holds 1–3 |

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

## Design return (review 1, 2026-11-06)

The first review passed the code and returned the design for its hold
statement. This section maps each review finding to the change that answers
it, the artifact that owns that change and the task that carries it. The
rulings are in [spec.md](spec.md#design-return-adjudication--2026-11-06).
T0–T26 were delivered at `6653277` and verify passed them there; the return
adds T27–T36 on top and changes no earlier task's proof except the base
commit.

| finding (member, severity) | answer | owner | task |
|---|---|---|---|
| Hold statement names one hold; 0120/0121/0122/0125 called proposed; "no accepted ruling is contradicted" (correctness 1, guarantees 1, independent 1, security, tests; medium, the `spec_defect`) | 0126's header rewritten in this design commit: holds 1 (1a/1b/1c), 2 and 3, accepted statuses, corrected "Not holds"; spec's Operator holds section matches. The index row, 0120's note (§3→§2 relabel plus §3/§4 items) and dated pointers in 0085 and 0089 remain | 0126 (done here); spec (done here); README, 0120, 0085, 0089 | T27, T28 |
| 0089 §5 reading listed under "Not holds" (correctness 5, independent 2; low/info) | moved into the list as hold 3 | 0126 (done here) | T28 (the 0089 pointer) |
| DeliverabilityReport "no production message currently qualifies" and DnsRecord "SPF and DMARC will record theirs" are false (correctness 2, guarantees 2, tests; low) | both descriptions corrected | `src/static/openapi.json` | T29 |
| "One stated exception" wording predates the SPF timer (guarantees 3; low) | both reads named | `src/deliverability.rs`, `src/deliverability/dns.rs`, the guide, `llms.txt` | T30 |
| Two `v=DMARC1` values give permerror; RFC says not applied (correctness 3; low) | discarded as no record; `dmarc.none.v1`; hints moved; AC10's example corrected and moved to AC21 | `src/deliverability/auth.rs`, the guide's rule table | T31, T32 |
| `v = DMARC1` reads as no record, `v=dmarc1` as one (independent 3; info) | RFC 7489 §6.4 ABNF in `Kind::is_ours` for DMARC; SPF unchanged | `src/deliverability/auth.rs` | T31, T32 |
| Null sender's HELO fed to DMARC; baseline unstated (correctness 4; info) | baseline stated (the pinned crate's RFC 9989 contract), behaviour unchanged, pinned by AC23 | 0126 §5 (done here); `auth.rs` tests | T33 |
| AC18's `git diff 7478073` fails since the v0.4.0 merge (correctness 6; info) | base `c95386e` | spec AC18 (done here); 0126 implementation record left as dated history | T34 |
| `cfg(test)` `Inputs.auth` default (correctness 7; info, "acceptable") | no change, ruled in the adjudication (point 6) | — | — |

**Criteria changed, with the evidence.** AC10 loses one example, "two
`v=DMARC1` values" → `dmarc.permerror.v1`. The criterion was wrong: RFC 7489
§6.6.3 step 5 says multiple records mean DMARC is not applied, and §11.2 keeps
`permerror` for a syntactically incorrect record. The replacement, AC21,
proves more: the `none` outcome, the walk continuing to a parent, and the
lookups record. AC4 and AC18 change their base commit from `7478073` to
`c95386e` because the v0.4.0 merge moved `Cargo.toml` and `Cargo.lock`'s own
version line. The dependency set is the same, and AC18 now proves that too.
No criterion is loosened. AC21–AC24 are new.

**Files the return touches** (beyond those listed above): `src/deliverability/auth.rs`
(`Kind::is_ours`, `Kind::select`'s multiplicity arm for DMARC, two hint
constants, fixtures), `src/deliverability.rs` and
`src/deliverability/dns.rs` (module doc comments only), `src/static/openapi.json`
(two description strings), `src/static/llms.txt` and
`docs/guide/deliverability.md` (the exception sentences, the rule table's
`dmarc.none`/`dmarc.permerror` lines), `docs/decisions/README.md`,
`0120-…`, `0085-…`, `0089-…` (record text only; no status changes). No new file
and no new rule id. `swarmail-html/7` already covers the byte change, since
`/7` has never shipped.

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
| The operator refuses an item of hold 1 or hold 3 | Each item names its remedy in 0126's header. Every remedy is a new design return, never a silent edit, and nothing merges on green under 0045 §8. |
| The record-text edits (README, 0120 note, pointers) contradict 0126's header | One owner: 0126's header. Each other text repeats the list by number. T27 and T28's `grep` proofs check the same three numbers and the 1a/1b/1c labels everywhere. |
| Changing DMARC multiplicity alters an existing wire or MCP fixture | The id moves only for a name holding two `v=DMARC1` values. T32 lists every fixture that asserts `dmarc.permerror.v1` and moves only the multiplicity ones. Report bytes still change under the unshipped `/7`, so no new id is needed. |
| The ABNF selector diverges from the crate's parser on an odd value | The selector only filters, and the crate still parses what passes. AC21's fixtures cover space, tab, case and `DMARC10`, and a value the selector admits that the crate refuses stays `dmarc.permerror.v1` (one record, unparseable). |
| RFC 9989's real text differs from the RFC 7489 and crate readings used for multiplicity, grammar or HELO | Each reading is cited in 0126 §5 and §7 with a one-arm remedy. The realm holds no RFC 9989 copy and the seats have no web access, so the implementer must not claim to have checked it. A later reading is a design return, not a hidden fix. |
| Coverage of the new selector arms | Every arm (WSP before and after `=`, the case-exact token, each delimiter, multiplicity) has an inline fixture. No `coverage(off)`. The exact gate and its selftest rerun on the new tree. |
| Re-verify cost | Every house gate reruns `--locked` on the new tree, since `auth.rs` and `openapi.json` change. The brokkr compile runs in verify if the implementer's sandbox has no pinned binary. |
