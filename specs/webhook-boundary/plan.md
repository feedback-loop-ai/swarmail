# Implementation plan: webhook boundary

Implements [spec.md](spec.md) and proposed
[0041](../../docs/decisions/0041-webhook-boundary.md). This is a design handoff;
implementation and red/green evidence remain tasks, not claimed results.

## Technical context

Single Rust 2024 crate using Tokio broadcast/JoinSet, reqwest 0.12 with
rustls, axum 0.8 and serde_json. The global mail ring holds 8,192 entries;
the dispatcher owns 32 delivery slots per normalized URL. Retry is four
attempts, each with a five-second timeout, separated by 100/400/1600 ms.
Default and extra-CA clients have existing caches. None of those bounds change.

There is no plan template under `.specify/templates/`; this document supplies
technical context, constitution check, structure, research, data model,
contracts, quickstart and risks directly. No new dependency, schema, feature
flag, production source file or public API hierarchy is required.

## Constitution check

The store/commit path, synchronous 250 guarantee, watcher ordering, oldest-only
pruning and ingest extraction remain unchanged (0001–0005). Webhooks stay
downstream of commit with nonblocking capacity admission. Preserve 0029's
normalized per-target budget, retries and tracked-commit drain, 0024's refusal
boundary and the existing stop fence. The semantic amendment is proposed
0041; accepted 0029 is not rewritten by this seat.

Use real servers and unique inboxes for wire tests. Deterministic private
tests stay inline and measured under 0020; seams are cfg(test)-only, local
to their owner and installed by tests. If an actual cold failure path is
needed, put only that test in a declared sibling `*_tests.rs` harness and
record the concrete reason. Do not move entire modules for coverage. Preserve
0035, gate scripts, exact 100/100/100, dependency allowlists and ANSI behavior.
No parent-process environment mutation, sleep synchronization, fixed-date
certificate assertion or probabilistic chaos rule is permitted.

## Project structure and files touched

| Path | Responsibility |
|---|---|
| `src/webhook.rs` | Header helper, common client builder, redacted URL/echo helper, shared payload envelope/Bytes, budget warning state, lag owner, tail sweep, final event; deterministic inline proofs and any strictly test-only observations. |
| `src/api.rs` | Use the shared redacted-target projection for both GET and PUT; preserve validate-all-before-set; update endpoint comments and inline error tests. |
| `tests/webhook.rs` | Real sinks for header fidelity, Basic auth, retry bytes, redaction and independent-budget regressions; reproducible one-versus-four memory observation. |
| `tests/api.rs` | Real HTTP atomic PUT rejection and GET/PUT echo assertions. |
| `tests/binary.rs` | Child-only proxy environment matrix, HTTP/HTTPS recording peers and final-count SIGTERM regression; reuse startup stdout parsing and child cleanup. |
| `tests/common/mod.rs` | Only necessary reusable real-sink/readiness helpers; avoid a new generic test framework. |
| `README.md`, `src/static/openapi.json` | Exact secret domain, credential redaction, proxy boundary, shared memory scope, drop windows, live metrics versus final log. |
| `docs/decisions/README.md`, `docs/releases/v0.1.0.md` | Implementation-stage proposed-decision index and dated repair note, without claiming the original release contained the repairs. |
| `specs/webhook-boundary/{spec,plan,tasks}.md`, `docs/decisions/0041-webhook-boundary.md` | Exactly four design artifacts. Later clarifications/evidence update their owning documents. |

No changes are planned to `src/lib.rs` stop order, `src/main.rs` log routing,
store/persist, protocol or extraction code, Cargo manifests/lock, bundles or
gate policy. Run-local logs, patch and result remain under ignored state.

## Research and resolved choices

Read both complete positions and four issue bodies. Inspected the task
directory: its three documents are historical run-12 gate work, not run-18
framing. The present commission is `run18-feature.txt`. Checked current
`src/webhook.rs`, API handlers, `RunningServer::stop`, `main::init_tracing`,
binary scaffolding, decision index and applicable rulings. Current base is
`33ac7d1`; the reported failures originated at `f39813d`.

