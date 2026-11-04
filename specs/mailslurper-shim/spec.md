# MailSlurper service-API compatibility shim

Date: 2026-11-02. Phase: design. Issue #314, run
`issue-314-a-mailslurper-compatib-df2066a8`, from `main` `5645bd9`.
Recipe `claude-careful` (accepted 0098/0099); review residuals rework under
accepted [0102](../../docs/decisions/0102-review-residuals-rework.md).

**Revised 2026-11-02 (review return, run
`issue-314-a-mailslurper-compatib-0acb1d84`).** Review of head `439eba5`
found one spec defect. The page size was fixed at 25 from recollection, but
upstream hard-codes 50 (`cmd/mailslurper/controllers/ServiceController.go`
and libmailslurper `controllers/MailController.go`, `length := 50`, cited by
the guarantees member and recalled the same way by the independent member).
The consumer reads only page 1, so with 26–50 mails in the store the shim
dropped mail that real MailSlurper shows. R5, AC8, the rulings table and
0118 §§3 and 7 now say 50. The review's low and info findings are answered
in R11, AC15, AC17 and AC18. [plan.md](plan.md) § Review return maps each
finding to the change that answers it. The first design is commit
`dd6765c`; this revision does not rewrite it.

Decision: 0118 — [A MailSlurper service-API shim on an opt-in listener, one global mailbox](../../docs/decisions/0118-mailslurper-shim.md), proposed.

