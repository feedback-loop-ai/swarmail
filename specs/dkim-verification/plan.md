# Plan: offline DKIM verification (#254)

Spec: [spec.md](spec.md) · Tasks: [tasks.md](tasks.md) · Decision:
[0125](../../docs/decisions/0125-offline-dkim-verification.md) (proposed).

**Revision 1, 2026-11-05.** The review returned the run to design. This plan is
repaired in place:
- the mechanisms that the implementation (`adeaa12`) replaced are rewritten as
  built;
- the return work is in [§ Revision 1](#revision-1-what-changes-and-which-finding-it-answers),
  with the finding each change answers.

The first plan's text is in git history at `8705096`.

## Technical context

- **Language and crate.** Rust 2024, a single crate. The evaluator lives in
  `src/deliverability.rs` (`evaluate_at(raw, receipt, &Inputs)`), beside the
  `html`, `links`, `spam`, `reach`, `dns` and `dkim` collectors.
- **The DKIM module.** `src/deliverability/dkim.rs`, as built in `adeaa12`:
  - `Rule` (17 variants; 18 after this revision);
  - the adapter and `name_of`;
  - `AUTHENTICATOR` (`LazyLock`, no name servers);
  - `canonicalize` / `canonical_len`;
  - `Scan` (`duplicate`, `has_v`, `has_l`, `h_names`);
  - `fields()`, the crate's view sorted by position;
  - `Context { outputs, first_parsed, body, receipt }`, `verdict` and
    `body_length`;
  - `collect(raw, receipt, lookups, out)`;
  - the inline suite: a signing kit, 17 rule pairs, the c= matrix, the RFC 8463
    vector and the 55,987-body oracle.
- **Shared boundary.** `headers(raw) -> (Headers, size)` in
  `src/deliverability.rs`. It caps at 64 KiB (`MAX_HEADER_BYTES`), and it treats
  a whitespace-only line as a continuation. `evaluate`'s `collect` already holds
  `fields` and `size` (line ~896).
- **The crate.** `mail-auth 0.14.0`. Facts the revision depends on, read from
  `~/.cargo/registry/src/index.crates.io-*/mail-auth-0.14.0`:
  - `verify_dkim` reads `SystemTime::now()` and calls the `pub(crate)`
    `verify_dkim_(.., now)`, which checks `x` before the body hash and the key.
    `DomainKey.p` and the public-key types are `pub(crate)`.
  - `AuthenticatedMessage::{body_offset, raw_body, dkim_signatures, errors,
    retain_dkim_signatures}` are public.
  - `DkimOutput::signature()` returns `Option<&Signature>`. That reference is
    the one `with_signature(&header.header)` stored, which points into
    `dkim_signatures()`.
  - `VerifySignature::domain_key()` is public and returns
    `<s>._domainkey.<d>.`, with a trailing dot.
  - The header parser treats a whitespace-only CRLF line as the header/body
    separator (`headers/parse.rs`, per the security finding).
  - A missing `a=` defaults to rsa-sha256, and `number()` reads non-digits or
    overflow as 0 (per the guarantees finding).
- **aws-lc-sys 0.46.0.** It uses the cc builder unless the build is FIPS,
  `NO_ASM`, a sanitizer build or `CMAKE_BUILDER`, and it ships pregenerated
  bindings for musl (`builder/main.rs` ~584, ~704–790).
- **Constraints.** 100% line, branch and function coverage, exact; no
  `coverage(off)`; no mocks on the wire (0004); the 0020 §5 disciplines.

## Constitution check (house rules)

| rule | how this plan meets it |
|---|---|
| 0001 accept == stored | `dkim::collect` is called only from the evaluator; nothing touches the SMTP path or `Store::insert` |
| 0002 subscribe-before-check | the wait loop is untouched |
| 0003 oldest-only pruning | no retention change |
| 0005 extraction at ingest | stored bytes are read, never rewritten; the boundary guard ties DKIM's body to the committed body |
| 0020 test placement | every new branch is reachable by a deterministic fixture, so all tests are inline in `src/deliverability/dkim.rs`; no harness file; the wire proof is in `tests/deliverability.rs` |
| 0045 §8 | the crate clock may contradict accepted 0085 §8, so the PR is held for the operator's acceptance of 0125 §8 (hold 2); nothing is self-accepted |
| dependency policy | no new package; `deny.toml` changes in a comment only (the PR is held anyway); house rules § Dependencies updated |
| PR hold (0056 addendum) | `Cargo.lock` additions and the `deny.toml` comment; plus holds 2 and 3 |
| decisions | 0125 stays `proposed`; 0120 gains a dated integration note; accepted 0085 is not edited |

## Project structure (files touched by revision 1)

| path | change |
|---|---|
| `src/deliverability/dkim.rs` | the boundary guard and the new rule; `Scan` gains `has_a`, `has_t`, `has_x` and `numbers_ok`; rows a, f and g as specified; identity pairing (deletes `first_parsed`); `owner_of`, shared by the adapter and row l; hints; new inline tests |
| `src/deliverability.rs` | pass `fields` and `size` to `dkim::collect`; a header comment stating the clock radius |
| `src/static/openapi.json`, `src/static/llms.txt`, `src/mcp.rs` | the live-surface sweep (spec § Live surfaces) |
| `docs/guide/deliverability.md` | the 18 rules, the radius, the work bound, the known limits, and the sweep |
| `docs/decisions/0120-deliverability-dns-policy.md` | a dated 2026-11-05 integration note (§4/§5/§7/§10) |
| `docs/decisions/0125-offline-dkim-verification.md` | revised by the chief in this design commit (§2, §5, §6, §8, header); the implementer adds nothing but evidence to §9 |
| `docs/decisions/README.md` | the 0120 and 0125 rows |
| `docs/house-rules.md`, `deny.toml` (comment only) | CDLA-Permissive-2.0 covers `webpki-roots` and `webpki-root-certs` |
| `tests/deliverability.rs` | the wire test asserts the MCP report too, if it does not already (AC11) |
| `Dockerfile` | **only** if a bounded local build or CI's docker smoke shows a missing tool; then exactly that `apk` package |

Not touched: `Cargo.toml`, `Cargo.lock`, `evals/`, the `deny.toml` allowlist,
`.github/`, `scripts/`, `.brokkr/`, and accepted 0085.

## Revision 1: what changes, and which finding it answers

Members: **G** guarantees, **C** correctness, **I** independent, **S**
security, **T** tests. Each row is the owning artifact's answer. Each
implementer-facing change has its own task in [tasks.md](tasks.md).

| finding | severity | answer | where | task |
|---|---|---|---|---|
| G1, C1, I-residual: crate wall clock vs accepted 0085 §8; the radius was understated | HIGH | 0125 §8 is redrafted as a proposed amendment, with the full radius (every crate-mapped verdict plus lookups) and a merge hold; operator options B/B′ are named; vendoring and re-implementation are ruled out with evidence; spec § Verification time and AC7's radius fixture | spec, 0125 (this commit) | R1, R9 |
| S: header/body parser differential (`" \r\n"`) | security, low | boundary guard: step 4 and `dkim.header_boundary_ambiguous.v1`, gated on "signed" | spec step 4, AC17 | R2 |
| G3: missing `a=`; non-numeric `x=`/`t=` read as 0 | MEDIUM | pre-scan row a (`has_a`, `numbers_ok` with digits fitting `u64`); `has_x`/`has_t` in rows f/g; `x=0` is an expiry | spec rows a, f, g; AC18 | R3 |
| I info: `l=abc` maps to unsupported with an `l=0` hint | info | falls under row a (malformed) | AC8, AC18 | R3 |
| C4, I-residual: `first_parsed + index` pairing; the equality check was dropped | LOW | identity pairing (`ptr::eq`) plus `expect`; `first_parsed` deleted; malformed-ARC fixture | spec step 8, AC19 | R4 |
| G11: trailing-dot `d=` gives `key_missing` | LOW | `owner_of` shared by the adapter and row l, applied to `domain_key()` | spec row l, AC9 | R5 |
| G4, C3, I(2): `l=` contract; AC8 oracle | MEDIUM | rows s/t restated in raw-byte terms; oracle = canonicalizer vs the crate's `bh=`; the RFC 6376 §3.4.5 gap named, with an instruction to #255 | spec rows s/t, AC8 | R7 (docs); code unchanged |
| G5, I(3), I(4), T2: stale Evaluation contract (signature, strict order, row q, equality expect, missing `v=`) | MEDIUM | spec § Evaluation rewritten to the code plus revision 1; rows a/c/q/r corrected | spec | none (text); R2–R5 make the code match the new steps |
| G6, I(1), T1: AC11 await clause unmet | MEDIUM | AC11 amended; the clause moves to #255 by name; the wire test asserts REST and MCP | spec AC11 | R6 |
| I(5): AC12 cites a two-process test that does not exist | LOW | AC12 reworded to two in-process servers | spec AC12 | none |
| G7: §6 says "mail-auth is not called" past a bound | MEDIUM | the honest work bound (strict parse first: ≤ 4 hashes, ≤ 8 MiB) | spec § Work bound, 0125 §6 | none (text) |
| G12: "16 lookups"; the 64 KiB claim | LOW | at most 48 names; the 64 KiB bound becomes true through the boundary guard | spec § Work bound | R2 |
| G2: resolver feature not off | MEDIUM | operator hold 3; 0125 §2 asks the question; conditional no-cache proof | spec § Operator holds, 0125 §2 | R8 |
| G8, C2: stale live docs (openapi, llms, guide, MCP; "no ambient time") | MEDIUM | spec § Live surfaces sweep | spec, AC15 | R7 |
| G13: guide contradictions (malformed list, `key_invalid` wording, `atps_unauthorized`) | LOW | rule table rows and § Live surfaces | spec, AC15 | R7 |
| G9: 0125 "without amending" 0120; README row "until #254" | MEDIUM | 0120 integration note; 0125 header corrected; README rows | spec AC21, 0125 header (this commit) | R10 |
| G10, I info: aws-lc-sys on Alpine | MEDIUM | no speculative `apk` change, on the builder-source evidence; one bounded build if a daemon exists; on a CI failure, add exactly the missing package | plan Risks | R11 |
| G14, C5: the CDLA comment covers two crates | LOW/info | `deny.toml` comment and house-rules sentence | spec § Dependency, AC13 | R10 |
| G15: stale plan/tasks | info | this plan and tasks.md repaired in place | plan, tasks | none |
| C6: `name_of` reads std's Hash layout | info | kept, and named as a known off-idiom read | spec § Key source | none |
| I info: `Neutral(_) \| None` read as `body_hash_mismatch` | info | kept; identity pairing removes the misattribution risk | spec row i | none |

**No criterion was weakened to pass review.** Three were changed because the
finding showed the criterion itself was wrong. The evidence for each:

- **AC8's oracle.** "`canonical_len` equals the `l=` the signer writes" cannot
  hold, because mail-auth's signer writes the raw length (0125 implementation
  item 2). The stronger oracle replaces it: Swarmail's canonicalizer is
  byte-equal to the crate's `bh=` input over 55,987 bodies.
