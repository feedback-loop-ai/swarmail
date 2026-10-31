# Ordered implementation tasks — attachments and ICS at ingest

All boxes start open. A test named here is a required witness, not a claim
that it exists or passed. See [spec.md](spec.md) AC1–AC10 and
[plan.md](plan.md).

- [ ] T1 — Confirm the SHA-256 path compiles at the locked tree: add
  `src/attachments.rs` with `sha256_hex` through rustls's ring hash provider
  and the four constants. **Test:** inline known-answer tests for `abc` and
  the empty string; `git diff --exit-code Cargo.lock` (AC1, AC3, AC10). If it
  does not compile, stop and report the blocker named in the plan's risks.
- [ ] T2 — Add `Attachment`, `IcsEvent` and `Email.attachments` to
  `src/model.rs`, both skipped by serde, with the strict map-only readers for
  the persisted object. **Test:** inline serialization proves old Email bytes
  for `None`, `Some([])` and a populated list; required-nullable, unknown
  member, wrong type and positional-array negatives refuse (AC6, AC8).
- [ ] T3 — Implement the ICS scanner in `src/attachments.rs`: UTF-8 and 1 MiB
  guards, unfolding, depth-tracked first-`VEVENT` reader, quoted-parameter
  colon rule, `TZID`, first-occurrence-wins, `SUMMARY` unescape, label cap.
  **Test:** unit tests for folded lines, `VTIMEZONE` `DTSTART`, `VALARM`
  `SUMMARY`, `TZID` and `Z`, escaped summary, two events, no event, over-cap
  input, invalid UTF-8, a name with a quoted parameter containing `:`, and a
  1025-byte field cut at a character boundary (AC5).
- [ ] T4 — Implement the candidate walk, hashing, exact `total`, the 250 cap
  and the first-fit budget; wire it into `build_email` with `None` on parse
  failure. **Test:** inline tests on generated MIME: disposition attachment,
  bare `text/calendar` body, `application/ics`, `.ics`-named octet-stream,
  attached `message/rfc822` as one entry, a part with no content type, a
  lowercased mixed-case type, 250 and 251 parts, 1 048 576 and 1 048 577
  octets, 524 288 + 524 288 + 1, a 1 MiB part before a small ICS, and
  `Some([])` for a plain mail (AC1, AC3, AC5).
- [ ] T5 — Persist: `attachments_json` and `attachment_bytes` in `COLUMNS`,
  `SCHEMA` and `ADDED_COLUMNS`; bind in `record_insert`; strict restore with
  the length cross-check. **Test:** `src/persist.rs` tests open a genuine
  old-schema file and migrate; restore `None`, `Some([])` and a stored list
  byte for byte; a damaged object, a wrong-length blob and a NULL in only one
  column refuse naming the column (AC6).
- [ ] T6 — Add `src/mcp/attachments.rs`: descriptors, closed output schemas,
  argument staging (`id` string, `index` non-negative integer, unknown keys
  refused) and the ordered refusals; route the names in `src/mcp.rs`, mark
  them read-only, append after the focused waits. **Test:** inline handler
  tests for every refusal arm and both encodings, including a text part with
  a NUL byte answering `base64`; a source-level test that the module names
  neither `mail_parser` nor `raw` (AC2, AC6, AC8).
- [ ] T7 — Wire proofs in `tests/mcp.rs`: real SMTP delivery of generated MIME,
  then list and fetch over HTTP MCP for AC1–AC4, the 50 MiB hostile fixtures
  included, with `250` and immediate queryability asserted and the retained
  bytes read from the data file (AC1–AC4, AC7).
- [ ] T8 — Revision and transport proofs: `tests/mcp_revisions.rs` catalogue
  of seventeen with eleven reads, March text-only descriptors and frozen
  bytes, June/November schemas and `structuredContent` parity in a new
  `tests/mcp_revisions/attachments.rs`; `tests/stdio.rs` delivers a 1 MiB
  fetch in each encoding under the 10 MiB cap (AC8).
- [ ] T9 — Restart proofs in `tests/persistence.rs`: list and fetch byte for
  byte after a restart, the unknown refusal for pre-column rows, cap eviction
  taking the columns with the row (AC6, AC7).
- [ ] T10 — Evals: add the two names to `SWARMAIL_TOOLS`; add the invite rig
  test that lists, matches `ics.uid`/`ics.dtstart` to `ics_property`, fetches
  `invite.ics`, checks bytes and hash with the crate's `ring`, and accepts.
  **Test:** `cargo test --manifest-path evals/Cargo.toml --locked`; catalogue,
  oracle and prompts unchanged by `git diff` (AC9).
- [ ] T11 — Document: README and `llms.txt` sections for the two tools, the
  four constants and the metadata-only answer; the 0110 row in
  `docs/decisions/README.md`. **Test:** the tracked-link guard and the
  existing doc-pin suites stay green (AC10).
- [ ] T12 — Run every gate locked (plan quickstart), the ingest bench before
  and after, and write the report: measured numbers, `Cargo.lock` and
  `deny.toml` unchanged, and the sentences "not measured: no paid trial" and
  "`invite-time` does not exercise the attachment tools (0055 §4)" (AC9,
  AC10).
