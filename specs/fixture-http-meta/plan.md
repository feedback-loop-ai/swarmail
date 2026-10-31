# Plan: fixture HTTP metadata

## Technical context

Rust, existing serde/serde_json/reqwest and Tokio; no new dependency or version.
`evals/Cargo.toml` defines a standalone workspace. The existing sequential
fixture-http subcommand reads bounded JSON lines and exposes one tool to the
public fixture origin. Production owns call shape, HTTP admission and RPC
projection in evals/src/fixture_http.rs. Tests already spawn the actual child
and a real port-zero HTTP listener in evals/tests/wrappers.rs.

## Constitution check

Preserve accepted 0001–0005, closed arguments under 0044, privacy under 0075,
standalone proof under 0055, exact root coverage and all existing dependencies
and routing. No paid/model calls, ambient credential reading, root-server edits
or decision acceptance. Commit unsigned and never push. Decision 0087 records
a compatible helper wire-contract repair, not an amendment weakening a ruling.
The chief's four-artifact restriction leaves the decision index unmodified;
the root inventory gate needs operator-owned scope resolution before green
delivery. Do not alter tests, omit the decision or hide it to evade that gate.

## Research and resolved questions

The supplied three schema snapshots permit request metadata. The older
Request definitions own _meta; 2025-11-25 introduces RequestParams inherited by
initialize and tool-call params. The 2025-03-26 JSON schema does not close the
metadata object; the later TypeScript explicitly allows extension keys.
All supplied progress texts make receiver notifications optional. Existing
initialize/ping/list already tolerate metadata; notifications already remain
silent. Test these paths without adding a second validation owner.

Choose the named GET-body refusal over empty-string normalization. The body
reader deliberately preserves Some(Null), distinct from omission. Maintain
that behavior for POST and check presence, not truthiness, for GET.

The requested plan template is absent at .specify/templates/plan-template.md;
only an unrelated workspace spec template exists. This plan supplies the
required technical context, constitution, structure, research, data model,
contracts and quickstart directly; no external template is needed.

## Project structure and files

| File | Work |
|---|---|
| evals/src/fixture_http.rs | One discarded Call field; split GET-body refusal; project only its fixed diagnostic. |
| evals/tests/wrappers.rs | Extend current real HTTP capture and real stdio tests; no second harness framework. |
| evals/wrappers.md | Document metadata discard, optional progress omission and GET-body refusal. |
| evals/issue151-meta-implementation.md | Later implementation receipts: exact red/green commands, exits, logs and limitations. |
| specs/fixture-http-meta/{spec,plan,tasks}.md and docs/decisions/0087-fixture-http-meta.md | This chief's only committed artifacts. |

No implementation edits to earlier historical specs or accepted decisions.
The decision index requires separately authorized documentation integration;
it is not silently added to this file list's implementation scope.

## Data model and implementation

Add a private Call field renamed `_meta`, defaulted on omission and consumed
with serde::de::IgnoredAny. Keep required name/arguments and both existing
`deny_unknown_fields` declarations. No flatten map, retained metadata tree,
token parser, sanitizer, recursive key removal or arbitrary params admission.
Existing bounded Value parsing remains the transport owner; discard does not
bypass its byte/recursion limits. No persistence or migration.

Split the method/body branch in Helper::call: unsupported methods keep their
current refusal; GET with Some(body) returns `http_get_body_not_supported`.
In Helper::rpc map only that fixed reason to an isError text result. Map all
other failures to fixture_request_refused. Keep existing call reconstruction
from method/path/body; metadata never enters HTTP arguments. No broad error
string projection or new error registry is needed.

## Contracts and proof layout

AC1–AC6 in the spec are the test oracle. Proposed focused tests (names may be
adjusted consistently in receipts):

- `actual_stdio_helper_accepts_request_metadata`: revision/location matrix,
  full stream capture, exact descriptors/IDs, GET/POST and metadata sentinels.
