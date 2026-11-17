# Operator ruling — 2026-11-16 (#355)

Intake for #355 found that the accepted run-console design shows data that
accepted decisions keep out of the JSON API: attachment labels and ICS
summaries (0110: MCP-only), link anchor text and source (0094 §2), and the
SMTP client IP and HELO (0126 §2).

The operator chose **"Amend for a UI projection"**:

- This delivery drafts one **proposed decision** that adds a single
  read-only projection route for the UI. It carries attachment labels
  (filename, content type, size, and the parsed ICS summary and times), link
  anchor text and source, and the SMTP client IP and HELO. The route is
  documented in `openapi.json` and covered by `tests/api.rs`.
- The decision amends 0110, 0094 §2 and 0126 §2 **by dated addendum only**.
  Accepted bodies are never edited.
- Attachment bytes stay MCP-only. The projection never serves content.
- The operator accepts the decision before the PR merges. The delivery
  builds the whole design, the projection included, against the proposed
  record.

The intake's other findings stand as framing:
- Accepted and dropped counts, and last-mail time, need read-only fields.
- Auth results come from the query-time deliverability report and are
  labelled as such.
- ARC shows "not evaluated" until #256 lands.
- There is no "Send sample mail" button and no chaos toggle.
