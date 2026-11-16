# Plan — web UI run console (issue #355)

Spec: [spec.md](spec.md) · Tasks: [tasks.md](tasks.md) · Decision: [0134](../../docs/decisions/0134-ui-projection-route.md) (proposed) · Reference: [prototype.html](prototype.html)

## Technical context

- One Rust crate. The HTTP plane is axum; `src/ui.rs` serves the UI routes and `src/api.rs` the JSON API. The UI's script and (after this change) its stylesheet are embedded with `include_str!` and inlined into the page.
- There is no JavaScript toolchain and no JavaScript engine in any gate, and none may be added. Script behaviour is therefore asserted on the served bytes (`tests/ui.rs` already does this) and reviewed by hand. This plan keeps the script small in moving parts for that reason.
- Tests speak the real protocols against a started server (`tests/common`): `start()`, `start_with(|cfg| …)` for a per-inbox cap, `smtp_send`, `SmtpConn`, `SseReader`, `http_json`, `http_get_text`.
- Coverage is exact: every new Rust line and branch is executed by the suite, and no exclusion attribute is used.
- Base: branch `feat/web-ui-console-355` at `4db525c`, on `main` `f05f2ee`.

## Constitution check

| Rule | How the plan keeps it |
|---|---|
| 0001 accept == stored | No change to `build_email`, `Store::insert` or the SMTP session. The only store change is one read-only method. |
| 0002 subscribe before check | `src/feed.rs` and the `await` path are untouched. The console reuses the one per-inbox feed. |
| 0003 oldest-only pruning | Untouched. The console displays the pruned counter; it never drives pruning. |
| 0005 extraction at ingest | The projection serializes stored `Email` fields. It never reads `raw`. A unit test gives it an `Email` with an empty `raw` (AC22). |
| Issue #56 | One CSP constant, unchanged; `textContent` only; bare `sandbox`; no mail-controlled `href`. |
| Dependencies | None added. `Cargo.toml`, both lockfiles and `deny.toml` are untouched. |
| Coverage | New Rust is a handful of straight-line functions, each reached by a wire test or an inline unit test. |
| Code under a proposed decision (0045 §8) | The projection contradicts 0110 §7, 0094 §2 and 0126 §2 until 0134 is accepted. The operator ruled on 2026-11-16 that the delivery builds against the proposed record and that acceptance precedes merge. The pull request is held. |
| Decision discipline | Accepted bodies are not edited. 0134 carries the amendment text; each amended record gets one appended dated pointer. |

## Project structure

| File | Change |
|---|---|
| `src/store.rs` | Add `Store::inbox_summaries() -> Vec<(String, usize, Option<i64>)>`, sorted by name: name, count and the `received_ms` of the deque's back. `Store::inboxes()` and its twenty callers stay as they are. One inline unit test. |
| `src/api.rs` | `InboxSummary` gains `last_received_ms: Option<i64>` (always serialized). `inbox_page` returns `Vec<InboxSummary>` from the new store method. `healthz` adds the two counters to the `ok` and `degraded` bodies. New `MessageProjection` types beside the compact view, new handler `get_projection`, one new `.route(…)` line. Inline unit tests for the cap and the degraded body. |
| `src/ui.rs` | `index`, `inbox_view` and `thread_view` all render one `console(…)` page; the two inbox handlers gain `State`. New `rail(…)` helper renders the inbox rows, the empty sentence, and the pager. `page()` and `render_message` keep their markup and take the stylesheet from `static/ui.css`. `pager()` and `esc()` are unchanged. The `STYLE` raw string is removed. |
| `src/static/ui.css` | New. The prototype's stylesheet, plus the legacy class names the message page uses. |
| `src/static/ui.js` | Rewritten around the existing wiring (same identifiers the tests pin). One file. |
| `src/static/openapi.json` | The projection path and `MessageProjection` schema; `InboxSummary.last_received_ms`; the `/healthz` descriptions. |
| `src/static/llms.txt` | One short paragraph naming the projection route, what it carries and that it serves no bytes; the two fields. |
| `tests/api.rs` | AC5, AC8, AC18–AC21, AC23, AC24. |
| `tests/ui.rs` | AC1, AC4, AC7, AC9–AC17, AC25–AC29. Existing tests untouched. |
| `docs/decisions/README.md` | One row for 0134. |
| `docs/decisions/0110-…`, `0094-…`, `0126-…`, `0027-…` | One appended `## Dated pointer — 2026-11-16 (proposed 0134)` section each. Nothing above it changes. |

