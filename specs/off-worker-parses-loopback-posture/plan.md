# Plan — off-worker parses, bounded results, and the loopback posture

The implementation approach for
[spec.md](spec.md): a one-word visibility change in `src/store.rs`, moves
and swaps in five `src/` files, two contract documents, two config
documents and one new bench. Zero new `src/` modules, zero new
dependencies, zero new routes. Ground truth re-verified at `66a45bb` —
the positions' line numbers all hold (the state-home migration moved no
`src/` line).

## Technical context

- **The seam exists.** `Store::offload` (`src/store.rs:677-686`) is the
  proven `spawn_blocking` + explicit-match-`resume_unwind` shape — 0023
  §5's panic symmetry in code, with the comment explaining why the
  explicit `match` is load-bearing. It is currently private
  (`async fn offload`); the parse move is one visibility word plus two
  call-site wraps. `insert_async`/`delete_async`/`clear_async`/
  `clear_all_async` are its existing callers.
- **The two parse callers.** The DATA arm builds at `src/smtp.rs:1002`
  and parks the delivery *after* building (`park_delivery`, then
  `insert_async`); the seed path is `build_seed_email` →
  `src/api.rs:883`, whose builder ends in the ordinary `build_email`
  (`src/smtp.rs:1159-1199`). `build_email` is pure CPU — no `.await`
  inside — so both wraps are `Store::offload(move || …).await` with the
  inputs moved in. The chaos `Data` check precedes the build and is
  untouched: nothing stores on a fired rule, and the rule's own error
  line still precedes any parse.
- **The park's window.** `park_delivery(&email.inbox)` is registered
  before `insert_async` and its guard releases on resolve or unwind; its
  comment promises liveness "from the DATA terminator until the insert
  resolves". With the parse offloaded *before* the park, the window
  would exclude the parse — so the park moves up one statement, ahead of
  the offloaded parse, keeping the gauge's documented window truthful.
- **The `Arc` precedent.** `Email.raw` is `#[serde(skip)]`
  (`src/model.rs:52-53`); serde `rc` is enabled (`Cargo.toml:41`);
  `EmailList.emails: Vec<Arc<Email>>` serializes byte-identically with
  the pin at `src/api.rs:1020-1043`. The three remaining deep clones:
  `src/api.rs:692` (`.map(|e| (**e).clone())` in the await `Found` arm),
  `:773` (`get_message`'s `Json((*e).clone())`), `:903` (`seed`'s
  `Json((*stored).clone())`).
