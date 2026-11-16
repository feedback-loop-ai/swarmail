# 0132 — A read-only UI projection of ingest-stored fields, and two read-only fields for the run console

- **Status:** proposed
- **Date:** 2026-11-16
- **Scope:** Issue #355 (rework the web UI as a run console). Run
  `issue-355-rework-the-web-ui-as-a-c69a5333`, branch `feat/web-ui-console-355`
  at `4db525c`, on `main` `f05f2ee`.
- **Specification:** [web UI run console](../../specs/web-ui-run-console/spec.md);
  [plan](../../specs/web-ui-run-console/plan.md),
  [tasks](../../specs/web-ui-run-console/tasks.md).
- **Authority:** the operator's ruling of 2026-11-16, "Amend for a UI
  projection" ([operator-ruling.md](../../specs/web-ui-run-console/operator-ruling.md));
  accepted [0005](0005-extraction-at-ingest.md) (extraction at ingest);
  accepted [0108](0108-compact-email-view.md) (the presentation cap and its
  totals).
- **Amends upon acceptance, by dated addendum only:**
  [0110](0110-attachments-and-ics.md) §7,
  [0094](0094-focused-mail-waits.md) §2,
  [0126](0126-spf-dmarc-alignment.md) §2 and
  [0027](0027-living-listeners-bounded-sessions.md) §2. This document carries
  the amendment text. Each of those records receives one appended dated
  pointer; no accepted body is edited.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md) and
  [0003](0003-oldest-only-pruning.md). Nothing here changes `build_email`,
  `Store::insert`, the SMTP `250`, the `await` path, the feed or pruning.
- **Holds:** the pull request that carries this record is not merged on
  green. The operator ruled that the delivery builds against the proposed
  record and that acceptance comes before merge.