## Approach

### 1. Store and API fields

`Store::inboxes()` iterates the inbox map and reads each deque's `len()`. The new method does the same and also reads `back().map(|mail| mail.received_ms)`: one O(1) read per inbox, no scan, so the map's shard lock is held no longer than today. Inserts use `push_back` and pruning uses `pop_front`, so the back is the mail most recently stored. `inbox_page` pages that vector and builds `InboxSummary` values; `src/ui.rs` reads the same values for its rows.

`healthz` reads `state.store.emails_inserted()` and `emails_dropped()` after the stopping check and writes them into the two remaining `json!` bodies. The stopping arm is not touched, because `a_healthz_inside_its_handler_at_the_stop_is_answered_503_stopping` pins that body byte for byte.

### 2. The projection

Types, in `src/api.rs` beside `CompactEmail`, all borrowing from the stored `Email`:

```rust
struct MessageProjection<'a> {
    id: &'a str,
    smtp_client_ip: Option<std::net::IpAddr>,
    smtp_helo: Option<&'a str>,
    links: Option<Bounded<ProjectedLink<'a>>>,
    attachments: Option<Bounded<ProjectedAttachment<'a>>>,
}
struct Bounded<T> { total: u64, items: Vec<T> }
struct ProjectedLink<'a> { url: &'a str, anchor_text: Option<&'a str>, source: LinkSource }
struct ProjectedAttachment<'a> { filename: Option<&'a str>, content_type: Option<&'a str>, size: u64, ics: Option<ProjectedIcs<'a>> }
struct ProjectedIcs<'a> { summary: Option<&'a str>, dtstart: Option<&'a str>, dtstart_tzid: Option<&'a str>, dtend: Option<&'a str>, dtend_tzid: Option<&'a str> }
```

No member uses `skip_serializing_if`: one shape, nulls explicit. `impl<'a> From<&'a Email> for MessageProjection<'a>` takes `PRESENTATION_CAP` items from each stored list; `links.total` is the stored vector's length and `attachments.total` is the stored list's own `total`. The dedicated structs are deliberate: deriving through `LinkMetadata`, `Attachment` or `IcsEvent` would publish `rel`, `sha256`, `stored` and `uid`, which the operator's ruling does not list.

The handler follows `get_message`: parse the query with `ViewQuery {}` under the label `GET /api/v1/messages/{id}/projection`, `state.store.get(&id)` or `404 "message not found"`, then `Json`. It runs on the async worker: the body is at most 500 short items (labels are bounded at ingest), with no parse and no blocking call.

### 3. The page

`console(title, pane, inbox, thread, rail)` writes: the doctype and head with the CSP meta and the one `<style>`; `<header id="status">` with the brand, the tagline `div` (the literal an existing test pins), the three listener chips, the two counters, `#chaos` and `#live`, each with an "unknown" placeholder the script replaces; `<main id="panes" data-pane="…">` with `<nav id="inboxes" class="pane">` (filter input, `#inbox-rows` with `data-limit`/`data-offset`, `#inbox-empty`, the pager, the rail foot), `<section id="stream" class="pane">` (header with `#stream-title`, `#stream-count`, `#clear-slot`, then `#view` with its two data attributes) and `<section id="reader" class="pane">` (`#reader-body`); a `noscript` note; the one `<script>`.

`rail(store, limit, offset, open)` renders each row as a link whose `href` is percent-encoded with the existing `NON_ALPHANUMERIC` set and whose visible name is `esc`aped, with the count, and `aria-current="true"` when the name equals `open`. `#inbox-empty` holds the server-chosen sentence and the `hidden` attribute when the page has rows. `index` passes its validated page; the inbox routes pass the default page and their inbox.

