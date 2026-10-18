# Implementation plan — storage and RPC boundaries

Implements [spec.md](spec.md) and proposed
[0047](../../docs/decisions/0047-storage-rpc.md). This is the design handoff,
not an implementation or verification receipt.

> **Superseded for #129 (2026-10-18):** step 2's data-file `File::try_lock`
> is replaced by [macos-identity-lock](../macos-identity-lock/plan.md) and
> amended 0047 §1 — no lock of ours on the database or its companions —
> and step 1's branch creation is history: the branch exists and later work
> appends to it. The remaining steps stand.

## Technical context

One Rust crate using std file locks, rusqlite, Tokio and existing HTTP/JSON
helpers. No dependency or configuration knob is needed. The checkout has a
spec template but no plan template; the required plan sections follow here.
`Persist` owns connection state and a sidecar handle. The cumulative awaits
registry is a mutex-guarded BTreeMap. `mcp::classify` and `handle` currently
disagree with the required malformed-object rule. Stdio already parses a
line once and retains an `Envelope`, with one stdout owner and 16 exchanges.

## Constitution check

- Preserve 0001 synchronous insert/commit-before-250, 0002 subscribe-before-
  scan, 0003 oldest-only pruning and 0005 extraction at ingest. Add startup
  exclusion only; no mutation gains a lock attempt or async visibility gap.
- Supplement accepted 0033's sidecar mechanism and amend accepted 0029's
  notification predicate through proposed 0047. Acceptance is the operator's.
- Retain WAL/FULL, the 5 s busy bound and raw SQLite proof connections;
  retain registry cap, exact extraction pins, webhook plane, envelope sender,
  Origin guard, prior bridge/delivery repairs and chart/publish protections.
- Follow 0020 test placement and 0035 coverage by effect. Deterministic tests
  stay inline; only concrete cold failure paths justify declared sibling
  harnesses. Public wire regressions use real servers. No mock acceptance
  proof, shared registry reset, sleep as synchronization or coverage exclusion.

## Research and resolved questions

The inspected base is `fd1d604`; `2a325c9` is its ancestor. Decisions end at
0046, making 0047 the next free number. Both panels agree file identity is
necessary. Linux std file locks and SQLite POSIX record locks must coexist;
the simplicity panel's exploratory probe is evidence for feasibility only,
not a substitute for Rust/raw-connection and subprocess regressions.

Canonicalizing a filename cannot identify hard links. Keep the sidecar as
spelled for existing compatibility and add a lock on the actual opened data
file. Keep guards after connection state so closing a guard cannot release
SQLite record locks while the connection is alive. Data-file precreation
must preserve SQLite's Unix 0644 creation ceiling (subject to umask).
No claim is made that these lock interactions are identical on every remote
filesystem or OS; fail on unsupported lock operations and test supported targets.

The current inbox predicate rejects LF both as blank and as a control. The
Prometheus encoder escapes LF and backslash distinctly. Thus an LF sentinel
is disjoint from every admissible inbox without changing the registry shape.
An empty sentinel is rejected because an empty label is equivalent to absent
in Prometheus and this family already has an unlabeled total.

`api::split_authority` already distinguishes omitted ports from invalid ones
and handles bracketed IPv6. Expose it minimally at crate scope and reuse it.
The HTTP body refusal currently combines the extractor cause and named limit;
share the oversize wording only, preserving unrelated extractor causes/status.

## Project structure and files

| Files | Implementation work |
|---|---|
| `src/persist.rs` | Add data-file guard, nontruncating creation/mode, error mapping and lifetime tests; update module contract. |
| `src/api.rs` | LF overflow sentinel, one-lock cumulative rendering and local accounting/encoding proofs; crate-visible authority parser and shared oversize message. |
| `src/mcp.rs` | One notification/reply predicate used by classification and handle; invert obsolete silence assertion and extend tables. |
| `src/stdio.rs` | Separate dial target from Host, local body refusal, reuse envelope error shaping, correct comments and unit tests. |
| `tests/binary.rs`, `tests/mcp.rs` | Real alias-server, port-80, malformed-object, oversized single/batch and recovery proofs. |
| `tests/common/mod.rs` | Only necessary real-process fixture support and bounded cleanup. |
| `README.md`, applicable existing contract-documentation tests | Lock scope, overflow label migration, bridge default port/body limit and residuals. |
| `docs/decisions/README.md`, 0033/0042/0043/0044/0039/0029 records | Index 0047 as proposed; dated proposal pointers only, retaining historical text and status. |

