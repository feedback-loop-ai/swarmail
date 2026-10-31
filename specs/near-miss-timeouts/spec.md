# Near misses on a timed-out wait

Date: 2026-10-31. Epic #164, issue #156. Design on main `d892acb`, which
carries the merged #152, #153 and #154 work.

Decision: 0111 — [Near misses on a timed-out wait](../../docs/decisions/0111-near-miss-timeouts.md), proposed.

## Feature and rationale

An agent whose `swarmail_wait_for_email`, `swarmail_wait_for_code` or
`swarmail_wait_for_link` times out today learns only `matched`, `total` and
`timed_out` (or a `status` of `timed_out`). It cannot tell a mistyped filter
from slow delivery, and it cannot tell "nothing arrived" from "the mail is
here, addressed to a look-alike". The eval harness's sibling-address and
slow-delivery traps are exactly those two confusions.

A timed-out wait now names its near misses: the retained mail in the
waiter's own inbox that the filter did not admit (bounded, newest first, each
entry saying which requested field it missed), the one retained mail whose
subject lies nearest the requested subject, and the inbox's last arrival time.
Everything is read from the store at the moment of the timeout, with no new
retention, index, counter or task, and every string and list is capped by a
named constant. The found path, `matched`, `total` and the wait owners are
untouched.

The metric this serves is the rate of recovery after a failed wait. The
design makes recovery mechanically possible and proves it with deterministic
scripted controls; it does not measure a live-model rate (see
[Harness and metric](#harness-and-metric)).

This additive public output contract preserves
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md),
[0004](../../docs/decisions/0004-per-test-inboxes-via-smtp-auth.md) and
[0005](../../docs/decisions/0005-extraction-at-ingest.md). Compatible work may
proceed under [0045 §8](../../docs/decisions/0045-delivery-repairs.md); the
proposal accepts nothing and permits no weakening.

## Evidence and authority

Both run-local positions, `.brokkr/state/design/positions/simplicity.md` and
`.brokkr/state/design/positions/robustness.md`, were read in full. The
intake framing directory `.brokkr/state/tasks/` is absent in this run; the
framing is the intake result `192c49a1-d999-47bc-ad69-66bfc1f8e986` and the
issue text. The code cited by both positions was checked on `d892acb`:

- the report wait's timeout arm, `src/mcp.rs` (`WaitOutcome::Timeout`, the
  `total` counted after the wait under decision 0018);
- the focused waits' `timed_out` constructor and `call` in
  `src/mcp/focused.rs`, with `List` and its `LIST_CAP` of 250;
