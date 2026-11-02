# Plan — MailSlurper service-API shim

Spec: [spec.md](spec.md). Decision:
[0116](../../docs/decisions/0116-mailslurper-shim.md) (proposed).

## Technical context

- A single Rust crate on axum 0.8, tokio, serde and chrono (all already
  dependencies). No new crate, no new feature, and both lockfiles and
  `deny.toml` stay untouched.
- The store's global, newest-first read already exists:
  `Store::list_all` → `compat::scoped(store, None)`. The global wipe exists
  too: `Store::clear_all_async`, used by `compat::delete_all` with 0014's
  `503` mapping.
- Bounded page work exists: `compat::page_work`, `serialize_capped` and
  `MAX_COMPAT_PAGE_BYTES`. Single-mail renders go through
  `compat::render_by_id`.
- The Host/Origin guard is `api::origin_guard`, keyed by
  `AppState::http_origin_guard: Option<u16>`.
- Commit failure is injected in wire tests with `common::refusing_trigger`
  (real SQLite, see `tests/compat.rs`
  `a_blocked_delete_refuses_the_shim_deletes_with_503`).

## Constitution check

- **0001 accept == stored.** The shim only reads committed inboxes, and
  AC13 reads immediately after each `250`. Pass.
- **0002 await subscribes first.** The wipe goes through `clear_all_async`,
  so emptied inboxes are released to watchers as with `delete-all`, and no
  new path touches the store. Pass.
- **0003 oldest-only.** Age codes are refused by name, and the cap is
  untouched. Pass.
- **0005 ingest extraction.** The projection reads stored fields only, and
  there is no `mail_parser` call in the new code. Pass.
- **0011.** Amended narrowly by proposed 0116 (a second spelling of the one
  wipe, on an opt-in listener). The PR is held for acceptance (0045 §8).
- **0020.** Unit tests stay inline in `src/compat.rs`. Wire shapes go in
  `tests/mailslurper.rs`. No new cold path, so no `_tests.rs` harness is
  required.
- **Coverage.** Every new arm is executed by a unit or wire test, listed
  in tasks.md. No `coverage(off)`.
- **Held paths.** None touched apart from the decision itself, which stays
  `proposed`.

## Project structure (files touched)

| File | Change |
|---|---|
| `src/config.rs` | `ServeArgs::mailslurper_listen: Option<String>` (`--mailslurper-listen`, env `SWARMAIL_MAILSLURPER_LISTEN`, value name `ADDR`, a doc comment stating off-by-default and the global mailbox); `Config::mailslurper_listen: Option<String>`; `From<&ServeArgs>` |
| `src/lib.rs` | After the POP3 bind, bind the listener when the address is set, then read its `local_addr`. Spawn its serve task only when `Some`, with an `AppState` clone whose `http_origin_guard` is the MailSlurper port when the bind is loopback. Add `RunningServer::mailslurper_addr: Option<SocketAddr>`. Update the inline test's `Config` literal. |
| `src/api.rs` | Extract `serve`'s graceful-plus-deadline race into `pub(crate) async fn serve_app(listener, app: Router, stop_rx, shutdown)`; `serve` calls it with `router(state)`. Make `origin_guard` `pub(crate)`. No behaviour change on the HTTP plane. |
| `src/compat.rs` | A MailSlurper section (detail below) with inline unit tests. The module doc names the third shim. |
| `Config` literals: `tests/common/mod.rs`, `tests/listen.rs`, `tests/persistence.rs`, `tests/stdio.rs`, `tests/starttls.rs`, `benches/read_stall.rs`, `src/compat/page_work_tests.rs`, `src/deliverability/concurrency_tests.rs`, `src/store/persist_tests.rs`, `src/webhook/ca_construction_tests.rs`, `src/lifecycle_tests.rs` | Add `mailslurper_listen: None`. `common` gains a helper that starts a server with the listener on `127.0.0.1:0`. |
| `tests/mailslurper.rs` (new) | The wire suite for AC1–AC11 and AC13–AC14, over a raw HTTP/1.1 client in the style of `tests/compat.rs`'s `http_raw`, so request headers are sent exactly as recorded |
| `tests/config_cli.rs`, `tests/binary.rs` | AC3: the flag and env parse, the binary binds the address, and an unbindable address fails startup |
| `docs/guide/compat-shims.md` | The title and intro cover three shims. A MailSlurper section (0116 §§1–6): the route table, the global-view sentence in bold, the ingress-prefix note, local, Docker and cluster recipes (chart `config` env plus a deployer Service on 4437, with the NetworkPolicy caveat), the omissions, the known differences (including "not counted in `/metrics`") and the *recalled* list. |
| `docs/guide/quickstart.md`, `docs/guide/network-posture.md` | Name `SWARMAIL_MAILSLURPER_LISTEN` (no default, off) wherever the listeners and env knobs are listed |
| `docs/decisions/README.md` | The 0116 row, `proposed` |
| docs-contract tests (`tests/store_contract_docs.rs` or `tests/compat.rs`) | Pin the MailSlurper section (AC15) |

