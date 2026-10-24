# Metric parser proof

Date: 2026-10-24. Epic #192, issue #202. Status: draft specification.
Design baseline: `0dc6b9289e4441b3ef90a01cbe896667c3b075c5`.

## Feature and rationale

Finish the decoded inbox metric proof with in-repository Prometheus text
exposition conformance tests and no new dependency. Runtime decoding already
exists: request activity and await metrics must use the same decoded inbox
identity. The remaining proof covers exact escaping and literal control/bidi
bytes over the real HTTP request and scrape path.

Accepted [0065](../../docs/decisions/0065-decoded-inbox-metrics.md) governs
identity; accepted [0043 §6](../../docs/decisions/0043-mcp-origin-boundary.md)
governs escaping. This work preserves their public contracts and all frozen
guarantees. No new decision is proposed or number reserved. Metrics measure
activity, including refused requests; they do not certify admission or storage.

## Operator clarification — 2026-10-24

The operator dropped the upstream-Prometheus-parser proof criterion and
requires an in-repo strict test reader instead. This revision replaces the
former AC3/R1/tool-discovery prerequisite and its dependent tasks in this
same directory. Upstream binaries, installations, versioned adapters, media-type
fallback experiments and universal scraper compatibility are not requirements.
Missing upstream tooling is no longer a blocker. The former counterevidence
proposal task is retired; no new semantic amendment is authorized here.

Authority: `.forge/epic192/issue202/README.md` and `operator-rulings.md`,
verified against that directory's SHA256SUMS, respectively
`4631780e3639832da1e30c507a768dd831a46ec9a51663b2c49f62ff3eaec5f1` and
`ce354e19307d268f9e9ab1e8f36c1897fd3d56c8049cbf65ab1f8fed1dac61fa`.
Read both current positions in `.brokkr/state/design/positions/` and intake
`.brokkr/state/tasks/epic-192-issue-202-label-the-in--4de6ce2b.md`.
The verified packet supplies the current direction; historical failed-run
receipts and dated provenance remain historical, not prerequisites to repeat.
This revision does not independently authenticate a journal or clear the
inherited `has_security_residual=true`, `max_residual_severity=low` facts.

## Panel synthesis and rulings

Adopt simplicity's existing-file, zero-runtime-change approach: reuse
`tests/common/metrics.rs`, the two existing test owners and the request-owned
post-response hold. Adopt robustness's independent byte expectations,
whole-scrape parsing, exact attributable sample set and bounded lifecycle
proof. Both positions reject changes to settled identity and escaping.

There is one concrete scope conflict: simplicity proposes representative C1
fixtures; robustness proposes every C1 scalar and the enumerated bidi forms.
Choose the full finite matrix below. The smaller option saves fixture text
and test time but leaves scalar-specific omissions untested. The chosen cost
is a larger, reviewable table in existing tests, not a new Unicode framework.
Use held `/count` responses for extra fixtures, as simplicity recommends;
retain the original CR/NUL `/await` refusal and the space/non-ASCII awaits.
Extra awaits would consume cumulative slots without improving serialization
proof. Bidi follows actual validation, never an invented control policy.

Reject a second reader, parser crate, generated capture schema, generic HTTP
client, default process isolation and repeated historical-panel authentication.
Reuse the existing reader with independent positive/negative controls and
literal expected wire bytes. The accepted cost is a local emitted-subset
conformance claim, not external-parser certification. Require an explicit
per-executable cap-headroom audit and fixture-specific observations; process
isolation is a fallback only for demonstrated interference. A transient size
check is not a parallel-safety proof.

Reject sanitization, valid-UTF-8 raw fallback, repeated/whole-path decoding,
render-only key rewriting, compatibility aliases and post-validation accounting.
They would change accepted semantics. Preserve literal plus, malformed-percent
treatment, encoded slash as segment data, case/padding/Unicode, valid alias
coalescing and the accepted `%FF` versus `%25FF` collision. Preserve all-request
totals, middleware/RAII ownership, MCP identities, the cumulative 4096-name cap
and reserved LF overflow; the live request registry remains uncapped. Separate
metric families do not acquire an atomic-snapshot guarantee.

## Acceptance Criteria

1. **AC1 — Escaping and exact wire identity.** Independently authored fixtures
   exercise decoded backslash, double quote and LF through the selected URI
   segment, real router and real `/metrics` response. Assert exact expected
   sample bytes for `\\`, `\"` and `\n`, then recover the original label
   bytes with the existing strict test reader. Distinguish literal backslash-n
   from LF and include a combined escape case. Expected wire strings and
   decoded identities must not use the production renderer, percent decoder,
   Debug formatting or an encoder copied into tests.
2. **AC2 — Literal matrix and complete parsing.** Cover CR, NUL, every C1
   scalar U+0080–U+009F, and bidi U+061C, U+200E–U+200F, U+202A–U+202E,
   U+2066–U+2069 individually, plus a mixed escape/control/bidi value and
   the existing CR/NUL pair. URI spellings encode valid UTF-8. Assert literal
   UTF-8 wire bytes for these non-escape scalars and byte-exact recovered
   identities. Parse the complete untrimmed, correctly de-framed scrape;
   require exactly one fixture-attributable request sample at value 1,
   no encoded alias and no extra fixture-bearing sample in any family.
   Check both wire bytes and parsed sample sets; substring-only or selected-line
   parsing does not satisfy this criterion.