- **AC11's await clause.** No message can satisfy it before #255, because the
  bulk `*.not_evaluated.v1` errors disqualify every report. It is moved to #255
  by name, not deleted.
- **AC12's "existing two-process comparison".** No such test exists. The
  criterion names the two-server test that does.

AC2's 1,024-bit clause moves to AC3: aws-lc will not sign with 1,024 bits, and
the published vector proves the 1,024-bit verify path.

## Design detail for the implementer

1. **`collect(raw, root: &Headers, size: usize, receipt, lookups, out)`.**
   - The order is fixed: strict parse, then fields, then signed?, then the
     boundary guard, then the bounds, then the re-parse, then verify, then pair,
     then the verdict.
   - "Signed" is `root.iter().any(|(n, _)| is_dkim_signature(n))` or a
     non-empty `fields(&strict)`.
   - The guard compares `strict.body_offset()` with `size`. Both are offsets of
     the first body byte; a normal-message fixture proves the convention.
   - Its finding uses `Rule::HeaderBoundaryAmbiguous.finding(0)` and returns
     through the same path as `limits_exceeded`.
2. **`Scan`.**
   - It gains `has_a`, `has_t`, `has_x` and `numbers_ok`.
   - `numbers_ok` is false when a `t=`, `x=` or `l=` value, with its FWS
     trimmed, is empty, has a non-digit, or does not fit `u64`.
   - Row a becomes:
     `duplicate || !has_v || !has_a || !numbers_ok`.
   - Row d becomes: `rsa-sha1 || (has_l && l == 0)`.
   - Row f becomes: `has_x && has_t && x <= t`.
   - Row g becomes: `has_x && x <= receipt`.
   - Once `numbers_ok` holds, the crate's `x`, `t` and `l` equal the literal
     values.
