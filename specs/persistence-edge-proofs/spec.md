# Specification — persistence edge proofs

Created: 2026-10-24. Status: draft for implementation.
Scope: epic #192, issue #187; this run's fresh candidate at `238372d`.

## Feature and rationale

Pin existing single-writer admission and legacy restore behavior with
regressions that distinguish the failures they claim to detect. Operators
need predictable path errors and alias exclusion; maintainers need a legacy
ordering test that rejects loss of the explicit tie-break and a concurrency
probe that cannot silently accept failed mutations. Restore order determines
the oldest prune candidate after restart, so a masked ordering regression
can change retention, even when all restored IDs are present.

This is a proof and documentation change. Shipping behavior, public APIs,
data-path semantics, schema, dependencies, accepted decisions, gates and
recipe policy remain unchanged. No decision is proposed or amended; in
particular, do not add or implement 0062 or reuse old stopped-worktree drafts.
If proof requires a semantic change, report the precise conflict for design
return rather than expanding this scope.

Authorities: accepted [0033](../../docs/decisions/0033-one-server-per-data-file.md),
[0042](../../docs/decisions/0042-single-writer-lock.md), and
[0047 §1](../../docs/decisions/0047-storage-rpc.md) govern ownership and errors;
[0014](../../docs/decisions/0014-one-ordered-mutation-boundary.md) governs
ordered mutations; [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md)
and the [house rules](../../docs/house-rules.md) govern proof placement and gates.
These links cite authority, not new decision declarations.

## Panel synthesis and rulings

Both run-local positions were read with the intake framing. They agree on
the scoped solution; there is no substantive opposing vote to invent.
The following choices adopt their shared conclusions and resolve the
remaining optional approaches explicitly.

| Choice | Ruling and cost |
|---|---|
| Post-open seam versus accepted explanation | Adopt simplicity's no-seam choice and robustness's explicit proof limit. Reject the optional callback/interleaving seam: it adds isolation and coverage obligations for namespace replacement outside the accepted boundary. Cost: no new executable proof that removal of the second check fails. Preserve that check. |
| Small local fixtures versus adversarial assertions | Adopt both positions' typed path assertions and adversarial legacy fixture. Reject a happy-path-only restore test, debug-string-only error assertion, and Python/planner-only ordering evidence. Cost: a deliberately unusual fixture index and a mandatory Rust mutation receipt. |
| Alias behavior versus guard attribution | Adopt robustness's distinction: relative paths and symlinked parents can share a pathname sidecar. Test their observable exclusion without claiming they alone prove identity rendezvous. Retain existing distinct-sidecar identity proofs; reject an additional helper-level convergence test as unnecessary here. Cost: these new path tests do not identify which guard refused. |
| Worker error handling | Adopt simplicity's local expectations and robustness's requirement that every mutation error fail the joined probe. Reject an error channel, retry loop, global hook or unrelated discarded-Result audit. Cost: worker panics are the diagnostic mechanism already owned by the join. |
| Broader failure analysis | Retain robustness's crash, locking and stable-namespace limits as constraints. Reject new webhook, TLS, SMTP, chaos or durability work: those risks have existing owners and proofs. Cost: this issue supplies no new proof for those unrelated surfaces. |

## Post-open validation: the authorized citation alternative

Accepted [0047 §1](../../docs/decisions/0047-storage-rpc.md) states exactly
(line wrapping normalized):

> Validate the opened metadata before locking too.

> The verified private, stable parent is the trust boundary, not a promise against hostile same-user replacement.

> Namespace replacement during lookup/open is outside the stable-namespace contract.

The existing `identity_guard` validates regular-file type, owner, link count
and private permissions before opening an existing guard and on the opened
handle before locking. It does not compare pre-open and post-open inode
identity. An artificial replacement between those steps can test the second
check, but does not establish a broader supported guarantee. The task
expressly permits the accepted defense-in-depth explanation, so no new seam
is justified for this scope. Existing unsafe-entry tests exercise the
predicate; they do not prove the post-open call cannot be removed. Keep the
call and record this rationale in its owning documentation.

