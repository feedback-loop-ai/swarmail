# 0037 — POP3 sizes count the normalized payload without transparency dots

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-18
- **Issues:** #84 and #86, with #71/#83 constraining reply construction.
- **Spec:** [Storage and maildrop repairs](../../specs/storage-maildrop/spec.md).
- **Amends:** [0028](0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md)
  §1's size definition, derivation mechanism and accepted allocation cost.
  Its accepted file stays unchanged; §§2 and 3 continue to govern.

## Context

0028 counts transmission transparency dots as message octets. RFC 1939 §11
excludes those added dots: a stored 30-octet CRLF payload with three
dot-leading lines is advertised as 33 although the client retains 30.
PASS also builds a full stuffed buffer for every size on the async worker.
The old cost justification is incorrect: `store.list` clones Arc handles,
not each message's payload, and the scratch buffer is freshly allocated.

## Ruling

1. Every POP3 size is the length of the stored payload after the existing
   line-ending normalization, excluding added transparency dots, status
   headers and the final `.\r\n`. Apply this to RETR, STAT, LIST in both
   forms, PASS and RSET. UIDL identifiers do not change.
2. Keep the existing payload normalization exactly: split on LF, remove
   its terminal LF and at most one terminal CR, append CRLF to each emitted
   line, and do not invent an extra line after a final LF. Empty raw input
   emits zero payload bytes. A CR inside a line remains data; a terminal CR
   is handled as the existing writer handles it. Preserve 8-bit bytes.
   Stuff a dot at the start of dot-leading lines for transmission only.
3. One serializer owns line traversal and emission. A buffer sink receives
   payload, normalized CRLF and added dots; an allocation-free counter sink
   counts the same emission excluding added dots. No second newline parser,
   ingest normalization, stored size cache, Email field or SQLite column.
   This replaces the scratch-buffer mandate while retaining derivation from
   the writer. `Email.size` remains raw stored length; `/raw` is unchanged.
4. PASS derives metadata in one awaited `Store::offload`, including the Arc
   listing, ordering and size pass. It remains O(maildrop bytes) per login.
   The session keeps IDs, counts and existing gone/deletion state, not raw
   message bodies. Extraction of links/codes still happens at ingest (0005).
5. RETR fetches the live Arc and appends the stuffed body directly into its
   one final reply buffer. The immutable message's snapshot count supplies
   the header. The entire reply uses one bounded write; deadlines never
   surround SMTP insertion or POP3 QUIT deletion.

## Alternatives and costs

The panel's shared serializer is adopted over separate counting/writing
walks. Its small sink abstraction has a coverage obligation, but prevents
normalization rules from drifting. An ingest-time derived field is declined:
it could eliminate repeated scans, at the price of another model invariant,
restore computation and writer/cache coupling. PASS instead retains CPU
cost and blocking-pool contention, explicitly accepted here.

Streaming RETR is declined for this batch. One output buffer satisfies the
requested one-copy limit and composes with complete-buffer reply semantics.
Concurrent large retrievals can still consume substantial RAM (roughly
12.5 GiB of extra payload at 256 × 50 MiB, before overhead and wire
expansion); this is no total-memory guarantee. The no-maildrop-lock posture
and external-removal DELE error of 0028 §3 are reaffirmed, not amended.

## Proof

Spec AC4–AC8 require a 30/33-byte dot fixture, independent unstuffed wire
oracles across newline/8-bit edges, agreement of every size reply, stable
UIDLs, live DELE parity, one-buffer RETR, allocation-free counting and
HTTP progress while PASS is offloaded. AC1–AC3 retain deadlines and STARTTLS
ordering. README/test helper changes accompany the implementation; this
proposed record does not assert implementation or successful gates.
