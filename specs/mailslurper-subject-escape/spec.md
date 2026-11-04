# The MailSlurper shim escapes `subject` as MailSlurper does

Date: 2026-11-04. Phase: design. Issue #326, run
`issue-326-the-mailslurper-shim-s-b72a01a4`, recipe `claude-careful`, from
`main` `66462c6`. Operator direction 2026-11-04: "file it and launch".

**Authority.** Accepted
[0118](../../docs/decisions/0118-mailslurper-shim.md) §4 (the item
projection), as amended by its **dated addendum of 2026-11-04, which is
proposed**: only the operator accepts it. This spec takes no new decision
number. Accepted [0005](../../docs/decisions/0005-extraction-at-ingest.md)
(extraction at ingest) and [0044](../../docs/decisions/0044-compat-validation.md)
§6 (a byte cap that refuses the whole page) bind the design. 0001–0003 are
not touched. The original shim spec is
[specs/mailslurper-shim/spec.md](../mailslurper-shim/spec.md); it gets a
dated addendum that points here (task T6), and its accepted text is not
rewritten.

## Feature

On the MailSlurper listener only, the `subject` of every `MailItem` is
served HTML-escaped with the substitution set of Go's `html.EscapeString`:

| Stored | Served |
|---|---|
| `&` | `&amp;` |
| `<` | `&lt;` |
| `>` | `&gt;` |
| `"` | `&#34;` |
| `'` | `&#39;` |

Every other character passes through unchanged. The substitution is
unconditional and single-pass: each stored character is replaced at most
once, so `&` is effectively first and no entity is re-escaped by a later
substitution. Text that already looks like an entity is escaped again, as
Go does (`&amp;` → `&amp;amp;`, `&#34;` → `&amp;#34;`).

Both routes that serve a `MailItem` carry it: each `mailItems` entry of
`GET /mail`, and `GET /mail/{id}`. Nothing else changes.

## Why

The first full Alkemio trial against Swarmail v0.3.0
(alkem-io/test-suites run 41808342506, `smtp-mode=plain`) ran 160 of 162
test files green. All five failures had one cause: the test compares
`mail.subject` exactly with a string such as
`… New Form response to &#34;<name>&#34;`, and the shim served a plain `"`.
The same tests pass against MailSlurper (run 41744902684), so MailSlurper's
service API serves the subject HTML-escaped.

**Observed versus inferred.** Only `"` → `&#34;` is observed. The spelling
`&#34;` (not `&quot;`) is Go's `html.EscapeString`, which escapes exactly
the five characters above; the other four substitutions are inferred from
MailSlurper being Go. No seat has read upstream (0118 §7's verification gap
stands). The records say so; nothing claims the four as observed. The
operator's re-run of the Alkemio trial is the acceptance evidence.

## Rulings this spec carries (chief, over the panel)

- **The escape is a view, not storage.** It runs while the MailSlurper item
  is serialized. `Email`, the store, ingest, persistence, search and filter
  matching keep the sender's subject. Escaping at ingest would break the
  native API, Mailpit, MailHog and `kind=subject` search, and would need a
  migration that double-escapes across a restart. Both positions agree.
- **The escape streams into the capped writer; nothing is allocated ahead
  of the cap.** Robustness showed that an eager escape (`String` built
  before serialization) can turn a ~50 MiB subject into a ~250 MiB
  transient that the 64 MiB page cap never sees. Simplicity's five-call
  `replace` chain has that defect, and it is rejected. Robustness's own
  remedy (an exact escaped-length pre-check, a `Cow` and a second refusal
  path) is also rejected: the escape is written through serde_json's
  `collect_str` as a `Display` value, so it is written in chunks straight
  into the existing `CappedBuffer`. The existing refusal is the only one,
  and the page cap bounds memory again. What simplicity loses: a ~15-line
  `Display` impl in place of a 5-line function, plus one writer-failure
  test.
