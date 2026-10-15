# Tasks — off-worker parses, bounded results, and the loopback posture

Ordered per the framing: **#49 first** (the worker cost and the bounds),
then **#48** (the posture). Each task names the test that proves it; the
coverage gate (exact 100/100/100, missed == 0) is the arbiter of "proved".
Tests follow 0020's disciplines: no sleep-as-sync, fresh stores and
per-test inboxes, cold paths in `src/<module>/<name>_tests.rs`, seams
`#[cfg(test)]`-only, benches record verbatim and assert no wall-clock
validity.

## #49 — the workers, the copies, the caps

- [ ] 1. Widen `Store::offload` to `pub(crate)` and wrap the DATA arm's
  `build_email` (`src/smtp.rs:1002`) in it; wrap the seed path's
  `build_seed_email` (`src/api.rs:883`) likewise. Move `park_delivery`
  up so the in-flight gauge is live from the DATA terminator through the
  parse and the commit. No other `src/store.rs` diff; `src/persist.rs`
  untouched; the parse stays outside the mutation boundary.
  **Proved by:** `tests/burst.rs` — a cap-sized (shrunk
  `max_message_size`) DATA accept ends in `250 2.0.0 OK: stored as …`
  and is queryable immediately after (0001 re-proved on the moved
  path); the seed leg in `tests/api.rs` drives the second caller; every
  existing mail-sending test exercises the wraps warm.

- [ ] 2. Swap the three deep clones to `Arc` serializations:
  `AwaitResponse.emails: Vec<Arc<Email>>` (await `Found` arm,
  `src/api.rs:692`), `get_message` (`:773`), `seed` (`:903`).
  **Proved by:** the extended inline pins (`src/api.rs`, the
  `EmailList` `:1020-1043` pattern) — each response is byte-identical to
  the whole-clone shape and carries no `raw` key; `tests/api.rs` wire
  legs for await/get/seed; `/raw` and POP3 legs unchanged (0022/0028).

- [ ] 3. Add `MAX_PAGE_SIZE: usize = 250` and one shared check beside
  `DEFAULT_LIST_LIMIT` (`src/api.rs:86-97` shape), naming field and
  accepted range in one sentence; wire the refusal into REST `list`
  (`:638`) and `await` (`:691`).
  **Proved by:** inline unit pins (250 accepted, 251 refused, the exact
  words) + `tests/api.rs` legs: list and await refuse over-cap with `400`
  + the sentence, accept exactly 250, and register no waiter on a
  refused await (0025 §5).

- [ ] 4. Wire the cap into the four compat pages — Mailpit list/search,
  MailHog v2 list/search (`src/compat.rs:92-93, 108-109, 212-213,
  241-242`) — refusals riding `ApiError`'s `400` envelope.
  **Proved by:** `tests/compat.rs` — one refusing leg per page (four),
  plus a boundary-accept leg; the envelope's counts stay coherent.

- [ ] 5. Wire the cap into MCP staging: `get_latest`'s `count`
  (`src/mcp.rs:268`), `swarmail_search_emails`' `limit` (`:235`), and
  both wait surfaces' `count` (`:307`), refusals riding the existing
  `-32602` staging shape via `refusal(field, …)`.
  **Proved by:** `tests/mcp.rs` — three refusing legs (error objects
  name field and range), a boundary-accept leg, and the wait cap on both
  planes with 0010's `count=0` semantics pinned unchanged; the
  assert-inertness pin (assert's `count`/`limit` never refuse) closes
  the rule.

- [ ] 6. Replace `contains`' per-mail allocations with one `(?i)`
  escaped-regex fold per request (`src/compat.rs:365-383`); every
  candidate still collected, no short-circuit; the needle still
  lowercased at the call site is subsumed by the fold.
  **Proved by:** the extended `contains` pins (`src/compat.rs:758-772`
  shape) — case-insensitive hits across text/html/subject/to/cc/from/raw,
  non-candidates (links, codes) silent, garbage needles total; a wire
  search leg in `tests/compat.rs` across planes.

- [ ] 7. Add `benches/read_stall.rs` (`harness = false`,
  `writer_stall`-shaped) with three lenses: large-mail await occupancy
  (parse inline vs offloaded, ticker overshoot), large compat search
  (deep-clone page reconstructed inline vs `Arc` page), the fold
  (lowercase-alloc shape vs regex) — plus the per-wake view cost.
  Record numbers verbatim; no wall-clock validity assertions.
  **Proved by:** `cargo bench --bench read_stall` runs clean and prints
  the table; the recorded numbers land in the PR description for the
  operator to rule on as measured (0023's discipline).

## #48 — the posture

- [ ] 8. Flip the three bind defaults to loopback
  (`src/config.rs:33-42` → `127.0.0.1:1025/1110/8025`) and keep
  `SWARMAIL_*_LISTEN` overriding each.
  **Proved by:** `tests/config_cli.rs:55-57` pins updated + the
  override pin; the binary smoke legs (already loopback-bound) stay
  green; the startup line shows the loopback default in its printed
  addresses (`smtp=` piped-parse contract untouched).

- [ ] 9. Add the Origin middleware to the HTTP plane
  (`from_fn_with_state` beside `count_in_flight`): a present `Origin` is
  `403` unless its authority is a loopback spelling of the bound HTTP
  port, enforced only when the listener is loopback-bound; `AppState`
  gains `http_origin_guard: Option<u16>`, computed in `src/lib.rs` from
  `http_addr`.
  **Proved by:** `tests/api.rs` — absent → pass; loopback + bound port →
  pass; wrong port → `403`; non-loopback host → `403`; malformed →
  `403`; off-loopback bind (`0.0.0.0:0` server) → inert. All arms
  deterministic; the UI's same-origin loopback shape passes.

- [ ] 10. Keep the container's contract: the Dockerfile passes
  `ENV SWARMAIL_SMTP_LISTEN=0.0.0.0:1025`, `SWARMAIL_POP3_LISTEN=
  0.0.0.0:1110`, `SWARMAIL_HTTP_LISTEN=0.0.0.0:8025`; the chart passes
  the same three binds through `config:`.
  **Proved by:** `scripts/chart-verify.sh` (values-only edit, the port
  asserts untouched); the Dockerfile legs reviewed in CI's image build.

- [ ] 11. Make the README tell the truth: README:80 (loopback default),
  the env-knob section (:159-160) with both shapes' defaults, a posture
  paragraph (the Origin rule, the webhook blind-dial exposure documented
  with the per-target 0029 §5 correction, no auth and why), the page
  cap; `llms.txt` states the cap; `openapi.json` gains `maximum: 250`
  beside every `limit`.
  **Proved by:** the docs review leg; `cargo test` (the embedded static
  assets compile); no gate change.

- [ ] 12. Commit decision
  [0030](../../docs/decisions/0030-loopback-plane-bounded-results.md)
  (status `proposed`) with this spec; on operator acceptance, land the
  `Amended by` marker on 0018 §3 and the index row — a landing task,
  not now. Run all gates: `cargo fmt --all -- --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo test --workspace --locked`,
  `brokkr compile --bundle bundles/self`, `scripts/coverage-gate.sh`,
  `cargo deny check licenses`.
  **Proved by:** the gates' own output; `src/persist.rs` at zero diff
  and `src/store.rs` at a one-word diff confirmed by `git diff --stat`.
