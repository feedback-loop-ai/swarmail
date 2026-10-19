# Operator handoff — corrections for closed issues (2026-10-19)

Prepared under issue #142 (epic #120, batch 6). **Nothing here has been
posted**: the delivery run sends no GitHub comments and rewrites no commit
messages. Each block below is a comment for the operator to post on the
named closed issue, or to adapt. The corrections of record are in
[`v0.1.0.md`](v0.1.0.md), "Errata and provenance".

## #114 (commit `33ac7d1`) — also #74, #93, #94

> Correction (2026-10-19, #142): this landing's message says links are cut
> "with the full named-entity set". It shipped a 71-entry subset. The full
> table, all 2,125 semicolon-ended WHATWG names, shipped in epic #120
> batch 4 (`18374cc`, #165), generated from a pinned snapshot and tested
> against every mapping in it (`src/extract/entities.rs`,
> `all_whatwg_entities_match_snapshot`).

## #118 (commit `582c73a`) — also #95, #100

> Correction (2026-10-19, #142): two statements in this landing's message
> are wrong.
>
> 1. "`/api/v1/message/latest` honours ?inbox= and ?query= via the shared
>    search (limit 1)" — `latest` honours `?inbox=` only. A nonblank
>    `?query=` is a 400 (search, then fetch the id), and `latest` reads
>    `Store::latest`, not the shared search
>    (`tests/compat.rs::latest_refuses_a_query_an_unknown_key_and_a_blank_inbox`).
> 2. "Thread keys strip … Re[n]: forms" — a numbered `Re[2]:` is not a
>    marker and stays in the fallback key
>    (`src/threads.rs`, `subjects_normalize_by_stripping_reply_markers`).
>
> This landing also had no release-note entry. One is now recorded in
> `docs/releases/v0.1.0.md` ("Unreleased — compat and validation sweep"):
> unknown query keys answer 400 on the native list/await/assert/count/inbox
> listing and on `latest` (the compat list and search keep upstream's
> tolerance); `messages_unread_count` became `messages_unread`; MailHog
> `From`/`Raw.From`/`kind=from` and Mailpit `ReturnPath` moved to the
> envelope sender; and a MailHog v2 page over 64 MiB refuses whole.

## #115 (commit `40ef305`) — also #75

> Correction (2026-10-19, #142): this landing's message says an
> unheaderable webhook secret is refused "with the byte named". The
> refusal is fixed, value-free text — `webhook secret must be a valid HTTP
> header value: no control characters but tab` — and names no byte of the
> secret (`src/webhook.rs`, `the_secret_domain_is_the_header_parsers`).

## #106 (closed with its list open)

> Follow-up (2026-10-19): the remaining and newer documentation drift is
> carried by #142 and corrected in epic #120 batch 6 — one gate inventory
> in `docs/house-rules.md`, dated errata in `docs/releases/v0.1.0.md`
> instead of a "frozen notes" claim, and the README facts (CI job names,
> coverage invariant, suite table, chart values, logging streams, body
> limits and SMTP session limits) checked against the code.
