# Issue #151 part 3 implementation

The fixture-http helper now admits and discards `params._meta` without opening
other call fields or tool arguments. Existing initialize/ping/list handling and
notification silence remain intact. Every present GET body returns the fixed
tool error `http_get_body_not_supported` before HTTP; unrelated failures retain
`fixture_request_refused`. POST keeps its exact JSON representation, including
explicit null versus omission. No root implementation, dependency, lock,
manifest, routing or decision status changed.

Authority: the issue151 README, operator rulings, smoke and three MCP snapshots
passed the packet's complete SHA256SUMS check. The supplied candidate-local
engine matches ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f;
all pinned public shim hashes pass. The existing pinned design check passed
for design fixture-http-meta at ca1c9a2. No GitHub, CI, journal, sibling checkout,
authenticated model call or paid trial was used.

## Failing-first real-process evidence

Before modifying evals/src/fixture_http.rs, added the tests and ran these exact
commands offline, each with `--manifest-path evals/Cargo.toml --locked --test
wrappers`:

| Command suffix | Exit | Observation |
|---|---:|---|
| `actual_stdio_helper_accepts_request_metadata -- --exact` | 101 | Full reply equality fails: call IDs 4/5 return -32602/tool_arguments_invalid instead of HTTP tool results when metadata is present. |
| `actual_stdio_helper_names_get_body_refusal -- --exact` | 101 | Full reply equality fails: present GET bodies return fixture_request_refused instead of http_get_body_not_supported. |
| `actual_stdio_helper_preserves_closed_calls -- --exact` | 0 | Malformed and unknown fields retain protocol refusals and produce zero HTTP requests. |
| `actual_stdio_helper_bounds_metadata -- --exact` | 0 | Oversized metadata retains the whole-line cap, safe process failure and zero HTTP requests. |

Logs and saved exits are under ignored .forge/implement151-meta (plain
provenance path). These were behavioral assertion failures after successful
compilation and actual executable/loopback exchanges, not setup failures.

## Proof coverage

All three revisions run against fresh real fixture-http subprocesses and a real
port-zero HTTP listener. Metadata coverage includes omission, empty objects,
integer/string progress tokens, nested extensions and opaque non-object values.
Exact complete reply sequences check version, IDs, the single closed descriptor
and no progress/initialized replies. Concurrent pipe drainage, bounded EOF/exit
and explicit reaping precede stream inspection. The listener is joined before
final request counts; every path/header/body and both full output streams are
checked for synthetic metadata sentinels.

Negative controls cover unknown call fields with/without metadata,
arguments._meta, unknown arguments, required fields, wrong shapes, unknown tools
and unsupported methods. GET tests cover omission and nine present JSON bodies;
POST controls check exact bytes for the same values and omission. Existing
identity, URL/path/header, proxy/redirect, timeout, size and byte-fidelity tests
are retained. The tests add no mocks or sleep-based absence assertions.

After repair the focused actual_stdio_helper_ group passes all five tests.
Standalone eval fmt and locked all-target clippy pass. The full wrappers target
passes all 19 tests, including the existing process-cleanup regression (124.41s).
The first independent full eval command exited 101 after all 52 library tests
and 31 executable tests passed: boundary::sequential_contracts_and_decision_links_are_honest
still required decisions 0063/0076 to be proposed. The verified operator rulings
and both tracked records already say accepted (operator, 2026-10-26). Updated
only those stale test assertions to require the recorded acceptance and date,
without editing or accepting either decision. Full-suite and committed-candidate
house proof remains pending; no failed or interrupted command is counted green.

## Known delivery prerequisite

The baseline root decision-inventory test fails with
`0084-fixture-http-meta.md: no index row` (exit 101). The preceding design added
0084 without its required docs/decisions/README.md row. The design explicitly
assigns that integration to the operator/documentation owner; this task permits
changes only under evals/. No index change, gate weakening or green-delivery
claim is authorized here. Root tests and coverage must still be run and their
actual exits recorded. Decision 0084 remains proposed.

CI owns the charter's macOS/chart/Docker/network/privileged-port evidence; #201
owns eval macOS gating. The operator owns live Claude/client smoke, paid trials
and the baseline. None is claimed as locally proved.
