# Tasks: ARC chain reports (issue #256)

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md). Tasks are in order. Each
names the test that proves it. "Inline" means the `#[cfg(test)] mod tests` of
the named file, which the coverage gate measures (0020 rule 1). Wire tests
speak real SMTP, REST and MCP against a real server.

## Ground

- [ ] **T0 — Re-read the pinned crate.** Read `mail-auth-0.14.0`
  `arc/verify.rs`, `arc/parse.rs`, `arc/seal.rs`, `message.rs`,
  `dns/lookup.rs` and `dkim/output.rs` and check plan § Research items 1 to
  10. Stop and report, without building around it, if an ambient read on the
  ARC path can reach a verdict or a lookup, or if a lookup can miss the
  adapter.
  *Proof:* a short implementation record appended to 0132 (dated, status
  unchanged) lists every ambient read on the path and each Research item as
  confirmed or corrected.
- [ ] **T1 — Turn on `arc`.** `Cargo.toml`: features
  `["aws-lc-rs", "arc"]`, comment updated.
  *Proof (AC18):* `git diff --exit-code f05f2ee -- Cargo.lock evals/Cargo.lock
  deny.toml`; `cargo build --locked`; `cargo deny check licenses`;
  `cargo tree --locked -i native-tls` finds no package. Every existing test
  passes unchanged, `arc_and_rejected_fields_do_not_shift_pairing` included
  (AC12).

## DKIM's parse (`src/deliverability/dkim.rs`)

- [ ] **T2 — Keep the strict parse; guard the re-parse.** `Prepared::Ready`
  carries the strict parse beside the parse DKIM verifies, with one accessor
  ARC reads. `prepare` re-parses only when the `DKIM-Signature` fields with
  `l=` plus the `ARC-Message-Signature` fields with `l=` number at most 16;
  otherwise it pushes `dkim.limits_exceeded.v1` at the first `DKIM-Signature`
  field with `l=` and returns `Limited`. Update that rule's hint and the
  module comment.
  *Proof (AC13):* inline `reparse_is_bounded_with_arc_fields`: one DKIM `l=`
  beside 15 ARC `l=` fields keeps its earlier verdicts; beside 16 it gives
  exactly one `dkim.limits_exceeded.v1` at that field's occurrence and no
  other `dkim.*` finding. `strict_and_full_parses_list_the_same_fields` still
  passes.

## The module (`src/deliverability/arc.rs`)

- [ ] **T3 — Rule table and member.** `Rule` (thirteen variants: id, state,
  hint, `ALL`), `State`, `ArcChain` with its `Default`, and the finding
  constructor over `Location::Header`.
  *Proof (AC1):* inline `rule_table_is_closed`: thirteen distinct ids, each
  `arc.<name>.v1`, each hint non-empty and free of characters that need
  escaping, each state as the spec's table; `member_serializes_in_key_order`
  pins the JSON of one value per state.
- [ ] **T4 — The sealing kit.** Test-only helpers: seal hop `n` with its own
  selector, domain and key from the previous hop's passing `verify_arc`
  output; a policy builder that declares each hop's key; helpers to rewrite
  or forge one field. No kit fixture that must pass carries `x=`.
  *Proof (AC2):* inline `kit_seals_verifiably`: one-, two- and three-hop
  chains report `pass` with `sets` 1, 2 and 3, no `arc.*` finding, and
  exactly the hops' key names in `dns.lookups`.
- [ ] **T5 — Steps 0 and 1: none and the boundary.**
  *Proof (AC3, AC6):* inline `no_arc_field_is_none` (unsigned mail, DKIM-only
  mail, a message the crate cannot parse); `rule_header_boundary_ambiguous`
  (a colon-less line before and after the ARC fields; a leading
  whitespace-only line; `ARC-Seal :` beside a valid chain and alone, the last
  at `arc-seal` with `occurrence: null`; a message whose DKIM guard already
  stopped), each with exactly that one `arc.*` finding and empty lookups for
  ARC names; `boundary_runs_without_a_dkim_signature`;
  `crate_lists_every_root_arc_field` (every spelling the root scanner names
  an ARC family, list equality by unfolded value).
- [ ] **T6 — Steps 2 and 3: the bound and the field verdicts.**
  *Proof (AC5, AC9):* inline `rule_limits_exceeded` (128 names pass, 129
  fail); `rule_malformed` as a table: a repeated tag, no `a=`, `x=abc`,
  `t=abc`, no `i=`, `i=0`, `i=51`, `i=4294967297`, an
  `ARC-Authentication-Results` whose `i=` is not first, a seal with `h=`, a
  seal with no `cv=`; `rule_unsupported`: `l=5`, `l=0`, `a=rsa-sha1` on a
  signature and on a seal. Each asserts the located field's root occurrence,
  `failing_instance`, and that nothing was looked up.
  `first_field_in_wire_order_decides` pins the precedence between two bad
  fields.
- [ ] **T7 — Steps 4 and 5: complete sets, distinct instances.**
  *Proof (AC2, AC9):* inline `rule_set_incomplete`: no
  `ARC-Authentication-Results` for one instance; ARC fields with no
  `ARC-Message-Signature`; no seal. Each is `fail`, located at the short
  family with `occurrence: null`, with the lowest unequal instance as
  `failing_instance`, and never `none`. Inline
  `duplicated_instances_are_refused`: two complete sets that both say `i=1`,
  written in both orders, give `arc.instance_invalid.v1` at the second field
  carrying it with `failing_instance` 1; 51 forged sets give the same rule;
  none causes a lookup.