Not touched: `Cargo.toml`, `Cargo.lock`, `evals/`, `deny.toml`, `charts/`,
`Dockerfile`, `.github/`, `scripts/`, `.brokkr/`, and `openapi.json` and
`llms.txt` (both describe the HTTP plane, which does not change).

## The `src/compat.rs` section

- `pub fn mailslurper_router(state: AppState) -> Router`: the routes
  `/mail` (GET list, DELETE wipe), `/mailcount`, `/mail/{id}` and
  `/mail/{id}/message`, layered with
  `from_fn_with_state(state.clone(), api::origin_guard)`, then
  `.with_state(state)`. There is no `count_in_flight` layer: `/metrics`
  reports the HTTP plane.
- `const SLURPER_PAGE_SIZE: usize = 25;`
- A closed list query: `pageNumber` only (`deny_unknown_fields`, read as a
  string and parsed by hand, so that `0`, `-1` and `x` share one named
  refusal).
- `struct SlurperItem<'a>(&'a Email)`, with a hand-written `Serialize` in
  0116 §4's key order.
- The page envelope: `mailItems` (skipped when empty), `totalPages`,
  `totalRecords`. `fn slurper_page_body(page, total, budget)` uses
  `serialize_capped`.
- `fn slurper_date(email)` uses `DateTime::from_timestamp_millis` and
  `%Y-%m-%d %H:%M:%S`, falling back to `received_at` verbatim on `None`.
- `fn slurper_body(email)` returns the HTML, else the text, else `""`.
- `PruneBody` holds the `pruneCode` value, with `deny_unknown_fields`.
  `fn prune_verdict(code: &Value) -> Result<(), ApiError>`.

## Approach details

1. **List.** Stage the query first, so a refusal costs no scan. Then
   `page_work`: `scoped(store, None)`, `total = all.len()`, and
   `start = (page - 1) * 25` with `checked_mul` (an overflow is past the
   end). `page_of` cuts the page, and `slurper_page_body` serializes it
   under `MAX_COMPAT_PAGE_BYTES`. Bytes go to `json_body`. Over budget is
   a `400`:
   `mail page exceeds byte budget {MAX}; fetch each mail by GET /mail/{id}`.
2. **Empty-page omission.** Pin the exact bytes
   `{"totalPages":0,"totalRecords":0}`.
3. **Count.** `mailbox_size(store, None)` inline (a sum of lengths with no
   render), answered as `{"mailCount": n}`. The empty query goes through
   `parse_query::<ViewQuery>` on `RawQuery`, as `delete_all` does.
4. **One item, its message.** Both go through `render_by_id`. The item is
   `json_body(serde_json::to_vec(&SlurperItem(e)))`. The message is
   `text/html; charset=utf-8` with `slurper_body(e)`. Both have closed,
   empty queries.