- **Not `ui::esc`.** It emits `&quot;`, and it is the web UI's XSS guard
  (#56). The MailSlurper escape is a separate private item in
  `src/compat.rs`.
- **Fences: the narrow set, each pinned once.** The issue's acceptance says
  "every other shim and the native API" keep the raw subject, and the task
  says search and filter matching stay unchanged. So the wire test checks
  the native API, Mailpit and MailHog, plus one Mailpit `kind=subject`
  search. All of these are assertions in one test, not separate tests.
  Simplicity's cut to a single other shim is overruled by the acceptance
  text. Robustness's restart leg is rejected. The view is recomputed on
  each request from a stored value this change does not touch, and
  `an_id_survives_a_data_file_restart` already covers persistence; a second
  server lifecycle proves nothing new.
- **No by-id cap.** `GET /mail/{id}` was uncapped before this change and
  stays uncapped. The escape can make that response's subject up to 5×
  the stored bytes, and the response is all it allocates. The addendum
  names this; adding a cap would be a separate contract change.
- **No flag, no dependency, no other field.** `body`, `fromAddress`,
  `toAddresses` and `ccAddresses` are not escaped. 0118 keeps `body`
  unescaped on purpose, and every body-reading test in the trial passed.

## Requirements

- **S1 — Escape set.** The served `subject` is the stored subject (or `""`)
  with exactly the five substitutions above, unconditional, each stored
  character replaced at most once.
- **S2 — Both item routes.** `GET /mail` (every item) and `GET /mail/{id}`
  serve the same escaped value for the same mail.
- **S3 — Body and addresses unchanged.** `body`, `fromAddress`,
  `toAddresses`, `ccAddresses` and `GET /mail/{id}/message` serve the same
  bytes as before this change.
- **S4 — Every other surface raw.** The native API, the Mailpit and MailHog
  shims, search and filter matching, and storage serve or match the stored
  subject unchanged.
- **S5 — The cap still bounds memory.** The escaped subject is written into
  the page's capped buffer as it is produced. No owned copy of the subject
  (escaped or not) is built before the capped writer. A page whose
  escaped bytes cross `MAX_COMPAT_PAGE_BYTES` is refused whole with the
  existing named `400`.
- **S6 — Records.** A dated 2026-11-04 addendum to 0118, proposed, with a
  pointer at §4. A dated addendum to `specs/mailslurper-shim/spec.md` with
  a pointer at R7/AC5. The guide states the escape, and keeps observed and
  inferred apart. No new decision number. No accepted text is rewritten.

## Acceptance Criteria

1. **AC1 — Escape table (unit).** In `src/compat.rs`'s inline tests, the
   escape maps each of `&`, `<`, `>`, `"` and `'` alone to its entity. The
   mixed `a&b<c>d"e'f` maps to `a&amp;b&lt;c&gt;d&#34;e&#39;f`. `&amp;` maps
   to `&amp;amp;` and `&#34;` to `&amp;#34;`. `""`, `plain` and the
   multibyte `Grüße ✓` are unchanged. A `MailItem` for an `Email` with no
   subject carries `"subject":""`. Every expected value is a literal,
   never computed by the escape under test.
2. **AC2 — Both routes on the wire.** `tests/mailslurper.rs` sends one mail
   over real SMTP with the subject `New Form response to "f" & <b>'x'</b>`
   and a text body containing `"q" & <i>'y'</i>`. It reads the listener at
   once after the `250`, with no polling. `GET /mail`'s `mailItems[0].subject`
   and `GET /mail/{id}`'s `subject` both equal the literal
   `New Form response to &#34;f&#34; &amp; &lt;b&gt;&#39;x&#39;&lt;/b&gt;`.
   The item's `body` and `GET /mail/{id}/message` equal the stored body
   byte for byte.
3. **AC3 — Raw everywhere else (wire, same test, same mail).** The native
   `GET /api/v1/messages/{id}` `subject`, Mailpit's
   `GET /api/v1/message/{id}` `Subject`, and MailHog's `GET /api/v2/messages`
   item `Content.Headers.Subject[0]` each equal the raw subject
   `New Form response to "f" & <b>'x'</b>`. Mailpit's
   `GET /api/v1/search?kind=subject` with the raw fragment `"f"` finds the
   mail. With the escaped fragment `&#34;f` it finds nothing.
4. **AC4 — The field-by-field pin.**
   `the_item_shape_is_pinned_field_by_field` expects the subject as the
   literal `Welcome &lt;b&gt;aboard&lt;/b&gt;`, no longer as
   `stored["subject"]`. Every other field of that pin is unchanged.
5. **AC5 — The cap bounds the escape (unit).** For a page holding one mail
   whose subject contains escapable characters,
   `slurper_page_body` fits the exact budget. Given a budget that ends
   inside the escaped subject's bytes, it returns `OverBudget`.
   Separately, formatting the escape into a `fmt::Write` that fails after
   `n` bytes returns `Err` for every `n` from 0 to one less than the
   escaped length, and never writes more than `n` bytes. This covers the
   error arm of every write the escape makes. The subject is serialized
   through `collect_str` (or an equivalent streaming writer), and
   `SlurperView` holds no owned `String` subject; review checks this.
6. **AC6 — The diff is fenced.** The production diff is confined to the
   MailSlurper section of `src/compat.rs`. `Email`, `store`, ingest,
   persistence, `haystacks`, `Filter`, the Mailpit, MailHog and native
   handlers, `slurper_body` and `ui::esc` are unchanged. `Cargo.toml`,
   `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are unchanged.
7. **AC7 — Records.** `docs/decisions/0118-mailslurper-shim.md` carries the
   2026-11-04 addendum and a one-line pointer at the end of §4. The
   addendum states that it is proposed, in prose. The record keeps exactly
   one Status header, `accepted`, and none of its earlier lines are deleted
   or changed. The addendum states that only `"` is observed and the other
   four are inferred. The decision index keeps 0118's status cell
   `accepted`; its notes cell names the proposed addendum.
   `specs/mailslurper-shim/spec.md` gets a dated addendum and pointers,
   with zero deleted lines. `docs/guide/compat-shims.md` states the escape
   in the item paragraph and in Known differences, observed and inferred
   apart. `the_guide_states_the_global_view_and_every_known_difference`
   pins that sentence with a new needle. `tests/store_contract_docs.rs`
   stays green.
8. **AC8 — Gates.** Every gate in
   [house-rules § Gates](../../docs/house-rules.md#gates--part-of-every-change)
   is clean, run with `--locked`. That includes the evals crate against the
   candidate's own `swarmail`, coverage at exact 100% with its selftest,
   the publish-guard and seal proofs, `cargo deny check licenses` and every
   recipe compile. No load proof is needed or applied.
9. **AC9 — The hold.** The PR body's first line states that the PR changes
   the wire form fixed by accepted 0118 §4 ("the stored subject"), and that
   it is held under accepted 0045 §8 until the operator accepts the
   2026-11-04 addendum. Green gates do not release it. The Alkemio re-run
   (the issue's fourth acceptance box) is the operator's evidence. This run
   does not perform it and does not claim it.

## Out of scope

The Mailpit and MailHog shims, the native API, search and filter
semantics, storage and ingest, the `body` and address fields, a by-id
response cap, a configuration switch, the decision index status, and
reading upstream MailSlurper source.
