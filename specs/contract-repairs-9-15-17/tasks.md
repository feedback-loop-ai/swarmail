# Tasks — contract repairs #9/#15/#17

Ordered. Each task is paired with the real-protocol test that proves it
(real servers via `tests/common`, never mocks). Implement executes 1-9 in
order; task 9 is verify-only. The exact 100/100/100 gate is the arbiter:
if an arm measures uncovered, the arm is dead or under-tested and the fix
is a test — `coverage(off)` is forbidden.

- [x] **1. Change the one hashed input in `group_threads`** — in
  `src/threads.rs:227`, hash the conversation's own first message instead
  of the union-find representative: `key: thread_key(&thread_node(emails[0]))`
  (the bucket is already sorted oldest-first at :221-225). Reword the
  module doc (:9-14), `Thread::key` (:31) and `thread_key` (:162-163):
  the anchor is the oldest mail's node — `Message-ID`, else
  `In-Reply-To`/first `References`, else the normalized-subject node,
  else the mail's own id. Zero diff to the union-find, the node naming
  and all grouping.
  *Test:* new unit regression in `src/threads.rs` —
  `the_thread_key_survives_a_reply_with_a_smaller_message_id`: root with
  `Message-ID z@x.io`, reply `<a@x.io>`/`In-Reply-To <z@x.io>`; assert
  `key(group([root])) == key(group([root, reply]))` (red on current
  code, where the representative flips to `id:a@x.io`). Proves AC1
  (the unit half) and AC2.
- [x] **2. Pin grouping as unchanged** — no code; the proof that pass 1
  and the `Components` union rules are untouched.
  *Test:* `a_reference_chain_is_one_thread_in_conversation_order`,
  `in_reply_to_alone_links_a_reply`,
  `subject_fallback_groups_chainless_mail`,
  `a_reference_only_links_when_the_parent_publishes_its_id`,
  `a_chainless_reply_on_the_same_subject_joins_the_chain`,
  `cyclic_references_collapse_into_one_thread`,
  `duplicate_message_ids_still_land_in_one_thread` pass **unmodified**.
  Proves AC3.
- [x] **3. Add the real-SMTP regression for the issue's exact scenario** —
  in `tests/ui.rs`, beside
  `thread_endpoints_group_a_references_chain_over_real_smtp` (:313):
  send `<z@x.io>` (inbox `keys`, subject "Stable"), record the key from
  `GET /api/v1/inboxes/keys/threads`, send `<a@x.io>` with
  `In-Reply-To: <z@x.io>`, assert the key is byte-identical and
  `count == 2`.
  *Test:* the regression itself, plus
  `thread_view_is_a_live_shell_for_one_conversation` (:238) staying green
  (the UI route addresses threads by the same key). Proves AC1.
- [x] **4. Write the migration note** — README *Threads and the live UI*
  (:308-316): the key is the hash of the conversation's first message's
  anchor, so it cannot change as replies arrive; pre-existing mail re-keys
  once on the next request (no data migration — threads are recomputed per
  request); the residual re-key conditions (oldest mail leaving the
  newest-200 window, deletion/cap-prune of the oldest, equal-received_ms
  id tie-break). One matching sentence in the `src/threads.rs` module doc.
  *Test:* the sentence is exercised by 1 and 3; nothing else to run —
  but `the_view_caps_at_the_newest_view_cap_emails` (:492) stays green,
  proving the window semantics did not move. Proves AC4.
- [x] **5. Flip the shim DELETE contract** — in `src/compat.rs`
  (:257-272): delete the `None => state.store.clear_all()` fallback.
  `Err(JsonRejection)` → `400` via
  `ApiError(StatusCode::BAD_REQUEST, "deletion requires a non-empty
  \"ids\" list".into())`; `Ok(Json(IdsBody))` with an empty `ids` → the
  same `400`; otherwise delete exactly `body.ids` through
  `state.store.delete` and reply `{"removed": n}`. Return
  `ApiResult<Json<Value>>` (`ApiError` is already imported). Reword the
  `IdsBody` (:71) and handler doc comments that endorse the wipe.
  *Test:* the rewritten `shim_deletes_go_through_the_store`
  (`tests/compat.rs:606`): no body → `400` + both seeded inboxes still
  list exactly their 4 mails (before/after, not just the status);
  empty body → `400` + nothing deleted; `not json at all` → `400` +
  nothing deleted; `{"ids": null}` and a wrong-typed `ids` → `400`
  (axum folds them into `JsonRejection`); `{"ids": []}` → `400` +
  nothing deleted. Proves AC5-AC8.