The logo is the prototype's inline `svg` without an `xmlns` attribute (inline SVG in HTML needs none, and the attribute's value would trip AC28).

### 4. The script

One file, in this order, keeping every identifier the existing tests pin (`view`, `inbox`, `threadKey`, `el`, `api`, `paint`, `paintSeq`, `load`, `feed`):

1. **Bindings and constants.** `TICK_MS = 2000`, `CACHE_MAX = 512`, `SEEN_MAX = 512`, the storage key.
2. **DOM helpers.** `el(tag, className, text)` stays the only way text enters the DOM. `chip(severity, text)` picks a class from a fixed table. `copyControl(text, what)` uses `navigator.clipboard.writeText` and reports a blocked copy.
3. **Formatting.** Relative time, clock time, sizes, the finding location by `kind`, and `shellSegment(name)`: `encodeURIComponent` plus percent-encoding of `!'()*`.
4. **Reads.** `readJson(path)` resolves to `{ status, body }` after parsing the body, and rejects only when the fetch or the parse fails. Every caller looks at `body` first and `status` second.
5. **Status.** `tick()` returns at once when `document.hidden` or when the previous tick has not settled; otherwise it reads `/healthz`, `/api/v1/chaos` and the inbox page, paints each independently (one failed read marks only its own part unknown or unavailable), and re-queues failed enrichment ids.
6. **Inboxes.** `lastSeen` (a `Map`; the name `seen` is taken by the paint guard) loaded from and saved to `sessionStorage` inside `try`; `paintRail(list)` rebuilds the rows, applies the filter and the dot rule, and toggles `#inbox-empty`.
7. **Enrichment.** `cache` (a `Map` from id to `{ projection, report }` states), `queue` (ids), and `pump()`, which takes one id, awaits its projection read and then its report read, stores the outcome, repaints that id's row and the reader if it is selected, and calls itself. `enrich(id, first)` adds an id unless it is cached or queued.
8. **Stream.** `paint(threads)` builds the header, the clear control, thread labels and rows from a frame; `load()` and the feed listener are today's, with the read-failure state added.
9. **Reader.** `select(id)` stores the id, sets `data-pane`, and calls `paintReader()`, which builds the envelope, the two panels and the tab strip from the selected mail and its cache entry.
10. **Boot.** If `inbox` is non-empty: `load()` and open the feed, wiring `onopen` and `onerror` to `#live`. Always: run `tick()` once and start the timer.

All list building uses `replaceChildren` and `appendChild`. No `.style`, no `href` except `'/ui/inbox/' + …` and `'/ui/message/' + …`.

### 5. The stylesheet

The prototype's rules, with these edits: the `:root[data-theme="dark"]` block and the `data-theme` selector are dropped (no toggle); the pane-switching rules are rewritten on `#panes[data-pane]` as R10.2 gives them, with the tablet breakpoint (`max-width: 1080px`) showing the reader in place of the two lists when `data-pane="reader"`; the demo-only rules (`.btn.primary` for Send, `.sample`, `.chip.chaos-on` as a pressed button) go or become static; the legacy classes `wrap`, `card`, `pill`, `mono`, `muted` and the `iframe` rule are defined on the same tokens so the message page follows the theme.

### 6. Records

Add the 0134 row to the index. Append to each of the four amended records one section, after its last line:

```markdown
## Dated pointer — 2026-11-16 (proposed 0134)

Proposed [0134](0134-ui-projection-route.md) §… amends §… of this record: …one sentence… . It is a proposal until the operator accepts it; this record's ruling stands as written until then.
```

### 7. Tests