Only `specs/storage-rpc/{spec,plan,tasks}.md` and
`docs/decisions/0047-storage-rpc.md` belong to this design commit. Product
documentation and older decision pointers change with implementation.

## Implementation sequence

1. Fetch and verify main contains `2a325c9`; create/check out
   `epic120-storage-rpc` without resetting work. Recheck the decision number.
   In this design environment fetch and branch creation failed because `.git`
   is read-only. Do not bypass that restriction or claim an unsigned/absent
   commit. When writable, sign these four design files before runtime edits.
2. Keep existing sidecar acquisition first. Open/create the database file
   read/write with no truncation and the creation mode above, attempt one
   exclusive `File::try_lock`, and retain its handle after connection state.
   Only then open SQLite, initialize, migrate, restore and permit binding.
   Contention maps to existing `OpenError::InUse` with supplied data path;
   other errors retain the actual failing path and source in `Io`. A newly
   created empty database file may remain after failure, like an inert
   sidecar; no SQLite contents may be modified before both locks succeed.
   Cover new open/lock error branches without changing mutation APIs.
3. Change overflow to LF and its comments. Keep existing-name-first
   increments. The current cumulative scrape calls `registry_total` and
   `registry_rows` under separate locks: move cumulative summation and row
   rendering into one held-map snapshot. Preserve the separate semantics of
   other families whose totals are externally maintained. No new registry
   shape or counter is needed. Pin validator,
   exact cap/folding arithmetic, same-snapshot total and decoded label distinction using a local
   registry. Do not reserve `_uncounted` or special-case it after capacity.
4. Make `reply_id` return a supplied ID including null; otherwise silence
   only for objects with string `method`, else null-ID reply obligation.
   Have `handle` call it. Preserve existing missing/non-string-method error
   text, non-object errors, batch bounds, concurrent effects and reply order.
5. Parse authority through the existing helper; default only an omitted port
   to 80. Format an IPv6-safe dial target and retain original Host. Preserve
   five-second dialing, exchange/response bounds and the HTTP path suffix.
6. After existing parsing/classification and before `rpc_post`, refuse
   `body.len() > MCP_BODY_LIMIT`. Share one oversize message formatter or
   constant beside the API limit, used by HTTP's 413 and stdio; preserve
   the present limit-and-cause text, pinned against a real HTTP overflow.
   Do not replace other extractor failures with an oversize cause.
   Factor a small local envelope error helper parameterized by code/message
   for -32600 and existing -32000 shaping. Return immediately for local
   refusal, including silent notifications; do not dial, retry or execute.
   Invalid JSON/UTF-8 retains parse-error precedence because parsing already
   occurs first. Do not rewrite upstream null-ID responses.
7. Land behavior, regressions, README/module claims and dated proposal
   pointers together. Reconcile 0033/0042 lock scope, 0043 §6/0044 §2 overflow,
   0029 §2/0039 §2/0044 §1 notifications and 0039 §§1/6 bridge behavior.
8. Collect behavioral red/green receipts per issue, then run the full final
   gate inventory. The implementing signed commit cites all four issues and
   names decisive tests; no push, PR, merge or issue closure is authorized.

## Data model and contracts

No persisted data changes. `Persist` gains one held file handle; failure
classification can move from Sqlite to Io when explicit data-file open fails.
Existing file permissions/content remain unchanged. Ownership lasts until
the final `Persist` drops, including surviving Store references.

Registry storage remains at most 4097 entries. Overflow lives in the same
metric family and snapshot, encoded as `inbox="\n"`; a legal literal
backslash-n encodes as `inbox="\\n"`. Counters are exact within their
existing integer domain. The cap limits attribution, never wait admission.

No new envelope type or schema validator. Existing Single, Batch,
InvalidBatch and Silent shapes define local replies. Explicit null IDs and
duplicates are preserved. Response order within an admitted batch stays
input order; separate lines can still complete out of order.

## Proof design

Implement the named tests in [tasks.md](tasks.md), allowing equivalent names
only if the final issue-to-test map records them. Alias wire tests must keep
the first server alive while proving second refusal, then verify mail and
restart. Local lifetime tests exercise failed SQLite startup and guard drop;
keep the existing raw-connection stalled-commit suite intact. Test Unix mode
without process-global umask mutation in parallel tests; use a subprocess if
controlling umask. No bind-mount guarantee without its own supported proof.