3. **Pairing.** `Field.parsed` becomes `Result<&Signature, &Error>`, which
   drops the index. The output is found with
   `outputs.iter().find(|o| o.signature().is_some_and(|s| ptr::eq(s, sig)))`,
   then `.expect("every parsed signature has one output")`. `Context` loses
   `first_parsed`.
4. **`owner_of(name: &str) -> Option<OwnerName>`.** It strips one trailing dot,
   then calls `OwnerName::parse`. The adapter's `get` calls it, and so does row
   l, with `signature.domain_key()`.
5. **Hints.**
   - The `malformed` hint lists the row-a causes.
   - `header_boundary_ambiguous`: "A whitespace-only line in the header block
     can end the headers at some receivers; remove it or fold it with text
     (RFC 5322 section 2.2.3)."
   - `unsupported` no longer mentions `l=abc`.
6. **Header comment** in `src/deliverability.rs`: the radius sentence from spec
   § Verification time, in place of "can only move verified to not verified".

## Research (decisions with their evidence)

1. **Backend.** mail-auth's features include no `ring`. `aws-lc-rs` enables
   `dns-hickory`. The aws-lc verifier accepts RSA keys of 1,024 to 8,192 bits.
2. **Offline injection.** `txt_lookup` reads `cache.get(name)` first. With an
   adapter that never misses, the resolver is never called.
