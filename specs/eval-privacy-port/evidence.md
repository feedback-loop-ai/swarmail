# Eval privacy continuation — implementation evidence

Recorded 2026-10-25 (Europe/Sofia). This is implementation proof, not a review
verdict. The independent verify and accepted-0058 careful review phases remain
required. Neither 0073 nor 0075 is accepted here.

## Candidate and authority

The unreviewed port is 529d9160970152cd9b767925be71afdae4c44fe7. Design commit
b678240 already amended AC13/AC23 and proposed 0075 with the dated operator
ruling. Implementation commit `c70dea9a0c052a3a242eab175ed21460f7db7ce3` (tree
`d58784c63e7090c23491965fa4663bbbd9aff34f`) reconciles the three dependent current
platform paragraphs and adds two native-canary assertions to existing tests.
Production source, existing assertions, root server, dependencies, locks, bundles,
shims and accepted decisions are unchanged from the handoff.

All eleven entries in `.forge/epic192/issue196/SHA256SUMS` passed from that
packet directory. The operator's 2026-10-24 row **macOS evidence for `evals/`**
in [operator-rulings.md](../../docs/rulings/epic-192.md)
places evals outside CI on every OS until #201 and assigns macOS-only eval
execution to #201's CI gating. Every Linux obligation remains. No macOS or CI
execution is claimed, and the previous blocked receipt remains historical.

The executed candidate-local `.forge/bin/brokkr` has SHA-256
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, matching
`bundles/self/tooling.json`. All seven public shims matched the pinned inventory.
Pinned gate scripts were executed directly under `bundles/self/scripts/`, as
the verify seat does. The candidate does not change those resources.

## Preserved suites and assertions (T2–T8)

The local machine receipts are in `.forge/privacy-continuation/`:
`audit.json`, `identities.json`, `mutations.json`, `release-control.json`,
`initial-gates.json` and `gates.json`, with named command logs. Each gate receipt
identifies command, actual exit, OS, candidate HEAD/tree, diff hash, engine digest
and log hash. `identities.json` records the pinned bundle resource hashes and
actual rustc/Cargo versions. The final gate receipts bind the committed
continuation tree; the initial receipts bind the implementation commit above.
No journal, sibling tree, GitHub or CI was consulted.

- All 17 eval source/test hashes recorded by the previous inventory match the
  committed handoff. The only current source difference is two added test
  assertions; no existing helper or assertion was removed.
- All ten baseline snapshots match their blobs in the handoff's parent. All ten
  packet snapshots match the new blob IDs in checksum-verified
  `prior-own-diff.patch`. The map's 122 destinations exist, including the two
  fault cases moved to compiled-only inline owners. All 91 baseline and 96
  packet declarations have counterparts; the five additional integrated tests
  bring fresh executed coverage to **127 passing eval tests**.
- The preserved #194 assertions still prove both signals, repeated signals,
  queued admission/commit/final-row races, actual-Pending cleanup holds and
  release, both cleanup bypass controls, identity-aware rescue/checked guard
  failure, observation/expiry failures, resistant descendants, Cargo ownership,
  stdout pressure, inherited-pipe bounds, exact prefix/active/refused-suffix
  accounting and report denominators. Existing URL/identity comparison is
  strengthened with distinct private directories; safe cleanup categories
  replace raw external error text without changing failure outcomes.
- Fresh full-suite execution covers exact actual role maps before wrapper
  additions, generated agent PWD, native argv/worker handoff, explicit wrapper
  configuration, lazy homes, a real offline/locked custom-target build, bounded
  Cargo/readiness drainage and independent canary nonpublication. All three
  reqwest builders explicitly use `no_proxy`; the dead-proxy oracle and safe
  Admin transport/read/JSON/status cases pass.
- The six artifact helper tests observe creation and descriptor normalization
  before payload, complete-file visibility, forced write failures and file/
  directory/symlink/hard-link collisions. The real failed-trial test passes under
  umasks 0000, 0002 and 0777 with a snapshot, exact 0700/0600 modes, unchanged
  parent and private residuals. No final-only chmod substitutes for this proof.
