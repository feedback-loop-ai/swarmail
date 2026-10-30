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
and 19 executable tests passed: boundary::sequential_contracts_and_decision_links_are_honest
still required decisions 0063/0076 to be proposed. The verified operator rulings
and both tracked records already say accepted (operator, 2026-10-26). Updated
only those stale test assertions to require the recorded acceptance and date,
without editing or accepting either decision. The boundary target then passed
all 15 tests. No failed or interrupted command is counted green.

After the boundary repair the complete standalone eval command passed all 236
tests (including the five real stdio tests), with saved exit 0. The transport
returned a 300-second timeout; the later saved exit and complete log, ending
with wrappers and doc tests, establish success independently of that timeout.

A further closed-shape control then failed (exit 101): serde's new field also
admitted a third positional call element, previously refused. Kept the explicit
ignored field and constrained positional calls to their former two-slot arity.
A positive legacy positional control and negative third-slot control prove that
only the named metadata slot is newly admitted. No new params or argument shape
is otherwise accepted or tightened. The final full suite was rerun after this repair on committed candidate
7169583df0dcf7ddf9dbfa5d034b7518b8598635 and passed all 236 tests (saved exit 0),
including all 19 wrappers tests and doc-test completion. Its transport timeout
was likewise superseded by the recovered command exit and complete log.

## Final commands and exits

All checks used CARGO_NET_OFFLINE=true. Code and test proof is bound to committed
candidate 7169583df0dcf7ddf9dbfa5d034b7518b8598635; the following receipt commit
changes this document only. Complete command arrays, candidate hashes, raw and
interpreted exits, timings and logs are retained under ignored
.forge/implement151-meta. Public shim hashes passed; gate scripts were executed
from their pinned bundle owners, as required by the charter.

| Command | Exit |
|---|---:|
| `cargo fmt --manifest-path evals/Cargo.toml --all -- --check` | 0 |
| `cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings` | 0 |
| `cargo test --manifest-path evals/Cargo.toml --locked --test wrappers` | 0 (19 tests; final full suite also reruns all 19 after the positional repair) |
| `cargo test --manifest-path evals/Cargo.toml --locked` | 0 (236 tests on the final code candidate) |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 101 |
| `.forge/bin/brokkr compile --bundle bundles/self` | 0 (76 valuations, 0 unruled, no warning) |
| `bash bundles/self/scripts/coverage-gate.sh` | 1 |
| `bash bundles/self/scripts/coverage-gate.sh selftest` | 0 |
| `bash bundles/self/scripts/publish-guard-test.sh` | 0 |
| `bash bundles/self/scripts/seal-branch-test.sh` | 0 |
| `cargo deny check licenses` | 0 |
| `bash bundles/self/scripts/design-check.sh selftest` | 0 |
| `bash bundles/self/scripts/delivery-proof-test.sh` | 0 |
| Fail-closed `cargo tree -i native-tls` absence proof | 0 (raw exit 101, specifically package not found) |

## Outstanding delivery prerequisite

Result is **broken**, not complete: root tests and the exact coverage command
fail because `0084-fixture-http-meta.md` has no decision-index row. Both
`every_decision_file_has_exactly_one_resolving_index_row` and the inventory's
negative-fixture comparison report that same missing row. Coverage stops at the
instrumented test failure, preserving target/coverage/run.Oz5Co4/run.log; no
exact-100% report or successful coverage verdict is claimed. The coverage
selftest and delivery tamper proof both pass without any gate changes.

The preceding design explicitly assigns index integration to the
operator/documentation owner, while this task permits changes only under evals/.
That owner must add the proposed 0084 row to docs/decisions/README.md before the
required root gates can be green. No index edit, decision acceptance, test
weakening, exclusion or denominator/floor reduction was made here. All tracked
implementation changes stay under evals/ and are committed unsigned; no push,
merge, tag, publication or issue closure was performed.

CI owns the charter's macOS/chart/Docker/network/privileged-port evidence; #201
owns eval macOS gating. The operator owns live Claude/client smoke, paid trials
and the baseline. None is claimed as locally proved.


## Retry verification — 2026-10-26

Rechecked committed candidate ed7ef1a657797252e292c62ed0ba0126a9c5be1f. The whole supplied evidence packet
again passes its SHA256SUMS check from the packet directory. The candidate-local
engine matches ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f,
and public shim hashes pass. Inspection found no further helper defect. This
retry changes only this receipt; the original failing-first evidence remains
recorded above. All implementation changes remain under evals/.

Fresh exact commands, candidate hashes, exits, timings, full logs and authority
hashes are under ignored .forge/implement151-meta-retry. All commands run with
CARGO_NET_OFFLINE=true and the existing bundle routing; scripts execute from
their pinned bundle owners after shim verification. The initial transport calls
timed out after 300 seconds. Later recovered exits and completed summaries
establish the following results independently of those transport timeouts.

