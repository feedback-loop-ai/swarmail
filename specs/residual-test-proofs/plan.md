# Plan: residual test proofs

## Technical context

One Rust 2024 crate, existing Tokio networking/time support, real HTTP/SMTP
integration servers and an ordinary `Mutex<BTreeMap<String, u64>>` readiness
registry. No dependency, build feature, data migration or production API is
needed. Linux and macOS are the socket proof platforms. Implementation starts
from the integrated `34eb99f` runtime tree plus this design commit.

The repository has no `plan-template.md` (checked `.specify` and `bundles`).
This plan supplies the requested template sections directly: technical context,
constitution check, project structure, research, data model, contracts,
quickstart and risks. No extra design files are needed.

## Constitution check

Preserve 0001–0005, 0041's independent webhook fan-out, 0047 §2's held snapshot,
and existing validation/retry contracts. Keep deterministic inline assertions
measured under 0020; do not create a cold scheduling loop or coverage exception.
Use the house gates and pinned tooling/proofs under 0050. Retain `engine` triage
and policy review under 0057; no route changes. Commit unsigned under 0056,
never publish. No semantic change means no proposed ADR is needed.

## Project structure and files

| File | Retained implementation |
|---|---|
| `tests/common/mod.rs` | Move/correct three descriptions; delete only unused `wait_for_metric` and its documentation after caller proof. |
| `tests/webhook.rs` | Own a bound non-listening socket, probe refusal, drop after final observation in `refused_connection_never_blocks_the_smtp_path`. |
| `src/api.rs` | Extend only inline `cumulative_scrape_uses_one_registry_snapshot` with a complete-function source assertion. |
| `Dockerfile` | Read-only byte comparison with `34eb99f`, preserving AC52.2. |
| `specs/residual-test-proofs/{spec,plan,tasks}.md` | Exactly the chief's committed design artifacts. |

Production `cumulative_rows`, `render_rows`, registries, dispatcher, SMTP/store,
Cargo files, gates, policies and accepted decisions are read-only context.
Temporary production mutants are uncommitted proof material and must be restored.

## Research and resolved choices

Inspection confirms the misplaced paragraphs and unused-wrapper declaration,
the released listener in the refusal fixture, and the correct one-guard
production implementation beside a non-discriminating idle-output test.
Both panel positions support the three-file approach. The spec records explicit
adoptions, rejections and the strict-normalization ruling with its maintenance
cost. Supported-platform socket behavior must be executed, not inferred.

## Implementation approach

1. Repeat an exact-word repository search for `wait_for_metric` before deletion,
   including hidden source/config files as applicable, excluding VCS objects,
   build output and ignored run logs. Review imports and non-Rust hits. The
   design's prose references are expected. Record no-callers evidence; if any
   executable use exists, retain the wrapper and return for scope resolution.
   Move the raw-text paragraph to `http_send_text` and the Origin paragraph to
   `http_send_text_with_headers`. Document mandatory verbatim Host on
   `send_with_host`; leave its request-building code byte-identical.
2. Locally create `tokio::net::TcpSocket::new_v4()`, call
   `set_reuseaddr(false)`, bind `127.0.0.1:0`, and obtain `local_addr()`.
   Never enable port reuse, listen, or connect this owning socket. Use
   `tokio::time::timeout(common::HELPER_DEADLINE, TcpStream::connect(addr))`
   (or existing imports of those names) for a single real probe; assert the
   resulting I/O error kind is `ConnectionRefused`. No retry or fallback.
   Keep the socket variable in the test and `drop` it only after the final
   count assertion. Keep baseline/send/poll/count operations unchanged. Adjust
   the nearby comment to avoid claiming a measured SMTP-versus-retry ordering.
3. Inside the existing snapshot test, retain its output assertion and add
   `include_str!("api.rs")` inspection. Restrict the source to the prefix before
   the actual `#[cfg(test)]` / `mod tests {` declaration. Find the full anchored
   signature `fn cumulative_rows(registry: &Registry, name: &str) -> String {`
   and the following anchored `fn render_rows(` declaration in that prefix.
   Extract the entire intervening function, including its signature and closing
   brace. Use mandatory boundary extraction (`expect`), never default/empty
   fallback. Compare to a literal of the reviewed current function:

   ```rust
   fn cumulative_rows(registry: &Registry, name: &str) -> String {
       let units = registry
           .lock()
           .expect("a readiness registry mutex cannot poison");
       render_rows(&units, name, "counter", units.values().sum())
   }
   ```

   At most normalize CRLF to LF and trim outer whitespace. Do not strip
   comments, normalize tokens, count `.lock()` occurrences, or search the
   embedded expected string. No parser, new source-policy framework or runtime
   hook. Explain the structural nature and accepted refactor coupling in the
   test comment. Keep all new assertion paths branch-stable and inline.
