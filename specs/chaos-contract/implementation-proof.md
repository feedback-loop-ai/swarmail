# Implementation proof — issue #181 current-main reentry

Local implementation evidence, 2026-10-25 (Europe/Sofia), run
`epic-192-issue-181-refuse-positi-9d907b26`. Parent: `f05b223c748cc0de8fe658dbac57d5d37db815ff`.
Independent engine verification/review and CI evidence remain separate.

## Implementation and compatibility

Reapplied the supplied five-file patch after verifying every packet checksum,
including patch SHA-256
`760c0cd47066e7969a64b09f3ba5fc5d12ff55b06a3344dddde865e419eb8543`.
Accepted 0070 governs the owner-local SeedRequest, ChaosConfig and ChaosRule
readers. Derived map-field decoding preserves the previous object semantics
and diagnostics. Private legacy sequence staging preserves old failures;
excess-element staging relies on serde_json rejecting unread input before
complete REST extraction. It is not a general deserializer guarantee.

Kept #197's tuple-returning `inbox_snapshot`, both named-query helpers and all
existing tests. A source comparison confirmed those helpers and the two named
query regression tests remain identical apart from whitespace. Added explicit
count GETs for #181 without changing the tuple contract. Mixed-defect seed
probes prove body refusal before query parsing, and query refusal before
construction/domain validation, against populated mail.

Production changes remain in `src/api.rs` and `src/chaos.rs`; handlers,
validators, setters, storage, SMTP, waits, pruning and extraction are unchanged.
No dependency, gate, routing or decision acceptance changed.

## Executed evidence

The initial focused baseline passed: API 71, MCP 44, webhook 27 and the OpenAPI
unit pin. Permanent positional tests failed against the original readers:
`[]` returned 200 and cleared chaos; a two-string seed array returned 200 and
inserted mail. Compatibility-only baseline runs passed the strengthened old
refusal tables, including raw duplicate/order cases, scalar and nested paths,
and short/wrong-type/excess arrays. Temporary skips excluded only known
positional successes and were removed before final testing; original readers
were restored to the implementation before final gates.

Final focused suites passed: API 74, MCP 44, webhook 27 and OpenAPI pin 1.
The full locked workspace suite passed, including 346 library tests. Its five
ignored tests retain their existing explicit/CI ownership; they were not
reported as executed.

| Criterion | Passing proof |
| --- | --- |
| AC1 | `the_served_openapi_publishes_the_shared_domain` pins bounds, closures, null alternatives, grammar clauses and envelopes; `machine_docs_are_served_from_the_binary` compares the real served document with the embedded JSON |
| AC2 | REST domain/type tables and `set_chaos_refuses_every_malformed_wire_value_by_name` reinstall a distinguishable multi-event baseline for each row and immediately compare complete GET state |
| AC3 | `https_target_is_delivered_over_tls` compares generated PEM bytes in PUT/GET, keeps secret redaction and proves verified real TLS delivery |
| AC4 | `a_wrong_typed_argument_refuses_by_name` and `a_malformed_arguments_container_refuses_before_any_effect` preserve seeded default-inbox count, IDs, records and raw bytes after each clear/seed refusal |
| AC5 | Seed, chaos and webhook tables assert literal complete existing paths/diagnostics, value-free native errors and distinct syntax/media/size statuses; webhook installed-state and credential-delivery proofs remain green |
| AC6 | Positional chaos/seed regressions cover 0070's exact bodies, every event, nested/excess sequences, raw duplicate/order and valid-earlier/invalid-later cases with immediate populated-state witnesses; `positional_seed_refusal_preserves_query_error_precedence` and all #197 tests pass |

Every local command in the [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
passed, including the exact coverage gate and selftest, publish-guard proof,
keyless seal proof and license check. Supplementary shim integrity, design
selftest, delivery tamper-boundary proof and fail-closed native-tls absence also
passed. The native-tls probe returned the explicit no-matching-package error;
other tool errors are not treated as absence.

The candidate-local `.forge/bin/brokkr` matched pinned SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Its `compile --bundle bundles/self` passed with bundle digest
`afdd9edbcedb5c472594e6773bdd0fd71ad7a853b1a10fefe0ca60b1f70a48f1`
and zero unruled policy valuations. Existing candidate routing is unchanged.

Coverage: **13981/13981 lines, 786/786 branches, 1527/1527 functions**;
exact zero missed in each dimension. Report evidence is under
`target/coverage/run.D8TiJz/`; run-local logs and statuses are under
`.forge/issue181-reentry/`. Those ignored receipts supplement this committed
record and do not fabricate journal history.

The five executed source files retained these SHA-256 digests after all gates:

```text
6d61d35c6bb562ef10c9184c69462f70f5b8b822178fbc2cc3ddd209a7e134ec  src/api.rs
483ecd8775513c3fe5c66a904c4c03d4fb3347509b919f0f0c81da072f6a0200  src/chaos.rs
ddf772bc8422292b79d6d758d01e552be8aa0163bb2f3849a2024626eb985da2  tests/api.rs
d76b5713855badfdc60d8b2f10ec9dc481a104127d5e12f03ed361d9591bc821  tests/mcp.rs
fcfd316fe94d4482e55a2ef6e6db0b703f391b143994d622239a765603f7da7e  tests/webhook.rs
```

## Handoff ownership

T11 is engine-owned independent verification/review through the unchanged
candidate bundle route; this implementer has not launched or claimed it.
CI retains chart/history/selftest, real Docker, network RustSec, tag-only rate
and privileged/platform port-80 proofs. Operator sealing and live GitHub
settings remain operator-owned. No remote service, journal or sibling worktree
was read; no push, merge, tag, publication, issue closure or acceptance occurred.
