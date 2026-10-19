# 0038 — State the store's memory exposure and parse-start timestamps

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance is the operator's.
- **Date:** 2026-10-18
- **Issue:** #87.
- **Spec:** [Storage and maildrop repairs](../../specs/storage-maildrop/spec.md).
- **Amends:** [0021](0021-watcher-channel-lifetime.md), Consequences' bounded
  memory/residue claim; [0026](0026-deployment-boundary-peers-tls-sizing.md)
  §3's claim that the per-inbox knob bounds restored memory.
- **Preserves:** 0001–0003, ingest extraction (0005), ordered mutations,
  watcher lifecycle/atomic registration (0021) and empty-inbox removal (0032).
  Accepted documents stay byte-identical; this is the proposed amendment.

## Context

The review measured about 780 bytes of retained channel overhead per
distinct awaited empty inbox name, with no further growth on repeated use
of the same names. A bound per name does not bound an unlimited population.
Likewise, a per-inbox message cap does not bound unlimited inboxes or all
resident representations. `received_at` samples the start of parsing, so
two sessions can commit in the opposite order to their timestamps. Issue
#87 expressly permits accurate documentation of these three existing
behaviors; this design selects that alternative for all three.

## Ruling

1. **Watcher residue is unbounded in aggregate.** A distinct subscribed name
   can leave a channel after its last receiver leaves. The review's roughly
   780 B/name is an observation on that build, not a portable constant or
   upper bound: names, map/allocator overhead and retained payload Arcs add
   costs. Reusing names reuses existing channels. Release remains through
   explicit clear/drain, last-mail deletion, clear-all and process teardown;
   timeout/cancellation alone promises no reclamation. Concurrent active
   waiters may re-register after release. Map capacity and allocators also
   prevent an RSS-return guarantee. The API's separate 4,096-name cumulative
   await-metrics registry cap does not cap `Store::watchers`.
2. **There is no global memory ceiling.** `SWARMAIL_MAX_PER_INBOX` limits
   message count in one inbox on insertion; zero disables it. Inbox
   cardinality is unlimited. RAM holds raw messages, decoded/extracted
   fields, indexes, channels and their retained Arcs, active protocol
   buffers and allocator overhead. An inbox-count × cap × message-footprint
   estimate helps capacity planning, but is not an enforced byte or RSS
   bound. README, store module docs and chart resource comments must agree.
3. **Restore loads the whole persisted state before binding.** It does not
   re-prune that state under a newly configured per-inbox cap. Size pod
   memory for restored objects and live workload, not just raw file bytes;
   data-file size is not equal to or an upper bound on resident memory.
   OOM during restore can produce a restart loop. The startup probe remains
   a time budget, not a memory remedy. No new cap, global eviction, SMTP
   admission refusal or restore truncation is introduced.
4. **`received_at` and `received_ms` describe parse-start wall time.** The
   sample occurs on entry to `build_email`, before parsing and acceptance.
   It is neither the sender's Date nor a commit sequence. Cross-session
   timestamp ordering can differ from acceptance order; clocks can step
   backwards and stamps can tie. Per-inbox lists, POP3 snapshot numbering,
   oldest-only eviction and SQLite `seq` restore ordering continue to use
   insertion/acceptance order. Compatibility `list_all` sorts timestamps
   with its existing tie-break. No historical stamps or UUIDs are rewritten,
   and no strict timestamp monotonicity is promised even within a session
   across wall-clock changes. Sequential-session acceptance ordering stays.
5. **Drop the unused index on open, including old files.** Replace creation
   of `emails_inbox` with `DROP INDEX IF EXISTS emails_inbox`; retain
   `emails_seq`. Test the same full-column `ORDER BY seq, rowid` query that
   restore actually executes, including every EXPLAIN plan row. No new
   schema version, migration service or index is needed.

## Rejected alternatives and accepted exposure

The robustness position's receiver guard would reclaim inactive channels,
but adds a cancellation/drop/shard-lock lifecycle beside frozen 0002. The
existing atomic subscribe and Closed-arm rescan are preserved instead.
Cost: repeated distinct awaits still grow memory until existing release
paths act; this is documented exposure, not a fixed leak.

A global admission cap is safer than cross-inbox eviction for isolation,
but would add error vocabulary, byte accounting, configuration and restore
policy at the synchronous mutation boundary. Neither is bought here. Cost:
an OOM remains possible and can interrupt availability, including restore.

Stamping under the mutation lock would improve timestamp/acceptance
agreement, but needs clock, tie and restart semantics; max(now,last_stamp)
alone does not order equal-millisecond messages by acceptance when IDs were
minted during parsing. Keep the existing meaning instead. Cost: API order
comparisons across surfaces can disagree; callers use per-inbox order for
acceptance-order assertions.

## Proof and publication

Spec AC9–AC12 require precise documentation, registry residue/reuse/release
pins, reversed-timestamp insertion/restore fixtures, legacy/fresh schema
tests and the full restore plan. A real SMTP/POP3 restart plus absent-index
assertion provides a production red/green regression. The three documented
alternatives use documentation-contract red/green tests: unchanged runtime
tests are preservation evidence, never claimed fixes. The index lists this
record as proposed and current docs link to it, without editing accepted
0021/0026. This record does not claim operator acceptance or implementation.

## Pointer — 2026-10-18 (proposed 0049)

[0049](0049-extraction-compat.md) §5, also **proposed**, updates §4 here (issue #138):
the compat views — scoped and global lists, searches and `latest` — share descending
`(received_ms, id)`, so a parse-start stamp that disagrees with acceptance order moves a
mail within those views; native inbox listings, POP3 numbering, pruning and restore keep
acceptance order.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit. The operator accepted
this record and the later record below on 2026-10-19; the text above, the
0049 pointer included, stays as written, and that pointer's "proposed" is
historical. Each marker uses the later record's own verb and scope. The
amendments this record makes now carry dated markers in
[0021](0021-watcher-channel-lifetime.md) and
[0026](0026-deployment-boundary-peers-tls-sizing.md); their earlier text is
unchanged.

- *(Updated by [0049](0049-extraction-compat.md) (accepted 2026-10-19): §4's
  compat scoped-order exception, as the pointer above describes — every
  compat view shares descending `(received_ms, id)`.)*