3. **Clock.** `verify_dkim` passes `SystemTime::now()` to the `pub(crate)`
   `verify_dkim_`. Its check is `x == 0 || (x > t && x > now)`, run before the
   body hash. No public path takes a time (spec § Operator holds).
4. **Fields.** The crate's `dkim_signatures()` and filtered `errors()`, sorted
   by position (0125 implementation item 1).
5. **`l=`.** The crate counts raw bytes and hashes `min(l, len)` raw bytes. A
   literal `l=0` reads as absent (0125 item 2).
6. **Keys and failure wording.** Any base64 builds a key, and aws-lc's ed25519
   path words a failed check as a library error (0125 item 3).
7. **Time in fixtures.** `DkimSigner` stamps `t=` from the wall clock.
   Explicit-tag fixtures splice the tag list and re-sign it through
   `canonicalized_dkim_headers` and `SigningKey::sign`. Receipts are generated
   from the signature's own `t`/`x`. The residual fixture uses an absolute
   1970 `x=2000`.

## Data model

- `MAX_DKIM_SIGNATURES = 16` and `MAX_DKIM_SIGNED_HEADERS = 128`.
- `struct Scan { duplicate, has_v, has_a, has_t, has_x, has_l, numbers_ok: bool, h_names: usize }`.
- `struct Field<'m> { value: &'m [u8], parsed: Result<&'m Signature, &'m Error>, scan: Scan }`.
- `struct Adapter<'l> { lookups: RefCell<&'l mut Lookups> }`. It has no
  `invalid` set: `owner_of` is the single judge.
- `enum Rule`: 18 variants, each with a `const` id, severity and hint. A test
  binds the list to the guide's headings.
- `canonicalize` / `canonical_len(body, c, limit)`.
- The report types are unchanged.

## Contracts

- The report JSON shape is unchanged, and `rule_set_id` is `swarmail-html/6`.
- `findings` may carry `dkim.*` entries at `header`/`dkim-signature`
  locations.
- `dns.lookups` may be non-empty in production.
- No REST, MCP, OpenAPI schema or UI member changes. Only descriptions change
  (§ Live surfaces).

## Quickstart

```bash
swarmail serve &
curl -X PUT localhost:8025/api/v1/inboxes/qa/dns \
  -d '{"txt":[{"name":"sel._domainkey.example.com","values":["v=DKIM1; k=ed25519; p=<base64>"]}]}'
# send a message signed d=example.com s=sel to qa@… over SMTP
curl localhost:8025/api/v1/messages/<id>/deliverability   # no dkim.* finding; the key lookup recorded
```