- `Filter::matches` and `Store::list` in `src/store.rs` (recipients are To,
  Cc and envelope RCPT, issue #55);
- the closed output variants of 0082/0094/0105 in `OUTPUT_SCHEMAS` and
  `focused::output_schema`, and the March captures under
  `tests/fixtures/mcp-structured-output/`, whose only timed-out line is an
  empty-inbox wait (`{"matched":0,"timed_out":true,"total":0}`);
- the harness: `S::SignupSibling` and `S::SignupSlow` in
  `evals/src/tasks.rs`, the sibling fixture that delivers both mails at
  admin prepare with the decoy last (`evals/src/fixture.rs`), the oracle's
  slow-delivery steps (`evals/src/oracle.rs`), the scripted helper modes in
  `evals/tests/common/mod.rs`, and the March-only client `evals/src/mcp.rs`.

Two intake statements are corrected by that evidence. The slow-delivery
scenario exists (`slow-delivery`, `S::SignupSlow`); nothing is added to the
catalogue. And a wait-start window would make the sibling trap's near-miss
list empty, because both fixture mails land before the agent's first call.

## Panel rulings

The positions agree on the frame: one read of the waiter's own inbox, taken
after the wait resolves, in the MCP adapter, bounded, infallible, with the
wait owners and the found path untouched. Where they differ, the chief
rules as follows. "Cost" names what the losing side gives up.

| Question | Simplicity | Robustness | Ruling | Cost |
| --- | --- | --- | --- | --- |
| Window | `since_ms`, the caller's own bound | wall-clock capture at park, `arrivals_since_start` | **Simplicity.** The only window in the caller's contract is `since_ms`. A wait-start window needs per-wait state next to 0002 and empties the sibling trap. | No "during the wait" claim. Every entry carries `received_ms`, and `last_arrival_ms` is whole-inbox, so the agent judges staleness itself. |
| Why an entry missed | the agent compares its filter with the shown `to`/`subject` | `Filter::misses` in the store, `matches` derived from it | **Reconciled.** Every entry carries `missed`, the supplied fields it failed — but computed in the adapter by single-field projections of the caller's `Filter` through the existing `Filter::matches`. One predicate, zero `src/store.rs` diff. | Simplicity loses one closed three-value vocabulary; robustness loses the store-level seam. |
| `nearest_subject` | an id and subject | a descriptor that says why it missed | **Robustness.** It is a full entry, identical in shape to an arrival, `missed` included. A bare subject is the sibling trap's best friend: the decoy matches the subject and lands last. | The object is larger by one entry; still bounded. |
| Who is nearest | every non-matching candidate | only candidates that passed every other field | **Robustness.** Only candidates whose `missed` is exactly `["subject"]` are ranked. "Nearest" then means "the mail that would have matched had the subject been right". | A mail that also failed `to` is never the nearest, even when its subject is closer. It is still in `arrivals` with its `missed`. |
| Ranking | distinct lowercase character bigram overlap | bounded edit distance | **Simplicity.** Bigram overlap over the clipped subject is linear, dependency-free and handles CJK; ties go to the newest, a zero score is `null`. | Crude on one-character filters (always `null`) and long subjects. Selection is the agent's; `arrivals` sits beside it. |
| Empty inbox | omit the key | always present, explicit `null`s | **Simplicity.** The object is present exactly when the inbox holds retained mail at the read. The March captures stay byte-identical and mail-free timeout tests pass unchanged. | "Empty" and "absent" collapse into one documented meaning: the inbox holds nothing. Modern clients read presence from the two closed timeout variants. |
| Revisions | all three, from the one captured Value | modern schema; March unstated | **Simplicity.** The harness client and missing-version clients speak March; a modern-only member cannot show the acceptance criterion. Decision 0111 names the departure from proposed 0082 §1. | March result text for a timed-out wait over a non-empty inbox gains a key. Descriptors and every captured baseline line are unchanged. |
| `last_arrival_ms` | max `received_ms` over the snapshot | the newest in acceptance order | **Robustness.** It is the `received_ms` of the first element of the newest-first list, the order 0001 and 0003 are built on; a test seeds out-of-order `received_ms` to pin it. | Under a clock step the two can disagree; acceptance order is the store's truth. |
| List cap / addresses | 10 / 5, de-duplicated | 8 / 4 with `to_total` | **10 / 5 with `to_total`.** The exact distinct-recipient count costs one integer and makes the cap visible. | None material. |
| Clipping | 256 bytes, no flag | ~200 bytes, boundary-safe | **256 bytes, char-boundary safe, no flag.** `email_id` leads to the exact mail through `swarmail_get_email`. | A 256-byte string may or may not be clipped; it is a hint, not the record. |
| Where the code lives | inside `src/mcp/focused.rs` | unspecified | **New `src/mcp/near_miss.rs`.** The vocabulary gets one owner; `List` in `focused.rs` gains a cap parameter and is shared. | One more file at exact coverage, which is no cost. |
| Focused waits | skip nothing | same predicate rule | **Both.** Nothing is skipped; `missed: []` on a focused timeout means "matched your filter, carried no code / no selected link". `swarmail_wait_for_email` skips mail `Filter::matches` admits, already counted in `matched`. | A threshold wait's filter-matching mail that failed the report is not a near miss; `total` counts it and the report tool explains it. |
| Harness | scripted controls, no catalogue change | oracle budget, decoy must still lose | **Both.** No goal text, scoring or timeout changes (0071 pins the catalogue). Scripted positive and negative controls; the negative presents the nearest subject and loses. | The live-model recovery rate is not measured. |

Refused on both sides: any computation inside `wait_for`, `wait_for_report`
or `wait_focused`; `list_all` or any cross-inbox read; accumulation in the
dispatcher or the firehose; a byte-offset `String::truncate`; an `isError`
from the near-miss path; a change to the found answer, to `total` or to the
wait deadline; a new store field, index, cache or task; a new tool, input
flag or dependency.

## Contract

### Candidates

After a wait resolves as timed out, the adapter takes **one** snapshot,
`Store::list(inbox, &Filter::default())`, of the inbox the wait used, on
`Store::offload`. From that one vector it derives everything below. A second
read is never taken; `total` keeps its own existing count and residual
(decision 0018).

A **candidate** is a snapshot entry with `received_ms >= since_ms` when the
caller supplied `since_ms`, and, for `swarmail_wait_for_email` only, one the
caller's `Filter` does not match. The focused waits exclude nothing.

### Entry

```json
{"email_id":"…","received_ms":1730000000123,"from":"app@example.test",
 "to":["alice.team@example.test"],"to_total":1,
 "subject":"Confirm your account","missed":["subject"]}
```

- `email_id`, `received_ms`: exact, as stored.
- `from`: the stored From address (envelope fallback included), or `null`.
- `to`: the distinct addresses the `to` filter already matches (To, Cc,
  envelope recipients, in that order), at most `NEAR_MISS_ADDRESSES` (5);
  `to_total` is the exact distinct count.
- `subject`: the stored subject or `null`.
- `missed`: the supplied filter fields this entry fails, in the fixed order
  `to`, `from`, `subject`; each decided by the caller's `Filter` projected to
  that one field and tested through `Filter::matches`. `since_ms` never
  appears: candidates already honour it. Unsupplied fields never appear.
- `from`, `subject` and each `to` string are clipped to
  `NEAR_MISS_FIELD_BYTES` (256) UTF-8 bytes on a char boundary.

### Object

```json
"near_misses": {
  "arrivals": [ … ],  "arrivals_total": 2, "arrivals_truncated": false,
  "nearest_subject": { … entry … } | null,
  "last_arrival_ms": 1730000004000
}
```

- `arrivals`: candidates newest first, at most `NEAR_MISS_LIST_CAP` (10);
  `arrivals_total` is the exact candidate count and `arrivals_truncated` is
  `total > shown`, through the shared `List`.
- `nearest_subject`: with a supplied `subject` filter, the candidate whose
  `missed` is exactly `["subject"]` and whose clipped lowercase subject
  shares the most distinct character bigrams with the clipped lowercase
  filter; ties go to the newest; a zero score, no such candidate or no
  subject filter gives `null`. Every candidate is scanned, not only the ten
  shown; per-candidate work is bounded by the clip.
- `last_arrival_ms`: the `received_ms` of the newest retained mail in the
  inbox in acceptance order, ignoring `since_ms` and the filter. Never
  `null`: the object exists only when the snapshot is non-empty.
- The object is present on a timed-out answer **iff** the snapshot holds at
  least one mail. An empty or removed inbox answers exactly today's bytes.
- The path is infallible: no `?`, no `isError`, no network, no persistence.
  Analysis-failure and shutdown errors keep their messages with nothing
  attached.

### Revisions

Each timed-out answer is one captured Value: text content on March, text
plus `structuredContent` on June and November. The output schema of
`swarmail_wait_for_email` gains a third closed `oneOf` variant (timeout with
`near_misses`); each focused wait gains a second closed `timed_out` variant.
Every member a variant emits is required (0094 §7); the entry and object are
closed, `missed` is an array of the enum `to`/`from`/`subject`,
`nearest_subject` is `anyOf` entry or `null`. Tool descriptors and March
descriptor bytes are not edited. README and `llms.txt` describe the member.

### Tenant boundary

The read is keyed by the wait's own `inbox`; nothing reaches `list_all`,
`inboxes()` or another inbox, and entries carry no inbox field. The claim is
exact: an answer about inbox X says nothing about inbox Y. It is not access
control. MCP is unauthenticated (decision 0004 names a development tool), so
a caller may still name any inbox directly; this feature must not claim
otherwise.

## Harness and metric

No catalogue, goal text, scoring or wait timeout changes: 0071 ties
catalogue edits to a version bump and refuses pooling across versions, and a
retuned trap would make fewer timeouts look like better recovery. The
scripted helper binary in `evals/tests/common/mod.rs` gains three modes,
each run through the real CLI, real MCP (March) and real SMTP in
`evals/tests/proof.rs`:

- **sibling recovery**: wait with a misspelled subject and `timeout_ms`
  1000, read `near_misses.arrivals`, choose the entry whose `to` contains
  exactly the identity, fetch it by `email_id`, confirm. The trial passes
  with no trap event.
- **sibling negative control**: the same wait, then act on
  `nearest_subject` — the decoy, which lands last and ties on subject. The
  application refuses the foreign token; the trial loses as
  `wrong_message`.
- **slow-delivery recovery**: the first wait (`timeout_ms` 1000) times out
  with `total: 0` and no `near_misses` key; release; wait again; confirm.
  The trial passes.

**The rate of recovery after a failed wait is not measured by this change.**
It needs paid live-model trials, which this run does not hold. The controls
prove recovery is mechanically possible from the timed-out answer alone and
that the tempting wrong choice still loses. Slow-delivery's goal already
prescribes the release, so recovery there is not attributable to this feature
(`evals/README.md`, honest limits); the report says so.

## Acceptance Criteria

Each criterion names a witness that can go red.

- **AC1 — Shape.** A timed-out `swarmail_wait_for_email`,
  `swarmail_wait_for_code` and `swarmail_wait_for_link` over an inbox holding
  mail carries `near_misses` with exactly `arrivals`, `arrivals_total`,
  `arrivals_truncated`, `nearest_subject` and `last_arrival_ms`; each entry
  has exactly `email_id`, `received_ms`, `from`, `to`, `to_total`, `subject`
  and `missed`. Real-wire tests in `tests/mcp.rs` and
  `tests/focused_waits.rs` assert the exact object after SMTP delivery.
- **AC2 — Candidates and `missed`.** For `swarmail_wait_for_email`, mail the
  filter admits is absent from `arrivals`; for the focused waits it is
  present with `missed: []`. With `to` and `subject` supplied, an entry
  failing only `to` reads `missed: ["to"]`; `since_ms` excludes older mail
  from `arrivals` and `arrivals_total` while `last_arrival_ms` still reports
  it. Inline tests in `src/mcp/near_miss.rs` and a wire test prove each.
- **AC3 — Bounds.** Eleven candidates show ten with `arrivals_total: 11` and
  `arrivals_truncated: true`; six distinct recipients show five with
  `to_total: 6`; a subject, sender and recipient longer than 256 bytes,
  including one whose 256th byte falls inside a multibyte char, are clipped
  on a char boundary and never panic. Inline tests exceed each cap.
- **AC4 — Nearest subject.** With a subject filter, the candidate with
  `missed == ["subject"]` and the highest bigram overlap is `nearest_subject`,
  the newest wins a tie, a zero score and a one-character filter give
  `null`, no subject filter gives `null`, and a candidate that also failed
  `to` is never chosen even with a closer subject. A CJK subject ranks.
  Inline tests cover each branch.
- **AC5 — Last arrival.** With `received_ms` seeded out of acceptance order,
  `last_arrival_ms` equals the newest-accepted mail's `received_ms`, not the
  maximum. Inline test.
- **AC6 — Absent when empty.** A timed-out wait over an empty or removed
  inbox is byte-identical to today on March; the captured
  `march-current-main.jsonl` and `march.jsonl` comparisons in
  `tests/mcp_revisions/structured.rs` pass unchanged, and no provenance
  record is re-baselined.
- **AC7 — Revisions and schema.** The same object appears in March text and
  in June/November `structuredContent` from one Value; the new closed
  variants validate every populated and empty timed-out answer under the
  finite evaluator, and negative controls (an extra member, a missing
  member, an unknown `missed` value, a non-null non-object `nearest_subject`)
  are refused. `tests/mcp_revisions/structured.rs` and the updated
  exact-shape cases in `tests/mcp_revisions/focused.rs`.
- **AC8 — Untouched paths.** The found answer, `matched`, `total`, the
  analysis-failure and shutdown errors, `src/store.rs` and both wait owners
  are byte- and line-identical; a test asserts the inbox's contents and
  count are unchanged after a timed-out wait. Existing tests plus a wire
  test.
- **AC9 — Tenant boundary.** With a sibling inbox that receives mail through
  SMTP AUTH while the waiter's wait is parked, the waiter's timed-out answer
  contains none of the sibling mail's id, subject or address, while the
  waiter's own non-matching mail appears. Real-wire test in `tests/mcp.rs`.
- **AC10 — Harness recovery.** The three scripted controls above pass in
  `evals/tests/proof.rs` through the real CLI; `evals/README.md` counts them
  and states the metric as not measured. No file under `evals/tests/data/`
  changes.
- **AC11 — Guarantees and gates.** 0002's subscribe-before-scan is proven
  unchanged by the empty diff on the wait owners; every house gate in
  `docs/house-rules.md` § Gates is clean and locked, including the evals
  crate, exact coverage with its selftest, the publish-guard and seal
  proofs and `cargo deny`.

## Delivery and limitations

- The snapshot is later than the terminal scan: a mail accepted between the
  deadline and the read is visible. The residual is one insert, the same
  `total` already carries (0018), and the next call succeeds.
- A burst over the per-inbox cap may evict candidates before the read; the
  block is then smaller or absent. Making it exact would need retention,
  which the issue forbids. Pruning stays oldest-only (0003).
- Mail fields are untrusted text in an agent's prompt path, the same exposure
  as `swarmail_search_emails`, and smaller when clipped.
- Decision number: 0094 reserved 0093 for this issue, but main now holds
  `0093-source-mail-comparison.md`; drafted as 0106 and renumbered to 0111 at
  integration (open PR #292 holds 0106–0107, #159 holds 0110).
- Commit unsigned; never push, merge, publish or accept a decision.
