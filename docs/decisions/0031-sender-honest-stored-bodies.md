# 0031 — The stored record is sender-honest: no body synthesis, and display-time rendering is not extraction

- **Status:** accepted (operator, 2026-10-16 — "accept all")
- **Date:** 2026-10-16
- **Amends:** [0005](0005-extraction-at-ingest.md) (accepted) — §Ruling's
  "nothing re-parses or regexes raw HTML at query time" gains one scoped
  carve-out (§3). 0005's ingest ruling on links and codes is **preserved
  verbatim**: extraction still happens once, at insert time, and query
  surfaces still read the stored fields. Filed with the run-11 fidelity
  batch (issues #39, #54, #55, #56, #57); the tree implements it.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md) (the decode
  and the part-type match run inside the existing offloaded parse, before
  the 250; `insert` gains no await), [0003](0003-oldest-only-pruning.md)
  (untouched), [0022](0022-bounded-data-raw-bytes.md) (`raw` stays the
  byte-honest truth; the render is bounded by the same ≤ 50 MiB cap),
  [0030](0030-loopback-plane-bounded-results.md) §2 (the rendering lives
  on the loopback compat plane, outside the mutation boundary).

## §1 The rule: the stored `text`/`html` fields hold only what the sender sent

Today `build_email` uses mail-parser's *synthesizing* accessors —
`body_text(0)` converts HTML→text and `body_html(0)` converts text→HTML —
so a text-only mail is stored with an `html` body the sender never sent,
and extraction over a `<style>` block invents verification codes from
stylesheet colors. The record, and every surface mirroring it (REST, MCP,
Mailpit, MailHog, webhook payloads), lies about the wire.

> **The stored record holds what arrived.** A text part stores `text` and
> `html: null`; an html part stores `html` and `text: null`; anything
> else stores neither. No accessor may synthesize a body the sender did
> not send, under any name, in any direction.

If a derived *stored* view is ever wanted, it is a new field with a new
name and this decision's successor — never an overwrite of the wire truth.
`raw` (0022) remains the complete byte-honest fallback for anything a
consumer misses.

## §2 Why the synthesis was a failure class, not a convenience

A test sink's worst failure is answering with something plausible and
wrong. A `codes` array containing `333333` because a stylesheet declared
`p{color:#333333}` is exactly that: an assertion can pass against an
invention, and the mail's `html` body can disagree with the bytes on the
wire. The synthesis also made honest text *dishonest in the other
direction* — a text-only mail's `html` field invited every consumer to
treat generated markup as sent markup. Extraction quality is a product
surface by 0005's own consequence clause; the extractor's inputs must be
the sender's, or its output is fiction with a straight face.

**Costs accepted, named.** An html-only mail reports `text: null` (MCP,
REST) and a text-only mail reports `html: null` — that is the feature,
not a regression. Mailpit's shape has no null in it: its `Text`/`HTML`
are empty strings for the absent part (compat maps
`unwrap_or_default`), so a client cannot distinguish "no part" from
"empty part" on those two fields — the native and MCP planes keep the
null and are the honest wires. A needle matching *decoded* text (`a&b`
against source `a&amp;b`) hits only via the raw source's encoded form or
not at all. Tests that asserted the synthesis are updated in the same
commit. Seq of consequences, not a hidden bill.

## §3 The carve-out: display-time rendering is not extraction

0005's letter says "nothing re-parses or regexes raw HTML at query
time". This decision scopes that sentence to *extraction* — the
precomputed-answer property it protects (links, codes, identical answers
on every surface, cost paid once on the hot path) — and **not** to
presentational rendering:

> **A display surface may render a stored body into another
> presentation at request time (e.g. `html_to_text` for a Mailpit
> snippet or a text/plain shim view), provided it derives no links or
> codes from the rendering, writes nothing back, and the render is
> bounded by the stored body's own 0022 cap.** That is a rendering, the
> same class of work a mail client does every time it shows a mail —
> not extraction.

Concretely, this batch's two fallbacks (`snippet`, `plain_response` in
`src/compat.rs`) render `mail_parser::decoders::html::html_to_text(html)`
when `text` is `None`, so Mailpit clients keep a text view for html-only
mail. **Cost accepted:** an html-only mail's snippet/plain view pays one
bounded render per request where ingest previously paid it once — on the
loopback compat plane, outside the mutation boundary, bounded by 0022.
**The fallback posture if review refuses even this:** an empty view for
html-only mail; the stored truth is identical either way, and the diff
shrinks by two lines.

The extraction boundary itself stays exactly where 0005 put it: links
are entity-decoded and codes are harvested from rendered text **once, at
ingest, inside the offloaded parse** — a hostile body costs once, on the
blocking pool, and a query never regexes raw HTML for links or codes.

## §4 What this decision does not do

- It does not move extraction to query time; §3 is a presentation
  carve-out, and links/codes remain ingest-time stored fields.
- It does not widen the dependency allowlist; `html_to_text` is
  mail-parser's own public decoder, already in the tree.
- It does not touch the wire-shape refusal disciplines (0029/0025/0018);
  nothing here refuses anything — it stops inventing.
- It does not alter `raw`'s 0022 contract or `/raw`'s byte-exactness.

## Clarification — 2026-10-18 (extraction fidelity)

This clarification cites [0040](0040-extraction-fidelity.md), which is
**proposed** — acceptance is the operator's; nothing here claims it. The
ruling above stays as accepted. Issues #74, #93, #94.

- **§1 — which parts are the bodies.** The stored `text`/`html` are the
  first part of the matching type among the outer message's body
  candidates, continuing past the parser's fallback candidates; nested
  multipart bodies count, attachments and an attached message's bodies do
  not.
- **§3 — codes are not harvested from rendered text.** Ingest no longer
  calls the display renderer: codes come from the plain text, or from the
  html's own text runs read by a small tokenizer (0040 §2–§3). The display
  carve-out — `snippet`, `plain_response` — is unchanged.
- **Historical rows.** These repairs govern fresh ingest only. Restore is
  verbatim: a row written by an older build keeps its stored `text`,
  `html`, `links` and `codes` — including a synthesized body, an `&amp;`
  link or a stylesheet code — across every restart, and nothing re-derives
  them from the stored raw mail. A new delivery of the same raw mail gets
  the repaired values. Old rows are outside the sender-honesty claim.
