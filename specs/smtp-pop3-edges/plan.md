# Implementation plan — SMTP and POP3 edges

Implements [spec.md](spec.md) and proposed
[0046](../../docs/decisions/0046-smtp-pop3-edges.md). This is a design handoff,
not a receipt claiming the patch has been applied or its gates passed.

## Technical context

One Rust crate; existing Tokio TCP, tokio-rustls, AsyncWrite and shared
`smtp::bound_io` suffice. SMTP/POP3 listeners own their session JoinSets.
POP3's OwnedWriteHalf exposes actual partial writes. HTTP/MCP/UI/compat
already share `model::validate_inbox`; SMTP's shared seed builder also calls
it. No production dependency, schema or public configuration field is added.
The checkout has a spec template but no plan template; this plan supplies
the required context, constitution check, structure, research, data model,
contracts and quickstart directly.

## Constitution check

- 0001: keep synchronous insertion and awaited commit before 250; cleanup
  follows session completion, and no new timer surrounds insert or deletion.
- 0002: refuse names before watcher registration; valid waits still subscribe
  before scan. No registry ownership/cap changes.
- 0003/0005: keep oldest-only pruning and ingest extraction. Do not transform
  accepted envelope data or alter snapshot/fetch behavior to preserve a reply.
- 0028: explicitly propose removal of username trimming in 0046, retaining
  501 refusal-without-identity-change. Preserve the accepted document's body;
  proposed status is not acceptance. The user explicitly requests verbatim
  names; record the contract amendment rather than hiding it as a refactor.
- 0020/0035: inline branch-stable tests remain measured; cold diagnostics
  belong in declared sibling harnesses; public wire tests remain in tests/.
  Every test-only seam has an installer. No exclusions or floor reduction.
- Preserve the named protected delivery/storage/chart features, license
  allowlist and native-tls absence. No bundle edits belong to this batch.

## Research and resolved questions

At baseline `src/smtp.rs` bounds `acc.accept(raw)` with
`cfg.tls_handshake_timeout`; `tests/starttls.rs` already tests stalled
handshakes. Keep this dedicated timeout, including its existing zero-value
posture. Both current positions agree the handshake is already bounded.

`src/lib.rs::run_on` creates `Webhooks::default`; `src/webhook.rs` holds
targets in an in-memory RwLock. `api::set_webhooks` validates the complete
list before `set`. No target persistence/restore exists. Extend PUT's
existing validator; prove refused replacements preserve the prior list.
Do not add a migration, startup validation or persistence for this concern.

Run 2 already contains both declared sibling harnesses: STARTTLS task-
completion diagnostics and POP3 workload/truncation diagnostics are concrete
cold paths under 0020. Retain them, with branch-stable helpers inline. Its
POP3 observer records write entry, completion and count; it does not record
pending polls. Recheck server span, delayed writes and exact completion,
then reproduce truncation with the old whole-reply deadline. The supplied
progress harness has unbounded client reads and abort/join only at its happy
end: add an independent whole-test deadline and failure-safe task cleanup.
This is a concrete remaining proof repair, not a claim that run 2 already
satisfies every obligation. Do not repeat
run 1's client-duration-only proof or misdescribe run 2's observation.

Both panels support opaque paths. A quote-aware scanner or source-route
stripper changes acceptance/storage without being required by #127. Keep
the limitations documented and preserve exact envelope bytes.

## Project structure and files

| Files | Work |
|---|---|
| `src/smtp.rs` | Connection close policy; accepted-socket option; path guard; AUTH validator; existing seed caller audit; local tests/seams. |
| `src/smtp/protocol_edges_tests.rs` | Declared cfg(test) cold-path STARTTLS regression and only necessary helpers. |
| `src/pop3.rs` | Accepted-socket option; strict delimiter/UTF-8 handling; verbatim USER; shared progress writer; local tests/seams. |
| `src/pop3/progress_tests.rs` | Declared cfg(test) real-socket progress test with observation and cold failure diagnostics. |
| `src/model.rs` | One inbox predicate and table tests. |
| `src/webhook.rs` | Filter validation through that predicate and targeted validation tests. |
| `src/api.rs`, `src/mcp.rs`, `src/ui.rs`, `src/compat.rs` | Audit existing validator callers; change production only if an admission path bypasses the rule; MCP inline tests may expand. |
| `tests/smtp_edges.rs`, `tests/starttls.rs`, `tests/pop3.rs`, `tests/common/mod.rs` | Wire regressions, cross-plane routing, hostile bytes, existing TLS preservation pins and small client support. |
| `tests/api.rs`, `tests/mcp.rs`, `tests/compat.rs`, `tests/ui.rs`, `tests/webhook.rs` | Every admission caller, no-effect refusals and positive controls. |
| `benches/read_stall.rs` | Three pipelined group measurements alongside existing lock-step probes. |
| `README.md`, `src/static/llms.txt`, `tests/listener_contract_docs.rs` | Public limits, identity/timeout semantics and executable documentation checks. |
| `docs/decisions/README.md`, 0027/0028/0034/0036/0044 | Index 0046 as proposed; append dated proposal pointers only, preserving historical text and statuses. |

