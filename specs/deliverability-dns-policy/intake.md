# Intake — epic #251 issue #253: deliverability DNS policy

Run: `issue-253-the-deliverability-dns-c704444e`; phase: intake; 2026-11-03; base main 8732445.
Decision: **0120**, proposed (number assigned by the dispatcher; not taken from the index).
Result: `resolved`. Nothing blocks the framing: no frozen guarantee (0001–0003) is contradicted,
and the semantic change arrives with its decision document (0120).

## Goal

Give the authentication stories (#254 DKIM, #255 SPF/DMARC, #256 ARC) one Swarmail-owned DNS
policy source: a declared TXT map by default (per server by file, per inbox by API), offline and
deterministic, with real DNS reachable only through explicit configuration and refused until #254
supplies a resolver — and every deliverability report recording which policy and which records it used.

## What was read

- `docs/house-rules.md`: frozen 0001–0003, the gate inventory, dependency policy, delivery route.
- `docs/decisions/0085-deliverability-report.md` (accepted): lazy analysis of stored bytes; Store-owned
  Weak-identity cache keyed by exact immutable config and rule-set id; §8 — #253 extends the pure
  collector with explicit immutable policy inputs, adds their full identity to the key, and owns
  expiry, refresh and wake semantics.
- `specs/deliverability-report-contract/spec.md`: §Contract (literal `{rule_set_id, findings}` shape,
  `deny_unknown_fields` on Report and Finding); §#253 extension seam (no registry, no plug-in trait,
  no live DNS inside the evaluator, complete immutable snapshot in the cache key).
- Epic #251 and the 2026-10-26 operator rulings: the epic body is not in the repository. The task
  block carries the ruling: authentication builds on an existing permissive crate such as `mail-auth`,
  widened once with justification, and only if real DNS genuinely needs a resolver.
- Code: `src/deliverability.rs` (`Report`, `Finding`, `Location`, `Severity`, `AuthEvidence` whose
  facts default to `Unknown`, `AnalysisConfig`, `Inputs`, the cache and job limits, `RULE_SET_ID`
  currently `swarmail-html/3`); `src/deliverability/html.rs`; `src/deliverability/concurrency_tests.rs`;
  `src/api.rs:864,2097` (REST route); `src/mcp.rs:457,724,1074,1140` (MCP tool and output schema);
  `src/config.rs` (`ServeArgs`); `Cargo.toml` and `deny.toml`.

## Intake findings that shape the framing

1. **No TXT resolver exists in the tree.** std resolves only A/AAAA through getaddrinfo, and reqwest's
   resolver serves webhook connects. A live TXT lookup would need a new crate. **Ruling for this story:
   real-DNS mode is configuration plus refusal.** It performs no lookup, so it needs no resolver and no
   widening. #254 supplies the resolver and takes the `mail-auth` widening under the 2026-10-26 ruling.
   `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` stay unchanged.
2. **"Every report records which policy and records it used" changes the report's literal shape.**
   `Report` and `Finding` are `deny_unknown_fields`. The spec fixes `{rule_set_id, findings}`. Exact-JSON
   tests exist (`tests/deliverability.rs:103,533,624,857`, `tests/bulk_sender.rs:336,364`). The shape also
   appears in `src/static/openapi.json`, `src/static/llms.txt`, the MCP outputSchema
   (`src/mcp.rs:1074`) and the MCP description. 0085 is accepted and names the spec as normative for
   field names. 0120 must therefore amend the shape: an additive, bounded `dns` object, with
   `RULE_SET_ID` advanced to `swarmail-html/4` under the spec's own rule that shape changes advance
   the version. **Hold:** this changes an accepted ruling's normative shape, so the shape change is not
   in force until the operator accepts 0120. See "Operator holds" below.
3. **No hashing crate.** Do not add a digest crate to fingerprint a policy. Identify a policy by its
   canonical content, compared exactly as 0085's cache key already does, and report the canonical
   source and the records.
4. **Report size bound.** A report holds at most 256 findings and 65,536 compact JSON bytes. Embedded
   policy and records must fit those bounds, or the report fails explicitly (`AnalysisError::Failed`).
   Nothing is truncated.
5. **Sibling collision.** #258 (links) and #260 (spam rules) touch the same report surfaces. Put the
   DNS source in a new module `src/deliverability/dns.rs`. Use `src/deliverability/dns_tests.rs` only
   for cold paths, per decision 0020. Keep edits to `deliverability.rs`, `api.rs`, `mcp.rs` and
   `config.rs` small and contiguous.