- [x] **6. Pin the valid case and the sole wipe** — no further production
  change; the same rewritten test keeps the previously valid path and
  0004's posture honest.
  *Test:* in `shim_deletes_go_through_the_store`: `{"ids": [a1, b2]}` →
  `200 {"removed": 2}`, `a2`/`b1` answer `200` on
  `GET /api/v1/messages/{id}`, `a1`/`b2` answer `404`; then
  `DELETE /api/v1/delete-all` (no body) → `200 {"removed": 4}` — the
  alias is unchanged and remains the only cross-inbox wipe. Proves
  AC9/AC10.
- [x] **7. Sweep the shim docs and draft decision 0011** — README API row
  (:293), the `delete` operation in `src/static/openapi.json` (:141-145),
  the shim line in `src/static/llms.txt` (:92-94), and new
  `docs/decisions/0011-shim-delete-contract.md` with status `proposed`:
  the contract, the five previously-destructive inputs, the
  per-test-isolation rationale (tying to #6's false-pass hazard), the
  rejected alternatives (wipe-behind-a-flag, inbox scope, silent
  `{"removed": 0}`) and the one-line migration for wipe-on-garbage
  clients.
  *Test:* `machine_docs_are_served_from_the_binary` (`tests/binary.rs`)
  stays green — the served OpenAPI must match the new contract text.
  Proves AC11/AC13.
- [x] **8. Flatten the DATA chaos arm and document the one policy** — in
  `src/smtp.rs` (:439-450), remove the `if rule.error.is_some()` nesting:
  a fired DATA rule sleeps, answers `rule.error.unwrap_or("451 4.3.0
  chaos")` and `continue`s — mail is never stored. One doc sentence on
  `ChaosRule::error` in `src/chaos.rs` (`None` = the stage's default
  failure). README *Chaos* section: the four-bullet delay-only table plus
  the pinned semantics (over-limit `552 5.3.4` precedes the DATA chaos
  check; the DATA delay applies after the body is received, before the
  reply).
  *Test:* rewrite the delay-only DATA section of
  `chaos_on_connect_mailfrom_and_data_with_and_without_error`
  (`tests/smtp_edges.rs:359-374`) — reply is exactly `451 4.3.0 chaos`
  and `GET /api/v1/inboxes/default/count` stays `0` (it currently
  endorses the bug by asserting the mail is stored); the custom-error
  DATA section (:375-395) keeps `554 rejected` with the count still `0`.
  Proves AC14 (data row)/AC16 (data)/AC17 (data).
- [x] **9. Pin the other three stages' delay-only behavior** — no further
  production change (connect/mail_from/rcpt already implement the table);
  tests only, in `tests/smtp_edges.rs`.
  *Test:* add delay-only MAIL FROM and RCPT sections to
  `chaos_on_connect_mailfrom_and_data_with_and_without_error` — each
  asserts the reply is exactly `451 4.3.0 chaos` (probability 100,
  `delay_ms: 5`, no sleeps); rewrite
  `connect_chaos_with_delay_only_still_drops_the_session` (:399) to
  assert the session yields **no line at all** —
  `timeout(1s, c.reply()) == ""` — neither a banner nor a `451`. The
  existing custom-error sections (connect `451 go away`, mail_from
  `450 busy`) keep pinning AC17. Proves AC14/AC15/AC16/AC17.
- [x] **10. Run the gates** — `cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `brokkr compile --bundle .`, `bash scripts/coverage-gate.sh`,
  `cargo deny check licenses`. The diff must show `src/store.rs`,
  `src/persist.rs`, `src/model.rs`, `src/extract.rs`, `src/api.rs`,
  `src/mcp.rs` and the 250 sequence untouched.
  *Test:* the gates themselves — exact 100/100/100, missed == 0.
  Proves the wave-level gate criterion and AC18.
