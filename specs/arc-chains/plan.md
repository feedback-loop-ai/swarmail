# Plan: ARC chain reports (issue #256)

Spec: [spec.md](spec.md) · Tasks: [tasks.md](tasks.md) · Decision:
[0132](../../docs/decisions/0132-arc-chain-reports.md) (proposed, two holds).

## Technical context

- **Language and crate:** Rust, the single `swarmail` crate. `mail-auth`
  `=0.14.0` is already a dependency (0125). This plan turns on its `arc`
  feature and adds nothing else.
- **Where reports are built:** `src/deliverability.rs`. `collect` calls
  `auth::collect`, which calls `dkim::prepare` (strict parse and guards),
  `dkim::verify`, SPF and DMARC on one shared `Lookups` borrow.
  `Collector::finish_with` sorts, removes duplicates, sums `total`, builds the
  `Report` and checks the 65,536-byte bound.
- **Receipt second:** `email.received_ms.max(0) / 1000`, already passed to
  `evaluate_at`.
- **Test-only override:** in test builds `Inputs.auth` is
  `Some(unknown facts)` by default and skips `auth::collect` (0126 section 6).
  ARC is evaluated inside `auth::collect`, so the override skips ARC too and
  the report carries the default member (`none`, 0 sets). A test of ARC sets
  `auth: None`. Production has one path and it evaluates ARC.
- **Constraints:** exact 100% coverage of lines, branches and functions, no
  `coverage(off)`; inline `#[cfg(test)]` tests by default (0020 rule 1); no
  mocks; no new dependency; no clock read in Swarmail's own code.

## Constitution check

| rule | how the plan meets it |
|---|---|
| 0001 accept == stored | no change to `Store::insert`, `src/smtp.rs` or ingest; AC11 proves the row and bytes are untouched |
| 0002 await subscribes first | the wait loop is not edited |
| 0003 oldest-only pruning | not edited |
| 0005 extraction at ingest | the report stays a lazy derived product (0085) |
| 0085 bounds and cache | at most one more finding and one small member; the cache key is unchanged |
| 0085 section 1 shape | amended by the `arc` member: hold 1 in 0132 |
| 0085 section 8 ambient time | Swarmail reads no clock; the crate's dead read is hold 2 in 0132 |
| 0120 one key source | every lookup goes through `Lookups` by the existing adapter |
| 0122 section 1.4 and section 2 | `swarmail-html/9`; weights by the existing function, no new exception |
| 0125 section 5 and section 6 | Swarmail owns the crate's silent defaults; the re-parse guard keeps the work bound |
| 0126 | no fact and no other verdict reads ARC |
| dependency allowlist | not widened; lockfiles and `deny.toml` byte-identical |
| 0020 test placement | inline tests in `arc.rs`; wire tests in `tests/deliverability.rs`; no seam, no harness file |

## Project structure (files touched)

| file | change |
|---|---|
| `Cargo.toml` | mail-auth features `["aws-lc-rs", "arc"]`; the comment names #256 |
| `src/deliverability/arc.rs` | **new**: the rule table, the member type, the six steps, inline tests and the sealing kit |
| `src/deliverability/dkim.rs` | `prepare` keeps the strict parse beside a re-parse and applies the re-parse guard; `scan` and the position sort become reusable by `arc.rs`; the module comment names the guard |
| `src/deliverability/auth.rs` | `collect` calls `arc::evaluate` after `dkim::prepare` and returns the chain beside the facts |
| `src/deliverability.rs` | `mod arc`; `Report.arc`; `Collector::finish_with` takes the chain; `RULE_SET_ID`; test literals |
| `src/api.rs`, `src/static/openapi.json` | the `ArcChain` schema, `required`, the descriptions |
| `src/mcp.rs` | `outputSchema` member and `required`; `MODERN_REPORT_DESCRIPTION` |
| `src/ui.rs` | one escaped ARC line in the report section |
| `src/static/llms.txt`, `docs/guide/deliverability.md` | the ARC section, the correspondence lines, the version history |
| `src/deliverability/{links,html,spam}.rs`, `html-rules.json`, `tests/*.rs`, `tests/fixtures/**` | the `swarmail-html/9` literals and the new member in expected reports |
| `tests/deliverability.rs` | the wire fixtures and the surface, assert and await proofs |
| `docs/decisions/README.md`, `docs/decisions/0085-*.md` | the 0132 row (proposed); one dated pointer on 0085 naming both holds, its text unchanged |

Nothing under `.github/`, `scripts/`, `.brokkr/`, `evals/` or the toolchain
files changes. If the eval harness pins a report shape that the new member
breaks, that is a stop-and-report, not a quiet edit (see Risks).