Only the three `specs/smtp-pop3-edges/` files and
`docs/decisions/0046-smtp-pop3-edges.md` belong to the design commit.

## Resume and implementation approach

1. Fetch origin and verify main contains `2a325c9`; create/check out
   `epic120-smtp-pop3` without resetting any existing work. Verify the design
   commit's identity and clean starting index. If Git metadata is sandbox
   read-only, disclose the exact failure; do not relocate metadata to evade
   it or claim a commit. The current design environment refused fetch at
   `.git/FETCH_HEAD`; local main and origin/main both resolve to `2a325c9`.
2. Run 2's patch includes the pre-synthesis versions of these four design paths.
   Preserve this synthesis. Apply all remaining original hunks using
   `git apply --index --exclude='specs/smtp-pop3-edges/*'
   --exclude='docs/decisions/0046-smtp-pop3-edges.md'
   .brokkr/state/epic120-smtp-pop3-run2.patch` (one command). Check first with the
   same exclusions and `--check`. These exclusions reconcile the mandatory
   design-first commit with the resume patch; no runtime hunk is omitted.
   Review every staged path/hunk against this spec, retaining only what holds.
   If this design could not be committed because metadata was read-only, first
   fetch/create the branch when permitted, validate and sign exactly its four
   files; do not fold an uncommitted design into the runtime patch silently.
3. Judge run 2's #121 repair first. Keep its small close helper at the connection owner:
   return session errors without shutdown, and bound shutdown only after Ok.
   Preserve original error and best-effort successful close. Keep task and
   socket lifetime together; do not release a separately counted permit.
4. Set TCP_NODELAY in both accept loops before any response. Keep failure
   connection-local, use existing test-only observation/injection patterns,
   and prove next admission. Do not add an option-policy abstraction.
5. Extend `envelope_path` with blank/control refusal, preserving framing,
   state precedence and returned slices. Extend `validate_inbox` once; wire
   AUTH decoding and POP3 USER to it. Preserve strict PLAIN field count and
   protocol-specific refusal transitions. POP3 must strip only its delimiter
   before strict decoding; retain argument treatment for other commands.
6. Audit each listed caller. Keep REST's existing error envelope, MCP -32602,
   UI/compat refusals and seed staging. Webhook filters call the same rule
   inside complete-list PUT validation. Do not retrofit trusted Store APIs.
7. Route every POP3 session write through the partial-write loop. Empty
   input completes; nonempty Ok(0) returns WriteZero; positive counts advance
   the slice; errors/timeouts propagate. Retain cap-refusal write and command
   reads as separate bounded operations. Keep assembled reply encoding.
8. Recheck existing tests, repair remaining gaps and docs, collect red/green
   receipts, then run all gates on
   the final tree. Preserve the original patch and prior notes as evidence;
   do not overwrite them or cite prior counts as a new run's results. Use
   `.brokkr/state/epic120/run2-commit-msg.txt` as the implementation message
   draft, updating its issue-to-test mapping and evidence to the actual tree.

## Data model and contracts

There is no schema migration. Email, nullable `envelope_from`, raw bodies,
recipients and stored extraction remain unchanged. Validation changes the
admitted name/path domain, not the representation of any accepted value.
Old control-bearing inbox rows remain; by-name validated routes refuse them.
Discovery and ID/global operations retain their existing contracts.

One validator table covers the full character policy. A caller inventory
must pair each route family with a representative refusal and valid positive
control; it must distinguish admission from internal Store operations.
For native await, assert no watcher is registered; for destructive scopes,
assert existing mail is intact; for webhook PUT, compare the installed list
before/after refusal. Preserve Prometheus control escaping unit tests even
though the old control-bearing API-name fixture must now assert refusal.

Correct README/module/helper claims that a reader receives all bytes "however
slowly": each next local write must complete positively within its window.
Correct 0036's dated proposal pointer consistently; keep historical text.
Update the progress harness comments/diagnostics and commit-message draft
that call elapsed writes "parked": name the observation actually made. Add
the armed-chaos path-refusal proof; run 2's new path matrix is unarmed.

Historical accepted decision bodies are immutable. Add dated pointers to the
new proposal, never wording that falsely declares operator acceptance. README,
module docs and the decision must agree on exact names and timeout scope.

## Proof design

Use real servers and generated TLS fixtures. Keep deterministic local helper
tests inline. Put the existing STARTTLS retry/deadline diagnostic and the
existing run-2 progress harness beside their owning modules under cfg(test).
Do not move unrelated inline tests to reduce the denominator. Always abort
and join spawned test tasks on cleanup, including the cold failure path.

For #121, the held read half, flood exceeding socket capacity and observed
reply timeout/task completion establish the failure while the peer is still
open. Fresh 220 after completion proves admission; EOF alone is insufficient.
Keep a small writer primitive test for never-ready shutdown and failure
preservation, alongside real TLS wire behavior.

