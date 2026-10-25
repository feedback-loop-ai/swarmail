# Issue #181 reentry framing

Run: `epic-192-issue-181-refuse-positi-9d907b26` · Phase: intake · 2026-10-25

Goal: Reapply the supplied #181 implementation to this candidate so positional
REST seed and chaos bodies refuse before mutation under accepted 0070, while
preserving object behavior, existing diagnostics and every merged #197 behavior.

Disposition: resolved for intake. Implementation, gates and review remain pending.
The existing `spec.md`, `plan.md` and `tasks.md` remain the implementation contract;
this framing records the current-base integration constraint and proof requirements.

## Authority and bounded evidence

The issue packet's README and operator rulings, together with its patch and
feature records, passed `sha256sum -c SHA256SUMS` from
`.forge/epic192/issue181/`. Operator acceptance of 0070 on 2026-10-24 is
recorded both there and in `docs/decisions/0070-object-only-chaos-and-seed.md`.
The decision's historical blocked/proposed narrative does not override its
accepted header or the operator's ruling. No further acceptance is requested.

Read the house charter and decision index, relevant 0025/0029/0048/0070
rulings and #197's 0074 record. Spot-checks covered `ChaosRule`/`ChaosConfig`,
`SeedRequest`, `set_chaos`, `seed_inbox`, the existing HTTP snapshot/query
helpers, the supplied patch, spec and gate owners. Local history identifies
`4b0281e` as #197's integration and `56ac195` as the current spec commit.
No GitHub, CI, journal or sibling-worktree access is needed or claimed.

The patch is `.forge/epic192/issue181/0001-Refuse-positional-JSON-request-bodies.patch`.
Its five owners are `src/api.rs`, `src/chaos.rs`, `tests/api.rs`,
`tests/mcp.rs` and `tests/webhook.rs`. Reconcile its test helper changes with
main's existing `inbox_snapshot`, `assert_named_query_refusal` and
`assert_named_query_refusal_with_body`; retain one snapshot owner and every
#197 test. Do not change recipe routing: this candidate's `bundles/` governs,
including where an older spec names a different reviewer roster.

## Checkable acceptance

1. **Positional admission and state:** Real HTTP chaos PUTs with `[]`,
   `[null]`, `[null,null,null,null]` and `{"data":[0,"451 kept",0]}`
   return extractor plain-text 422. A seed POST with
   `["sender@example.test","recipient@example.test","positional subject","positional body","<p>positional html</p>"]`
   returns native JSON 400. Include empty/short seed arrays and complete,
   malformed and excess-element rule arrays at all four chaos events, as in
   spec AC6. Immediately after EACH refusal, GET proves the complete installed
   multi-event chaos configuration or keeper inbox count, IDs, records and raw
   bytes equal the populated baseline. No replacement or insertion is hidden
   by reseeding or a terminal-only read.
2. **Reader compatibility:** Successful `SeedRequest`, `ChaosConfig` and
   non-null `ChaosRule` admission is map-only at the existing owners.
   Literal regression fixtures retain existing refusal sentences, paths and
   envelopes for scalar/root, short-array, duplicate/unknown/missing-field
   and nested/order-sensitive cases. Use raw JSON for duplicate/order probes.
   New sequence refusals name the expected object/struct and applicable path
   without values. Object defaults/omissions, optional strings, scalar-null
   refusal, event-null clears, REST whole-config replacement and MCP single-event
   updates keep their prior behavior. Private legacy diagnostic decoding must
   never pass a positional success to construction, insertion or config swap.
3. **Probability and schema:** Preserve 0/100 success; REST 101..255 native
   JSON 400; negative, missing, null, fraction, wrong-type and overflow
   probability extractor 422; MCP HTTP 200/-32602 with its existing sentence.
   Each refusal retains complete populated configuration immediately afterward.
   Retain published OpenAPI bounds, required probability, both closed objects,
   null-capable events, delay default and every 510-UTF-8-octet reply-grammar
   clause. Constant/clause unit pins and real GET `/openapi.json` prove the
   served contract and its operation-local 400/422 descriptions (spec AC1/AC2).
4. **Existing #181 wire proofs:** Complete spec AC3..AC6 using the supplied
   patch: generated loadable CA bytes survive webhook PUT echo/GET verbatim;
   secret/userinfo redaction and real-sink credential delivery stay green;
   every refused MCP clear faces identifiable mail and retains exact count,
   IDs, contents/raw bytes; seed/webhook/chaos wrong-shape/path matrices keep
   literal existing diagnostics and immediate complete-state witnesses.
   Seed/webhook data errors remain value-free JSON 400; syntax 400, media 415
   and size 413 remain distinct. Positive default, null-clear and empty-list
   controls still pass.
5. **#197 preservation:** All existing strict-query tests pass unchanged in
   behavior, including `query_free_native_operations_refuse_by_name` and
   `native_query_refusal_preserves_error_precedence`. Seed body extraction/
   `json_body` errors still precede query refusal; query refusal still precedes
   construction/domain work and insertion. Reconcile helpers without deleting
   or weakening any query, state, precedence or documentation assertion.
6. **Candidate proof:** Every command below passes on the committed candidate,
   exact coverage has zero missed lines/branches/functions and its selftest
   passes. No new dependency, allowlist widening, coverage exclusion, gate or
   routing change is introduced. All implementation work is committed unsigned
   before verification/handoff; the existing engine owns later review/ship.

## Frozen-guarantee proximity

Nearest is 0001 (accept == stored): seed admission must refuse before construction
and insertion; no mutation-boundary/storage change is authorized. 0002
(subscribe before check) and 0003 (oldest-only pruning) are outside the change;
no await/watch/prune code changes are planned. Preserve 0005 ingest extraction
and existing chaos domain validation before its single swap. Accepted 0070
supplies the narrow exception to 0029 sections 3/6, while 0025 null/probability
semantics and 0048's local seed 400 mapping remain binding.

## Verify commands and ownership

Run focused proofs before the full inventory; no benchmark is required for
this admission-only repair. These suites use real servers, not mocks:

```bash
cargo test --locked --test api --test mcp --test webhook
cargo test --locked the_served_openapi_publishes_the_shared_domain
cargo test --locked --test api query_free_native_operations_refuse_by_name
cargo test --locked --test api native_query_refusal_preserves_error_precedence
```

The house inventory is authoritative. Use the candidate-local binary (digest
verified during intake) and the pinned script copies after validating the
public shims; these are the verify seat's executions of the house commands:

```bash
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

The pinned verifier also requires fail-closed TLS absence, preserving its
existing proof (an unrelated cargo-tree error cannot count as absence):

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf '%s\n' "$out" >&2
  exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2
  exit 1
fi
```

Record passing receipts against the final candidate; compilation must not emit
the policy-sweep warning refused by `bundles/self/scripts/verify-seat.sh`.
Coverage outputs belong under `target/coverage/`. These commands are required
implementation/verify work, not claims of gate execution by intake.

CI owns the charter's chart/history/selftest, real-Docker smoke, network RustSec,
tag-only rate and privileged/port-80 platform proofs. Operator sealing and live
GitHub settings remain operator-owned. No local macOS, namespace, network or
live-service acceptance is added. No push, merge, tag, publication, issue closure
or decision acceptance is authorized. Nothing required to frame this task is
missing from this workspace.