## Acceptance Criteria

1. **AC1 — Honest guard evidence.** The implementation documentation cites
   the three exact 0047 statements above, preserves the post-open
   `check_guard` call before `try_lock`, and describes the no-seam rationale
   and absence of a new race/mutation proof. No inode-equality or hostile
   same-user replacement guarantee is asserted.
2. **AC2 — Dangling data symlink.** On supported Unix proof platforms, a
   fresh dangling symlink with an existing target parent fails real
   `Persist::open` as typed `OpenError::Io`: exact supplied data path,
   `ErrorKind::NotFound`, and raw ENOENT. Its entry is still a symlink,
   `read_link` is unchanged and its target remains absent. An already-created
   literal-path sidecar is permitted. README and persistence module docs
   state this existing Io(ENOENT) refusal and absence of target creation.
3. **AC3 — Path spellings.** Isolated tests explicitly use a genuinely
   relative path and a symlinked parent, with no process cwd/environment
   mutation. For each spelling, opening an absent database succeeds. For
   each alias/absolute-real-path pair, both acquisition orders produce typed
   `OpenError::InUse` carrying the exact rival spelling, and dropping the
   owner permits reuse through the rival spelling. Preserve existing
   file-symlink, hard-link and cross-directory hard-link proofs. Document
   these paths under the existing stable shared namespace boundary.
4. **AC4 — Discriminating legacy restore.** Keep the old-schema fixture and
   exact `first, tied-a, tied-b` write-order assertion in
   `a_legacy_duplicate_seq_restores_in_write_order`. Strengthen fixture DDL
   so removing only `rowid` from the actual `restore_query` makes that same
   Rust test fail on the reversed tied records with locked SQLite. Record
   baseline pass, precise mutation, assertion failure, restoration and
   restored pass. Keep `the_restore_scan_reads_the_seq_index` green on its
   independent ordinary-schema database. No mutation remains in final code.
5. **AC5 — Checked concurrency mutations.** Every `delete`, `clear` and
   `clear_all` Result in `run_boundary_round` is checked: Err fails the
   worker and existing joins fail the test. Ok(false) and Ok(0) remain
   legitimate outcomes. Preserve all sixteen barrier-released roles, cap
   one, fresh per-round files, no injected storage failure, final nonempty
   witness and live/reopened order, count, counter and cap comparisons.
6. **AC6 — Scope and guarantees.** Changes are confined to owning tests,
   comments and persistence documentation. Keep synchronous accept-after-
   storage, subscribe-before-check, oldest-only pruning, ingest extraction,
   plan/commit/apply order, WAL/FULL and the 5000 ms busy default. Keep both
   guards before SQLite/restore/bind, connection close before guard release,
   literal sidecars, nonblocking acquisition, no unlink and metadata-only
   reads of existing database identity. No production SQL, path policy,
   schema, dependency, accepted decision, gate or recipe change.
7. **AC7 — Delivery evidence.** Implementation/verify must pass targeted
   Rust and real-process regressions, every current
   [house-rule gate](../../docs/house-rules.md#gates--part-of-every-change)
   and the fail-closed native-tls absence proof, with exact coverage and
   pinned tooling. Retain the selected engine route and all four configured
   independent reviewers. Local results do not certify macOS or CI-only
   checks. Missing evidence remains outstanding, never a claimed pass.

## Delivery boundary

This phase commits spec.md, plan.md and tasks.md only. Its result is
`drafted`; the sequence's deterministic check owns design advancement.
Implementation, mutation receipts, full gates and policy-selected review
remain downstream obligations. Use codex-fast's pinned Luna xhigh
implementation and Qwen Max correctness, Opus security, official DeepSeek
Flash guarantees and DashScope GLM 5.3 tests review, without DeepSeek Pro or
fallback. The engine selects phases; this seat dispatches none. Never push,
merge, tag or publish.
