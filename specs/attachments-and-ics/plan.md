# Implementation plan — attachments and ICS at ingest

Implements [spec.md](spec.md) under proposed
[0110](../../docs/decisions/0110-attachments-and-ics.md).

## Technical context

One production Rust crate: Tokio, Axum, serde, mail-parser 0.11.9, rusqlite,
rustls 0.23 with the `ring` feature. The evals crate is a separate workspace
outside production coverage and already depends on `ring` directly. Add no
dependency, feature flag, configuration knob or allowlist entry; `Cargo.lock`
stays byte-identical. The `.specify/templates/plan-template.md` is absent; this
file supplies the technical context, constitution check, structure, research,
data model, contracts, quickstart and risks the dialect asks for.

## Constitution check

- **0001** — the walk, hash, budget and ICS scan are pure functions of the
  parsed message, called from `build_email` on the blocking pool before
  `insert_async`; none returns an error, so no new failure path sits between
  DATA and `250`. The columns ride the one insert transaction.
- **0002** — no change to subscribe-before-check; no new watcher.
- **0003** — pruned ids delete their whole row; the two new columns are in
  the row. No child table, so no second delete site.
- **0005** — list, fetch and ICS fields read stored fields only. The fetch
  handler holds no parser and no `raw` access.
- **0010** — seed shares `build_email`.
- **0020** — every new branch is warm on the real wire or in an inline unit
  test; no harness file is planned. If a cold arm appears during
  implementation it moves to a declared sibling harness, never behind
  `coverage(off)`.
- **0022/0038** — the transient parse is unchanged; the retained bound is
  stated in 0110 §8.
- **0029/0044** — argument refusals are `-32602` by name, value-free, before
  any store read.
- **0031 §1** — the record stays sender-honest: no synthesis, labels as sent.
- **0082 §1 / 0094 §7** — March bytes frozen, descriptors appended.
- **0045 §8** — this is compatible work under a proposed record.

## Research

- *mail-parser 0.11.9 (read in source by the panel; the implementer confirms
  by compiling).* `Message::attachments()` yields parts whose disposition is
  attachment; a bare `text/calendar` part with no disposition lands in the
  text-body candidates instead, so the calendar arm of the candidate
  predicate is required. `MessagePart::contents()` is the decoded body;
  `attachment_name()` is the disposition filename, else the content-type
  name; `content_type()` gives `ctype()`/`subtype()`. The parser decodes every
  part at parse time; nothing here adds a second decode.
- *SHA-256 without a new crate.* rustls exposes
  `rustls::crypto::ring::cipher_suite::TLS13_AES_128_GCM_SHA256`; its
  `.tls13()` suite's `common.hash_provider` is a `&'static dyn
  rustls::crypto::hash::Hash` whose `hash(&[u8])` output is the digest. One
  helper, `attachments::sha256_hex`, wraps it with a comment naming 0110 §5,
  and two known-answer tests pin it: `abc` →
  `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`, `""` →
  `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. If the
  provider turns out not to be reachable through a public path at the locked
  version, implementation stops and reports: it does not hand-roll a digest
  and does not add `sha2` without the operator (0110 §5).
- *Bridge arithmetic.* `src/stdio.rs` `RESPONSE_CAP` is 10 MiB; modern
  results carry the JSON in `content[0].text` (a string, so the inner JSON's
  `\"` escapes become `\\\"`) and again in `structuredContent`. A 1 MiB
  worst-case text payload is at most 2 MiB as JSON, 4 MiB inside the text
  block, plus 2 MiB structured: about 6 MiB. Base64 payloads escape nothing:
  about 2.7 MiB. Both fit; 2 MiB would not in the worst case. The list answer
  has its own arithmetic: unbounded it reached 25.5 MiB (250 parts, eight
  1024-byte control-byte labels), so its entries stop at 3 MiB of compact
  JSON, at most 9 MiB on the wire once doubled in the text block and repeated
  as `structuredContent`; the measured worst is 8.93 MiB (backslash labels).
