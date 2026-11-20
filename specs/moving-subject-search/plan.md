# Implementation plan

## Technical context and panel ruling

Adopt simplicity's single retry owner in `NativeSource::poll` and reuse the eligibility logic in `owned_listing`. Adopt robustness's absolute per-page cutoff and complete-scan accounting. Rule against sharing cleanup's entire retry loop. It sleeps 10 ms and its final error can describe an earlier move even when a later request failed independently. Separate caller control costs one small poll-local state transition and preserves the measured cadence and cause. Repeated requests cost real client CPU and HTTP time inside the measured window. Accept that cost up to the wait cutoff even when a permanently inconsistent responder produces the same typed refusal.

Simplicity's zero-new-files recommendation applies to implementation. This design gets its own spec directory so the deterministic check can identify it. Keep the original reference-suite design and append its implementation ledger separately. The scripted proof establishes which requests overlapped a move. The host proofs establish the real coordinate outcomes even if their searches happen to settle on every attempt.

## Research, data model and contracts

In `evals/src/native_source.rs`, `collect` calls the shared client's pager and `scan` currently maps its error to a string-bearing failure before `poll` sees it. The eligible search reads one scope. Factor the common neighbour and scope predicate and typed refusal check from cleanup. Keep the typed collection result through the poll decision. Use a wait-local indication of the last unanswered move and reset it on a complete read. Fold eligible results and update last_scan only after successful in-budget collection. A refused scan cannot mark a previously seen key missing. Preserve the stream scanner's error conversion, cleanup's deadline and diagnostic, and the work deadline used for checked fetches.

Compute the poll cutoff once from the trial work deadline less REQUEST_BOUND. Pass it to candidates for every page and bound the in-flight future at that instant. Guard entry and wakeups before I/O and check the cutoff before accepting a result. Schedule one ordinary poll interval after a refusal. If no retry fits, expire by the cutoff without issuing I/O. When the cutoff cancels a retry after movement, preserve the moving diagnostic. Map independent request errors by their existing typed cause. Use the existing Failure, timing and row reason through testexec's classifier. No wire schema, digest, lockfile, dependency, root production code or classifier change is needed.

## Project structure and quickstart

Extend the scripted Mailpit search handler in `evals/tests/testexec.rs`. Snapshot the first page and old total, then apply the scheduled same-subject foreign delivery before returning that snapshot. Keep both totals within two pages and log offsets per search independently of cleanup. Use fresh sources and a short injected budget beyond the reserved tail. Bound fixture waits and join paths. Time the locked eval testexec binary against the built candidate and record elapsed test time in `specs/reference-suite/tasks.md`.

In `tests/mail_compare/api_section.rs`, add thin wrappers around real_suite and extend its existing discovery/refusal table. Use suite_vars, pinned_args and scoped_driver. Copy story D's two cfg_attr ignore reasons, including the exact non-Linux /proc marker. In `docs/evals/README.md` correct both stale status passages and extend the existing run_one instructions. Build the locked driver and eval binary, set MAIL_COMPARE_BIN and SWARMAIL_EVALS_BIN, and verify the applicable MAIL_COMPARE_MAILPIT or MAIL_COMPARE_SMTP4DEV executable SHA-256 against its target entry in docs/evals/draft/sources.lock.v5.json before launch. Run separately:

```sh
run_one api_section::testexec_real_mailpit_p32
run_one api_section::testexec_real_smtp4dev_p8
run_one api_section::testexec_real_smtp4dev_p32
```

The existing wrapper records /proc/loadavg on both sides, preserves the exit and uses timeout 600 taskset -c 0-31 with one exact ignored test. Only the driver's hold enters systemd-run --user --scope -p Delegate=yes. The dispatcher owns positive execution and PR evidence.

## Constitution check

Use the existing shared clients and accepted 0132 outcome vocabulary. Preserve frozen store, await, pruning and ingest guarantees. Keep the pager and both dependency locks intact. Run the house inventory in the foreground with the candidate binary and locked workspaces. No collection, freeze, publication or push belongs to this work.

## Risks

- A partial page containing all expected recipients could falsely pass or manufacture disappearance. Stage the eligible response until the pager and deadline both accept it. Verify counts, identities and accounting.
- A ready loopback socket or delayed timer can cross the cutoff. Test an expired budget before connection and a stalled in-flight response with a bounded watchdog.
- Shared CI load can obscure timing assertions. Trigger deliveries by request ordinal and prove ordering by page counts. Use short test budgets and record the runtime against story D's roughly five seconds.
- An absent binary or undelegated scope could masquerade as target failure. Require named prerequisite refusals locally, use the established scoped driver on the dispatcher, and retain exact evidence for every non-clean run.
- Owned asymmetry (review return 1, stated, not changed). The poll's cutoff, the work deadline less REQUEST_BOUND, now bounds every page of every poll scan. The native await's HTTP call and the stream's scans are still bounded by the full work deadline. A poll answer still in flight at the cutoff is therefore refused where an await's arriving then would be taken: one window, one scan wide. AC3 requires it. Because every page is bounded by the cutoff, a pager timeout in a poll is always the wait's own cutoff. It ends as the wait's timeout, saying what the last attempt saw: still moving, a short count, or nothing complete yet.