The `await … no_findings_at_or_above=warning` example moves to #255 (AC11).

## Risks

| risk | mitigation |
|---|---|
| **The operator refuses 0125 §8** (hold 2) | The PR does not merge. Options B (`retain_dkim_signatures` plus one deterministic id for every `x=`-bearing signature after receipt) and B′ (an upstream timed API) are named in spec § Operator holds and 0125 §8. A new design return builds the one chosen. Nothing is self-accepted, and nothing merges on green |
| **The operator refuses the compiled-in resolver** (hold 3) | mail-auth 0.14.0 cannot build without a DNS backend (`compile_error!`). The only way to meet the literal brief is a different crate or upstream, which is an operator scope ruling. The two unreachability proofs stand meanwhile |
| **The boundary guard misfires on ordinary mail.** Offsets disagree by convention (for example, LF-only mail) | Both values are offsets of the first body byte. Fixtures cover CRLF, LF-only and no-body messages whose boundaries agree, and each asserts no `header_boundary_ambiguous`. If a real convention difference appears, normalise the comparison and test it; never drop the guard |
| **The boundary guard and unsigned mail** | "Signed" gates the guard. AC1's unsigned whitespace-line fixture proves no finding |
| **The pre-scan disagrees with the crate on FWS inside a numeric tag** | The scan trims FWS the way the crate's tag parser does. A fixture with folded `x=` digits verifies cleanly. A disagreement fails closed as `malformed`, never as a pass |
| **Identity pairing: the crate stores a copy, not a reference** | Verified: `with_signature(signature)` stores the `&header.header` reference that `dkim_signatures()` returns. If a crate upgrade copies it, the `expect` fails one analysis loudly (503) and the exact pin makes that a deliberate change |
| **`domain_key()` and the adapter's asked name differ** | Both come from the crate's one `domain_key()`. AC9's trailing-dot fixture asserts `selector_invalid` and no lookup |
| **Wall-clock residual** | The radius is stated, a pole fixture exists (absolute 1970 `x=`), AC7 has a radius fixture, and the operator holds the merge |
| **The `l=` interop gap** | Fail-closed and documented in the guide, 0125 and the spec, with the #255 instruction. No detector is built (return adjudication) |
| **aws-lc-sys on Alpine (docker smoke) and macOS** | The builder-source evidence is in the spec. If a daemon is present, run one bounded build: `timeout 600 docker build --cpu-quota=$(( $(nproc)/4 * 100000 )) .` or the closest bounded form the daemon accepts, and report the applied CPU limit. Otherwise CI's `docker smoke` and macOS jobs are the named CI-owned proof. On failure, add exactly the missing `apk` package. Never merge red |
| **Two rustls providers** | Every Swarmail TLS config pins ring. The existing real-HTTPS webhook and STARTTLS suites must stay green (AC13) |
| **RustSec on the new tree** | A CI-owned, network-backed job. An advisory needing an ignore is a second widening: stop and report |
| **Live-surface snapshot tests** (the MCP description, OpenAPI) | Update the snapshot with the text. A test that pins the old false text is corrected, not kept |
| **Coverage of new branches** | Each new branch has a fixture: the guard (three ways), each `Scan` flag, the `owner_of` arms, and the pairing `expect` (unreachable by construction, so it is an `expect`, not an arm). Exact 100% |
| **Load** | Optional. If run: `timeout 600 cargo test --locked --test deliverability -- --test-threads=$(( $(nproc)/4 ))` (at most nproc/4 workers, at most 10 minutes). Report the load applied. Nothing paid |

## Panel rulings carried into this plan

The spec's [return adjudication](spec.md#return-adjudication--2026-11-05-revision-1)
rules revision 1, and its
[first adjudication](spec.md#panel-adjudication--2026-11-05) stands as history.
In short:
- **Simplicity wins** on the guard's short-circuit, the narrow pre-scan, the
  pointer pairing, `domain_key()`, the honest work bound, no `l=` detector, no
  speculative Dockerfile change, and no new rule-set id.
- **Robustness wins** on "fits `u64`", the ARC fixture, the bounded docker
  build when a daemon exists, the radius fixture in AC7, and the conditional
  no-cache resolver proof.
- **Both** route the clock and the resolver to the operator as merge holds.