5. **Delete.** The checks run in this order:
   - the query (`ViewQuery`): a `400` naming the key;
   - the body (`Result<Json<PruneBody>, JsonRejection>`): a rejection is a
     `400` reading `deletion requires the JSON body {"pruneCode":"all"}`;
   - `prune_verdict`;
   - `clear_all_async`, which maps `Ok(n)` to
     `200 text/plain; charset=utf-8` with `n`, and `Err` to
     `503 storage commit failed`.

   The verdicts:
   - `"all"`: proceed.
   - `"60plus" | "30plus" | "2wksplus"`:
     `prune code "<c>" is not supported: swarmail evicts only the oldest mail past a per-inbox cap (decision 0003); send "all"`.
   - any other string: `unknown prune code "<c>"; send "all"`.
   - a non-string or `null`: `pruneCode must be the string "all"`.
6. **Projection.** `toAddresses` is `email.recipients` in order when that
   list is non-empty, otherwise the `email.to` addresses. `ccAddresses` is
   the `email.cc` addresses. `fromAddress` is
   `compat_sender(email).unwrap_or("")`: reuse it, never re-derive it.
7. **Startup wiring.** The bind sits with the other binds, before any
   `tasks.spawn`. The serve task uses the same `shutdown_rx` and drain
   deadline through `api::serve_app`, so `RunningServer::stop` joins it.

## Research notes

- **Verified, binding:** the consumer's two calls and headers, the fields
  `mailItems`, `totalRecords`, `subject`, `body` and `toAddresses`, and the
  omission of `mailItems` when empty (the operator's description).
- **Recalled:** everything listed in 0116 §7. No seat in this run has web
  access, so no claim is presented as read from upstream source.

## Data model

Unchanged: no new stored field, no schema change, no persistence change.

## Contracts

0116 §§3–5 and spec R3–R10. There is no OpenAPI entry: the listener is a
separate plane, and its contract is the guide's MailSlurper section.

## Quickstart

```bash
swarmail serve --mailslurper-listen 127.0.0.1:4437
MAIL_SLURPER_ENDPOINT=http://localhost:4437/mail npm test     # the consumer, unchanged
# container: -e SWARMAIL_MAILSLURPER_LISTEN=0.0.0.0:4437 -p 4437:4437
```

## Risks

| Risk | Mitigation |
|---|---|
| **Recalled upstream shapes are wrong** (page size, `dateSent`, count body, age-code names) | The consumer-read fields are pinned from the operator's description, not from recall. Every recalled item is labelled in 0116 §7 and the guide. The operator checks them before accepting. A consumer-field mismatch blocks acceptance, and any other mismatch is a dated addendum. |
| **A global wipe on a shared Swarmail destroys parallel sessions' mail** | The listener is off by default, so only an operator enables it. Only the literal `{"pruneCode":"all"}` wipes. Every AC10 refusal carries a before/after count. The guide states the global mailbox in bold. |
| **DNS rebinding reaches the wipe on a loopback bind** | The same `origin_guard`, keyed to the MailSlurper port (AC4) |
| **The cluster cannot reach 4437 through the chart** | The guide gives the deployer-Service recipe and the NetworkPolicy caveat. Chart support is a follow-up named in 0116. The nightly needs only the env and one Service. |
| **A large page stalls the async worker or exhausts memory** | The build and serialization run on the blocking pool, under a byte cap that refuses the whole page by name (AC12) |
| **A refused filter breaks a client that sends one** | A named `400` beats a silent wrong match. Alkemio sends none, and the guide lists the refused keys. |
| **Every `Config` literal must change** | The compiler finds every literal, and the plan lists the known ones. `clippy --all-targets` catches the benches. |
| **Coverage regresses on the optional-listener arms** | Every existing test runs the `None` arm, and `tests/mailslurper.rs` runs `Some`. The date fallback has an `i64::MIN` unit test. Each prune-verdict arm has a unit test as well as its wire refusal. |
| **The PR cannot merge before 0116 is accepted** | Stated in 0116's consequences. The PR is held for the operator: expected, not a failure. |
| **Content-type strictness refuses a client that omits `application/json`** | Alkemio's client sends JSON with the JSON content type. Refusing is 0011's rule for a destructive route, and the guide documents it. |