- `tests/api.rs`: `/healthz` counters on a fresh server, after two mails, and under a cap of 1 (`start_with`), compared with `/metrics`; `last_received_ms` against the message list, across a second mail and a clear; the projection over real SMTP with a multipart mail (`SmtpConn` with a chosen `EHLO`), for a seeded mail, its exact key sets, its refusals, and the unchanged serializations; the OpenAPI additions.
- `src/api.rs` inline: the projection cap and unknown lists on a hand-built `Email` with an empty `raw`; the degraded `/healthz` body.
- `src/store.rs` inline: `inbox_summaries` after inserts, after a prune under a cap, and after a clear.
- `tests/ui.rs`: new tests only. A helper `console_pages(addr, inbox, key)` fetches the three console routes so each static assertion runs over all of them. The CSS assertions strip whitespace from the one `<style>` element and cut the `@media (max-width: 680px)` block by brace matching.

## Traps in the existing tests

The script and the stylesheet are now part of every console page, so assertions that used to look at a few hundred bytes of shell now look at all of it. These existing assertions constrain the new text; none of them is edited.

| Existing assertion | Constraint on the new page |
|---|---|
| `!page.contains("inbox view")` on `/ui/inbox/iv` | The words "inbox view" must not appear anywhere in the script, the stylesheet or the shell, comments included. |
| `!page.contains("is empty")` on an inbox route | No string or comment says "is empty". The empty stream reads "Empty and live." |
| `!page.contains("No inboxes yet")` on a non-empty store; `contains("No inboxes on this page.")` past the end | The server writes the sentence into `#inbox-empty`; the script holds neither. |
| `!page.contains(">next<")` on `/?limit=0` | No element or string in the page reads exactly `next`. |
| `contains("pg-b")` without `pg-a` or `pg-c` on `/?limit=1&offset=1` | The rail renders only the selected page. |
| `<div class="muted">Built for a program waiting on mail. ·` on `/` | The tagline `div` keeps that exact opening. |
| `!page.to_ascii_lowercase().contains("fastest")` | No such word in the page. |
| `the_executable_script`: one script element; `document.getElementById('view')`; `dataset.inbox`; `'/api/v1/inboxes/'`; `new EventSource(`; `textContent`; no `.innerHTML` | Keep the bindings as written. |
| `api('feed')`; `'threads/' + encodeURIComponent(threadKey)`; `paintSeq++`; `seen !== paintSeq` | Keep `load()`'s guard and its local name `seen`; the last-seen map is `lastSeen`. |
| Message page: `<title>A &amp; B · Swarmail</title>`, `<h2>A &amp; B</h2>`, `<span class="pill mono">555666</span>`, no script element | `page()` and `render_message` keep their markup. |
| Blank or control inbox is a JSON `400` | The handlers validate before they render anything. |

## What each ruling changes in the code

- Ruling 1 (`/healthz` counters): §1, AC5, AC6.
- Ruling 2 (narrow, bounded projection): §2, AC18–AC23.
- Ruling 3 (envelope from thread data): §4 step 9; the projection has no envelope member (AC20).
- Ruling 4 (navigation): §3 `rail` links; no router code in §4.
- Ruling 5 (`sessionStorage`): §4 step 6.
- Ruling 6 (serial queue): §4 step 7, AC11.
- Ruling 7 (no mail `href`): §4 closing rule, AC25.
- Ruling 8 (failure is not empty): §4 steps 4–5, AC10, AC13, AC15.

## Risks