## Research (decided, with evidence)

Read in `mail-auth-0.14.0/src`. Task T0 re-reads each before code is written.

1. **`arc = []`** (`Cargo.toml`, `[features]`): no dependency, so no lockfile
   change.
2. **`verify_arc` order** (`arc/verify.rs`): a rejected ARC field gives
   `Neutral`; no `ARC-Message-Signature` gives `None` without reading the
   other two families; more than 50 gives `ChainTooLong`; unequal family
   counts give `BrokenChain`; then per sorted position the instance, then
   `cv=`, then on the last position `x=` and the body hash; then the newest
   signature's key lookup and check; then each seal, newest first.
3. **The clock** (`arc/verify.rs`): one `SystemTime::now()` after the count
   checks, used only in
   `signature.x == 0 || (signature.x > signature.t && signature.x > now)`.
4. **One cache call per lookup** (`dns/lookup.rs`, `txt_lookup`): the cache's
   `get` is called once and its answer returned. The adapter always answers,
   so the calls in one `verify_arc` are, in order, the newest signature's key
   and then each seal's key from `N` down.
5. **Sorting** (`message.rs`, `finalize`): the three lists are sorted by `i=`
   with `sort_unstable_by_key`, and only when a seal parsed and no ARC field
   was rejected. Steps 3 and 4 of the spec establish both conditions, and
   step 5 (distinct instances) makes the order a function of the bytes,
   before any code relies on it.
6. **Rejected fields** (`message.rs`, `push_arc_error`): a rejected ARC field
   lands in `errors()` under its header name with whatever error its parser
   returned, which need not be an `Error::Arc`. Fields are therefore matched
   by name, never by error variant.
7. **Silent defaults** (`arc/parse.rs`, `parse.rs`): `a=` defaults to
   rsa-sha256; `number()` returns `None` for garbage and the callers read
   that as 0; a repeated tag overwrites; `i=` is cast `as u32`, so
   `i=4294967297` reads as 1; an `ARC-Message-Signature` without `i=` parses
   with instance 0; `ARC-Authentication-Results` takes the first `i` tag it
   finds, wherever it is.
8. **`l=`** (`message.rs`, `parse_ams`; `dkim/canonicalize.rs`,
   `canonical_body`): strict mode rejects a non-zero `l=` unhashed; `l=0` and
   an unreadable `l=` hash the whole body; non-strict mode hashes one prefix
   per distinct `l=`.
9. **Result mapping** (`dkim/output.rs`): a lookup error becomes `TempError`
   for `Dns(Resolver(_))` and `PermError` otherwise. The existing adapter
   answers a declared `tempfail` with the former and "no record" with
   `RecordNotFound`.
10. **Sealing for fixtures** (`arc/seal.rs`): `ArcSealer::seal` writes
    `cv=pass` only from a passing `ArcOutput`, sets `t=` from the wall clock
    and writes `x=` only when an expiration is configured. The kit never
    configures one.

## Data model

```rust
// src/deliverability/arc.rs
pub enum State { None, Pass, Fail, NotEvaluated }   // serde: snake_case

pub struct ArcChain {
    pub state: State,
    pub sets: u64,
    pub failing_instance: Option<u32>,
    pub reason: Option<String>,
}
```

`ArcChain` derives what `Report` derives, with `deny_unknown_fields`, and has
a `Default` of `none` with 0 sets. `Report` gains `pub arc: ArcChain` between
`dns` and `total`. `Rule` is a private enum of thirteen variants with `id`,
`state`, `hint` and a `#[cfg(test)] ALL`, as `dkim::Rule` has.

## Contracts

- **REST** `GET /api/v1/messages/{id}/deliverability`: the body gains `arc`.
  OpenAPI: `DeliverabilityReport.required` becomes
  `["rule_set_id","findings","dns","arc","total"]`; `ArcChain` is a closed
  object with `state` (enum of four), `sets` (integer, minimum 0),
  `failing_instance` (integer 1 to 50 or null) and `reason` (string or null),
  all required.
- **MCP** `swarmail_get_deliverability_report`: the same member in
  `outputSchema` and `required`; keys are sorted on this surface, so `arc`
  comes first.
- **assert, await, wait:** unchanged.

## Quickstart

