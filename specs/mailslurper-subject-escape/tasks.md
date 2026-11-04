# Tasks — the MailSlurper shim escapes `subject`

Each task is paired with the test that proves it. Order matters: T1 before
T2–T4; T8 last.

- [x] **T1 — The escape type.** Add the private `GoHtmlEscaped` `Display`
  type (plan § Approach) to the MailSlurper section of `src/compat.rs`,
  with a doc comment that names the observed `"` and the inferred
  `& < > '`. Do not reuse or change `ui::esc`. *Proof:* AC1's inline table
  test (each character, the mixed string, `&amp;` → `&amp;amp;`,
  `&#34;` → `&amp;#34;`, `""`, `plain`, `Grüße ✓`), with literal expected
  values.
- [x] **T2 — Wire it into the item.** `SlurperView.subject` becomes
  `GoHtmlEscaped<'a>`, serialized through `collect_str`. No owned subject
  `String` anywhere. Keep `a_slurper_item_has_exactly_its_keys_in_order`
  green unchanged, and add a no-subject `Email` asserting `"subject":""`.
  *Proof:* AC1's `None` case, and the existing key-order test.
- [x] **T3 — The cap bounds the escape.** Add the failing-writer sweep (a
  `fmt::Write` with a byte budget that refuses a chunk whole, driven over
  `GoHtmlEscaped("a\"b")` for `n` in `0..7`). Add a page budget that ends
  inside the escaped subject. *Proof:* AC5. Every `?` in the escape takes
  its error arm, and `slurper_page_body` returns `OverBudget` at the cut,
  while the exact budget still fits.
- [x] **T4 — The wire pins.** In `tests/mailslurper.rs`, add the AC2/AC3
  test: one SMTP mail; the listener's `/mail` and `/mail/{id}` subject equal
  to the literal escaped string; `body` and `/mail/{id}/message`
  byte-identical to the stored body; native, Mailpit item and MailHog v2
  item subjects raw; Mailpit `kind=subject` search hits on `"f"` and misses
  on `&#34;f`. Edit `the_item_shape_is_pinned_field_by_field` to the
  literal `Welcome &lt;b&gt;aboard&lt;/b&gt;` (AC4). *Proof:* those tests,
  against a real server.
- [x] **T5 — The fence.** Confirm that the production diff is confined to
  the MailSlurper section of `src/compat.rs`. Leave `Cargo.toml`,
  `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` untouched. *Proof:*
  AC6: `git diff 66462c6 --stat -- src Cargo.toml Cargo.lock evals/Cargo.lock deny.toml`
  lists only `src/compat.rs`, and review reads the hunk.
- [x] **T6 — Records beside the decision.** In `docs/decisions/README.md`,
  append to 0118's notes cell: "dated addendum 2026-11-04 (proposed, issue
  #326): the MailSlurper `subject` is HTML-escaped". Leave the status
  cell `accepted`. In `specs/mailslurper-shim/spec.md`, add a dated
  2026-11-04 addendum section at the end, a pointer line under the header
  and one under R7 and AC5. Insertions only, linking
  `../mailslurper-subject-escape/spec.md` and 0118's addendum. *Proof:*
  `tests/store_contract_docs.rs`
  (`every_index_row_matches_its_records_status`,
  `every_decision_file_has_exactly_one_resolving_index_row`) stays green.
  `git diff 66462c6 --numstat -- specs/mailslurper-shim docs/decisions/0118-mailslurper-shim.md`
  shows 0 deleted lines.
- [x] **T7 — The guide.** In `docs/guide/compat-shims.md`, the item
  paragraph says `subject` is HTML-escaped as MailSlurper serves it. Add a
  Known-differences bullet, written exactly as
  `*`subject` is HTML-escaped, `body` is not*`. It says: `"` → `&#34;` is
  observed; `&amp;`, `&lt;`, `&gt;` and `&#39;` are inferred from Go's
  `html.EscapeString`; the native API, Mailpit and MailHog keep the raw
  subject; escaped bytes count toward the 64 MiB page. Move `subject` out
  of the "from the operator's description" list only if the text stays
  true. *Proof:* add that bullet's heading as a needle in
  `the_guide_states_the_global_view_and_every_known_difference` (AC7).
- [ ] **T8 — Every gate, locked.** Run the full inventory of
  [house-rules § Gates](../../docs/house-rules.md#gates--part-of-every-change):
  fmt, clippy `-D warnings`, `cargo test --workspace --locked`,
  `cargo build --locked --bin swarmail`, the evals crate's fmt, clippy and
  tests against that binary, every recipe compile (reading each output for
  the policy-sweep warning), `scripts/coverage-gate.sh` at exact 100% and
  its `selftest`, `scripts/publish-guard-test.sh`,
  `scripts/seal-branch-test.sh`, and `cargo deny check licenses`. Record no
  load proof; none is needed. *Proof:* AC8, each command's exit status in
  the verify receipt. Commit unsigned, never push. The PR body's first line
  carries AC9's hold sentence.