| Risk | Mitigation |
|---|---|
| **Script behaviour has no gate.** A logic error in the dot, the queue or the reader passes every test. | Static assertions pin the wiring and the forbidden constructs (AC4–AC17, AC25). The design removes the hardest client state: inbox switching is navigation, enrichment is one serial chain, and every frame repaints from a full snapshot. The delivery lists the manual checks under spec § Not gate-provable and reports them as manual, never as passed gates (AC33). |
| **A page-wide `contains` assertion trips on new text.** | The table above lists every such assertion. Run `cargo test --locked --test ui` after each script or stylesheet edit. |
| **The projection contradicts three accepted rulings until 0134 is accepted.** | The operator's dated ruling directs the build; the pull request is held for acceptance; the accepted bodies are not edited, only pointed at. If the operator declines, the route, its types, its tests and the reader's client, anchor, source and attachment rows are removed, and the rest of the console stands. |
| **The number 0134 is taken twice elsewhere.** Branches `feat/arc-chains-256` and `feat/compare-v4-resources-359` each carry a different proposed 0134. | 0134 is the next free number on this branch, which is all a seat can see. The record says so in an integration note; the operator renumbers at integration, as with earlier dispatcher-assigned numbers. |
| **Coverage falls below 100%.** | New Rust is straight-line. The only data-dependent arms (unknown lists, the cap, the open-inbox marker, the empty sentence) each have a named test. No defensive branch is written that a test cannot reach. |
| **The console loads the server it watches.** A 200-row view asks for up to 400 enrichment reads, and the deliverability service has four analysis slots. | One request in flight (ruling 6), so the console holds at most one slot; cached reports cost no slot; nothing is fetched while the tab is hidden beyond the queue already running. The poll is three small reads every 2 seconds. |
| **A missed new-mail dot.** `received_ms` is stamped before the insert, so two mails stored out of stamp order can leave `last_received_ms` lower than a value already seen. | Accepted and stated: the dot is a hint, the count and the stream are the truth. Scanning each inbox for its maximum stamp would hold the map's shard lock for O(mails) on every poll, on the map the accept path writes. |
| **`sessionStorage` is unavailable or holds junk.** | Every access is inside `try`; anything but an array of `[string, number]` pairs is treated as empty; the page then works from memory. |
| **Hostile mail.** Markup, `javascript:` URLs, long lists, spoofing text. | `textContent` only; no mail `href`; bare `sandbox`; lists capped at 250 with totals; classes from a fixed table. Residual, stated: bidirectional or look-alike characters can still mislead a reader's eye. That is a limit of showing hostile text, not a DOM hole. |
| **Hostile inbox name in a copied snippet.** | The URL is single-quoted and the segment fully percent-encoded; the MCP snippet goes through `JSON.stringify` (R5.5). |
| **Clear inbox destroys evidence by a slip.** | Two clicks, the second on a different control; one request; no optimistic repaint. It is the script's only write and AC14 pins that. |
| **`/healthz` consumers.** Probes and scripts read the body. | The change is additive; status codes and `status` values are unchanged; the stopping body is byte-identical; the chart's probes read only the status code. |
| **Several console tabs.** Each holds one SSE connection, and browsers cap connections per origin. | Unchanged from today's inbox page: one `EventSource` per page, never one per inbox (AC9). |
| **`llms.txt` or contract-doc tests pin wording.** | Run `tests/store_contract_docs.rs` and `tests/agent_setup_docs.rs` with the documentation task, not at the end. |
| **ARC lands (#256) while this is open.** | The `arc` chip is one literal; whichever change merges second updates it. |
| **Page weight.** Every console route now inlines the whole script and stylesheet. | Tens of kilobytes, served from memory on a local tool. No external load is introduced to avoid it. |

## Manual review (the delivery's quickstart)

Start the candidate, open `/` in a browser, and with `swarmail` taking mail on its SMTP port:

1. Send mail to two inboxes. Both appear in the rail within a few seconds, with counts and "last mail" times; each shows a dot.
2. Open the first. Its dot clears; the second keeps its dot across the navigation. The stream is live: a further mail appears with the fresh marker and no reload.
3. Rows fill in their attachment and severity chips one at a time. Stop the server: the listener chips read unknown, the rail and stream say unavailable, and `#live` reads Reconnecting. Nothing reads as empty.
4. Open a mail with an HTML part, a link, a code, a PDF and a calendar part. Check the envelope, the client row, "Extracted at ingest", the authentication row and all four tabs. Copy a code and a snippet.
5. Set a chaos rule through the API: the chip reads "Chaos on" with the event; clear it: "Chaos off".
6. Arm "Clear inbox", choose "Keep": nothing is sent. Arm and choose "Clear": the stream empties from the server's answer and the inbox leaves the rail.
7. Narrow the window below 680px: one pane at a time, with the back controls. Switch the system theme: both themes render.