- `actual_stdio_helper_preserves_closed_calls`: malformed/misplaced fields,
  unsupported tool/method and fixture request-count controls.
- `actual_stdio_helper_names_get_body_refusal`: all present JSON body kinds,
  omitted GET success, legitimate POST byte controls and generic-error control.
- `actual_stdio_helper_bounds_metadata`: oversized whole record over stdin,
  safe reader failure, no echo and no HTTP effect.

Reuse existing RuntimeDirs and environment setup; each test owns its child,
listener, temporary directories and channels. Capture all headers, not only
X-Eval-Identity. Drain stdout/stderr concurrently, close stdin, require bounded
EOF/exit and explicitly reap. When testing early reader exit, handle the pipe's
broken-write result without deadlock. Shutdown and join the listener/capture
owner before inspecting final request totals. Do not substitute sleeps for
causal completion or weaken existing cleanup/deadline tests.

## Quickstart and validation

Run offline with CARGO_NET_OFFLINE=true and install nothing. Before edits,
add focused tests and run each to demonstrate the intended behavioral failure;
record an assertion failure, not a compiler/setup failure. After repair run the
same commands, then the full independent suite:

```bash
cargo fmt --manifest-path evals/Cargo.toml --all -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --locked --test wrappers
cargo test --manifest-path evals/Cargo.toml --locked
```

Verify .forge/bin/brokkr SHA256 equals
ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f and verify
bundles/self/scripts/shims.sha256. Run the charter's entire local inventory:
root cargo fmt --all -- --check; cargo clippy --workspace --all-targets --locked
-- -D warnings; cargo test --workspace --locked; .forge/bin/brokkr compile
--bundle bundles/self; cargo deny check licenses; and pinned copies of
coverage-gate.sh, coverage-gate.sh selftest, publish-guard-test.sh and
seal-branch-test.sh under bundles/self/scripts/. Compile must have zero unruled
valuations and no policy-sweep warning. Additionally run pinned design-check.sh
selftest and delivery-proof-test.sh, and fail closed on native-tls absence:
`cargo tree -i native-tls` must fail specifically because the package is absent,
not because dependency resolution or another prerequisite failed.

After this design commit run `bash bundles/self/scripts/design-check.sh check .`.
Save gate commands/exits under ignored .forge/design151-meta; report failures
without claiming completion. Implementation reruns gates on its own committed
candidate; intake/design receipts are not implementation proof. Root exact
coverage remains mandatory and separate from standalone eval test evidence.

## Risks

| Risk | Mitigation and limit |
|---|---|
| Closed-field repair accidentally admits arbitrary arguments | Named discarded field only; unknown sibling and arguments._meta negative tests. |
| Opaque metadata leaks through diagnostics or HTTP | No data path; finite named error only; sentinel checks across complete streams and full HTTP capture. Coincidental matching legitimate tool bytes are not scrubbed. |
| Metadata bypasses argument-size tests | Exercise the real whole-line reader with oversized metadata; preserve cap before parsing and parser limits. |
| GET change corrupts null/empty POST bodies | Presence-based GET refusal; POST controls assert exact JSON body bytes and omission distinction. |
| Extra output or refusal side effects escape tests | Read to EOF, reap children and join capture owners before counting; per-test resources and explicit bounds. |
| Scope leaves mandatory decision inventory red | Preserve exact four-file commit and report the failure; operator/documentation owner must authorize index integration. No gate waiver or evals-only scope expansion. |
| Full suite exceeds a tool-call deadline | Keep per-command logs and explicit saved exits; use a sufficient bounded workspace invocation and recover receipts. A missing exit is not a pass. |
| Root green mistaken for eval/live proof | Run standalone eval checks independently; operator owns live client smoke/baseline, CI owns external proof. |
| HTTP completed but reply lost | Preserve existing no-retry behavior; no exactly-once or durability claim. |
