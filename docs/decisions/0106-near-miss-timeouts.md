# 0106 — Near misses on a timed-out wait

- **Status:** proposed
- **Date:** 2026-10-31
- **Scope:** Epic #164, issue #156, on main `d892acb` (merged #152, #153, #154).
- **Specification:** [Near misses on a timed-out wait](../../specs/near-miss-timeouts/spec.md).
- **Plan and proofs:** [plan](../../specs/near-miss-timeouts/plan.md),
  [tasks](../../specs/near-miss-timeouts/tasks.md).
- **Supplements:** the public output inventories of proposed
  [0082](0082-mcp-structured-output.md), [0094](0094-focused-mail-waits.md)
  and [0105](0105-mcp-report-output.md); refines 0082 §1's March sentence for
  one result shape, as named in §4 below.
- **Preserves:** 0001–0005, 0010, 0018, 0020, 0025, 0029/0030, 0044, 0047,
  0071 and every accepted ruling; 0079/0080 deviations and proposal statuses
  unchanged.
- **Number:** 0094 recorded #156 as `0093-near-miss-timeouts`; main's #163
  delivery now holds `0093-source-mail-comparison.md`. 0106 is the next free
  number locally. The collision is the operator's to resolve; nothing here
  renumbers another record.

## Context and authority

Issue #156 asks that a timed-out `swarmail_wait_for_email`, and #154's
focused waits, name their near misses — mail that arrived to other
recipients in the window, the nearest subject match and the inbox's last
arrival time — read from the store with no extra retention and bounded in
size, so that the slow-delivery and sibling-address traps show recovery in
the eval harness while no near miss leaks across the AUTH-is-inbox boundary.

A contrarian panel argued the design; the chief's rulings are in the
specification. The change adds a public output member to three tools, so it
carries a proposed decision. Only the operator accepts it. Compatible
implementation may proceed under accepted 0045 §8; nothing accepted is
contradicted and no frozen guarantee is weakened.

## Proposed contract

1. **One read, own inbox, after the wait.** When a wait resolves as timed
   out, the MCP adapter takes one `Store::list(inbox, &Filter::default())`
   snapshot of the wait's own inbox and derives every member from that
   vector. Nothing is computed inside `wait_for`, `wait_for_report` or
   `wait_focused`; no `list_all` or cross-inbox read; no store field, index,
   cache, counter or task is added; nothing accumulates in the dispatcher or
   the firehose. The found answer, `matched`, `total` (and its 0018 residual),
   the error arms and the wait deadline are unchanged.
2. **The member.** A timed-out answer carries `near_misses` exactly when the
   snapshot holds at least one mail: `arrivals` (candidates newest first, at
   most 10, exact `arrivals_total`, `arrivals_truncated`), `nearest_subject`
   (an entry or `null`) and `last_arrival_ms` (the newest retained mail's
   `received_ms` in acceptance order). A candidate honours the caller's
   `since_ms`; for `swarmail_wait_for_email` it is mail the filter does not
   admit, for the focused waits every retained mail. Each entry is
   `email_id`, `received_ms`, `from`, `to` (distinct To/Cc/envelope
   addresses, at most 5, with exact `to_total`), `subject` and `missed` — the
   supplied filter fields the entry fails, in the order `to`, `from`,
   `subject`, each decided by the caller's `Filter` projected to that field
   through the existing `Filter::matches`. Strings are clipped to 256 UTF-8
   bytes on a char boundary. `nearest_subject` ranks only candidates whose
   `missed` is exactly `["subject"]`, by distinct lowercase character bigram
   overlap with the requested subject; ties go to the newest; zero gives
   `null`. The path is infallible.
3. **Tenant boundary.** The read is keyed by the wait's `inbox`; an answer
   about one inbox says nothing about another, proven over SMTP AUTH on the
   real wire. This is not access control: MCP remains unauthenticated under
   0004, and the feature claims nothing more.
4. **Revisions.** The object is part of the one captured Value: text content
   on March, text plus `structuredContent` on June and November. The output
   schema of `swarmail_wait_for_email` gains a third closed `oneOf` variant
   and each focused wait a second closed `timed_out` variant; every emitted
   member is required (0094 §7). Tool descriptors and March descriptor bytes
   are not edited. Where proposed 0082 §1 says March "retains descriptor/
   result bytes exactly", this record states the one refinement: a March
   timed-out result over a non-empty inbox gains the member. Every captured
   March baseline line, all of them empty-inbox timeouts, stays
   byte-identical, and no provenance record is re-baselined.
5. **Harness and metric.** No catalogue, goal, scoring or timeout change
   (0071). Three scripted controls through the real CLI prove that recovery
   from the timed-out answer is mechanically possible on the sibling and
   slow-delivery tasks and that acting on `nearest_subject` in the sibling
   trap still loses as `wrong_message`. The rate of recovery after a failed
   wait is **not measured**: it needs paid live-model trials this delivery
   does not hold, and slow-delivery's goal already prescribes the release.

## Synthesis and rejected alternatives

Adopt simplicity's window (`since_ms`, the caller's own bound) over
robustness's wall-clock capture at park: a wait-start window is per-wait
state beside 0002 and empties the sibling trap, whose mails land before the
agent's first call. The cost is that the block makes no "during the wait"
claim; `received_ms` on every entry and the whole-inbox `last_arrival_ms`
carry that judgement to the agent.

Adopt robustness's substance on "why it missed" and on `nearest_subject` as
a full descriptor, but through single-field projections of the existing
`Filter::matches` in the adapter rather than a new store seam, which keeps
one predicate and a zero `src/store.rs` diff. Rank only candidates that
passed every other supplied field. A bare nearest subject would present the
sibling decoy as the match.

Adopt simplicity's omit-when-empty and all-revisions presentation over
robustness's always-present nulls and a modern-only member: the harness and
missing-version clients speak March, and the captured baselines stay
untouched. Adopt robustness's acceptance-order `last_arrival_ms`, byte caps
with boundary-safe clipping, and infallibility. Reject the per-wait id
baseline, the nine-value reason enum, edit distance, the whole-object trim
loop, REST and compat surfaces, a new tool or input flag, and any dependency.

## Proof and ownership

Spec AC1–AC11 and tasks T1–T12 name the witnesses: inline derivation tests,
real-wire shape, tenant and unchanged-store tests, revision and schema
proofs with negatives and the untouched March comparisons, the three eval
controls, and every house gate locked with exact coverage. CI-owned contexts
remain CI's. No push, merge, publication, issue closure or acceptance is
authorized by this record.