- **Integration note:** 0132 is the next free number on this branch. Two
  other open branches each carry a different proposed 0132 (ARC chain
  reports, #256; mail comparison protocol v4, #359). The operator assigns the
  final numbers at integration; this record's file name, its index row and the
  links in its specification move together.

## Context

The run console the operator accepted on 2026-11-16 shows, for the mail a
person has open: its attachments and calendar invites, the anchor text and the
source of each link, and the SMTP client's address and HELO name. Ingest
already stores all of it (0110, 0094, 0126). Three accepted rulings keep it
out of every REST response:

- 0110 §7: "No REST route, no compat field, no webhook change:
  `Email.attachments` and the bytes are `#[serde(skip)]`", with REST list and
  fetch routes deferred to "a separate decision with its OpenAPI, refusal and
  `tests/api.rs` obligations".
- 0094 §2: "Skip the new optional vector in existing Email serialization on
  all surfaces."
- 0126 §2: the observed client address and HELO name are internal and no
  `Email` serialization carries them.

Intake reported the conflict and blocked. The operator ruled: amend, with one
read-only projection route for the UI, by dated addendum, bytes still
MCP-only. The same ruling let two intake findings stand: the status bar's
accepted and dropped counts, and the inbox list's last-mail time, "need
read-only fields".

The compact view of accepted 0108 already serves anchor text and attachment
labels on the REST list (`view=compact`). It does not serve the link source,
the calendar fields or the SMTP observation, and it is a list view shaped for
agents. This record does not change it.

## Decision

1. **One route.** `GET /api/v1/messages/{id}/projection`. It has no other
   method. It sits behind the same Origin guard as the whole HTTP plane
   (0030 §2, 0043) and adds no CORS header.

2. **The body.** One JSON object with exactly five members, always present:

   - `id`: the message id.
   - `smtp_client_ip`: the stored client address as a string, or `null`.
   - `smtp_helo`: the stored HELO or EHLO argument, or `null`.
   - `links`: `null` when the stored occurrences are unknown (a row from
     before 0094); otherwise `{ "total", "items" }`. `total` is the exact
     stored count. `items` holds the first 250 occurrences in stored order,
     each exactly `{ "url", "anchor_text", "source" }` with `anchor_text` a
     string or `null` and `source` `"text"` or `"html"`.
   - `attachments`: `null` when the stored list is unknown; otherwise
     `{ "total", "items" }`. `total` is the stored list's exact total. `items`
     holds the first 250 stored parts in part order, each exactly
     `{ "filename", "content_type", "size", "ics" }`. `ics` is `null` or
     exactly `{ "summary", "dtstart", "dtstart_tzid", "dtend", "dtend_tzid" }`,
     each a string or `null`, as ingest stored them (0110 §4: no timezone
     resolution). A labels-only restored list (0110 §6) is served the same
     way.

   The cap of 250 is 0108's presentation cap. No member is omitted when
   empty or null: a client sees one shape.

3. **What it never serves.** No attachment bytes and no member named
   `content`, `bytes`, `body`, `raw`, `text`, `html`, `sha256`, `stored`, `uid`
   or `rel`. Attachment bytes and hashes stay with the MCP attachment tools
   (0110). The link `rel` stays with the focused link wait (0094). The
   operator's ruling lists anchor text and source, labels, sizes, calendar
   summary and times, and the client address and HELO name; the projection
   carries those and nothing more.

4. **Refusals.** An unknown or pruned id is `404 {"error":"message not
   found"}`. Any query key is `400` through the shared closed-query parser,
   naming the route and the key. Another method is `405`. A guard-refused
   request is `403`.

5. **Stored fields only.** The handler reads one stored `Email` and serializes
   fields `build_email` wrote. It does not read `raw`, run extraction or
   evaluate deliverability (0005). It adds no work to the accept path and no
   field to the row.

6. **`/healthz` counters.** The `200 ok` and `503 degraded` bodies gain
   `emails_inserted_total` and `emails_dropped_total`: the store's lifetime
   counters, the same values `/metrics` exposes as
   `swarmail_emails_inserted_total` and `swarmail_emails_dropped_total`.
   "Dropped" counts mail evicted by the per-inbox cap (0003), not chaos
   refusals. Status codes, `status` values and the `degraded` names are
   unchanged. The `503 stopping` body stays exactly `{"status":"stopping"}`.

7. **`last_received_ms` on the inbox list.** Every item of
   `GET /api/v1/inboxes` gains `last_received_ms`: the `received_ms` of the
   mail most recently stored in that inbox (the newest retained position), or
   `null` if the inbox holds no mail at the read. It is always serialized. It
   is read from the store on each request and is never persisted. It is the
   server's ingest stamp, never the mail's `Date` header. An emptied inbox is
   not listed (0032), so a quiescent list shows no `null`.

8. **Nothing else moves.** `GET /api/v1/messages/{id}`, the message lists,
   the compact view, the thread view, feed frames, webhook payloads, MCP
   results and the compat shims keep their bytes. `Email`'s `#[serde(skip)]`
   attributes stay. The console's one write, "Clear inbox", uses the existing
   `DELETE /api/v1/inboxes/{inbox}/messages` and needs no ruling here.

## Dated addenda carried by this record — 2026-11-16

These take effect when the operator accepts this record. Until then each
amended ruling stands as written.

- **To 0110 §7.** "No REST route" is amended to: one REST route, §1 above,
  serves the attachment list's labels, sizes and calendar summary and times.
  It serves no bytes and no hash. REST list and fetch routes for attachment
  content remain deferred. `Email.attachments` stays `#[serde(skip)]`.
- **To 0094 §2.** "Skip the new optional vector in existing Email
  serialization on all surfaces" stands for every `Email` serialization. The
  occurrences' `url`, `anchor_text` and `source` gain one read, §1 above,
  which is not an `Email` serialization. `rel` is not served by it.
- **To 0126 §2.** The observed client address and HELO name stay out of every
  `Email` serialization. They gain one read, §1 above.
- **To 0027 §2.** The literal `ok` and `degraded` bodies gain the two members
  of §6. The escalation contract (which status code, which `status`, which
  listeners are named) is unchanged, and so is the stopping answer.

## Rejected alternatives

- **Drop the fields from the console** (intake's option b). The operator
  did not take it.
- **Un-skip the fields on `Email`.** Every read, feed frame and webhook
  payload would grow, which 0094, 0110 and 0126 each ruled out to keep
  existing bytes fixed.
- **Reuse the compact view.** It lacks the link source, the calendar fields
  and the SMTP observation, and widening an agent-facing view for a browser
  pane changes a contract agents are measured against.
- **Serialize the stored types as they are.** `LinkMetadata`, `Attachment`
  and `IcsEvent` would publish `rel`, `sha256`, `stored` and `uid`. Each extra
  member widens an amendment the operator ruled narrowly.
- **Counts from `/metrics` parsed in the browser.** No contract changes, but
  the operator's ruling asks for read-only fields, and a text-format parser
  in the page is a second, unowned reader of the exposition format.
- **Counts on the projection route.** A per-message route cannot answer
  before the first mail exists, and a store-wide counter does not belong to a
  message.
- **A new status route.** A second new route for two integers that the
  health surface can carry beside the listener truth it already tells.
- **The maximum stamp for `last_received_ms`.** Exact under out-of-order
  inserts, but it scans every retained mail of every listed inbox on each
  poll, under the lock of the map the accept path writes. The back of the
  deque is O(1).

## Costs

- Three accepted rulings each gain an exception: one REST read of fields they
  kept internal. The exception is one route, read-only, bounded, with no
  bytes.
- `/healthz` answers grow by two members. A consumer that compares the whole
  `ok` or `degraded` body for equality would break; none is known in the
  repository, the chart's probes read the status code only, and the one
  byte-pinned body (stopping) is left alone.
- `last_received_ms` can step backwards by a few milliseconds when two mails
  are stored out of stamp order. The console's new-mail dot can miss that
  mail; the count and the stream cannot.
- One more route and schema to keep in `openapi.json`, `llms.txt` and
  `tests/api.rs`.

## Proof obligations

Stated as acceptance criteria AC5, AC6, AC8 and AC18–AC24 of the
specification: the counters over the wire and in the degraded body; the stamp
across inserts and a clear; the projection's values over real SMTP, its exact
member sets, its refusals, its cap and its independence from `raw`; the
unchanged bytes of every other surface; and the OpenAPI document.