4. Capture focused current-pass output. Temporarily replace only production
   `cumulative_rows` with a valid two-snapshot body: sum under a scoped guard,
   release it, acquire another guard, then pass the saved total and second map
   to `render_rows`. Keep the expected literal unchanged. The behavioral output
   assertion still passes; the new source assertion must fail. A compiler error
   or unrelated failure is not evidence. Save the failure diagnostic, restore
   exactly the saved original function, check that the production diff is empty,
   and rerun the same test to green before any final gates. Never commit mutants.
5. Exercise affected real-server consumers, inspect the allowed diff and run
   the complete house inventory. Record command, platform, candidate identity,
   exit status and relevant diagnostic in ignored run-local evidence. The
   independent verifier owns its pinned proof execution; author evidence does
   not replace that seat or CI's additional proofs.

## Data model and contracts

No persistent data or public schema changes. The reservation is one test-owned
socket with lifetime setup → probe → PUT → baseline → SMTP → first count →
exhaustion observation → final count → drop. Timeout is a failure bound, not a
latency success criterion. The snapshot test owns its private map; the existing
output contract includes the disjoint LF overflow row. The source literal pins
one implementation of the already-accepted one-snapshot contract and rejects
local delegation, without claiming semantic analysis of arbitrary Rust.

## Quickstart and proof sequence

```bash
cargo test --locked --lib api::tests::cumulative_scrape_uses_one_registry_snapshot -- --exact
# Repeat that same command with the temporary compiling mutant, require the
# source assertion failure, restore the production function, then repeat green.
cargo test --locked --test webhook refused_connection_never_blocks_the_smtp_path -- --exact
cargo test --locked --test api
cargo test --locked --test lifecycle
cargo test --locked --test binary
```

Run the refused-target command on both Linux and macOS. The API suite covers
Host/Origin and row consumers; lifecycle and binary include helper-deadline and
HTTP reliability consumers. Full workspace tests cover remaining metric polls.
Do not write redundant new tests for documentation movement or deleted wrappers.

Then execute the authoritative [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change)
in full, using the candidate-local digest-matched `.forge/bin/brokkr` for bundle
compilation and pinned proof scripts/checked shims as required. That inventory
includes fmt, locked clippy/workspace tests, compile, exact coverage and its
selftest, publish/seal proofs and license checks. Execute the pinned verifier's
fail-closed native-tls check: only a failed `cargo tree -i native-tls` explicitly
reporting package absence proves absence; success or unrelated failure refuses.
CI-owned chart/history/selftest, Docker smoke, advisory audit, tag release-rate
and privileged port-80 proofs remain distinct obligations per the house rules.
Neither platform availability nor green design validation waives any gate.

## Risks

| Risk | Mitigation and accepted cost |
|---|---|
| A listener changes refusal to handshake timeout; early drop releases ownership. | Use a bound, non-listening socket with reuse disabled, actual bounded refusal probe and explicit final drop. One descriptor remains held. |
| Linux behavior is assumed on macOS. | Run the same real regression on both; fail visibly and retain missing platform evidence as outstanding, with no silent fallback. |
| A green idle-output assertion blesses split locks. | Require a compiling split-lock mutant to fail the new assertion, then restore and rerun. |
| Source extraction finds its own expected string. | Limit to production prefix, anchor full signature/next declaration, require boundaries and compare complete function. |
| Harmless refactoring trips the guard. | Deliberately review/update the tiny expected body and rerun the mutation proof; accept formatting sensitivity rather than add a parser. |
| Source pin is mistaken for runtime lock instrumentation. | State its structural limit and retain real output checks; no claim about arbitrary callee internals. |
| New inline cold branches break exact coverage. | Use straight-line assertions, preserve measurement, and run exact gate/selftest; no harness move or exclusion to evade it. |
| Wrong helper removal or normalization weakens observations. | Repeat caller search, preserve live baseline/row helpers and verbatim Host/Origin code, run existing consumers. |
| Mutant or unrelated cleanup leaks into final patch. | Restore saved production bytes, inspect final diff, compare Dockerfile to baseline and restrict retained edits to the named regions. |
| Design success is confused with delivery proof. | Keep tasks pending until executed receipts exist; independent verify/review and CI retain their roles. |