- *#155 reconciliation.* #155 landed first with `model::Attachment`,
  `Email.attachments`, `attachments_json` (three-tuples) and `attachment_list`.
  This delivery widened the first three and removed the fourth; the strict
  reader accepts both stored shapes as 0110's reconciliation section states.

## Project structure and files touched

| File | Change |
|---|---|
| `src/attachments.rs` (new) | the five constants; `candidates()` predicate and walk; `sha256_hex`; `budget` first-fit; `ics::scan` (unfold, depth-tracked first-`VEVENT` reader, unescape); the `encoding` classifier; inline unit tests for every branch |
| `src/model.rs` | `Attachment { filename, content_type, size, sha256, stored, ics, content: Option<Vec<u8>> (#[serde(skip)]) }`, `IcsEvent { dtstart, dtstart_tzid, dtend, dtend_tzid, summary, uid }`, `Email.attachments: Option<Vec<Attachment>>` (#[serde(skip)]), strict map-only readers in 0097 §2's style for the persisted object |
| `src/smtp.rs` | `build_email` calls `attachments::extract(&message)` beside `extract_link_metadata`; `None` when the parse failed |
| `src/persist.rs` | `COLUMNS`, `SCHEMA`, `ADDED_COLUMNS` gain `attachments_json TEXT` and `attachment_bytes BLOB`; `record_insert` binds both; `row_to_email` restores strictly with the length cross-check; tests for every refusal through direct `UPDATE`s as the `link_metadata_json` tests do |
| `src/mcp/attachments.rs` (new) | `LIST` and `GET` names, descriptors, `output_schema`, `call` with the argument staging and the ordered refusals; `#[cfg(test)]` assertion that the module references neither `mail_parser` nor `raw` |
| `src/mcp.rs` | route the two names in `call_tool`, add them to the read-only match, the `OUTPUT_SCHEMAS` map and the descriptor append after `focused::descriptors()` |
| `src/lib.rs` | `pub mod attachments;` |
| `tests/mcp.rs` | AC1–AC4 on the real wire: generated MIME over SMTP, list and fetch through HTTP MCP |
| `tests/mcp_revisions.rs`, `tests/mcp_revisions/attachments.rs` (new) | the seventeen-tool catalogue with eleven reads, March text-only descriptors, June/November schemas, structured-output parity |
| `tests/persistence.rs` | AC6 round trip across a restart, NULL rows, the three refusals |
| `tests/stdio.rs` | a 1 MiB fetch in each encoding under the bridge cap |
| `evals/src/wrappers.rs` | two names in `SWARMAIL_TOOLS` |
| `evals/tests/fixture.rs` | the invite rig test of AC9 |
| `README.md`, `src/static/llms.txt` | the two tools, the constants, the metadata-only answer |
| `docs/decisions/README.md` | the 0110 index row (implementation commit, as 0094's was) |

## Data model

```text
Attachment { filename: Option<String>, content_type: Option<String>, size: u64,
             sha256: String, stored: bool, ics: Option<IcsEvent>,
             content: Option<Vec<u8>> }      // Some iff stored; skipped by serde
IcsEvent   { dtstart, dtstart_tzid, dtend, dtend_tzid, summary, uid: Option<String> }
AttachmentList { total: u64, parts: Vec<Attachment>, labels_only: bool }  // labels_only: restore-only, skipped by serde
Email.attachments: Option<AttachmentList>    // Some(empty) known empty, None unknown
attachments_json: {"total": u64, "parts": [ {filename, content_type, size, sha256, stored, ics} ]}
attachment_bytes: BLOB = concat(parts[i].content for stored i, in order)
restored only:    attachments_json = [[filename, content_type, size], …], attachment_bytes NULL  // 0108's rows → labels_only
```

`total` is at least `parts.len()`, `parts.len()` is at most 250, the sum of
stored sizes is at most 1 048 576 and equals the blob length. The persisted
object omits `content`; restore splits the blob by the stored parts' sizes.
The tuple shape is never written; restore reads it labels-only, which the
compact view serves and both tools answer as unknown (0110 §6). (The
`AttachmentList` line was reconciled with the code on 2026-11-01; see the
implementation notes in tasks.md.)

## Contracts

Descriptors (prose owes the figures 250, 1 MiB, the raw route and the
encodings):

```json
{ "name": "swarmail_list_attachments",
  "inputSchema": { "type": "object", "properties": { "id": { "type": "string" } },
                   "required": ["id"], "additionalProperties": false } }
{ "name": "swarmail_get_attachment",
  "inputSchema": { "type": "object",
                   "properties": { "id": { "type": "string" },
                                   "index": { "type": "integer", "minimum": 0 } },
                   "required": ["id", "index"], "additionalProperties": false } }
```

Output schemas are closed objects: list `{id, total, truncated, attachments}`
with `attachments` an array of at most 250 closed entries whose `ics` is
`anyOf [IcsEvent, null]`; get `{index, filename, content_type, size, sha256,
encoding (enum utf-8|base64), content}`. `sha256` carries
`pattern ^[0-9a-f]{64}$`.

## Implementation sequence

1. Types and constants (`model.rs`, `attachments.rs` skeleton), the digest
   helper with its known-answer tests, the ICS scanner with its unit tests.
2. The candidate walk, hashing and first-fit budget in `attachments.rs`;
   `build_email` wiring; inline tests on generated MIME covering the
   calendar arm, nested message, label caps, 250/251 and the budget edges.
3. Persistence: columns, binding, strict restore, refusal tests.
4. The two tools, descriptors, schemas, catalogue and revision tests.
5. Wire suites: `tests/mcp.rs`, `tests/persistence.rs`, `tests/stdio.rs`,
   the revision module.
6. Evals: `SWARMAIL_TOOLS`, the invite rig test.
7. Docs, bench before/after, every gate.

## Quickstart and verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --manifest-path evals/Cargo.toml --locked
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh && bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh && bash scripts/seal-branch-test.sh
cargo deny check licenses
cargo bench --bench ingest   # before and after; report, do not assert
git diff --exit-code Cargo.lock deny.toml
```

## Risks

- **The rustls hash path is not public at the locked version.** Mitigation:
  it is the first task; the implementer compiles it before anything else. If
  it fails, implementation stops with a named blocker for the operator
  (promote `ring` or add `sha2`); no hand-rolled digest, no silent drop of
  the hash field.
- **Memory and data-file growth.** Up to 1 MiB per mail beside `raw`.
  Mitigation: the bound is pinned, stated in 0110 §8 and asserted through the
  blob length under the hostile fixtures of AC4; `raw` already dominates.
- **250 latency.** Hashing and copying up to 1 MiB and walking every part
  adds to each ingest. Mitigation: linear in bytes the parser already
  decoded; mail without attachments pays only the walk; the ingest bench is
  run before and after and reported honestly.
- **Restore refusals on real files.** A strict reader turns a damaged row
  into a refused start. Mitigation: the same posture 0094/0097 chose, with
  the column named in the error; tests cover each refusal.
- **Reconciled with #155.** Same names, different stored shape. Mitigation:
  the reconciliation is written in 0110 and both shapes restore, proved by
  `src/persist.rs` and `tests/persistence.rs`.
- **Bridge cap on the largest fetch.** Mitigation: the arithmetic is in the
  decision and `tests/stdio.rs` delivers a 1 MiB part in both encodings.
- **Bridge cap on the largest list.** Mitigation: `MAX_LIST_JSON_BYTES`, proved
  by `tests/mcp_revisions/attachments.rs` over HTTP and the binary bridge.
- **Exact coverage.** New branches in the scanner and the refusal ladder.
  Mitigation: every branch has a named test in tasks.md; generated MIME
  reaches each wire arm; no seam is needed, and 0020 §2 governs if one
  appears.
- **The metric cannot be measured here.** Mitigation: AC9 states the offline
  proof and the exact not-measured sentence; no paid trial is run.