The current source confirms the success-only tail loop, two proxy-inheriting
builders, three raw URL log sites, raw host-error fallback, secret-only echoes,
JSON Value construction and request `.to_string()` copy. The default `serve`
filter is `swarmail=info,tower_http=off` and the stream is stdout, so no logging
level or routing change is needed. Correct the panel's stderr test proposal.
The spec's ruling table resolves all genuine panel disagreements.

## Data model and implementation

Stored Email, target input JSON, ledger fields and metrics names are unchanged.
Add only ephemeral private representations:

- A borrowed Serialize envelope with static event `received` and `&Email`.
- A budget value holding the existing JoinSet and `Option<Instant>` for the
  next allowed warning (or equivalently last-warning time). Keep suppression
  beside capacity, not in a second map. The normalized SinkKey stays intact.
- `axum::body::Bytes` carrying the serialized Vec allocation into reqwest's
  body conversion. Cheap clones extend ownership through targets/retries.

Implementation order:

1. Change the secret validator to return HeaderValue after length, parser and
   edge-SP/HTAB checks. Use `from_bytes(secret.as_bytes())`; do not replace
   the parser with byte-range logic. Validation discards the returned value;
   delivery installs it. Mark sensitive header values if appropriate to the
   existing HTTP type, while ensuring errors never print the input. Preserve
   all-list validation before the set. Directly installed invalid targets
   exercise delivery refusal without constructing malformed requests.
2. Add `client_builder()` carrying rustls, no-redirect and `.no_proxy()`.
   Both cache builders derive from it; extra CA adds its certificate. Keep
   certificate parsing/verification and cache eviction unchanged.
3. Add one safe URL display and one redacted-target projection beside webhook
   ownership. Strip both username and password on parsed URLs; valid URLs
   without userinfo may retain their original spelling. Malformed display
   returns a constant. Route all URL fields and both echoes through this
   policy. Remove the raw URL from the host-validation error; use fixed
   named clauses for unsupported schemes rather than reflecting untrusted
   input. Review reqwest error formatting too: strip its attached URL before
   stringification where needed (retain the failure cause and safe explicit
   URL field), and prove no credentials survive in full captured lines.
4. Serialize with `serde_json::to_vec(&Envelope { ... })` only after admission
   establishes nonempty room. Convert Vec to Bytes without copying; pass
   Bytes clones to each task and each request body. Preserve timeouts,
   retry schedule, content type and existing fixed-payload behavior. No
   store reread, serializer offload, extra queue or intermediate Value.
5. Integrate suppression with the budget map. A small decision helper receives
   the monotonic instant; the dispatcher supplies now and increments dropped
   regardless of its result. Duplicate keys within one pass must see the
   updated warning state, while `claimed` still protects pre-spawn capacity.
   A successful admission leaves the deadline alone. Retire state under the
   existing dead-and-drained rule. Do not add a recovery event or timer task.
6. Move lag warning plus `lagged.fetch_add` into one helper called by live
   and tail handling. Replace the tail with an explicit match loop continuing
   through every Lagged and ending only at Empty/Closed. Preserve matching
   semantics and abort/join terminal accounting. After all joins, snapshot
   once and emit the specified info event on both drain/closed exits, even
   when the ledger is empty. No await or new producer work enters the sweep.

## Contracts and proof design

**Header/API:** use complete prior-target snapshots and mixed valid/invalid
PUTs. Exercise 4,096/4,097 bytes with ASCII and multibyte UTF-8, empty/internal
tab/padded values and all named controls. A raw HTTP sink can inspect byte
values even when a UTF-8 header cannot be represented by a convenience
`to_str` accessor. Assert received secret bytes and no secret in refusals.