- Real oracle-all-pass, null-idle-failure, wrong-action and overlapping isolation
  proofs pass over SMTP/HTTP/MCP. Runtime-owner tests retain private directories
  through body abort/Cargo cancellation and the actual cleanup fence.
- A fresh offline/locked release build passes. Three real null trials with
  former fault values `panic`, `server_death` and `panic_after_agent_readiness`
  retain Fail/GaveUp, status 0 and exit 1. The release matrix returns oracle Pass
  and null Fail, both status 0, exit 1. All **51** source-declared test-control
  names are injected; none creates control state. Dev inertness is also covered
  by the full eval suite.

This Linux executor is `Linux-7.0.0-38-generic-x86_64-with-glibc2.43`, host
`x86_64-unknown-linux-gnu`, rustc 1.98.0 / Cargo 1.98.0. The custom-target and
artifact execution above are Linux receipts, not portability claims.

## Mutation audit and repairs (T9)

The old aggregate diff digest is not used as final-candidate identity. Source/
test hashes, patch blob identity, mutation regions, actual logs and assertions
bind the reusable evidence. Nine supplied receipts remain valid: fixture and
worker inheritance, raw output, unsafe executable metadata, unsafe Clap errors,
Cargo overflow acceptance, lossy native worker argv, permissive final modes and
missing pre-payload descriptor normalization. Their actual red exit is 101 and
restored exit is 0; `audit.json` records decisive assertions and both log hashes.
Counted producer completion precedes the raw-output/overflow failures; artifact
and metadata mutations fail their mode/null assertions, not missing artifacts.

The two agent/server inheritance receipts instead stopped at UTF-8 decoder
unwraps after native caller bytes leaked. That did not meet the required
intended-exposure assertion. The sole test repair adds the already-used raw
`NONUTF8_PARENT_\xff` exclusion assertion before decoding in each existing probe.
The complete-map/value/real-outcome assertions remain in place. Fresh mutations
remove `RuntimeDirs::apply`'s `env_clear` independently for each focused test:

| Focused runner test | Mutated exit and decisive failure | Restored exit |
| --- | --- | --- |
| an_external_agent_gets_literal_argv_and_identical_prompt_and_env | 101; caller native environment canary crossed the closed command boundary | 0 |
| swarmail_gets_only_its_fixed_runtime_map | 101; caller native environment canary crossed the closed command boundary | 0 |

The exact commands are `cargo test --manifest-path evals/Cargo.toml --locked
--test runner <name>`. `mutations.json` records the candidate/tree, source/test
and mutated hashes, source region, OS and actual exits. Every mutation was
restored before final gates. The two #194 cleanup-await negative controls also
execute in the fresh full eval suite; they retain their own causal hold/release
and identity-safe rescue assertions.

## Gates and remaining phase ownership (T1, T10–T11)

All 17 initial gate/identity commands exited 0 on the implementation tree above:
root and eval fmt, locked all-target clippy and full tests; both self and careful
bundle compiles; exact root coverage and its selftest; design selftest; delivery
tamper-boundary, publish-guard and keyless seal proofs; licenses; fail-closed
native-tls absence. Both compiles were free of policy-sweep warnings. The root
coverage report has **12953/12953 lines, 762/762 branches and 1437/1437 functions**
covered, with zero missed. The gate logs identify their report directories under
`target/coverage/`. Root tests include the decision index uniqueness/status
contracts; 0075 has exactly one proposed row and remains proposed.

`initial-gates.json` retains that first pass. `gates.json` and the named logs
record the final committed-tree execution of every command in the plan's final
verification block, using the pinned script equivalents. A missing/nonzero
receipt is not proof. The Linux eval suite is separate from root coverage.

T11 remains unchecked because the review phase alone supplies the five fresh
candidate-bound careful results: Astra correctness, official Flash security,
Sonnet 5.5 tests, Astra guarantees and independent Fable 5.1. The careful bundle
compiled; this author supplies no review verdict or permission to bypass a hold.
The engine owns advancement and verified-candidate binding.

Charter CI-only chart/history/selftest, Docker smoke, network RustSec, tag rate
and privileged port-80 execution remain CI-owned; macOS eval execution is
separately #201-owned under AC13. No push, merge, tag, publication, issue closure
or decision acceptance occurred. Commits are unsigned under accepted 0056.
