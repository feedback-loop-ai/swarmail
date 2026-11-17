# Web UI run console (issue #355)

Decision: 0134 — [a read-only UI projection route and two read-only fields](../../docs/decisions/0134-ui-projection-route.md) (proposed; acceptance is the operator's)

- **Run:** `issue-355-rework-the-web-ui-as-a-c69a5333`, design phase, chief seat, 2026-11-16.
- **Reference design:** [prototype.html](prototype.html), accepted by the operator on 2026-11-16 (sample data, invented mail).
- **Operator ruling:** [operator-ruling.md](operator-ruling.md), 2026-11-16, "Amend for a UI projection".
- **Plan and tasks:** [plan.md](plan.md), [tasks.md](tasks.md).
- **Hold:** the delivery builds the whole design against proposed 0134, as the operator ruled. The pull request is not merged on green: it waits for the operator's acceptance of 0134.

## What and why

The embedded web UI is today a list of cards: an inbox index, a live thread list and a server-rendered message page. A person watching a test run has to click through three pages to answer the questions a run raises: is the server taking mail, which test inbox just got something, what code or link did that mail carry, and would it be delivered.

The operator accepted a redesign: one **run console** with three panes (inboxes, live stream, reader) under one status bar. This specification ports that prototype onto real data. The console paints from the JSON API and the SSE feed that already exist, plus the three additions the operator ruled or the framing found necessary:

1. one read-only per-message **projection route** for the fields accepted decisions keep out of every `Email` serialization (attachment labels and ICS fields, link anchor text and source, the SMTP client address and HELO name);
2. two **counters on `/healthz`** for the status bar;
3. a **`last_received_ms`** member on the inbox list for the last-mail time and the new-mail dot.

Nothing here touches the accept path. `build_email`, `Store::insert`, the SMTP `250`, the `await` subscribe-then-check order and pruning are not changed and not called differently.

## Authorities

Accepted decisions this design relies on and does not change:

- [0001](../../docs/decisions/0001-synchronous-insert-lossless.md), [0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md), [0003](../../docs/decisions/0003-oldest-only-pruning.md): the frozen guarantees. The console only reads.
- [0005](../../docs/decisions/0005-extraction-at-ingest.md): extraction happens at ingest. The projection serves stored fields and never parses `raw`.
- Decisions 0021 and 0032: a cleared inbox ends its feed and no longer exists; an emptied inbox is not listed.
- Decision 0030 §2 and decision 0043: the Origin guard wraps the whole HTTP plane, the new route included. No CORS header is added.
- Decision 0044 §3: the inbox listing's closed `limit`/`offset` query, shared by `/api/v1/inboxes` and the UI index.
- [0085](../../docs/decisions/0085-deliverability-report.md): the deliverability report is lazy and computed at query time. The console labels it so.
- [0108](../../docs/decisions/0108-compact-email-view.md): the presentation cap of 250 entries with an exact total beside a longer list. The projection reuses that cap.
- Issue #56: mail text reaches the DOM through `textContent` only, previews are sandboxed, one CSP string serves every UI route.

Accepted decisions that proposed 0134 amends by dated addendum, on the operator's ruling:

- [0110](../../docs/decisions/0110-attachments-and-ics.md) §7 ("No REST route"): attachment labels and ICS fields gain one REST read. Bytes stay MCP-only.
- [0094](../../docs/decisions/0094-focused-mail-waits.md) §2: link occurrences gain one REST read. Every existing `Email` serialization still skips them.
- [0126](../../docs/decisions/0126-spf-dmarc-alignment.md) §2: the observed SMTP client address and HELO name gain one REST read. Every existing `Email` serialization still skips them.
- Decision 0027 §2: the `ok` and `degraded` `/healthz` bodies gain two counters. Status codes, status values, the `degraded` names and the `stopping` body are unchanged.

## Rulings between the positions

The panel wrote two positions. Where they agree the design adopts both. Where they conflict, these are the rulings.

| # | Question | Simplicity | Robustness | Ruling | What the losing side costs |
|---|---|---|---|---|---|
| 1 | Where do the accepted and dropped counts come from? | Parse the Prometheus text of `/metrics` in the browser; no new field. | Two fields on `/healthz`. | **`/healthz` fields.** The operator's ruling says the counts "need read-only fields"; a text parser in the browser is not a field, and a per-message route cannot answer with zero mail. | One more amended contract (0027 §2, additive) and two more members in every probe answer. |
| 2 | What does the projection carry? | Only the fields no serialization has. | The same, bounded, with absent byte keys proven. | **Both.** Only the amended fields, capped at 250 with exact totals, and no `rel`, `sha256`, `stored`, `uid` or content member. | Simplicity loses reuse of `LinkMetadata`'s derive (a small borrowed struct is written instead), because the derive would publish `rel`, which the ruling does not list. |
| 3 | Where do envelope fields come from? | The thread data already in the page. | No position. | **Thread data.** One mail has one serialization of its envelope. Intake criterion C7 is corrected. | The reader joins two sources by message id. |
| 4 | How does the page change inbox? | A link: full navigation, no router, no history state. | No position; wants in-flight work cancelled on a selection change. | **Navigation.** The browser tears the page down, which cancels every in-flight request and closes the feed without any code. | Switching inbox reloads the page, unlike the prototype's instant switch. |
| 5 | Where does the new-mail dot keep "last seen"? | Page memory only; no storage. | Page memory; a refresh starts clean. | **`sessionStorage`, one fixed key, with a memory fallback.** Under ruling 4 page memory dies on every inbox click, so a dot on an inbox the person never opened would vanish. That breaks intake criterion C5. | About fifteen lines of script and one storage key, proven only by manual review. |
| 6 | How many enrichment requests at once? | All painted rows, once per id, cached. | Rendered window only, 4–6 in flight, `AbortController`, LRU cache. | **One at a time, newest first, selected mail first, bounded cache, failures retried.** The deliverability service admits four analysis jobs and answers `503` when all are busy; a serial queue holds at most one of them. No `IntersectionObserver`: ruling 4 already cancels on a switch. | Robustness loses windowing: a 200-row view makes up to 400 serial requests. Simplicity loses "fire them all": severity chips fill in progressively. |
| 7 | Are mail links clickable? | No position. | Never a mail-controlled `href`; text with a copy control. | **Text with copy.** `textContent` does not stop a `javascript:` URL, and CSP does not govern a clicked link. | One click less convenient. |
| 8 | Is a failed read an empty state? | Paints "unavailable". | Every pane has an error state distinct from empty; error bodies are parsed before `ok` is judged. | **Robustness.** A false "no mail" is the worst answer a test sink can give. | A few more strings and branches in the script. |
| 9 | Theme toggle? | `prefers-color-scheme` only. | No position. | **No toggle.** The prototype has no toggle control either. | None. |
| 10 | One script file or several? | One `ui.js`. | Any split is concatenated into one element and checked as served. | **One file**, and the tests read the served page, so a later split is still checked whole. | None. |

Two points neither position raised are ruled here as well.

- **Clear inbox is in.** The accepted prototype has a two-step "Clear inbox" control, and the issue names exactly two demo-only controls ("Send sample mail" and the chaos toggle). The control uses the existing `DELETE /api/v1/inboxes/{inbox}/messages`; it adds no contract. It is the console's only write, and the acceptance criteria pin that. The operator may strike it at acceptance without touching the rest of the design.
- **Listener chips show state, not addresses.** The prototype prints `SMTP 127.0.0.1:1025`. No API reports the SMTP or POP3 bind address, and a bind address is not the address a client uses behind a container or a Service, so printing it would mislead. The chips show each listener's name and state; the HTTP chip adds `location.host`.

## Corrections to the intake framing

The intake's seventeen criteria stand except where the code or the operator's ruling shows otherwise. Each correction is carried by the criteria below.

- **C3** placed the counts on the projection route. Corrected to `/healthz` (ruling 1).
- **C6** took the link count from the projection. Corrected: the row's link count is the length of the mail's `links` in the thread data, which needs no request; the attachment count comes from the projection.
- **C7** took the envelope from the projection. Corrected to the thread data (ruling 3).
- **C9** asked for a `srcdoc` "built from escaped HTML". That is right for the server-rendered message page and wrong in script: the script assigns the `srcdoc` property, and escaping first would display entities. Corrected to the property setter on a sandboxed frame.
- **C10** named a finding "message" field. `Finding` has `rule_id`, `location`, `severity`, `fix_hint` and `weight`; there is no message.
- **C11** put the no-inboxes sentence in the script. The existing test `the_index_pages_and_links_its_pages` requires that a non-empty store's page never contains "No inboxes yet", and the script is now part of that page. Corrected: the server chooses the sentence and the script only shows or hides it.
- **C12** forbade `http:`. The reader marks a plain-http link, so the scheme word must be writable. Corrected to: no `http://` and no `https://`.
- **C15** scoped the change to `src/ui.rs`, `src/static/*`, `src/api.rs` and tests. `last_received_ms` is read from the inbox deque, which `src/store.rs` owns, so one read-only store method is in scope. The OpenAPI document is `src/static/openapi.json`. The appended dated pointers in four accepted records are in scope.

## Requirements

### R1. Routes and shell

1. `GET /`, `GET /ui/inbox/{inbox}` and `GET /ui/inbox/{inbox}/thread/{key}` serve the same console page. No UI route is added or removed.
2. The page has one header `#status` and one `<main id="panes">` holding three panes: `<nav id="inboxes">`, `<section id="stream">` and `<section id="reader">`, each with class `pane`.
3. `#view` stays, inside `#stream`, as the element the stream is painted into. It keeps `data-inbox` and `data-thread` exactly as today. On `/` `data-inbox` is the empty string.
4. `#panes` carries `data-pane`, server-rendered: `inboxes` on `/`, `stream` on the two inbox routes. The script sets it to `reader` when a mail is opened, and the reader's back control sets it to `stream`.
5. The page carries exactly one `<script>` element and one `<style>` element, both embedded at compile time (`src/static/ui.js`, `src/static/ui.css`).
6. `GET /ui/message/{id}` is unchanged in markup: server-rendered, escaped, no script, `404` for an unknown id. It is the console's permalink and the reader without JavaScript. It takes the new stylesheet, which keeps the class names that page uses (`wrap`, `card`, `pill`, `mono`, `muted`).
7. The existing refusals hold: a blank or control-bearing inbox segment is `400` on both inbox routes, and `/` keeps the closed `limit`/`offset` query.

### R2. Status bar

1. Three listener chips, `#listener-smtp`, `#listener-pop3` and `#listener-http`, painted from `GET /healthz`. The script parses the JSON body **before** it looks at the status code, so a `503` that names a degraded listener is shown by name. States: `ok`, `degraded`, `stopping`, and `unknown` when the read fails or the body is not the expected object. The HTTP chip shows `location.host`.
2. `#count-accepted` and `#count-dropped`, painted from the `/healthz` members `emails_inserted_total` and `emails_dropped_total` (R7). They are lifetime counters and are labelled as such; "dropped" is labelled as mail evicted by the per-inbox cap. The script never computes a live count from them. A `stopping` answer carries no counters; the bar keeps the last values it painted.
3. `#chaos` shows the chaos state read-only from `GET /api/v1/chaos`: "Chaos off" when all four events are null, otherwise "Chaos on" with each active event's name and probability. A failed read shows "Chaos unknown", never "off". It is not a button.
4. `#live` follows the feed: "Live" after the `EventSource` opens, "Reconnecting" on an error while it retries, "Offline" when it is closed, "Idle" on `/` where no inbox is selected and no feed is open. It is never set from page load or from a successful poll.
5. One timer drives the status reads and the inbox poll every 2 seconds. A tick is skipped while the page is hidden and while the previous tick is still in flight.
6. There is no "Send sample mail" control, no chaos toggle and no "Sample data" badge.

### R3. Inboxes pane

1. The server renders the first paint: one row per inbox on the page, each a link to `/ui/inbox/{percent-encoded name}` showing the name and the count, with the open inbox marked `aria-current="true"`. `/` renders the page its validated `limit`/`offset` query selects, with the existing pager; the inbox routes render the default page (limit 100, offset 0).
2. The row list `#inbox-rows` carries `data-limit` and `data-offset`, the validated page. The script polls `GET /api/v1/inboxes?limit=…&offset=…` built from those two attributes and repaints the rows. It never reads `location.search`.
3. Each row shows the time of the last mail as a relative time ("last mail 12s ago") computed from `last_received_ms` (R7).
4. **New-mail dot.** The script keeps a last-seen `last_received_ms` per inbox name. A row shows the dot when the polled value is greater than the last-seen value. The open inbox never shows a dot and its last-seen value follows every poll. An inbox first observed on the page's first poll gets no dot unless storage already holds an older value for it; one first observed later does.
5. Last-seen values are compared only with server stamps, never with the browser clock. They live in a `Map`, persisted to `sessionStorage` under one fixed key as an array of `[name, ms]` pairs, at most 512 pairs, oldest dropped first. Storage that throws or holds anything else is treated as empty and the page falls back to memory.
6. `#inbox-filter` filters the listed rows by substring in the browser. No match shows "No inbox matches."
7. The empty sentence lives in `#inbox-empty`, written by the server: "No inboxes yet — point SMTP at this server and send something." when the store has no inbox, otherwise "No inboxes on this page." The script shows that element when a poll returns no rows and hides it otherwise. The script holds neither sentence.
8. A failed poll keeps the last painted rows and shows "Inboxes unavailable — retrying". It never shows the empty sentence.

### R4. Stream

1. The stream paints the selected inbox's thread view: `GET /api/v1/inboxes/{inbox}/threads` (or `/threads/{key}` on the thread route) for the first paint, then every `threads` frame of the existing feed `GET /api/v1/inboxes/{inbox}/feed`. The page opens exactly one `EventSource`. The existing paint token (`paintSeq`) keeps guarding a late first response against a newer frame.
2. Threads are listed newest thread first, as the API orders them. A thread of more than one mail gets a label ("Thread · N mails") that links to its thread route. Rows inside a thread are newest first.
3. Each row shows the sender, the receive time, the subject, every extracted code, the link count (from the mail's `links`), the attachment count with a calendar marker when a part carries ICS fields (from the projection), and one severity chip (from the deliverability report): the worst severity and the finding count, or "no findings".
4. A row that arrives in a frame after the first paint is marked fresh until it is opened.
5. The header shows the inbox name and the mail count. When the inbox list reports more mail than the view holds (the thread view is capped), the count reads "newest N of M".
6. On the first paint of an inbox that has mail and no selection, the newest mail is selected for the reader. This does not change `data-pane`.
7. States, each distinct: "Pick an inbox to watch its mail." on `/`; "Empty and live. Send mail with SMTP AUTH user … and it appears here as it is accepted." for an inbox with no mail; "Stream unavailable — retrying" when the first read fails. No string in the page contains the words "is empty" (an existing test forbids them in the shell).
8. **Clear inbox.** A "Clear inbox" control appears when the open inbox has mail. The first click only arms it ("Clear N?" with "Clear" and "Keep"). The second click sends one `DELETE /api/v1/inboxes/{inbox}/messages`. The stream is repainted from the server's next answer, never optimistically; a failed request shows "Clear failed" and changes nothing.

### R5. Reader

1. With no mail selected the reader shows "Pick a mail to read it."
2. **Envelope**, from the thread data: subject, From (display name and address), To, envelope sender, receive time and size. **Client**, from the projection: the HELO name and the client address, or "—" when a value is null (seeded mail has neither).
3. **Extracted at ingest**, a panel with exactly that label, holding only ingest-time data: codes, each with a "Copy" control; links, each with its anchor text, its URL and its source (`text` or `html`), with an "http" marker on a plain-http URL; attachments, each with filename, content type and size, and for a calendar part its summary and its start and end as the calendar spelled them, with their `TZID` when present. When the projection reports `links` as null (a row from before link occurrences were stored) the panel lists the bare URLs from the mail's `links` and says the anchor text is unknown. When it reports `attachments` as null the panel says the attachment list is unknown. A list longer than the 250 the projection returns says "showing 250 of N".
4. **Authentication**, a separate row labelled "Authentication (query-time report)". It is never inside or labelled as "Extracted at ingest". For each of `dkim`, `spf` and `dmarc` it shows the worst-severity finding whose `rule_id` starts with that name and a dot: the rule id without the mechanism prefix and the trailing version, coloured by severity. A mechanism with no finding reads "no finding". No chip ever reads "pass": the report has no pass result, and the absence of a finding is not one. The `arc` chip is the literal "not evaluated" (issue #256).
5. **Tabs**, built by the script with ids `tab-preview`, `tab-text`, `tab-deliverability` and `tab-tests`:
   - **Preview:** an `iframe` whose `sandbox` attribute is set to the empty string before its `srcdoc` property is assigned the mail's HTML, and before it is attached. No sandbox token is ever added. Without an HTML part: "No HTML part."
   - **Text:** the text body in a `pre`, or "No text part."
   - **Deliverability:** the rule set id and the total weight, then each finding with its severity, rule id, location and fix hint, worst first. The location is rendered from its `kind`: a header name and occurrence, a MIME part path, or a link's part path and byte offset. No findings: "No findings from this rule set." The tab always ends with "No findings does not mean the mail will be delivered. These are demo and bulk-sender checks only."
   - **For your test:** copyable snippets for the open inbox: a REST wait, a REST exact-count assert, and an MCP wait for the mail's code or link. The base URL is `location.origin` at run time. In the `curl` lines the URL is single-quoted and the inbox segment is percent-encoded, including the five characters `encodeURIComponent` leaves (`!`, `'`, `(`, `)`, `*`), so no shell metacharacter of a hostile inbox name survives a paste. The MCP snippet is produced with `JSON.stringify`, so an inbox name or an anchor text cannot break out of its string.
6. The reader header links to `/ui/message/{id}` ("Open standalone") and, at narrow widths, shows a back control to the stream.
7. States: "Details unavailable — retrying" when the projection read fails; "Deliverability unavailable — retrying" when the report read fails; "Message not found (pruned or cleared)." when either answers `404` or the selected mail leaves the view. A failed report is never shown as "no findings".

### R6. The projection route (proposed 0134 §1–§5)

1. One new route: `GET /api/v1/messages/{id}/projection`. It has no other method.
2. The body is one object whose five members are always present:

   ```json
   {
     "id": "0193…",
     "smtp_client_ip": "192.0.2.7",
     "smtp_helo": "mailer.example.org",
     "links": {
       "total": 2,
       "items": [ { "url": "https://app.example/verify?t=1", "anchor_text": "Confirm email", "source": "html" } ]
     },
     "attachments": {
       "total": 1,
       "items": [ {
         "filename": "delivery.ics", "content_type": "text/calendar", "size": 612,
         "ics": { "summary": "Delivery window", "dtstart": "20261118T090000", "dtstart_tzid": "Europe/Sofia", "dtend": "20261118T120000", "dtend_tzid": "Europe/Sofia" }
       } ]
     }
   }
   ```

3. `smtp_client_ip` and `smtp_helo` are the stored observation or `null` (mail that did not arrive over SMTP, or a row from before the fields).
4. `links` is `null` when the stored occurrences are unknown, otherwise `total` (the exact stored count) and `items` (the first 250 in stored order). An item has exactly `url`, `anchor_text` (string or `null`) and `source` (`"text"` or `"html"`).
5. `attachments` is `null` when the stored list is unknown, otherwise `total` (the stored exact total) and `items` (the first 250 stored parts in part order). An item has exactly `filename`, `content_type`, `size` and `ics`; `ics` is `null` or exactly `summary`, `dtstart`, `dtstart_tzid`, `dtend` and `dtend_tzid`, each a string or `null`. A labels-only restored list is served the same way.
6. The body never has a member named `content`, `bytes`, `body`, `raw`, `text`, `html`, `sha256`, `stored`, `uid` or `rel`. Attachment bytes stay MCP-only.
7. Refusals: an unknown or pruned id is `404 {"error":"message not found"}`; any query key is `400` naming the route and the key, through the shared closed-query parser; another method is `405`; a guard-refused request is `403` as on every route.
8. The handler reads one stored `Email` and serializes stored fields. It does not read `raw`, run extraction or ask for a deliverability report.
9. No existing response changes: `GET /api/v1/messages/{id}`, the lists, the compact view, the thread view, the feed frames, webhook payloads, MCP results and the compat shims keep their bytes.
10. The route, its response schema and its `400`, `403` and `404` answers are documented in `src/static/openapi.json`, and `src/static/llms.txt` names the route.

### R7. Two read-only fields (proposed 0134 §6–§7)

1. `GET /healthz`: the `200 ok` and `503 degraded` bodies gain `emails_inserted_total` and `emails_dropped_total`, the two store counters `/metrics` already exposes as `swarmail_emails_inserted_total` and `swarmail_emails_dropped_total`. The `503 stopping` body stays exactly `{"status":"stopping"}`.
2. `GET /api/v1/inboxes`: every item gains `last_received_ms`, the `received_ms` of the mail most recently stored in that inbox (the newest retained position), an integer, or `null` if the inbox holds no mail at the read. The member is always serialized. It is read from the store on every request and is never persisted or mirrored. It is the server's ingest stamp, never the mail's `Date` header.
3. Both are documented in `src/static/openapi.json`.

### R8. Safety

1. Mail-controlled and client-controlled text (subjects, addresses, names, codes, URLs, anchor text, filenames, content types, calendar fields, HELO names, inbox names, finding fields, error bodies) reaches the DOM only through `textContent` and the `srcdoc` property of the sandboxed frame.
2. The script contains none of `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `eval(` or `Function(`, and no `.style` access: appearance is chosen by class from a fixed set, never from mail data.
3. The script assigns `href` only to same-origin paths that start with the literal `/ui/`. It never builds a link from a mail URL, never calls `window.open` and never assigns `location`.
4. The preview frame has the bare `sandbox` attribute. The page never contains `allow-scripts`, `allow-same-origin`, `allow-top-navigation`, `allow-popups` or `allow-forms`.
5. The CSP of every UI route is the existing string, byte for byte: `default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:`.
6. The console page loads nothing from outside: no `http://`, no `https://`, no protocol-relative `src` or `href`, no `<link`, no `@import`, no `url(`, no `@font-face`. Fonts are system stacks.
7. The script's only write is the Clear inbox request (R4.8). It issues no `PUT`, `POST` or `PATCH`, names `/api/v1/chaos` only for the read, and never names a seed, webhook, DNS or attachment-content route.
8. Caches and last-seen values are `Map`s. No plain object is indexed by a mail-controlled or client-controlled key.
9. No dependency is added. `Cargo.toml`, `Cargo.lock`, `deny.toml` and `evals/Cargo.lock` are unchanged.

### R9. Bounded enrichment

1. The projection and the deliverability report of a mail are fetched lazily, at most once per message id while the result is cached, through one serial queue: one request in flight at a time, the selected mail first, then painted rows newest first.
2. The report is never requested with `reachability`.
3. Results are cached in a `Map` of at most 512 message ids, oldest dropped first. A failure is not cached: the row shows "unavailable" and the id is queued again no sooner than the next status tick, only while its row is painted.
4. A `404` is cached as "pruned or cleared"; it is a normal race with pruning, not an error.

### R10. Layout and themes

1. The layout, spacing, type and colour tokens follow the prototype: a sticky status bar, three columns at desktop width, two at tablet width, one at phone width.
2. At phone width (`max-width: 680px`) exactly one pane is shown, chosen by `#panes[data-pane]`; the block that does it is, up to whitespace:

   ```css
   @media (max-width: 680px) {
     #panes > .pane { display: none; }
     #panes[data-pane="inboxes"] > #inboxes,
     #panes[data-pane="stream"] > #stream,
     #panes[data-pane="reader"] > #reader { display: block; }
   }
   ```

   Because the server writes `data-pane`, the right pane shows before any script runs.
3. Light is the default theme; `@media (prefers-color-scheme: dark)` supplies the dark one. There is no toggle and no stored theme.
4. `prefers-reduced-motion: reduce` turns the pulse and arrival animations off.

### R11. Scope

Changed: `src/ui.rs`, `src/static/ui.js`, `src/static/ui.css` (new), `src/api.rs`, `src/pop3.rs` and `tests/smtp_edges.rs` (test-only: pinned bodies follow the additive 0134 members), `src/store.rs` (one read-only method), `src/static/openapi.json`, `src/static/llms.txt`, `tests/ui.rs`, `tests/api.rs`, `docs/guide/rest-api.md`, `docs/guide/focused-waits.md`, `docs/guide/attachments.md`, `docs/decisions/0134-ui-projection-route.md` and its row in `docs/decisions/README.md`, one appended dated pointer in each of the 0110, 0094, 0126 and 0027 records, and this directory. Untouched: `Cargo.toml`, `Cargo.lock`, `deny.toml`, `evals/`, `scripts/`, `.brokkr/`, `.github/`, `src/smtp.rs`, `src/extract.rs`, `src/feed.rs`, `src/model.rs`.

## Deviations from the prototype

Each is deliberate and named, so review does not read it as an omission.

| Prototype | Console | Why |
|---|---|---|
| "Send sample mail", chaos toggle, "Sample data" badge | absent | Demo-only (issue #355). |
| Listener chips print bind addresses | name and state; HTTP adds `location.host` | No API reports the SMTP or POP3 address, and a bind address misleads behind a container. |
| `dkim=pass` chips inside "Extracted at ingest" | a separate "Authentication (query-time report)" row; "no finding", never "pass"; `arc` "not evaluated" | The results come from the query-time report (0085), which has no pass result; ARC has no evaluator (#256). |
| Inbox switch without a reload | a link to `/ui/inbox/{name}` | Ruling 4. |
| One tag per attachment in a stream row | one count and one calendar marker | A mail may list 250 parts. |
| Empty stream names port `:1025` | names the SMTP AUTH user only | The port is not known to the page. |
| Preview note says remote images still load | says scripts are off | The inherited CSP allows images only from the same origin or `data:`. |

## Acceptance Criteria

Every criterion names where it is proven. "Served page" means the bytes of a real `GET` against a started server, as `tests/ui.rs` already does.

**Shell and routes** (`tests/ui.rs`)

- **AC1.** `GET /`, `/ui/inbox/{inbox}` and `/ui/inbox/{inbox}/thread/{key}` answer `200` with a page that contains `id="status"`, `id="panes"`, `id="inboxes"`, `id="stream"`, `id="reader"` and `id="view"`, and exactly one script element. `data-pane` is `inboxes` on `/` and `stream` on the other two.
- **AC2.** Every test present in `tests/ui.rs` and in `src/ui.rs`'s `tests` module at commit `4db525c` passes with its assertions unmodified. If the implementation finds one that cannot, the delivery names it, keeps its invariant and says why; none is expected.
- **AC3.** `GET /ui/message/{id}` ships no script element, escapes a markup subject, and answers `404` for an unknown id (the existing tests, unmodified).

**Status bar** (`tests/ui.rs`, `tests/api.rs`, `src/api.rs`)

- **AC4.** The served page contains `id="listener-smtp"`, `id="listener-pop3"`, `id="listener-http"`, `id="count-accepted"`, `id="count-dropped"`, `id="chaos"` and `id="live"`. The served script contains `'/healthz'`, `degraded`, `stopping`, `emails_inserted_total`, `emails_dropped_total` and `'/api/v1/chaos'`, and the strings "Chaos unknown", "Reconnecting", "Offline" and "Idle".
- **AC5.** Over the wire, `/healthz` on a fresh server answers `200` with `emails_inserted_total == 0` and `emails_dropped_total == 0`; after two accepted mails the first is `2`; on a server whose per-inbox cap is 1, after two mails to one inbox the second is `1`. Both equal the values `/metrics` reports in the same test.
- **AC6.** In `src/api.rs`'s unit tests the `degraded` body carries both counters beside `status` and `degraded`, and the existing stopping tests pass unmodified (the body is exactly `{"status":"stopping"}`).
- **AC7.** The served page does not contain "Send sample mail", "Sample data" or `aria-pressed`, and `#chaos` is not a `button` element.

**Inboxes** (`tests/api.rs`, `tests/ui.rs`)

- **AC8.** `GET /api/v1/inboxes`: every item has the key `last_received_ms`; after one mail it equals that mail's `received_ms` from the message list; after a second mail to the same inbox it equals the second mail's `received_ms`; after the inbox is cleared the inbox is not listed.
- **AC9.** `#inbox-rows` carries `data-limit` and `data-offset` equal to the validated page (`/?limit=1&offset=1` gives `1` and `1`; an inbox route gives `100` and `0`). The open inbox's row carries `aria-current="true"`. The served script contains `'/api/v1/inboxes?limit='`, does not contain `location.search`, and contains `new EventSource(` exactly once.
- **AC10.** On an empty store every console route's page contains "No inboxes yet"; on a non-empty store no console route's page contains it. The served script contains "No inbox matches." and "Inboxes unavailable — retrying".

**Stream** (`tests/ui.rs`)

- **AC11.** The served script contains `api('threads')`, `'threads/' + encodeURIComponent(threadKey)`, `api('feed')`, `paintSeq++` and `seen !== paintSeq` (the existing wiring assertions), and the literals `'/projection'` and `'/deliverability'` exactly once each.
- **AC12.** The served script does not contain `reachability`.
- **AC13.** The served script contains "Pick an inbox to watch its mail.", "Empty and live.", "Stream unavailable — retrying", "newest " and "no findings". No served console page contains "is empty".
- **AC14.** The served script contains `method: 'DELETE'` exactly once, on a request built with `api('messages')`, and the strings "Clear inbox", "Keep" and "Clear failed". It contains none of `'PUT'`, `'POST'`, `'PATCH'`, `/seed`, `/webhooks`, `/dns`, `/raw` or `/attachments`, and `/api/v1/chaos` exactly once.

**Reader** (`tests/ui.rs`)

- **AC15.** The served script contains `'tab-preview'`, `'tab-text'`, `'tab-deliverability'` and `'tab-tests'`; "Extracted at ingest"; "Authentication (query-time report)"; "no finding"; "not evaluated"; "Pick a mail to read it."; "Details unavailable — retrying"; "Deliverability unavailable — retrying"; "Message not found (pruned or cleared)."; "No HTML part."; "No text part."; and "No findings does not mean the mail will be delivered." It does not contain `'pass'`, `"pass"` or `=pass`.
- **AC16.** The served script contains `setAttribute('sandbox', '')` and `.srcdoc =`. No served console page contains `allow-scripts`, `allow-same-origin`, `allow-top-navigation`, `allow-popups` or `allow-forms`.
- **AC17.** The served script contains `location.origin` and `JSON.stringify(`. Every `swarmail_…` tool name in the served script is a tool the same server lists in MCP `tools/list`.

**Projection route** (`tests/api.rs`, `src/api.rs`)

- **AC18.** A multipart mail sent over real SMTP with a known `EHLO` name, an HTML part with one anchor, a text part with one URL, a PDF attachment and a calendar part: the projection's `smtp_helo` is the `EHLO` name, `smtp_client_ip` is the client's loopback address, `links.total` is 2 with the anchor's text and `source` `"html"` on one item and `source` `"text"` on the other, and `attachments.items` carries both parts' filename, content type and size, with the calendar's summary, start and end under `ics` and `ics` `null` on the PDF.
- **AC19.** For a seeded mail `smtp_client_ip` and `smtp_helo` are `null`, and `links` and `attachments` are objects with a `total`.
- **AC20.** The body's keys are exactly `attachments`, `id`, `links`, `smtp_client_ip` and `smtp_helo`; a link item's are exactly `anchor_text`, `source` and `url`; an attachment item's are exactly `content_type`, `filename`, `ics` and `size`; an `ics` object's are exactly `dtend`, `dtend_tzid`, `dtstart`, `dtstart_tzid` and `summary`. The serialized body contains none of the member names listed in R6.6.
- **AC21.** An unknown id answers `404` with `{"error":"message not found"}`; `?x=1` answers `400` with an error that names `GET /api/v1/messages/{id}/projection` and `x`; `POST`, `PUT` and `DELETE` answer `405`; the answer carries no `access-control-` header.
- **AC22.** A unit test in `src/api.rs` builds an `Email` with 251 stored link occurrences and an attachment list whose `total` is 300 with 250 parts: the projection has `links.total == 251` with 250 items and `attachments.total == 300` with 250 items. The same test gives the `Email` an empty `raw`, so the values can only have come from stored fields; a second case with unknown lists serializes both as `null`.
- **AC23.** After the route exists, `GET /api/v1/messages/{id}`, the message list, the thread view and a feed frame for the AC18 mail contain none of `link_metadata`, `smtp_client_ip`, `smtp_helo` or `attachments`, and the existing model test `the_observed_envelope_never_serializes` passes unmodified.
- **AC24.** The served OpenAPI document has `paths["/api/v1/messages/{id}/projection"]["get"]` with `200`, `400`, `403` and `404` responses and a `MessageProjection` schema; `InboxSummary` documents `last_received_ms` as an integer or null; the `/healthz` description names both counters; the existing dangling-`$ref` walk passes.

**Safety** (`tests/ui.rs`)

- **AC25.** The served script contains none of `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `eval(`, `Function(`, `.style`, `window.open`, `location.assign`, `location.replace`, `location.href =` or `setAttribute('href'`. Every occurrence of `.href =` in it is followed by the literal `'/ui/`.
- **AC26.** Markup payloads (`<img src=x onerror=alert(1)>` and `"><script>alert(1)</script>`) sent through the subject, the From display name, the `EHLO` name, an attachment filename, a calendar summary, an anchor text and a code-bearing text body: every console route's page for that inbox contains neither `<img src=x` nor `<script>alert`, and has exactly one script element; the thread view and the projection return each payload as an equal JSON string. An inbox named `a"><b` is served with `data-inbox="a&quot;&gt;&lt;b"`.
- **AC27.** The CSP meta of `/`, an inbox route, a thread route and a message page equals the string in R8.5, compared with a literal the test owns.
- **AC27a.** Every UI route answers with `X-Frame-Options: DENY`, error answers included (review finding: the Clear inbox control must not be clickjackable; the CSP meta cannot carry `frame-ancestors`).
- **AC28.** No served console page (empty store, benign inbox name) contains `http://`, `https://`, `<link`, `@import`, `url(`, `@font-face`, `src="//` or `href="//`.

**Layout** (`tests/ui.rs`)

- **AC29.** The served style contains `@media (prefers-color-scheme: dark)`, `@media (prefers-reduced-motion: reduce)` and one `@media (max-width: 680px)` block which, with whitespace removed, contains `#panes>.pane{display:none}` and the three `data-pane` selectors of R10.2 followed by `{display:block}`. No rule outside a `prefers-color-scheme: dark` block sets `color-scheme: dark`.

**Delivery**

- **AC30.** Every gate in `docs/house-rules.md` § Gates is clean, the coverage gate at 100% of lines, branches and functions with its selftest, with no coverage exclusion attribute.
- **AC31.** `git diff main --stat` shows no change to `Cargo.toml`, `Cargo.lock`, `deny.toml`, `evals/`, `scripts/`, `.brokkr/` or `.github/`.
- **AC32.** `docs/decisions/0134-ui-projection-route.md` has status `proposed` and one row in `docs/decisions/README.md`; the 0110, 0094, 0126 and 0027 records differ from `main` only by lines appended after their last line (`tests/store_contract_docs.rs` passes).
- **AC33.** The delivery's report lists the items under "Not gate-provable" as manual review and does not report them as passed gates.

## Not gate-provable

No JavaScript engine runs in any gate and no dependency may be added to get one. These are checked by a person in a browser and listed as such in the delivery; the static criteria above are as far as a gate goes.

- Live repaint of the stream and the fresh marker on a new row.
- The new-mail dot: it appears on an inbox that is not open, survives opening a different inbox, and clears when its own inbox is opened.
- The serial enrichment queue: one request in flight, and a failed row recovering on a later tick.
- The listener chips on a degraded `503`, and the live indicator across a feed drop.
- The Clear inbox confirm flow.
- Copy controls, the tab strip, the inbox filter.
- Layout at desktop, tablet and phone widths in both themes.

## Out of scope

- A global feed, or one `EventSource` per inbox.
- Attachment download, a chaos toggle, "Send sample mail", message deletion.
- ARC evaluation (#256), a TLS or certificate badge, listener bind addresses.
- A JavaScript test harness, a client-side router, a theme toggle.
- Restyling `GET /ui/message/{id}` beyond the shared stylesheet.