- [ ] **T8 — Step 6: expiry at receipt.**
  *Proof (AC7):* inline `rule_signature_expired_and_not_evaluated`: the newest
  signature rewritten with `x=1000` is `arc.not_evaluated.v1` at receipt 999
  and `arc.signature_expired.v1` at receipt 1000, both with empty
  `dns.lookups`; `t=` equal to `x=` is expired; a literal `x=0` is expired;
  an `x=` on an older signature changes nothing. No assertion reads the wall
  clock.
- [ ] **T9 — Step 7: verification, mapping and attribution.** One
  `verify_arc` call through a counting wrapper over the existing adapter;
  one `match` with the spec's arms and no other.
  *Proof (AC2, AC4, AC8, AC9):* inline, one test per arm:
  `rule_instance_invalid` (sets 1 and 3 name instance 2; a chain that starts
  at 2 names instance 1);
  `rule_cv_invalid` (`cv=pass` at 1, `cv=none` at 2, `cv=fail`);
  `rule_body_hash_mismatch`; `rule_key_missing` (the signature's key and a
  seal's key undeclared; a selector outside the grammar, with no recorded
  lookup); `rule_key_invalid` (a value that is no key record, a revoked key,
  a key of the other type); `rule_signature_mismatch` (a wrong key of the
  same type for `i=1` of 2 and for `i=2` of 3; the oldest seal's `b=` altered
  after the second hop sealed, which names instance 2);
  `rule_not_evaluated_is_never_clean` (a declared `tempfail` on the
  signature's key and on a seal's key; `qualifies` is false at every
  threshold); `occurrences_are_root_occurrences` (a chain written
  oldest-first).

## Wiring (`auth.rs`, `deliverability.rs`)

- [ ] **T10 — The member in the report; `swarmail-html/9`.** `auth::collect`
  returns the chain; `Report.arc` sits between `dns` and `total`;
  `finish_with` takes it; `RULE_SET_ID` and every literal that names the id
  move.
  *Proof (AC3, AC10, AC14, AC17):* inline `arc_member_invariants` runs every
  kit fixture through `evaluate_at` and checks the spec's five invariants;
  `arc_reports_are_byte_identical` evaluates each fixture on two fresh
  services; `arc_moves_no_fact` compares the six facts and every non-`arc`
  finding for a passing, failing and absent chain;
  `arc_verdict_ignores_dkim_l` (AC12); the existing shape and drift tests,
  updated; `grep -rn "swarmail-html/8"` finds only history.

## Surfaces

- [ ] **T11 — OpenAPI and REST.** The `ArcChain` schema, `required`, the
  report and route descriptions.
  *Proof (AC15):* the structural OpenAPI tests, updated; a test validates a
  passing and a failing report against the published schema.
- [ ] **T12 — MCP.** `outputSchema` member and `required`;
  `MODERN_REPORT_DESCRIPTION` names the four states and the `arc.*` rules;
  the structured-output fixtures regenerated, with hash and provenance.
  *Proof (AC15, AC19):* `tests/mcp_revisions` and the stdio report witness,
  updated; the description drift test.
- [ ] **T13 — Message page.** One escaped ARC line in the report section,
  and the section's stale "demo and bulk-sender checks only" wording
  corrected.
  *Proof (AC15):* inline `ui.rs` test for each state; `tests/ui.rs` for a
  stored chain.

## Wire proofs (`tests/deliverability.rs`)

- [ ] **T14 — Chains over real SMTP.** With a per-inbox declared policy:
  single-hop pass, two-hop pass, a seal at `i=1` that does not verify, a body
  modified after sealing.
  *Proof (AC2, AC15):* each report read over REST and MCP
  `structuredContent` with the whole `arc` member and the whole `arc.*`
  finding asserted, and the page line for the two-hop pass and the broken
  seal.
- [ ] **T15 — Accept == stored.**
  *Proof (AC11):* `an_arc_message_is_stored_before_its_report`: the `250`,
  an immediate read, and the row and raw bytes identical before and after the
  report. `cargo test --locked --test burst --test p2 --test rate` unchanged.
- [ ] **T16 — assert and await.**
  *Proof (AC16):* an exact assert and an await with
  `no_findings_at_or_above=error` do not count the broken-seal message; of
  two messages that qualify without ARC, the one given a valid chain still
  qualifies, and the await returns both.

## Records and docs

- [ ] **T17 — Guide, `llms.txt`.** An "ARC (`swarmail-html/9`)" section: the
  four states, the member, the thirteen ids with severity, state and hint,
  the evaluation order, the correspondence lines beside `spf=`, `dkim=` and
  `dmarc=`, the `x=` limit, the "newest seal that fails" note, the statement
  that `arc=pass` is not sender authentication, and the version history line.
  *Proof (AC1, AC19):* the guide-binds-code test for the `arc.*` table; the
  doc-contract tests.
- [ ] **T18 — Decision records.** Add the 0132 row to
  `docs/decisions/README.md` (proposed, two holds named). Append one dated
  pointer to accepted 0085 saying 0132's two items wait for the operator
  under 0045 section 8; its text above is unchanged. Change no status.
  *Proof:* `git diff f05f2ee -- docs/decisions/` shows the new record, the
  row and the appended pointer only; `bash scripts/design-check.sh check`
  still reads 0132 as proposed.

## Gates

- [ ] **T19 — Every house gate, `--locked`.** fmt, clippy, `cargo test
  --workspace`, the `swarmail` binary, the evals fmt, clippy and tests
  against that binary, the recipe compile loop, the coverage gate at exact
  100% and its selftest, publish-guard, seal-branch, `cargo deny check
  licenses`.
  *Proof (AC20):* the gate log in the delivery result, each command with its
  exit status. A gate that cannot run in the seat's box is named as not run,
  never reported as passed.