3. **AC3 — Independent and falsifiable reader.** Extend the existing reader
   control test with independently specified exact expectations. Retain
   rejection of illegal escapes, raw LF inside labels, unterminated quoting,
   missing delimiters and malformed/trailing sample syntax. A controlled wrong
   expected byte/identity or extra fixture sample must fail a concrete
   assertion, followed by restored-green. Timeout or build failure is not
   regression sensitivity. Claim only conformance to the emitted text subset.
4. **AC4 — Accounting and refusal lifecycle.** Observe extra fixtures using
   the existing request-owned hold after the real `/count` response exists.
   Retain the CR/NUL `/await` refusal witness. Controls (CR/NUL/LF/C1) keep
   their exact 400/body bytes and no active/cumulative await registration
   before, during or after; admitted backslash/quote/bidi names retain their
   actual count response. Release and join each request, then prove its live
   row is removed. No lock may span a scrape. Preserve hold drop/cancellation
   tests without claiming arbitrary client-disconnect cleanup from them.
5. **AC5 — Original issue criteria.** Retain two concurrent real parked awaits,
   one inbox with a space and one non-ASCII inbox. After observed readiness,
   one fully parsed stable scrape contains all six value-1 decoded rows across
   `swarmail_http_requests_in_flight`, `swarmail_awaits_active` and
   `swarmail_awaits_registered_total`, without encoded aliases. Seed and join
   both successfully, prove live cleanup and cumulative retention. Preserve
   helper and HTTP invalid/mixed-invalid UTF-8 tests: accepted 0065 requires
   whole-original-segment fallback, stronger than the issue's optional
   unlabelled allowance; extractor 400/body and no-registration stay unchanged.
6. **AC6 — Isolation and preservation.** Exclusive names, fresh stores,
   ephemeral ports, request-owned reached/release channels, bounded joins and
   shutdown pass the normal parallel suites. Prove cumulative fixture headroom
   below 4096 per executable; added `/count` cases register no awaits. No
   sleeps as synchronization, registry resets, global-total assertions,
   dependencies, license widening, new production seams or coverage exclusions.
   Preserve synchronous accept==stored, subscribe-before-scan, oldest-only
   pruning and ingest extraction. No storage/SMTP/webhook/TLS/routing repair.
7. **AC7 — Candidate-bound local proof.** Run the targeted suites in the plan
   and every [house gate](../../docs/house-rules.md#gates--part-of-every-change),
   exact zero-missed coverage and its selftest included, on the committed
   candidate. Verify the supplied engine digest, shim integrity and fail-closed
   native-tls absence. Record commands, exits, HEAD/tree and limitations;
   historical receipts cannot substitute. Design validation is structural and
   does not complete implementation tasks.

## Downstream delivery ownership

Retain candidate recipe routing and five separate protected results under
accepted [0058](../../docs/decisions/0058-codex-review-routing.md): Astra
correctness, official DeepSeek Flash security, Sonnet 5.5 tests, Astra
guarantees and independent Fable 5.1. Preserve the hard security hold and
unknown-fact refusal under [0050](../../docs/decisions/0050-delivery-gates.md).
The chief supplies no clearance or next-phase decision. Tests alone cannot
clear inherited true/low; fresh protected review owns disposition.

CI owns chart/history/selftest, Docker, network RustSec, tag-rate and real
port-80/platform proofs. The operator owns external settings, sealing and
merge. No local criterion requires macOS, namespaces, GitHub, network,
journals or sibling worktrees. No push, merge, tag, publication, issue closure
or decision acceptance. Commit unsigned under 0056.

## Historical evidence account retained — 2026-10-24

The following is the earlier chief's account, retained as history. Its read
claims and candidate references are not claims of fresh inspection by this
chief; the verified packet and current clarification above govern this run.

Read both run-local positions, current intake
`.forge/tasks/epic-192-issue-202-residual-re-e-7d8e6287.md`, stopped triage
`.brokkr/state/tasks/triage-first-continue-epic-192-i-72123301.md` and result
`.forge/results/f874c220-a8fa-4ac3-b56c-d8f088da2430.json`.
The full recovered packet retains these historical aggregate events:

- 581 / `a0d79d66efad4f680c3f1e4f3690c7803227605d4b26fbd42ab91a221b186717`.
- 796 / `d6640e1b01d6c3d364af29f1b062bbef0e629b518a9a1d432aa5484d41fdf9d8`.

Both carry `has_security_residual=true`, `max_residual_severity=low`,
`fixes_applied=false`, `spec_defect=false`. All eight member notes were read.
The newer five-role panel `ce193b6b-3136-4f1a-a56f-08334eea4bc4`, recovered
under `.forge/epic192/issue202/recovered-review-evidence/`, retains those facts
in every member on this baseline. Its copied inspection records seq 625,
`REVIEW-RESIDUAL-UNSAFE`, hard stop. All ten original receipt hashes match
their inventories. Copy integrity does not authenticate an unavailable source
journal; neither journal database was queried here.

Preserve original notes, completed documentation repairs, draft/acceptance
provenance, the finite manual 0065-to-0043 pointer audit, unrelated 0017
numbering ambiguity, historical environment restrictions, harness filename
limits and the possible concurrent-fixture false failure. New evidence is
appended and candidate-bound, never substituted into those historical records.