**Lag/accounting:** current-thread test subscribes then inserts 8,200 unique
small mails without yielding, using a small inbox retention cap to save memory
without changing the ring. Signal both watches before the dispatcher runs;
assert the full ledger and zero network attempts. Use direct real broadcast
receivers for Empty/Closed and fan-out cases. To prove repeated lag, a local
cfg(test) synchronous observation after the first lag can fill the real ring
again before the next read; calculate both lost-mail totals and the retained
notification total exactly. No mocked receiver, schedule guessing or global
hook. Keep existing post-stop commit, in-flight completion and budget tests.

**Proxy:** spawn a fresh binary per client variant to avoid cached-client
environment contamination. Set all upper/lowercase proxy variables on the child
only and remove both bypass spellings. A real recording proxy accepts and
records connections/request lines and responds, so the old defect is visible
as proxy traffic, not merely a timeout. Race expected target arrival against
unexpected proxy arrival under a failure deadline. Exercise default HTTP and
extra-CA HTTPS using generated valid certificates. After target completion,
stop/join the child while the proxy recorder remains active, then synchronously
drain pending accepts and assert zero connections. Test-side HTTP clients also
avoid inherited proxies. Preserve redirect and untrusted-certificate tests.

**Credentials:** capture the whole output of budget, non-2xx, transport and
malformed-target paths at debug level, not just the explicitly redacted URL
field. Check plain and percent-encoded usernames/passwords are absent. Assert
the actual sink receives Basic auth and the installed configuration is intact;
only the outward copy is redacted. Cover query/fragment residuals explicitly.

**Payload sharing and memory:** a per-instance cfg(test) observation records
builds and a clone of the built Bytes. Observe the actual built reqwest Request
immediately before sending and inspect `body().as_bytes()` against that live
backing address/length. Keeping the source allocation alive prevents allocator
address reuse from faking identity. The test must drive the real send path;
observing a helper clone before a later `.to_string()` is insufficient. If
necessary, extract the existing request build into one private helper and
execute that very Request. Do not add public testing API or a production
counter. One mail to four sinks and one mail through four failing attempts
each build once; every request body has identical backing and wire bytes.
Also prove zero builds for unmatched, all-refused and stopping mail.

For the issue's memory observation, use fresh child processes with identical
configuration, fixed 16 mails of 4 MiB, and one versus four stalled targets.
Record baseline and ready-state RSS, serialized payload sizes and slot counts
on the same host/build. Use header/readiness barriers so every request is in
flight and hold responses; drain/discard received payload rather than retaining
it in the recorder. Do not sleep to guess readiness or count receiver memory
as sender memory. Repeat at a second payload size to distinguish full-body
retention from bounded socket/TLS overhead. Report observations as supporting
evidence, not a portable absolute-RSS gate. Exact allocation identity and
summed live payload bytes are the deterministic acceptance oracle; unexplained
full-payload-per-extra-target growth blocks completion and must be investigated.

**Warnings:** test supplied instants at t, t+60s−1ns and t+60s; no test waits
a minute. Exercise independent and normalized duplicate keys, successful
admission between refusals, and retirement/re-add. A real stalled sink with
readiness signals fills the budget; a controlled burst then pins the exact N
dropped and one warning. Count only drop events, not unrelated logs.

**Final event:** real `serve` child with inherited RUST_LOG removed uses its
default info filter. Wait for one stalled sink to read the request, signal
SIGTERM, continue draining stdout so the pipe cannot deadlock, join, and parse
exactly one final event through EOF. Assert all five numbers and exit code.
This ordering proves in-flight state without sleeps. The embedding test retains
Webhooks and compares the post-stop snapshot to the event's values.

For every child issue, record a behavioral failure against reviewed/current
old behavior and a passing repaired test. Use an authorized isolated checkout
or a narrowly labeled old-behavior mutant; never reset the user's working
tree. Log SHA, command, expected/observed behavior and result under ignored
state. A test that fails to compile is not red evidence. Tests for new code
structure may need a baseline-compatible behavioral counterpart.