*Dated addendum of 2026-11-04 (issue #326, proposed): the item's `subject` is HTML-escaped; see the end of this spec and [the subject-escape spec](../mailslurper-subject-escape/spec.md).*

## Feature

Swarmail already emulates MailHog's and Mailpit's HTTP APIs
([compat shims](../../docs/guide/compat-shims.md)). It does not emulate
MailSlurper's service API. Suites that read and clear mail through
MailSlurper therefore cannot switch to Swarmail without code changes. The
concrete consumer is Alkemio's `@alkemio/tests-lib`
(`getVerificationLink()`, `deleteMailSlurperMails()`): it makes exactly two
calls against `MAIL_SLURPER_ENDPOINT`, and its nightly server-API run on
their test cluster is waiting on this shim.

This change adds a MailSlurper shim on its own opt-in listener
(`--mailslurper-listen`). It serves `GET /mail`, `GET /mail/{id}`,
`GET /mail/{id}/message`, `GET /mailcount` and `DELETE /mail`, over
Swarmail's whole store as one global mailbox, with documented known
differences and a wire-level test per route.

## Why

- **Drop-in for real suites.** Alkemio changes one environment variable
  and nothing else.
- **Fidelity where the client looks, honesty elsewhere.** The fields the
  consumer reads are pinned byte for byte. Fields Swarmail cannot produce
  without query-time MIME parsing (against 0005) are omitted and listed,
  not invented.
- **The operator chooses global semantics.** MailSlurper has one mailbox,
  so its delete wipes everything. Serving that only on a listener the
  operator enables keeps the HTTP plane's per-inbox posture (0004, 0011)
  intact on every other deployment.

## Rulings this spec carries (from the panel)

The design panel argued two positions: *simplicity* (routes at the root of
the HTTP port, no new listener) and *robustness* (a dedicated opt-in
listener with hardened inputs). The chief's rulings are listed below, and
[0118](../../docs/decisions/0118-mailslurper-shim.md) records why.

| Question | Ruling | Losing side's cost |
|---|---|---|
| Where served | **Dedicated listener, off by default** (robustness and the operator's preference). The HTTP port serves no MailSlurper path under any prefix. | Simplicity's zero-config chart story is lost. A cluster publishes 4437 with a Service of its own until a chart follow-up lands. |
| Listener guard | **The HTTP plane's Host/Origin guard on the new listener**, keyed to its own port (simplicity's DNS-rebinding point, adopted) | None |
| Chart, Dockerfile | **Untouched** (simplicity) | Robustness wanted the port exposed. That needs a `scripts/chart-verify.sh` change (a held path), so it becomes a follow-up issue. |
| `/healthz` degraded flag for the listener | **No.** The HTTP plane has none: axum's accept loop retries internally. | None |
| Mailbox | **Global view and global wipe** (both positions; operator framing) | Isolation is per Swarmail, not per inbox, for MailSlurper clients |
| `/mail/{id}/message` | **Ships**, marked *recalled* (the issue asks for it "if cheap", and it is one handler over the same `body`) | Simplicity's "never ship on recollection" is answered by labelling the claim and by 0118 §7 |
| `/messageraw`, attachments, `/pruneoptions`, `/version`, auth | **Not served** (both positions) | None |
| `contentType`, `isHtml`, `xmailer`, … | **Omitted** (both positions on most; a synthesized `contentType` was proposed by simplicity and rejected: it would disagree with upstream for every multipart mail) | A client that branches on `contentType` sees it absent |
| `dateSent` | **`YYYY-MM-DD HH:MM:SS` UTC** (simplicity's recollection of upstream) over RFC 3339 (robustness) | An RFC 3339 parser on the client would need the space-separated form |
| Page size | **50**, as upstream's source hard-codes it (review return 2026-11-02; this was 25 from recollection, the defect the review returned). Not configurable: no flag, env or `?limit=` (both positions on the return) | Fixed at upstream's value. A client wanting more pages uses `pageNumber`. |
| List order and `dateSent` source | **Receive time, UTC**, newest first by `(received_ms, id)`, kept and documented as a deliberate difference (both positions on the return). Upstream orders by `dateSent` DESC, which is the parsed `Date` header in the server's local zone. | A client sorting by `dateSent` sees a different order for a sender with a skewed clock. Fidelity on a field the consumer never reads loses to a deterministic "newest mail is the one just sent" for the link scan. |
| A page past the end | **`200` with the totals**, kept and documented as a *recalled* difference (both positions on the return). Upstream may answer `404 Page number out of range`. | A client that pages past the end does not get upstream's error. Building a recalled `404` is how the 25 crept in. If the recalled check is `pageNumber > totalPages`, it would also contradict the operator's observed empty-store `200`. |
| `ccAddresses` | **Kept**, documented as an extra key relative to upstream's item (simplicity on the return) | None for a JSON client. Removing it would churn the byte-exact item test for nothing. |
| `/metrics` claim (tests F2) | **Scoped prose, no behavioural test** (simplicity's refusal, with a different reason). The in-flight gauge is one process-global total that every concurrently running test moves, and no seam holds a request inside the listener's router. Any assertion on it is either vacuous or flaky. The guide names the gauge exactly instead. | The claim stays prose. Robustness wanted an assertion. |
| Off-worker build on the listener (tests F1) | **Proved directly** (both positions on the return): the existing held-page-build harness test also holds `GET /mail` on the listener | One more held site in an existing test |
| Wipe across restart | **Adopted** (robustness on the return): a wipe persists across a data-file restart, so a nightly run never scans a stale link | One more leg in the restart test |
| Store-read `500` seam, chaos count test | **Rejected.** Store reads are infallible in-memory walks, so a `500` arm would be dead code. The shim adds no ingest path, so the chaos count is already pinned where ingest lives. | None |
| Restart test | **Adopted** (robustness): an id survives a data-file restart | One more wire test |
| Age-based prune codes | **Refused by name** (both positions) | No age pruning |
| Compression | **Identity** (both positions) | None |

## Requirements

- **R1 — Opt-in listener.** `serve` takes `--mailslurper-listen <ADDR>` /
  `SWARMAIL_MAILSLURPER_LISTEN`, with no default. Unset means no socket.
  When set, the listener binds before any server task is spawned, after
  the data file is restored, and a bind failure fails startup.
  `RunningServer` exposes `mailslurper_addr: Option<SocketAddr>`. The
  listener serves only the R3 routes and drains at shutdown under the HTTP
  plane's drain deadline.
- **R2 — Guarded like the HTTP plane.** The listener is wrapped in the
  same `origin_guard` (0030 §2, 0043). On a loopback bind, a `Host` must
  be a loopback spelling of *the MailSlurper listener's* port, and a
  present `Origin` must be one too. Off loopback, a present `Origin` must
  match the `Host` authority.
- **R3 — Routes.** `GET /mail`, `GET /mail/{id}`, `GET /mail/{id}/message`,
  `GET /mailcount`, `DELETE /mail`, with the shapes of 0118 §§3–5. Every
  other path is a `404`. The HTTP plane's `/mail` and `/mailcount` stay
  `404`.
- **R4 — Global mailbox.** Reads span every inbox, newest first by
  `(received_ms, id)`. `pruneCode` `all` clears every inbox through
  `Store::clear_all_async`.
- **R5 — Paging.** `pageNumber` (1-based, default 1), page size **50**
  (upstream's hard-coded `length := 50`), `totalPages =
  ceil(totalRecords/50)`, `totalRecords` counted from the same snapshot as
  the page. An invalid `pageNumber` is a named `400`. A page past the end
  is a `200` with the totals and no items. `mailItems` is omitted whenever
  the page is empty.
- **R6 — Closed queries.** Every key except `pageNumber` on `GET /mail`,
  and every key on the other routes, is a `400` that names the key.
- **R7 — Projection.** Exactly the keys of 0118 §4, in that order, from
  ingest-time fields only.
  *(Amended for `subject` by the 2026-11-04 addendum below, proposed.)*
- **R8 — Bounded work.** List pages are built and serialized on the
  blocking pool into a buffer capped at `MAX_COMPAT_PAGE_BYTES`. An
  over-cap page is refused whole with a named `400`. Single-item routes
  render through `render_by_id`.
- **R9 — Destructive contract.** Only `Content-Type: application/json`,
  no query, and a body of exactly `{"pruneCode":"all"}` wipe. The age
  codes `60plus`, `30plus` and `2wksplus` are refused by name. Every other
  input is a `400` that deletes nothing. A failed commit is `503 storage
  commit failed`. Success is `200 text/plain; charset=utf-8` with the
  decimal count removed.
- **R10 — Identity encoding.** No `Content-Encoding` on any answer,
  whatever `Accept-Encoding` asks for.
- **R11 — Documentation.** `docs/guide/compat-shims.md` gains a
  MailSlurper section: the route table, the global-view sentence, the
  ingress/prefix note, the container and cluster recipe, the omissions, the
  known differences and the *recalled* items. The guide pages that list
  every listener or env knob name the new one. 0118 gets its index row.
  Every page that names the shims names three: the guide's title and intro,
  the README's feature and layer lines, and `docs/guide/repo-layout.md`.
  `src/static/llms.txt` no longer says `DELETE /api/v1/delete-all` is "the
  only cross-inbox wipe". It is the HTTP plane's only one, and the opt-in
  MailSlurper listener's `DELETE /mail {"pruneCode":"all"}` is the other
  (0118).
- **R12 — Unchanged elsewhere.** No new dependency or feature (both
  lockfiles and `deny.toml` untouched), no chart, Dockerfile, `.github/`,
  `scripts/` or `.brokkr/` change. Frozen guarantees 0001–0003 are
  unweakened. Coverage stays at the exact 100%.

## Acceptance Criteria

Wire tests speak real SMTP into the server and real HTTP/1.1 to the
MailSlurper listener (no mocks, 0004/0020). "Before/after count" means an
exact count over **both** seeded inboxes, taken through the native API.

1. **AC1 — Unchanged client recording.** Mail is sent over SMTP to two
   inboxes (one via AUTH, one unauthenticated). The recorded Alkemio
   request, `GET /mail` with `Accept: application/json` and
   `Accept-Encoding: gzip, deflate, br`, answers `200`,
   `Content-Type: application/json`, no `Content-Encoding`, and a body
   whose `mailItems` holds both mails newest first, `totalRecords` = 2.
   The verification link regex-matches in an HTML-only mail's `body`,
   including an `href`-only link with no visible text. `toAddresses`
   contains the recipient address as a bare string. Then the recorded
   `DELETE /mail` with `Content-Type: application/json` and
   `{"pruneCode":"all"}` answers `200`, `text/plain; charset=utf-8`, body
   `2`. A following `GET /mail` is exactly
   `{"totalPages":0,"totalRecords":0}`, and the native API shows both
   inboxes gone.
2. **AC2 — Off by default.** With the listener unset, `RunningServer`'s
   `mailslurper_addr` is `None`, and the HTTP plane answers `404` to
   `GET /mail`, `GET /mailcount` and `DELETE /mail` with an
   `{"pruneCode":"all"}` body. A before/after count proves nothing was
   deleted.
3. **AC3 — Flag and env.** `--mailslurper-listen` and
   `SWARMAIL_MAILSLURPER_LISTEN` parse into `Config::mailslurper_listen`,
   and the binary binds that address. An address that cannot bind fails
   startup with a non-zero exit before SMTP answers.
4. **AC4 — Guard.** On a loopback bind, a request whose `Host` names a
   different port, or a non-loopback host, is a `403`, and so is a
   foreign `Origin`. A `DELETE /mail {"pruneCode":"all"}` refused this way
   deletes nothing (before/after count).
5. **AC5 — Item shape.** `GET /mail/{id}` answers exactly the keys `id`,
   `dateSent`, `fromAddress`, `toAddresses`, `ccAddresses`, `subject`,
   `body`, `attachments`, in that order, and no others. Each is pinned:
   - `dateSent` equals the stored receive time formatted
     `YYYY-MM-DD HH:MM:SS` UTC.
   - `fromAddress` is the envelope sender.
   - `toAddresses` is the `RCPT TO` list, so a Bcc-only recipient is
     present.
   - `ccAddresses` is the header Cc list.
   - `body` is the stored HTML for an HTML-and-text mail, the stored text
     for a text-only mail, and `""` for a bodyless mail.
   - `attachments` is `[]`.
   An unknown id is a `404`.
   *(`subject` is amended by the 2026-11-04 addendum below, proposed.)*
6. **AC6 — Message route.** `GET /mail/{id}/message` answers
   `text/html; charset=utf-8` with exactly the item's `body`, and `404`
   for an unknown id.
7. **AC7 — Count.** `GET /mailcount` answers `{"mailCount":n}`, where n is
   the exact sum over both inboxes, and `{"mailCount":0}` when empty.
8. **AC8 — Paging.** With 51 mails over both inboxes, page 1 (no
   `pageNumber`, as the consumer sends it) holds exactly the 50 newest in
   `(received_ms, id)` order, and page 2 holds exactly the oldest one. Both
   carry `totalPages` 2 and `totalRecords` 51, and the two pages' items
   together are the 51 sent ids with none repeated. `pageNumber=3` is
   exactly `{"totalPages":2,"totalRecords":51}`. The literal page size 50
   is asserted once, so the test pins upstream's value and not the shim's
   own constant. `pageNumber` set to `0`, `-1` or `x` is a `400` naming
   `pageNumber`.
9. **AC9 — Closed queries.** `GET /mail?message=x` (and each of `from`,
   `to`, `start`, `end`, `orderby`, `dir`, and an unknown key) is a `400`
   naming the key. A query key on `/mailcount`, `/mail/{id}`,
   `/mail/{id}/message` and `DELETE /mail` is a `400`.
10. **AC10 — Destructive refusals.** Each of these is a `400`, and a
    before/after count over both inboxes is unchanged:
    - an absent body, an empty body, `{}`, `{"pruneCode":null}`,
      `{"pruneCode":7}`, `{"pruneCode":"ALL"}`, `{"pruneCode":"forever"}`,
      `{"pruneCode":"all","x":1}`, and a non-JSON body;
    - `{"pruneCode":"all"}` sent as `text/plain`, and
      `{"pruneCode":"all"}` with `?x=1`;
    - `60plus`, `30plus` and `2wksplus`, each answered with a reason that
      names the code.
11. **AC11 — Commit failure.** When the store's commit fails during the
    wipe, `DELETE /mail` answers `503` `storage commit failed`, never a
    count.
12. **AC12 — Page cap.** A page whose serialized JSON exceeds the byte
    budget is refused whole with the named `400`. The budget fits exactly
    and refuses the next byte (a unit test over the page builder with a
    supplied budget).
13. **AC13 — Accept == stored.** A `GET /mail` issued immediately after
    each SMTP `250`, with no wait or retry, counts that mail (0001).
14. **AC14 — Restart.** With a data file: send, stop, restart on the same
    file, and `GET /mail/{id}` returns the same item. Then wipe with
    `DELETE /mail {"pruneCode":"all"}`, stop, restart on the same file
    again, and `GET /mail` is exactly `{"totalPages":0,"totalRecords":0}`.
15. **AC15 — Docs.** `docs/guide/compat-shims.md` has a MailSlurper section
    with the global-view sentence, the cluster recipe and each known
    difference of 0118 §6. Among them: the page size is 50; `dateSent` is
    the receive time in UTC, not the `Date` header in local time; the list
    is ordered by receive time, not `dateSent`; a page past the end is a
    `200`; an unknown id is a `404`; `ccAddresses` is an extra key; the
    listener's requests are not in `swarmail_http_requests_in_flight`; and
    a list page over 64 MiB is a `400` that the consumer does not handle.
    The page title names all three shims. A docs-contract test pins the
    global-view sentence, the listener flag, the page size 50 and the title.
    The decision index lists 0118 as proposed, and `store_contract_docs`
    agrees. `src/static/llms.txt` scopes the "only cross-inbox wipe" claim
    to the HTTP plane and names the MailSlurper route. The existing
    `/llms.txt` byte-equality test keeps the served file equal to the
    source, and a docs-contract assertion pins the scoped wording. README
    and `docs/guide/repo-layout.md` name MailSlurper among the shims.
16. **AC16 — Gates.** Every gate of
    [house rules § Gates](../../docs/house-rules.md#gates--part-of-every-change)
    is clean, `--locked`. That includes the evals crate against the
    candidate's binary, the coverage gate at exact 100% with its selftest,
    the publish-guard and seal proofs, and `cargo deny check licenses`.
    `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are unchanged.

17. **AC17 — The listener's list build leaves the async worker.** While
    `GET /mail` on the MailSlurper listener is held inside the compat gate
    on the blocking pool, together with the HTTP plane's held page builds,
    a new SMTP peer is greeted, `/healthz` answers and no gate expires.
    Releasing the gate finishes the request with a `200`. This proves 0044
    §5 at the shim's own build site, not only through the shared
    `page_work` (tests F1).
18. **AC18 — The wire client fails by name.** Every raw HTTP exchange in
    `tests/mailslurper.rs` runs its connect and its read-to-EOF under a
    named deadline (issue #102's house helpers, or the same
    `HELPER_DEADLINE`). A wedged listener fails the test with the step's
    name instead of hanging the CI slot (tests F3).

## Out of scope

Implicit TLS for SMTP (its own issue). Chart-native exposure of the
listener (a follow-up). Search, sorting and date filters. Age-based
pruning. Attachments. Compression. Paid trials.

## Dated addendum — 2026-11-04: `subject` is HTML-escaped (proposed)

Issue #326. The text above is unchanged. This addendum is proposed; only
the operator accepts it, and until then R7 and AC5 stand as written (the
stored subject). The full requirement, acceptance criteria and the
evidence are in
[the subject-escape spec](../mailslurper-subject-escape/spec.md) and in
the dated addendum of
[0118](../../docs/decisions/0118-mailslurper-shim.md).

- **R7 / AC5, `subject`.** The item's `subject` is the stored subject (or
  `""`) with exactly Go's `html.EscapeString` substitutions, in one pass:
  `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` → `&#34;`, `'` → `&#39;`.
  It applies to `GET /mail` and `GET /mail/{id}`. `body`, the addresses,
  `GET /mail/{id}/message`, the native API, the Mailpit and MailHog shims,
  search and storage are unchanged.
- **Observed and inferred.** Only `"` → `&#34;` is observed (Alkemio
  trial run 41808342506). The other four are inferred from MailSlurper
  being Go; no seat has read upstream.