For local bridge refusal, use a cfg(test)-only observation at the actual dial
entry if necessary and assert zero calls; prove the observer works with a
small valid request. Pair it with real server effect checks and following-line
recovery. This avoids inferring no dial merely from a network error. Retain
all seam installers and test every new production branch. A real port-80
exchange is mandatory; a runner bind failure is outstanding evidence, not
permission to substitute an ephemeral-port test or ignore the test silently.

Run new wire tests against an isolated `2a325c9` binary and the repaired one.
If a private fixture cannot compile on the baseline, port only the fixture
or restore exactly the baseline faulty behavior in an isolated control tree;
record which control was used. The 40 MiB baseline may already preserve ID
on a broken pipe: its decisive assertion is deterministic local -32600 with
shared wording and no dial, not ID alone. Keep all control mutations out of
the final tree and record expected/actual red assertions and final greens.

## Quickstart and validation

Run targeted library tests and `cargo test --locked --test binary --test mcp`
after the corresponding changes. Final implementation gates, in order:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo fmt --all
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all
cargo test --workspace --locked
cargo fmt --all
bash scripts/coverage-gate.sh
cargo fmt --all
bash scripts/coverage-gate.sh selftest
cargo fmt --all
bash scripts/publish-guard-test.sh
cargo fmt --all
cargo deny check licenses
```

No TMPDIR override; exact missed == 0 lines/branches/functions. Run the
existing fail-closed native-tls absence check too. Verify owns
`brokkr compile --bundle bundles/self` from the tooling-pinned binary;
record a sandbox refusal honestly and do not install or replace the tool.
CI retains chart/advisory/release-rate obligations. Do not widen this batch
into the epic's later delivery-policy repairs or claim their warnings fixed.

If cargo itself is refused, stop, preserve the patch at
`.brokkr/state/epic120-storage-rpc.patch`, and report the blocker. If Git or
signing is blocked, preserve reviewable design files and a design-only patch
without bypassing permissions or creating an unsigned commit. Artifact-only
validation checks nonempty files, required headings/checkboxes, relative
links and proposed status; it does not prove implementation or a signed commit.

## Risks

| Risk | Mitigation and accepted limit |
|---|---|
| Aliases split WAL and lose acknowledged mail. | Identity lock before SQLite in addition to sidecar; both alias orders and live-owner/restart proof. |
| Guard close interferes with SQLite locks. | Connection state drops first; raw-connection/BEGIN IMMEDIATE and existing busy/commit tests remain mandatory. |
| Data-file creation broadens permissions or truncates contents. | Unix 0644 ceiling subject to umask, nontruncating open, existing-mode/content and failed-startup tests. |
| Remote lock semantics, replacement or old alias writers bypass exclusion. | Fail unsupported calls; document scope. No distributed lease, hostile-namespace or universal bind-mount promise. |
| Restart through a different hard-link basename misses an existing WAL. | Keep the configured pathname stable across restart; test alias exclusion without claiming WAL relocation or historical repair. |
| Overflow label collides after validation/encoding changes. | Coupled validator and independent decoded-label tests; `_uncounted` remains legal. Dashboards migrate their overflow selector. |
| Expected exact row is first encountered after cap. | Explicitly retain bounded attribution policy; test tracked `_uncounted` before cap and untracked names after it. |
| Server/bridge disagree about malformed objects. | One predicate plus independent expected-response tables and real HTTP/stdio tests. |
| Oversize rejection leaks network work or changes IDs. | Gate before dial; test zero dial, exact envelope shapes, no effects and continued processing. |
| Local body limit differs from remote version/proxy. | Document unmatched upstream null-ID residual; no blanket rewriting of valid upstream replies. |
| Huge stdin or slow stdout consumes memory/stalls polling. | Retain documented residual; local limit bounds outgoing bodies, not input allocation or process exit time. |
| Port 80 unavailable or privileged in runner. | Provision an appropriate existing test environment; report missing proof rather than skip or use a mock acceptance test. |
| A proposal is mistaken for an accepted ruling or implementation. | New proposed 0047, dated pointers with status retained, explicit uncompleted tasks and gate receipts. |