For #131, observe the actual server write operation on the real OwnedWriteHalf.
Retain the cfg(test) observer recording each write entry, completion and
positive byte count; it must not manufacture progress or bypass transport
I/O. It does not observe `Poll::Pending`. Bound the test by an independent
failure deadline. Begin with run 2's 1 s idle windows and 50 ms paced drains
per 256 KiB. Require server completion beyond one window and at least two
writes delayed by 50 ms, plus the decisive old-deadline truncation control.
Duration is delayed-write evidence, not proof of individual pending polls.
Pacing creates a workload and supplies no event-ordering barrier. Kernel capacity varies;
the test must fail with a named diagnostic if the required condition was not
established, not silently count client pacing as proof. Adjust workload/buffer
size from observations within the existing message cap. Assert exact body and
terminator and no premature EOF; retain stalled-reader release tests.

Run the identical progress workload against restored whole-reply `write_all`
semantics and require a behavioral failure. The observer must still observe
real write activity in that control, rather than making a missing observer the cause
of the red. Helper tests with a bounded in-memory AsyncWrite cover partial and
zero writes; these supplement, never replace, the real-protocol regression.

For every issue, record test name, base/mutant behavior, expected assertion,
actual red result and final green result. Wire tests should run against
`2a325c9`; where a private helper/seam cannot compile there, restore only the
old faulty behavior in an isolated test tree and state that limitation.
Compile failure is never a behavioral red. Restore all mutants before gates.

## Quickstart and validation

Targeted suites include `smtp_edges`, `starttls`, `pop3`, `api`, `mcp`,
`compat`, `ui`, `webhook` and `listener_contract_docs`, plus the named library
helper/harness tests. Extend `read_stall`'s `--pipeline-groups` mode and record
all three shapes on comparable baseline/repaired builds, not just lock-step.

On the final implementation tree run, in order (run `cargo fmt --all` before
each check invocation as commissioned):

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
cargo deny check licenses
```

The code block lists gate order; execute `cargo fmt --all` again before
each subsequent check, not only before the first. No TMPDIR override. Coverage is exact missed == 0 in lines, branches and
functions. Also run the established fail-closed native-tls absence proof.
Verify owns `brokkr compile --bundle bundles/self` from the tooling-pinned
binary; a sandbox refusal is recorded as unproven, not a reason to discard
otherwise valid work or assert a warning-free compile. No installs in verify.

If cargo is refused, report it immediately, save the uncommitted patch at
`.brokkr/state/epic120-smtp-pop3.patch` only after preserving the original
resume evidence under a distinct name, and stop implementation delivery.
If signing is impossible, stop without an unsigned commit. The implementing
commit's imperative subject cites #121, #126, #127, #128 and #131; its body
maps each issue to the actual decisive tests and explains the reason for the
repairs. No push, PR, merge or issue closure is authorized.

The design seat validates nonempty artifacts, Acceptance Criteria/Risks
headings, ordered task checkboxes, relative links and proposed 0046, then
stages and signs exactly those four paths. It reports `drafted`, never
`designed`; the deterministic sequence step owns validation. If metadata
is read-only, leave the files reviewable, preserve a design-only patch and
record the uncommitted state in the result rather than claiming success.

## Risks

| Risk | Mitigation and remaining cost |
|---|---|
| A failed TLS closer consumes a second window or leaks detached work. | Return its error and drop at the owner; task completion plus fresh admission proves release. Missing close_notify after failure is intentional. |
| Accepted-socket setter failure kills the listener. | Connection-local drop and injected-failure/next-peer tests; keep cap refusal bounded. |
| Path refusal resets a valid envelope or runs chaos first. | Both caller matrices with the applicable chaos rule armed at 100%, nested-MAIL precedence and exact stored envelope assertions. |
| Padding or lossy decoding routes into another inbox. | One predicate, exact-byte routing test, pre-existing U+FFFD inbox negative control. |
| Historical names become inaccessible by name. | No silent migration; document discovery and global/ID cleanup limits; do not promise future cap eviction. |
| Webhook validation expands into unrelated #135 work. | Filter predicate only; atomic PUT refusal and memory-only lifecycle evidence. |
| Client pacing falsely proves server progress. | Record actual write entry/completion/count and unfinished server output beyond one window; require the old deadline to truncate the same workload. Do not claim pending polls from durations. |
| Runner jitter causes false timeout failures. | Initial 1 s window and 50 ms per 256 KiB pace, server observation, independent diagnostic deadline; adjust fixture size rather than weakening assertions. |
| Progress retains memory and slots indefinitely. | Disclose local-write semantics and the 12.5 GiB raw-message-scale exposure, not an RSS ceiling; retain silent-peer release proof. Future STLS requires write/flush reassessment. |
| Coverage pressure hides cold paths or production code. | Narrow declared harnesses under 0020; no coverage exclusions; exact by-effect gate and selftest unchanged. |
| Old patch overwrites the new design. | Exclude exactly the four already-authored artifacts on indexed application; review all remaining hunks and doc references. |
| Proposed amendments are presented as accepted history. | New 0046, dated proposal pointers only, explicit operator-owned status. |
| Sandbox prevents branch or signed commit. | Report actual refusal, preserve reviewable files/patch, never evade metadata restrictions or commit unsigned. |