## Quickstart and validation

Use a feature branch. Run targeted unit/API/webhook/binary suites as each task
lands, then the required gates:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --locked webhook
cargo test --workspace --locked --test api --test webhook --test binary
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
cargo deny check licenses
```

Do not override TMPDIR. Preserve exact coverage and all six gates; a denied
environment is a blocked gate, not a pass. The implementation phase owns code,
protected review and later landing. This design seat never pushes or closes
issues. Its only staging/commit scope is the four named design artifacts.

Validate each artifact is nonempty, spec Acceptance Criteria, plan Risks,
task checkboxes, relative links and proposed 0041. Inspect staged paths before
the imperative, rationale-bearing signed commit. If Git is read-only, preserve
the artifacts and an ignored patch; do not relocate Git metadata or claim a
commit. Record the refusal in the design result.

## Design-seat validation receipt — 2026-10-18

Formatting (write followed by check), locked workspace all-target clippy,
bundle compilation and license checks passed on the unchanged production tree.
`cargo test --workspace --locked` exited 101 in the library suite: 204 passed,
25 failed, with real socket operations refused as `PermissionDenied: Operation
not permitted`. Coverage was not run because it repeats those denied tests;
the full-test and coverage gates remain outstanding. These checks do not prove
the proposed repair, whose tasks remain unchecked. Logs are under ignored
`.brokkr/state/design/run18-chief-*.log`.

`git switch -c run18-webhook-boundary` failed creating the ref lock because
`.git` is mounted read-only. Staging exactly the four paths also failed to
create `.git/index.lock` with the same refusal, so a commit cannot proceed.
The four artifacts are uncommitted; no branch,
alternate Git directory or permission bypass was used. An ignored patch at
`.brokkr/state/design/run18-webhook-boundary.patch` preserves the exact draft.
A writable Git environment is required for the requested signed design commit,
and a socket-capable environment is required for the remaining gates.

## Risks

| Risk | Mitigation / accepted cost |
|---|---|
| Header acceptance differs from received bytes. | One library parser plus explicit edge-OWS refusal; raw-wire empty/tab/UTF-8 tests. Padded secrets deliberately become 400. |
| Credentials leak through an error instead of a URL field. | Full-line debug/default capture, safe validator sentences, scrub attached reqwest URL, malformed/encoded fixtures. Query tokens remain an explicit residual. |
| Future client path inherits proxies. | One policy builder and real child-process tests for both current clients; retain TLS/redirect evidence. |
| Build count conceals request copies. | Inspect actual Request body identity while source allocation lives, exact bytes on retries, and matched memory observations. |
| Memory experiment varies by allocator/kernel. | Same host/build, fixed readiness, distinct live-payload identity as the deterministic oracle; do not claim total process memory independent of target count. |
| Serializing large mail delays dispatcher progress. | Remove owned intermediate copies; keep no-serialization admission/stop paths. CPU offload requires a separate evidenced design. |
| Embedded producers indefinitely refill the tail. | Document quiescence before drained; no throughput assumption or silent count cap. Existing run_on refusal/stop sequencing remains. |
| Ledger double-counts completed tasks or promises late detached commits. | Post-join exact assertions; retain existing tracked-commit scope and explicitly inherit 0024's detached commit residual. |
| Warning reset restores log amplification. | Success leaves window intact; duplicate-key/retirement tests. Config churn may reset state and is not a persistent-identity promise. |
| Final line lost to filtered logs or hard exit. | Default-info real-binary test and explicit graceful-only observation; no durable notification guarantee. |
| Test-only hooks evade coverage or add globals. | Per-instance cfg(test) hooks with installing tests, inline deterministic placement, unchanged 0020/0035 exact gates. |
| Sandbox blocks commit or real sockets. | Preserve exact evidence and patch, report outstanding work honestly; no bypass of read-only Git or claimed green gates. |
