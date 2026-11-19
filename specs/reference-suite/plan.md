# Implementation plan

## Technical context and constitution check

Use the standalone Rust eval workspace and the bench driver's existing Linux ownership boundary. The draft protocol supplies corpus seed 163. Its client implementations already build in both workspaces. Preserve the root server's synchronous accept, subscribe-before-check, oldest-only pruning and ingest-time extraction. Keep both dependency trees within the existing locks and license allowlist. The decision inventory already lists accepted 0132. Run the complete [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change), including the separate eval workspace against the candidate binary.

## Research and panel rulings

Adopt simplicity's one-coordinate invocation, shared queue and single holder. The API holder's cell sequence validates API-specific fills and windows. Give testexec a separate bounded suite-window command sequence using the same holder operations. This costs two control round trips but retains one owner of the sampler, process tree and receipt.

Adopt robustness's disjoint address plan. SignupCrowd derives four aliases from its primary, including a literal trailing 2. At ordinary ordinal 1 that alias equals primary 12. Serializing those two jobs leaves late delivery and cleanup ambiguous. Numbering from 100 gives up that pair in the measured suite. Retain 1/12 as a negative ownership test. Use simplicity's Mailpit proof and story B's existing test binary and scoped_driver. This exercises exact filtering and per-ID cleanup under overlap. The cost is that a second end-to-end MailHog stream proof falls outside this story. Cover its count predicate in deterministic tests and keep story C's stream proof.

Adopt robustness's causal accounting and handler drain. Keep section-level report recomputation with this implementation. A second cross-section admission engine, new supervisor, TLS campaign or new fixture scenario would expand the story without proving a reference trial. Full-run admission and collection orchestration stay with story E.

## Project structure and contracts

- In `docs/evals/draft/protocol.v4.json` and `benches/mail_compare/protocol.rs`, describe and validate the suite recipe, seed, mapping, allowed coordinates and bounds. Update the outstanding-section bookkeeping only when the section works. Preserve historical validators, current draft status and freeze guards. Keep shared-client digest metadata consistent with the implemented modules.
- In `evals/src/main.rs` and `lib.rs`, wire `testexec` using the api-mcp conventions for driver, mode, protocol, lock, documents, chunk, run ID, output and pinned executable/runtime arguments. Reject malformed or mismatched coordinates before spawning. Refuse existing output. In `testexec.rs` assign IDs 100+i, flow by i modulo five in spec criterion 1 order, for i=0..99. Order claims by ascending SHA-256 of the ASCII string `testexec/<decimal seed>/<decimal id>`, breaking ties by ID. Use that digest's first eight bytes big-endian for each public trial seed. Record the mapping, keeping it equal across repetitions and targets. One next-index queue and one active trial per worker suffice.
- In `fixture.rs` and its HTTP handling, retain one listener pair per worker. Install fresh private state and provision each trial through the existing prepare path. Pin accepted requests to that generation and track outstanding work. Drain before handing the listener to another trial. Cut the suite if quiescence cannot be proved within budget. In `oracle.rs`, factor the existing magic/reset solution bodies to accept the existing MailSource alongside solve_hard, preserving reset-reuse and existing MCP callers. Run tasks::score on the real snapshot. Restrict identity-bearing app actions and redirects to the worker's fixture origin.
- Implement `evals/src/native_source.rs` on the existing path build of `benches/mail_compare/clients/`. Select wait-best from the validated evidence. Reuse stream readiness, retry-on-reconnect scans and backstop in the shared observer, generalizing its predicate for complete subject/count answers. Use public scenario addresses only. For multi-mailbox crowd, union complete results by (origin mailbox, target ID), retaining real To, subject, text and HTML. Recheck identity after fetch. Instrument the shared family filter boundary before local rejection, including filters inside family modules. Preserve existing list/search results. Track distinct IDs solely for accounting and owned cleanup. Surface partial pages, malformed fetches and contradictory identities as operation failures.
- In `evals/src/mail.rs`, observe the final DATA reply before QUIT, keeping stage and code even across cancellation. Associate receipts with trial and send ordinal outside oracle decisions. For a count wait use the acceptance completing the required matching set, and use the wait-return instant on the same monotonic clock. Keep absent endpoints explicit. Clip each I/O, worker join and cleanup to an absolute deadline. A delayed QUIT, lost reply or crash cannot authorize resend.
- Reuse `benches/mail_compare.rs`, the holder's window/cleanup operations and `evals/src/apibench.rs` control framing. Start sampling before releasing ready workers, end the window after their last end, and record both round trips separately from suite wall time. Read RUSAGE_SELF at the trial bookends (including co-resident fixture CPU). Place eval worker threads on the client CPUs before starting them. Budget against the existing 560 s inner chunk and 600 s outer proof cap. Close the holder and verify target identity, read-back limits, reap, leaf removal, listener and scratch receipts even after failure.

## Data model and verification

Persist a planned coordinate with policy, method and ordered trial identities. Reserve a started row atomically with claim. Record worker, scope, flow, start/end, send/wait evidence, scorer judgement, foreign_seen, outcome cause and cleanup disposition. Keep tokens, bodies and credentials out of evidence. Bind launch, suite window, CPU deltas, samples and final cleanup to that coordinate. Derive wall intervals, class totals and coverage from rows. Reject duplicate/missing IDs, contradictory classes, resource identity mismatches and missing cleanup. For global-only parallel targets, report the deferred cleanup result on the suite. A teardown failure invalidates sealing regardless of scored passes.

## Quickstart for dispatcher proofs

Document these exact invocations in `docs/evals/README.md`. Build the locked root candidate and mail_compare bench, the locked eval binary, and obtain the prebuilt root mail_compare test executable (`bin`) from Cargo's artifact record. Set MAIL_COMPARE_BIN and SWARMAIL_EVALS_BIN to those built executables. Use story B's pinned_args to supply Cargo's candidate Swarmail and set MAIL_COMPARE_MAILPIT to the lock-pinned Mailpit executable. Record candidate digest and verify lock-backed inputs before launch. Only scoped_driver wraps the actual driver hold with `systemd-run --user --scope --quiet -p Delegate=yes --`. Cargo, the test process and eval client run outside that scope.

```bash
run_one() (
  cat /proc/loadavg || exit 2
  rc=0
  timeout 600 taskset -c 0-31 "$bin" --ignored --exact "$1" --test-threads=1 --nocapture || rc=$?
  cat /proc/loadavg || exit 2
  exit "$rc"
)
run_one api_section::testexec_real_swarmail
# Separate dispatcher invocation
run_one api_section::testexec_real_mailpit
```

## Risks

- A late HTTP request or SMTP reply can outlive the caller. Keep the original identity, close its admission to new work, and settle active handlers before reuse or global clear. Preserve unknown acceptance and unproved cleanup explicitly.
- A noisy filter can count the same foreign ID many times or delete a neighbour. Validate the complete address map, instrument pre-exact candidates, and bind each mutation to verified ownership. Keep typed failure causes alongside the oracle's string error interface.
- Paging and filter semantics differ by family. Verify route and cleanup capabilities against lock v5 citations, including MailHog's delete route, and preserve documented capability gaps. Keep setup refusal, fill shortfall, ordinary deadlines and target exits distinguishable.
- Wall-clock deadline tests would exhaust CI headroom. Use Tokio paused time or immediately resolving fixtures and bounded event barriers. Keep real launch tests Linux-marked and record per-binary runtimes. The two ignored runs require separate dispatcher evidence before merge.