6. **Per-inbox maps are new state beside the Store.** A map is snapshotted together with the selected
   message (0085 §4's selected-snapshot rule) and enters the cache key. Recommendation: in-memory and
   process-local, not persisted. Persisting would change the data-file format, which this brief does
   not ask for.

## Framing

**Goal (one sentence):** Add an offline-deterministic, Swarmail-owned DNS policy source (declared map
per server and per inbox, real mode configured and refused) that every deliverability report records,
with no DNS on the SMTP path and no new dependency.

### Acceptance criteria (each can fail)

- **AC1 — Default policy.** With no DNS configuration, every report records a policy whose source is
  `default` (empty declared map) and an empty records list. A test asserts the exact JSON for a message
  that carries a DKIM-style header and no DNS declarations.
- **AC2 — Server map.** `serve --dns-policy <FILE>` (env `SWARMAIL_DNS_POLICY`) loads a declared map.
  A malformed, oversized or duplicate-keyed file makes serve exit non-zero before it binds any listener.
  Test: a config/binary test asserting exit status, message and no bound socket.
- **AC3 — Per-inbox map.** One API call sets the map for one inbox, and one clears it. The inbox map
  overrides the server map for that inbox only. An invalid body gets a 4xx and changes nothing; other
  inboxes are unaffected. Test: real HTTP in `tests/deliverability.rs`.
- **AC4 — Lookup interface.** A Swarmail-owned typed lookup `resolve_txt(owner_name)` returns one of
  `Records(canonical list)`, `NoRecords` or `TempFail`. The declared map yields all three outcomes
  deterministically. No resolver, clock or network is touched. Test: unit tests over the map, one per
  outcome, plus a fixture with DKIM-selector, SPF and DMARC owner names.
- **AC5 — Real-mode refusal.** The real-mode flag (name chosen by design) parses. `serve` refuses to
  start with a fixed message naming #254, and binds no listener and issues no lookup. Test: exit status
  and message in a binary test.
- **AC6 — Recorded policy and records.** Every report, clean or not, carries the policy source and each
  record used (owner name, type, canonical value, outcome) in canonical order, independent of map
  insertion order. Test: two maps with the same content in different insertion order produce
  byte-identical reports.
- **AC7 — Determinism across restart.** The same message under the same configuration produces
  byte-identical report JSON from two serve processes. Test: two processes, byte comparison.
- **AC8 — Cache identity.** Changing an inbox's effective map makes the next report for the same message
  reflect the new policy; the old cached report is not served. An unchanged map reuses the cached slot,
  proven by an owner-local unit test that counts evaluations.
- **AC9 — Off the SMTP path.** A declared `TempFail` for a name does not affect delivery: DATA still
  returns `250`, and the message is queryable immediately. Test: `tests/` SMTP suite with an inbox map set.
  This covers 0001 and 0005.
- **AC10 — Rules and fixtures.** Every rule id 0120 introduces has one passing and one failing fixture.
  If the design adds no rule, this criterion is met vacuously and the notes say so. #253 owns no
  DKIM/SPF/DMARC verdict; #254/#255 do.
- **AC11 — Frozen and accepted rulings.** `tests/burst.rs`, `tests/p2.rs`, `tests/lifecycle.rs`,
  `tests/deliverability.rs`, `tests/bulk_sender.rs`, `tests/mcp_revisions.rs` and `tests/api.rs` pass.
  Test edits are limited to the rule-set id and the new `dns` field. No existing test is deleted to
  make a gate pass.
- **AC12 — Coverage.** `scripts/coverage-gate.sh` reports 100% exact (lines, branches, functions).
  `scripts/coverage-gate.sh selftest` passes. Every new production line is executed by a test.
- **AC13 — Decision record.** `docs/decisions/0120-*.md` exists with status `proposed`, number 0120,
  and a row in `docs/decisions/README.md`. The spec's §Contract and §#253 seam are updated. No decision
  is accepted, and no other number is used.
- **AC14 — No dependency change.** `Cargo.toml`, `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are
  byte-identical to main. `cargo deny check licenses` passes.

### Verify gate (all `--locked`; report each exit code)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked --bin swarmail
(cd evals && cargo fmt --all -- --check)
(cd evals && cargo clippy --all-targets --locked -- -D warnings)
(cd evals && SWARMAIL_BIN=<candidate swarmail from cargo build --locked --bin swarmail> cargo test --locked)
(for r in .brokkr/recipes/*/; do brokkr compile --bundle "${r%/}" || exit 1; done)
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

Each recipe compile's output is read for `lets the run go on without reading`. CI-only proofs
(the required contexts, the port-80 bridge, Docker smoke, RustSec) are not claimed by the local run.

## Design choices left to the design council (recommendations)

- **Owner-name keys.** Full lowercase owner names with trailing dot stripped, for example
  `s1._domainkey.example.com`, so the map key matches the lookup interface directly.
- **Flag and endpoint names.** `--dns-policy` / `SWARMAIL_DNS_POLICY` for the server map. Real mode as
  `--dns-resolver real`. Per-inbox as `PUT|DELETE /api/v1/inboxes/{inbox}/dns` with a JSON body.
- **Limits.** Declared map file at most 64 KiB; at most 64 records per report; owner names at most
  253 bytes; TXT strings with control characters refused. Excess fails the report explicitly, never
  truncates.
- **Keep DNS apart from `AuthEvidence`.** `AuthEvidence` holds trusted verdict facts that #254/#255 fill.
  The policy source is a separate input in `Inputs`, and both enter the cache key.
- **Interface shape.** A concrete typed enum or generic over a small trait, not a registry or dynamic
  plugin (spec §#253 seam). Real mode is a variant that is refused.

## Operator holds (do not block the framing)

1. **Report shape (0085 normative shape).** The `dns` field and the `RULE_SET_ID` bump amend an accepted
   ruling's normative shape. The operator should accept 0120 before this shape is treated as in force.
   Until then, the PR stays held for review. The assistant must not merge it on green alone. This is a
   hold, not a block: the code and the decision can be built and proven now.
2. **Real-DNS resolver.** Deferred to #254 under the 2026-10-26 `mail-auth` ruling. This story takes no widening.

## Notes for the implement and verify seats

- Commit unsigned (`commit.gpgsign=false` is repo-local). Never push or merge. Never touch gpg-agent
  (no autostart, no kill). Keep any test `GNUPGHOME` out of `$HOME/.gnupg`.
- Keep worktrees and cargo builds on `/home`, not `/tmp` (RAM tmpfs with a quota).
- No load proof is planned. If one is added, bound it: at most nproc/4 CPU workers for at most 10 minutes,
  as a named, timed command, with the load reported.
- Sibling stories #258 and #260 run in parallel on the same report surfaces. Keep to your own files and
  rebase onto main only with the operator's go-ahead.