- **The cap precedent.** `DEFAULT_LIST_LIMIT` and `checked_wait_timeout_ms`
  (`src/api.rs:86-97`) are the 0018 §3/0025 shared-sentence shape: one
  const, one check, refusal naming the bound. The nine staging sites:
  `src/api.rs:638, 691` (REST list/await), `src/compat.rs:92-93,
  108-109, 212-213, 241-242` (the four compat pages' `take(params.limit
  .unwrap_or(50))`), `src/mcp.rs:268` (get_latest `take(count)`), `:235`
  (search limit), `:307` (wait `want`). MCP staging refusals are
  `-32602` error objects (the `refusal(field, domain)` formatter at
  `src/mcp.rs:262`); REST/compat refusals are `400` `ApiError` JSON.
- **The fold site.** `contains` (`src/compat.rs:365-383`) collects every
  haystack (no short-circuit — "so a miss looks like a hit", the compat
  envelope's counting contract) and its caller lowercases the needle
  once (`run_search`). `regex` is already a dependency (`Cargo.toml`,
  used by `extract.rs`); `Regex::new(&format!("(?i){}", regex::escape
  (needle)))` per request is one small alloc replacing two full-size
  per-mail ones. The `contains` unit pins live at `src/compat.rs:758-772`.
- **The bind literals.** `src/config.rs:33-42` — three `default_value`
  strings, pinned at `tests/config_cli.rs:55-57`. The chart maps knobs
  generically (`charts/swarmail/values.yaml:77` `config: {}` — the
  deployment template forwards every `config.SWARMAIL_*` entry); the
  Dockerfile's `ENTRYPOINT ["/swarmail", "serve"]` passes no binds, so
  the image inherits the binary default today.
- **The middleware slot.** `router()` already layers one middleware
  (`.layer(axum::middleware::from_fn(count_in_flight))`,
  `src/api.rs`); the Origin guard is a sibling
  `from_fn_with_state` layer. `serve()`'s caller has `http_addr`
  (`src/lib.rs:151`) and constructs `AppState` (`src/lib.rs:186-194`) —
  the guard's port rides in a new `AppState` field (`None` = inert for an
  off-loopback bind, `Some(port)` = enforce). The one inline `AppState`
  construction in the test module (`src/api.rs:1167-1180`) gains the
  field. The UI never calls `/mcp` (`src/static/ui.js` has no reference)
  and is same-origin loopback under the default.
- **The bench pattern.** `benches/writer_stall.rs` (`harness = false`)
  prints a before/after table from one process — record verbatim, rule
  as measured, no wall-clock validity. The new bench copies its shape
  (current-thread runtime, one store, explicit lens sections) and
  reconstructs the *old* shape inline where the tree no longer has it
  (deep-clone page, lowercase-fold search) so both legs measure the same
  data. Cargo.toml gains one `[[bench]]` stanza; benches are outside the
  coverage report exactly as the existing two are.

## Constitution check — frozen guarantees

- **0001 (accept == stored)** — the reply is still written only after
  `insert_async` resolves; the parse is an earlier, independent leg.
  Which thread *parsed* is not the guarantee's subject (0023 §1's own
  argument). The cap-sized accept leg re-proves `250` → immediately
  queryable on the moved path, over the real wire.
- **0002 (subscribe before check)** — `wait_for` is untouched; the wait
  surfaces' `count` cap refuses *before* `store.wait_for` is called (no
  waiter registered — 0025 §5 carried to the new refusal).
- **0003 (oldest-only pruning)** — no prune, evict or delivery-in-flight
  change; the park moves but its window only widens.
- **0005 (extraction at ingest)** — extraction still happens before the
  250 and never at query time; the thread it happens on was never the
  ruling's subject.
- **0014 (one ordered mutation boundary)** — the parse runs before the
  boundary and the boundary's body is untouched; a hostile body cannot
  hold the boundary lock through a regex pass.
- **0020 (test disciplines)** — no sleeps; the Origin and cap legs are
  deterministic request/response pairs; the bench asserts no wall-clock
  validity and records numbers verbatim; no new harness file is needed
  because no cold path is minted (the rejected `451` arm is the only
  candidate and it was refused — 0030 §6).
- **0022/0028** — `/raw` serves stored bytes, POP3 serves octets: no
  change to either path; the `raw`-never-on-JSON-wire property is
  extended, not introduced.
- **0024/0027** — shutdown, drain and listener liveness untouched; the
  new middleware answers from a constant map lookup, not a lock.
- **0029** — the cap sentence has exactly one owner (the shared check);
  every refusal precedes its effect; the MCP staging refusals reuse the
  existing `-32602` shape and the `refusal()` formatter.

## Project structure — files touched

| File | Change |
|---|---|
| `src/store.rs` | one word: `pub(crate)` on `offload` (0023's seam, now shared) |
| `src/smtp.rs` | DATA arm: park moves up; `build_email` wrapped in `Store::offload` |
| `src/api.rs` | `AwaitResponse.emails` → `Vec<Arc<Email>>`; `get_message`/`seed` serialize the `Arc`; `MAX_PAGE_SIZE` + the shared check; the nine staging sites' refusals; the Origin middleware + `AppState.http_origin_guard: Option<u16>` |
| `src/compat.rs` | `contains` folds via one `(?i)` regex; four page sites refuse over-cap |
| `src/mcp.rs` | three staging sites refuse over-cap (`get_latest` count, search limit, wait count) |
| `src/lib.rs` | computes the guard port from `http_addr`, constructs the new `AppState` field |
| `src/config.rs` | three `default_value` literals → `127.0.0.1:<port>` |
| `src/static/openapi.json` | `maximum: 250` beside every `limit` |
| `src/static/llms.txt` | the page-cap sentence |
| `README.md` | posture paragraph (binds per shape, the Origin rule, the webhook SSRF sentence), the page cap, README:80 truth |
| `Dockerfile` | three `ENV SWARMAIL_*_LISTEN=0.0.0.0:<port>` lines |
| `charts/swarmail/values.yaml` | three bind entries in `config:` |
| `Cargo.toml` | one `[[bench]] read_stall` stanza |
| `benches/read_stall.rs` | new: three lenses (await, compat search, fold) + the wake cost |
| `tests/{config_cli,api,compat,mcp,burst}.rs` | pins and wire legs (the driver map below) |

`src/store.rs` shows no other diff; `src/persist.rs` none.

## Contracts — the wire-visible deltas

- **Pages:** `limit`/`count` > 250 → REST/compat `400` with the shared
  sentence (`{field} must be at most 250`); MCP `-32602` (staging) /
  `isError` (wait) with the same words. 250 exactly → accepted, every
  plane. Defaults unchanged (100 / 50). `assert`'s `count`/`limit`
  behave exactly as today.
- **Search:** same matches as today on ASCII and realistic needles; the
  fold is simple case folding (0030 §4 names the margin).
- **POST /mcp and every HTTP route:** a present non-loopback `Origin`
  (loopback-bound server) → bare `403`; everything else identical.
- **Binds:** the binary defaults to `127.0.0.1:1025/1110/8025`; the
  image and the chart keep `0.0.0.0` explicitly; `SWARMAIL_*_LISTEN`
  still overrides.
- **Startup line:** unchanged in shape; the printed addresses show the
  loopback default when defaults are in force (the `smtp=` piped-parse
  contract of `4daa332` is untouched).

## Coverage — the driver map (exact 100/100/100)

Every new production line is warm under `cargo test`:

- **`offload` visibility + two wraps** — every mail-sending test
  executes them; `tests/burst.rs`'s cap-sized accept leg and the seed
  leg in `tests/api.rs` drive the large-input arms explicitly.
- **The park move** — the call site runs on every delivery; the
  in-flight gauge's existing sequencing tests stay green.
- **The three `Arc` swaps** — `tests/api.rs` legs for await/get/seed
  (response shape, no `raw` key), plus the extended inline
  byte-identical pins (`:1020-1043` pattern) for each response type.
- **The shared cap check** — inline unit pins (250 accepted, 251
  refused, the sentence's words); then one refusing wire leg *per
  staging site* so every call site's refusal arm is warm: REST list +
  await, four compat pages, MCP get_latest/search/wait — plus one
  boundary-accept leg per plane (the `Ok` arm) and the assert-inertness
  pin.
- **The Origin middleware** — `tests/api.rs`: absent → pass; loopback +
  bound port → pass; wrong port → `403`; non-loopback host → `403`;
  malformed Origin → `403`; off-loopback bind → inert (a `0.0.0.0:0`
  server passes an attacker Origin). All arms warm, all deterministic.
- **The fold** — the extended `contains` pins (case-insensitive across
  fields, raw searchable, non-candidates silent) + a wire search leg in
  `tests/compat.rs`.
- **The bind literals** — `tests/config_cli.rs:55-57` updated pins.
- **`AppState` field** — the inline test construction compiles it;
  `lib.rs`'s construction runs in every binary test.

## Risks

| Risk | Mitigation |
|---|---|
| The Origin guard breaks a legitimate client that sends a *cross-origin* loopback Origin (e.g. a dev-server origin on another port) | That is the correct answer — cross-origin is what the guard refuses; the client's fix is one documented sentence. The bridge, agents, curl and every suite send no `Origin` and are unaffected; the guard's inert-off-loopback-bind leg is pinned so the explicit posture stays available. |
| Hand-rolled Origin parsing mishandles garbage | The parser is a total function, fail-closed: any authority that is not a recognized loopback spelling of the bound port is a `403`, never a panic or a pass. Unit pins cover malformed/oversized/hostile inputs. |
| The parse move widens the panic surface silently | It does not: 0023 §5's `resume_unwind` re-raises into the awaiting session exactly as an inline panic would, and no new arm is minted (0030 §6). The existing offload panic-symmetry pins cover the shape. |
| A 50 MiB accept leg makes the suite slow or memory-hungry | The leg overrides `max_message_size` downward (the `tests/smtp_edges.rs:174` pattern) and drives the stall lens at a size that proves the point in milliseconds; the bench — not the suite — carries the 50 MiB lens. |
| The `count` cap collides with 0010's wait semantics | 0010 pinned `count=0` (at least one) — untouched. The cap only refuses > 250; a trigger above 250 is refused loudly (poll or assert instead), pinned on both planes. |
| Chart/values edits trip the helm gate | Values-only edit through the existing generic `config:` mapping; `scripts/chart-verify.sh` runs in CI and in the verify seat. |
| The exact gate sees a cold arm | No cold arm is minted: the refused `451` was the only candidate. The driver map above names the test for every new branch; a missed line is a test bug, fixed by a test — never `coverage(off)`. |
| The bench goes flaky | It copies `writer_stall.rs`'s discipline: record verbatim, rule as measured, assert no wall-clock validity, one process, current-thread runtime. |

## Quickstart — how the operator sees it

```console
$ cargo run -- serve            # 127.0.0.1:1025 / 1110 / 8025 — README:80 is true
$ curl -s -X POST localhost:8025/mcp -H 'Origin: http://evil.example' … → 403
$ curl -s 'localhost:8025/api/v1/inboxes/x?limit=251' → 400 {"error":"limit must be at most 250"}
$ docker run -p 1025:1025 … ghcr.io/…/swarmail   # ENV keeps 0.0.0.0 inside the container
$ cargo bench --bench read_stall                  # the three lenses, recorded verbatim
```
