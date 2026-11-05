# Plan: offline DKIM verification (#254)

Spec: [spec.md](spec.md) · Tasks: [tasks.md](tasks.md) · Decision:
[0125](../../docs/decisions/0125-offline-dkim-verification.md) (proposed).

**Revision 3, 2026-11-05.** The review returned the run to design a third
time, on two false sentences: the clock account missed the crate's
report-sampling read, and the boundary finding pointed at occurrence 0 when the
root list holds no DKIM-Signature field. The return work is in
[§ Revision 3](#revision-3-what-changes-and-which-finding-it-answers), with the
finding each change answers and its task. Revision 2's text below stays; where
revision 3 changes a mechanism, § Revision 3 rules.

**Revision 2, 2026-11-05.** The review returned the run to design a second
time. This plan is repaired in place again:
- the mechanisms that the revision 1 implementation (`38fd111`) corrected only
  in 0125's dated record are rewritten as built (technical context, design
  detail);
- the return work is in [§ Revision 2](#revision-2-what-changes-and-which-finding-it-answers),
  with the finding each change answers;
- [§ Revision 1](#revision-1-what-changes-and-which-finding-it-answers) stays as
  dated history.

The first plan's text is in git history at `8705096`, and revision 1's at
`eaa4178`.

## Technical context

- **Language and crate.** Rust 2024, a single crate. The evaluator lives in
  `src/deliverability.rs` (`evaluate_at(raw, receipt, &Inputs)`), beside the
  `html`, `links`, `spam`, `reach`, `dns` and `dkim` collectors.
- **The DKIM module.** `src/deliverability/dkim.rs`, as built in `38fd111`:
  - `Rule` (18 variants);
  - the adapter, `name_of` and `owner_of` (which is `OwnerName::parse`);
  - `AUTHENTICATOR` (`LazyLock`, no name servers);
  - `canonicalize` / `canonical_len`;
  - `Scan` (`duplicate`, `has_v`, `has_a`, `has_t`, `has_x`, `has_l`,
    `numbers_ok`, `h_names`);
  - `fields()`, the crate's view sorted by position;
  - `Context { outputs, body, receipt }`, `verdict` (identity pairing by
    `ptr::eq`) and `body_length`;
  - `collect(raw, root, size, receipt, lookups, out)`, whose guard is at
    ~l.672 (`.filter(|message| message.body_offset() == size)`);
  - the inline suite: a signing kit, 18 rule pairs, the c= matrix, the RFC 8463
    vector (its two published keys are in `rfc8463_appendix_a_verifies_both_signatures`),
    the 55,987-body oracle and the boundary tests (`stray_line_after`,
    `stray_line_before`, `boundary_*`).
- **Shared boundary.** `headers(raw) -> (Headers, size)` in
  `src/deliverability.rs`. `Headers` is `Vec<(Vec<u8>, Vec<u8>)>`: owned,
  lowercased names and unfolded values, **no positions**. It caps at 64 KiB
  (`MAX_HEADER_BYTES`), treats a line starting with SP/HTAB as a continuation,
  skips a colon-less line, and keeps a field only when its name is non-empty
  and every byte is in 33..=126 (so `DKIM-Signature :` is dropped).
  `evaluate`'s `collect` passes `&fields` and `size` to `dkim::collect`
  (~l.923).
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
  - The header parser folds a whitespace-only line inside the block into the
    previous field, as Swarmail does (`headers/parse.rs`, `scan_value`). It
    ends the headers at a CRLF line with no colon that does not start with
    SP/HTAB, and a whitespace-only first line makes `parse_with_opts` return
    `None` (0125 implementation of the return revision, item 1; *revision 1's
    text here was wrong*).
  - Its folded-name scanner accepts names `headers()` skips, such as
    `DKIM-Signature :` (0125 implementation record, item 1).
  - A missing `a=` defaults to rsa-sha256; `number()` reads non-digits as 0 and
    saturates overflowing digits to `u64::MAX` (*revision 1 said "as 0"*).
  - It reads `x=0` as no expiry, so a `t=1000; x=0` field reaches key fetch;
    with `r=y` it asks `_report._domainkey.<d>` on a failing signature.
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

## Project structure (files touched by revision 3)

| path | change |
|---|---|
| `src/deliverability/dkim.rs` | `record()` loses its `_report._domainkey.` arm, and the `DkimReportRecord` import goes with it; `record()`'s doc comment is rewritten; `Rule::located(Option<u64>)` behind `finding(usize)`; the guard's refusal is located at `(root_count > 0).then_some(0)`; tests: the adapter's exact `_report` variants, the `rp=0` leg, the `s=_report` pass, the lone spaced-field fixture, value-order assertions, strict/full field equality |
| `tests/deliverability.rs` | exact vectors for the `nokey` and `tempfail` legs; a comment on the `lookups == []` pin |
| `docs/guide/deliverability.md`, `src/static/llms.txt`, `src/static/openapi.json` | the `null` location sentence; the unreachable sampling read; the await warning; Known limits without the `_report` selector; llms.txt's field-name trigger |
| `Cargo.toml` | the mail-auth comment only |
| `docs/decisions/0121-deliverability-link-checks.md` | a dated 2026-11-05 integration note on §7 (no edit of §7) |
| `docs/decisions/0125-offline-dkim-verification.md` | revised by the chief in this design commit (header, §5, §8, return revision 3); the implementer appends evidence only |

Not touched: `Cargo.lock`, `evals/`, `deny.toml`, dependencies or features,
`.github/`, `scripts/`, `.brokkr/`, accepted 0085, the shared `headers()`
scanner, `RULE_SET_ID`, and any schema member.

## Project structure (files touched by revision 2, history)

| path | change |
|---|---|
| `src/deliverability/dkim.rs` | the count conjunct in `collect`'s guard; the `header_boundary_ambiguous` hint gains the name clause; the module and `collect` comments name the real triggers; tests: the count-mismatch fixture, the AC22 premise and agreement tests, the corrected `rule_signature_mismatch`, the lookup pin in `x_zero_with_t_is_malformed`, the derived `x=` in `forged_signatures_verify_for_both_algorithms` |
| `src/deliverability/dns.rs` | module comment: bytes, receipt time and configuration, except 0125 §8 |
| `src/mcp.rs` | `MODERN_REPORT_DESCRIPTION` wording (spec § Live surfaces) |
| `src/static/llms.txt`, `docs/guide/deliverability.md` | the #253 "no verification" paragraph; occurrence k as the root occurrence; the boundary rule's fourth trigger |
| `src/static/openapi.json` | none expected: `DeliverabilityLocation` already defines a root occurrence, which the conjunct makes true; edit description text only if a sentence contradicts the spec |
| `Cargo.toml` | the rustls comment only |
| `deny.toml` | the CDLA-Permissive-2.0 comment only (already an operator hold) |
| `docs/decisions/0125-offline-dkim-verification.md` | revised by the chief in this design commit; the implementer appends evidence only |

Not touched: `Cargo.lock`, `evals/`, the `deny.toml` allowlist, dependencies or
features, `.github/`, `scripts/`, `.brokkr/`, accepted 0085, any snapshot of
report bytes other than what the hint change forces, and the shared `headers()`
scanner.

## Project structure (files touched by revision 1, history)

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

## Revision 3: what changes, and which finding it answers

Members: **G** guarantees, **C** correctness, **I** independent, **S**
security, **T** tests. The spec's
[revision 3 adjudication](spec.md#return-adjudication--2026-11-05-revision-3)
gives the rulings. Every finding the panel reported at or above low, and every
info finding with a concrete correction, has its own task in Phase U.

| finding | severity | answer | where | task |
|---|---|---|---|---|
| S: a second ambient clock read (`is_within_pct`, report sampling) missing from the clock account | spec defect; low security residual | delete `record()`'s report-record arm, so the sample is unreachable; state every crate clock read, reachable or not | spec § Key source, § Verification time, § Operator holds, AC24; 0125 §8 (this commit) | U1, U2, U5 |
| I1: `header_boundary_ambiguous` at occurrence 0 when the root holds no DKIM-Signature field | low; spec defect | occurrence `null` in that case, 0 otherwise | spec step 4, rule table, AC17, AC22, AC25; 0125 §5 (this commit) | U3, U5 |
| G3: 0121 §7 still says the evaluator is free of clock and resolver access | low | a dated integration note on 0121 | spec § Scope, AC21 | U6 |
| C2: `fields()` value order unasserted; the empty value's position never checked | low | assert the root list equals the crate's list by unfolded value in the combined message, with an empty value between two fields | AC22 | U4 |
| C3: strict and full parse counts assumed equal | info | a test, not a `debug_assert!` (adjudication) | AC26 | U4 |
| C4: the wire test's `lookups == []` reads as a contract | info | a comment marking it a measured ask order | AC26 | U4 |
| T: `[0]` indexed reads on the `nokey`/`tempfail` wire legs | info | exact-vector equality | AC26 | U4 |
| I (info b): the `Cargo.toml` hickory comment | info | reworded | spec § Live surfaces | U5 |
| G1, C1, I (info a), S: the crate's `x=` read against accepted 0085 §8 | HIGH (held) | unchanged hold 2, now covering exactly one reachable read | spec § Operator holds; 0125 §8 | U7 (PR body) |
| G2: the compiled resolver | MEDIUM (held) | unchanged hold 3 | 0125 §2 | U7 |
| robustness F4: three holds could be partially accepted into a contradictory record | position | spec § Operator holds and 0125's header: one merge question | spec, 0125 header (this commit) | U7 |

**No criterion was weakened.** AC15 loses one known limit (the `_report`
selector) because revision 3's code removes the limit. AC24's `s=_report`
fixture asserts the stronger behaviour: a pass with the key declared. AC17 and
AC22 gain the `null` case. The criterion "never a finding at an occurrence the
root list does not hold" was sound, so the code moves to meet it. 0125 §8's
"one read" sentence was sound once true, so the code moves to make it true.

### Design detail for revision 3

1. **`record()`** keeps two arms:
   `if !name.contains("._domainkey.") && name.contains("._atps.") { AtpsRecord::parse_records(..).into() } else { DomainKey::parse_records(..).into() }`.
   Delete `DkimReportRecord` from the `use mail_auth::…` list. Adapter's
   `answer` is unchanged: it records the name, then parses, so `dns.lookups`
   bytes do not move. The doc comment says the record type is chosen by name
   because the crate asks the cache by name only. A report record is never
   built, because the crate's sampling read is reached only through one
   (0125 §8). An `_atps`-shaped name with no `_domainkey` label is still read
   as ATPS.
2. **`Rule::located(self, occurrence: Option<u64>) -> Finding`** holds the
   body of today's `finding`. `finding(self, occurrence: usize)` becomes
   `self.located(Some(occurrence as u64))`. In `collect`, the guard's `else`
   arm becomes
   `return out.push(Rule::HeaderBoundaryAmbiguous.located((root_count > 0).then_some(0)));`.
   `root_count` is already computed above it. No other call site changes.
3. **Measure, don't guess.** The adapter test's two `_report` cases assert the
   exact `TxtRecord` variant that `DomainKey::parse_records` gives for
   `v=DKIM1; ra=postmaster` and for `nonsense`. Run it once, read the variant,
   and pin it. Name the variant (`DomainKey(_)`, or `Error(..)` with its exact
   error), never `DkimReport`.
4. **`s=_report` fixture.** Sign with the kit using
   `Sign { selector: "_report", ..Sign::default() }` (the kit already has the
   field), and declare the kit's key at `_report._domainkey.example.com`.
5. **Order assertion.** In `crate_lists_every_root_dkim_field`'s combined
   message, insert the empty-value case between two parseable cases (the
   existing `cases` order already does this if the empty value is not last;
   check it). Replace the two count assertions with
   `assert_eq!(root.iter().map(unfold).collect::<Vec<_>>(), ours.iter().map(unfold).collect::<Vec<_>>())`.
   Hoist `unfold` out of the loop.
6. **Strict/full equality** (`strict_and_full_parses_list_the_same_fields`):
   take a message with an `l=` kit signature, a non-`l=` kit signature and a
   missing-`bh=` field. Assert that `fields(&strict)` and `fields(&full)` have
   equal lengths, and equal `value.as_ptr()` offsets into `raw`, position by
   position.

## Revision 2: what changes, and which finding it answers

Members: **G** guarantees, **C** correctness, **I** independent, **S**
security, **T** tests (T found no defect). Each row names the owning artifact
and the task that carries it to code. The spec's
[revision 2 adjudication](spec.md#return-adjudication--2026-11-05-revision-2)
gives the rulings.

| finding | severity | answer | where | task |
|---|---|---|---|---|
| G3a, S (spec defect), C2, I2: boundary trigger text says a whitespace-only line after/before the field | spec defect | step 4, the rule-table row and AC17 name the real triggers; the inside-block whitespace line is a passing fixture | spec step 4, rule table, AC17 | S2, S7 |
| G3e, S: "the crate's header block then equals Swarmail's" | spec defect | deleted; a passing guard proves the body and (with the conjunct) the DKIM field list, nothing else | spec step 4 | S2 |
| C3, I3: occurrence k counts the crate's fields, not the root's | LOW | count conjunct in the guard makes k the root occurrence; premise test; docs say root occurrence | spec step 4, rule table, AC22 | S1, S2, S3, S7 |
| I4: fail-open corner (root field, no crate field, offsets agree) | info | closed by the same conjunct | spec step 4, AC17 | S2 |
| G3b: `owner_of` strip-then-parse | spec defect | `owner_of` is `OwnerName::parse` alone (code already right) | spec § Key source; this plan's detail 4; tasks R5 corrected | none in code; R5 text corrected |
| G3c: "overflow reads as 0" | spec defect | saturates to `u64::MAX`; row a refuses both | spec row a; adjudication r1 annotated | none |
| G3d, G6: lookup enumeration false; 0125 §3 overbroad | spec defect / info | per-row enumeration deleted; contract = guaranteed absences + pinned measured presences; 0125 §3 narrowed to in-grammar names | spec § Report and version, AC9; 0125 §3 (this commit) | S5 |
| S: `rsa-sha1` fixture "re-signed" | spec defect | "rewritten, not re-signed" | spec rule table | none |
| C4, I6: `rule_signature_mismatch` asserts passes under a failure comment | LOW | same-type different key for both algorithms with RFC 8463's published keys; vacuous lines removed | spec rule table, AC5 | S4 |
| robustness 2.7: a passing fixture hardcodes `x=9000000000` | low (0020 §5) | derive `x` from the signer's `t=` | AC23 | S6 |
| G4: guide/llms "no DNS/authentication verification … ships here" | LOW | rewritten | spec § Live surfaces, AC15 | S7 |
| G5: MCP "one finding per signature that does not verify" | LOW | OpenAPI's wording | spec § Live surfaces, AC15 | S7 |
| G6: `dns.rs` "pure function of the stored bytes and the configuration" | info | receipt time plus the 0125 §8 exception | spec § Live surfaces, AC15 | S7 |
| C5, I5: `Cargo.toml` "rustls/ring everywhere"; `deny.toml` CDLA comment | info | comments corrected; no startup provider install (adjudication) | spec § Live surfaces | S7 |
| C6: AC11 await clause moved by text only | info | 0125 records that #255's intake inherits it as a required criterion | 0125 (this commit) | none |
| G1, G2, C1, I1, S residual: clock vs accepted 0085 §8; compiled resolver | HIGH (held) / MEDIUM | unchanged operator holds 2 and 3; not re-argued, not self-accepted | spec § Operator holds, 0125 §2/§8 | S8 (PR body) |
| I7: aws-lc-sys on macOS/Alpine, RustSec | info | CI-owned, unchanged | plan § Risks | none |
| stale plan/tasks (G3 correction list) | spec defect | technical context, detail 1/4/5 and tasks R2/R5 corrected; R1–R12 checked off as delivered | plan, tasks | none |

**No criterion was weakened.** AC17's first two failing fixtures change because
the finding showed them wrong: mail-auth 0.14.0 folds an inside-block
whitespace line, and `boundary_whitespace_line_inside_the_block_is_no_difference`
proves the offsets agree. The real triggers replace them, and the old fixture
becomes a passing one, so AC17 now covers more cases than before. The
lookup enumeration is replaced, not loosened: two of its rows were measured
false (`failure_reports_ask_for_the_report_name`, and the key lookup of
`t=1000; x=0`), and the replacement pins both measured cases exactly.

## Revision 1: what changes, and which finding it answers

*Dated history: revision 1's table, delivered in `38fd111`. Where a row's
mechanism was later corrected, § Revision 2 and the technical context rule.*

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
   - *(Revision 2.)* It also requires
     `first.len() == root.iter().filter(|(n, _)| is_dkim_signature(n)).count()`.
     `first` is the strict parse's list, which includes strict-rejected `l=`
     fields through `errors()`, so like is compared with like. Shape:
     `.filter(|m| m.body_offset() == size && first.len() == root_count)`.
     The guard runs before the bounds, so 17 valid fields still give
     `limits_exceeded` (both lists hold 17).
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
4. **`owner_of(name: &str) -> Option<OwnerName>`** is `OwnerName::parse`, which
   already strips exactly one trailing dot; `owner_of` strips none of its own
   (*revision 1's "strips one trailing dot, then parses" was wrong*). The
   adapter's `get` calls it, and so does row l, with `signature.domain_key()`
   unchanged.
5. **Hints.**
   - The `malformed` hint lists the row-a causes.
   - `header_boundary_ambiguous` (as built, plus the revision 2 clause): "A
     line in the header block that is not a header field (no colon, or only
     whitespace at the very top), or a DKIM-Signature field name that not every
     parser accepts (such as a space before the colon), ends or splits the
     headers differently at different receivers; remove it, fold it into the
     previous field, or write the name as `DKIM-Signature:` (RFC 5322 sections
     2.2 and 2.2.3)." The exact words are the implementer's; they must name
     both conditions the code tests.
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
| **The count conjunct's premise is false** (the crate drops a field Swarmail names `dkim-signature`) | AC22's spelling table runs first (task S1). A single dropped field makes counts differ, which refuses (fail-closed). Only a drop and an extra field in one message could mislocate, and the premise test rules out the drop. If any spelling fails, the implementer stops and reports; the guard is never weakened and the shared `headers()` scanner is not changed |
| **The conjunct refuses legitimate mail** | Only a DKIM-Signature field whose name is not RFC 5322 syntax (a space before the colon) can make counts differ while offsets agree. Every existing passing fixture, the c= matrix, the RFC 8463 vector and the 16-field fixture must stay clean; the full suite is the proof |
| **The count and the re-parse disagree** (the non-strict `l=` re-parse lists a different number of fields than the strict one) | Both parses read the same header block; the re-parse only moves `l=` fields from `errors()` to `dkim_signatures()`. The existing `l=` tests and `precedence_and_occurrences` assert exact occurrences across a re-parse |
| **Spec text drifts from the code again** | The spec header's standing rule: an implementation record that corrects a mechanism edits spec.md in the same commit (task S8 checks it). No checker script is added (adjudication) |
| **A future rustls builder without a provider** (ring and aws-lc-rs both compiled) | The `Cargo.toml` comment states that every rustls config names ring explicitly. No `install_default` call is added; review catches a new call site |
| **The operator refuses hold 2 or 3 after revision 2** | Unchanged from revision 1's rows above: the PR does not merge, and the operator rules on B/B′ or another crate |
| **Load** | Optional. If run: `timeout 600 cargo test --locked --test deliverability -- --test-threads=$(( $(nproc)/4 ))` (at most nproc/4 workers, at most 10 minutes). Report the load applied. Nothing paid |

## Revision 3 risks

| risk | mitigation |
|---|---|
| **The sample's unreachability rests on a crate internal** (`unwrap_txt` rejecting a foreign variant) | mail-auth is pinned exactly (`=0.14.0`). AC24's adapter test, `rp=0` leg and `DkimReport` grep make the property checkable, and an upgrade re-runs all of them. A crate upgrade that changes `txt_lookup` is a reviewed dependency change, held for the operator anyway |
| **A `_report`-named selector changes verdict** (`key_invalid` becomes a real key verification) | Intended: the old verdict came from typing records by name, and it was a documented known limit. Only the never-shipped `/6` bytes move. AC24's passing fixture proves the new verdict |
| **The `_report` lookup answer changes type** (`DomainKey` parse, not a report record) | `dns.lookups` records the name before the parse, so the bytes are identical. `failure_reports_ask_for_the_report_name`'s `rp=100` and `rp=0` legs assert the same verdict and lookups |
| **A consumer does not expect `occurrence: null` on a DKIM finding** | `null` is already in accepted 0085 §1 and OpenAPI's `DeliverabilityLocation` schema. The MCP structured output schema already admits it (`src/mcp.rs`, the header location's `occurrence` is `anyOf` a coordinate or `null`). The guide, llms.txt and OpenAPI say when it occurs |
| **`located`/`finding` leaves an uncovered arm** | Both functions run on every boundary fixture. Both values of `root_count > 0` are taken: by the existing fixtures and by AC25's lone-field fixture. Exact 100% |
| **The strict/full equality test pins pointer offsets that a crate upgrade moves** | It is meant to fail on such an upgrade. Fix the occurrence mapping first, then the test |
| **Partial acceptance of the holds** | Spec § Operator holds and 0125's header say they are one merge question. The PR body says the same (U7) |
| **Load** | None needed. If run: `timeout 600 cargo test --locked --test deliverability -- --test-threads=$(( $(nproc)/4 ))`, at most nproc/4 workers and 10 minutes. Report the load applied |

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

Revision 2 (spec's
[revision 2 adjudication](spec.md#return-adjudication--2026-11-05-revision-2)):
- **Simplicity wins** on the count conjunct over root mapping with
  `occurrence: null`, on deleting the per-row lookup enumeration, on no
  checker script, and on no startup provider install.
- **Robustness wins** on pinning the measured lookup presences by exact tests,
  wrong-key proofs for both algorithms, the derived `x=` fixture, and AC15's
  grep binding the false sentences themselves.
- **Both** fold the record into the spec and keep the three holds.

Revision 3 (spec's
[revision 3 adjudication](spec.md#return-adjudication--2026-11-05-revision-3)):
- **Simplicity wins** on deleting the report-record arm over normalising `rp`,
  and on a test rather than a `debug_assert!` for parse-count equality.
- **Robustness wins** on stating every crate clock read, reachable or not, on
  one merge question for the three holds, and on the guide's await warning.
- **Both** place the boundary finding at `null` only when the root list holds
  no DKIM-Signature field, add the 0121 note, and tighten the order and wire
  assertions.
