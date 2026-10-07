# 0005 — Extraction happens at ingest, not query time

**Status**: accepted (original design, v0.1)

## Ruling

Links and OTP-looking codes are extracted from the parsed message **once,
at insert time** (`src/extract.rs`), and stored as first-class fields
(`links`, `codes`) on the email. Query endpoints, MCP tools and the UI read
the stored fields; nothing re-parses or regexes raw HTML at query time.

## Why

Agents and tests ask the same question over and over: "give me the URL and
the code from the last mail." Doing it at ingest means the answer is
precomputed on the hot path (1.58 µs/mail includes it), every surface
returns it identically, and the query path stays trivial. Doing it at query
time would make every consumer an HTML parser and every tool answer
slightly differently.

## Consequences

- The extractor runs on every accepted mail — its cost is inside the
  published bench numbers, and its 98%+ coverage is a gate.
- Extraction quality is a product surface: false positives in `codes`
  (bare `\d{4,10}`) are visible to every consumer at once, so extractor
  changes are reviewed like contract changes.
- Raw bodies remain stored (`raw`, skipped in JSON) for anything the
  extractor missed.