```bash
# a DNS policy that declares each sealer's key
#   {"txt":[{"name":"arc1._domainkey.fwd.example","values":["v=DKIM1; k=ed25519; p=..."]}]}
# send a message carrying one ARC set sealed with that key over SMTP
# GET /api/v1/messages/<id>/deliverability
#   "arc":{"state":"pass","sets":1,"failing_instance":null,"reason":null}
# send the same message with its body changed after sealing
#   "arc":{"state":"fail","sets":1,"failing_instance":1,
#          "reason":"arc.body_hash_mismatch.v1"}
#   and one arc.body_hash_mismatch.v1 finding at arc-message-signature, occurrence 0
```

## Implementation approach

1. **The feature and the parse (T1, T2).** Turn the feature on and prove the
   lockfiles did not move. Change `Prepared::Ready` to carry the strict parse
   as well as the parse DKIM verifies, and give `Prepared` one accessor for
   the strict parse that returns `None` only for `Stopped` and an unparsable
   message. Add the re-parse guard between the two parses, counting `l=` from
   the strict parse's own field lists.
2. **`arc.rs`, step by step (T3 to T9).** One public entry,
   `evaluate(strict, root, size, adapter, receipt, out) -> Result<ArcChain, AnalysisError>`,
   that runs the spec's steps 0 to 7 in order and pushes at most one finding.
   Each step is a small function over the crate's public lists and Swarmail's
   root list. The lookup count is a counter owned by a thin wrapper around
   the existing adapter for the one call. The result mapping is a single
   `match` whose last arm is `signature_mismatch`; no arm exists that a
   fixture cannot reach, and no `unreachable!` or `expect` stands in for one
   beyond the adapter's existing "answers on the first poll".
3. **Wiring and version (T10).** `auth::collect` returns the chain with the
   facts; `collect` hands it to `finish_with`. No fact and no other collector
   reads it. The id literals move in the same task.
4. **Surfaces (T11 to T13).** The two schemas, the description, the page
   line.
5. **Wire proofs (T14 to T16)** and **docs and records (T17, T18)**, then the
   gates (T19).

The kit seals per run with the crate's `ArcSealer` and the DKIM kit's keys,
one selector and domain per hop. A hop is sealed from the previous hop's
passing `verify_arc` output, obtained through the same adapter. Failing
fixtures are made by declaring a different key, by editing bytes after
sealing, or by forging fields that never reach the crypto.

## Risks

| risk | likelihood | mitigation |
|---|---|---|
| The operator refuses hold 1 (the member) | medium | 0132 names the remedy: drop the member, keep the findings. The member lives in one type and one `finish_with` argument, so removal is small. The run returns to design. |
| The operator refuses hold 2 (the dead clock read) | low | 0132 names the two scope remedies; neither is built. The verifier call is one site. |
| A crate fact in Research is wrong | low | T0 re-reads each before code. A stop condition applies: if any ambient read on the ARC path can reach a verdict, or a lookup can miss the adapter, the seat stops and reports. Every mapped crate result has a fixture that pins it. |
| The lookup-count attribution drifts on a crate upgrade | low | the exact pin `=0.14.0`; fixtures for `i=1` of 2 and `i=2` of 3 fail on any reorder. |
| Turning the feature on changes a DKIM verdict | low | the crate's ARC fields are filtered out of DKIM's field list by name today; AC12 runs every existing test and adds the `l=` independence proof. |
| The re-parse guard changes `dkim.limits_exceeded.v1`'s reach | certain, narrow | only a message with a DKIM `l=` and more than 16 `l=` fields in total; the guide and the hint say so; the id is bumped. |
| Step 3's `i=` check for `ARC-Authentication-Results` rejects a real sealer's field | low | RFC 8617 section 4.1.1 puts the instance first. The fixture set includes the crate sealer's own output. A rejected field is an error finding, never a silent pass, so a false positive is visible and reportable. |
| A chain with `x=` on its newest signature goes unverified | unknown frequency | reported as an error, never clean; the guide states it; 0132 section 5 names the alternative for the operator. |
| The exact coverage gate refuses a defensive arm | medium | the design has no dead arm by construction (the catch-all is reached by ordinary failures). If one appears in code, restructure; never exclude. |
| Report bytes change in every pinned fixture | certain | T10 and T12 regenerate them with provenance, as the earlier id bumps did. |
| The eval harness pins the report shape | low | `evals/` builds its own loose report values today. If its tests fail on the member, stop and report: `evals/` is outside this change. |
| Hostile mail drives cost | low | strict parse: at most four body hashes; at most 51 lookups and 51 signature checks; `h=` capped at 128 names; no load proof is asked for or run. |
| A verdict depends on the crate's unstable sort | closed by design | step 5 refuses duplicated instances before the crate's sorted lists are read, so every later step sees a total order. |