| Check (exact command arrays retained with receipts) | Exit |
|---|---:|
| Independent eval fmt / locked all-target clippy | 0 / 0 |
| Real stdio `actual_stdio_helper_` group | 0 (five tests) |
| Full locked eval suite | 0 (236 tests and doc-test completion; 580.89 seconds) |
| Root fmt / locked all-target clippy | 0 / 0 |
| Root locked workspace tests | 101 |
| Candidate-local brokkr compile | 0 (76 valuations, 0 unruled, no warning) |
| Exact coverage / coverage selftest | 1 / 0 |
| Publish guard / keyless seal proof | 0 / 0 |
| License check | 0 |
| Design selftest / delivery tamper proof | 0 / 0 |
| Fail-closed native-tls absence proof | 0 (raw 101, package specifically absent) |

Both root failures remain the two decision-inventory assertions described
above: 0084-fixture-http-meta.md has no index row. Coverage stops on those same
assertions and preserves target/coverage/run.oCWrno/run.log. Exact 100% coverage
is still unproved on this candidate. No evidence input or decision is missing;
the remaining work is index integration, expressly assigned outside this
evals-only seat by proposed decision 0084 and the committed design. The exact
row for that owner to add to docs/decisions/README.md is:

```markdown
| [0084](0084-fixture-http-meta.md) | Discard fixture HTTP request metadata and name GET-body refusals | proposed — awaiting the operator | (Epic #164, issue #151 part 3) |
```

That row would index the proposal without accepting it. The owner must then
rerun the failing gates; no green outcome is inferred here. This retry preserves
the root index and all gate assertions and reports **broken** because required
local gates remain red. CI and operator proof ownership remains as recorded
above. This receipt is committed unsigned; no push or publication is performed.

## Integration correction — 2026-10-28

The preceding receipts name 0084 as it appeared at the recorded candidate
commits. Integration commit `76678c3` records its later allocation to the dsh
fresh-home decision, renumbers this proposal to 0087 and adds its index row.
That integration does not retroactively change the earlier failures or accept
the proposal; the later verify and ship results have their own evidence.

## Standalone eval proof at 841ac40 — 2026-10-30

Review return (decision 0102) of run issue-151-operator-continuation--c45a11bd
at 841ac403c42ffa7da4f466ca56ddcfce5731c062 (tree
55612de1b1c24631cca6b96856e2d17018723f46). Two panel findings were repaired,
both inside evals/:

- `evals/Cargo.toml` again says "Until issue #201 wires CI and pinned verify,
  run the standalone formatting, lint and full tests separately" (the
  09852d8 wording). Merge 6d4fc4c had taken main's text, which claimed CI and
  pinned verify run these checks; neither does (`.github/workflows` has no
  evals job and `verify-seat.sh` runs the root inventory only). This matches
  `evals/README.md`.
- The standalone suite was run at this head. Any later commit touches only
  this record and that comment.

Provenance: no other eval suite was active (no `swarmail-evals` or `cargo test`
process, no `/tmp/swarmail-evals-tests-*.lock` holder) before the run. The root
binary was built with `cargo build --locked --bin swarmail` (fresh, from this
tree), copied to a private read-only path and passed as an absolute
`SWARMAIL_BIN`. Its SHA-256 was
`e08783233f3e48e17dfee0a07f078abc020c0482e7c587bbdefde6fa8a8b6db4` before the
first command and unchanged after the last. All commands used
`CARGO_NET_OFFLINE=true` and `--locked`.

| Command | Exit |
|---|---:|
| `cargo fmt --manifest-path evals/Cargo.toml --all -- --check` | 0 |
| `cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings` | 0 |
| `cargo test --manifest-path evals/Cargo.toml --locked` — run as separate target invocations (`--lib`, `--bins`, each of the ten `--test` files, `--doc`) because this seat's tool bounds one silent call | 0 each |

Passed tests per target: lib 52, bin 19, baseline 3, boundary 15, fixture 17,
identity 5, proof 6, provenance 27, report 18, runner 37, scoring 18,
wrappers 19, doc 0 — 236 in all, none failed or ignored. The tool never held
one uninterrupted full-suite command: a first attempt was cut off when its
call ended and counts for nothing; the per-target runs above are the proof.

Root: `cargo fmt --all -- --check` exits 0. The root inventory (tests, bundle,
exact coverage and selftest, publish guard, seal proof, licenses, native-tls
absence) is the verify seat's receipt at this head; this change touches no
root file. CI-owned checks, the paid baseline and external publication are not
claimed.
