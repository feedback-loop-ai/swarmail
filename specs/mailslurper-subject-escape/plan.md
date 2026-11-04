# Plan — the MailSlurper shim escapes `subject`

Spec: [spec.md](spec.md). Authority: accepted
[0118](../../docs/decisions/0118-mailslurper-shim.md) §4 and its proposed
2026-11-04 addendum.

## Technical context

- One crate, Rust 2024. The touched production code is the MailSlurper
  section of `src/compat.rs` (`SlurperView`, `SlurperItem::serialize`).
- `SlurperItem` is the only producer of a `MailItem`. `GET /mail` reaches
  it through `slurper_page_body` → `serialize_capped` (a `CappedBuffer` of
  `MAX_COMPAT_PAGE_BYTES`, 64 MiB, on the blocking pool). `GET /mail/{id}`
  reaches it through `render_by_id` → `serde_json::to_vec`, uncapped.
- serde_json 1.0.151 (already locked) implements `Serializer::collect_str`
  with an adapter that escapes each `fmt::Write::write_str` chunk as JSON
  and writes it straight to the underlying `io::Write`. A writer error
  comes back as a serde error, and `serialize_capped` already maps any
  error to `OverBudget`. Checked in the registry source,
  `serde_json-1.0.151/src/ser.rs`, `collect_str`.
- The ingest cap is 50 MiB (`src/smtp.rs` `MAX_MESSAGE_SIZE`), so one
  subject is bounded by it.

## Constitution check

| Rule | Holds because |
|---|---|
| 0001 accept == stored | No write path is added. The escape reads a committed `Email`. |
| 0002 await subscribes first | Untouched. |
| 0003 oldest-only pruning | Untouched. A page is cut from an immutable `Arc<Email>` snapshot. |
| 0005 extraction at ingest | No MIME is parsed at request time. A fixed transform of one stored scalar is presentation, like `dateSent`'s format. |
| 0044 §6 / 0118 §3 byte cap | The escape streams into the capped buffer (S5), so the cap bounds memory again. The refusal is still whole, still named. |
| 0045 §8 code under a proposal | The change contradicts accepted 0118 §4's "the stored subject" until the operator accepts the addendum. The PR is held (AC9). |
| 0020 test placement | Every new test is inline in `src/compat.rs` or a public-API wire test in `tests/mailslurper.rs`. No cold path, no harness file, no seam. |
| Coverage 100% exact | Every arm of the escape is executed by AC1. The error arm of every write is executed by AC5's failing-writer sweep. |
| Dependencies | None added. |

## Project structure (files touched)

| File | Change |
|---|---|
| `src/compat.rs` | A private `Display` type for the Go escape. `SlurperView.subject` holds it and serializes through `collect_str`. Inline tests (AC1, AC5). |
| `tests/mailslurper.rs` | A new wire test (AC2, AC3). Edit the field-by-field pin (AC4). A new guide needle (AC7). |
| `docs/decisions/0118-mailslurper-shim.md` | The §4 pointer and the 2026-11-04 addendum, written in the design commit. |
| `docs/decisions/README.md` | The 0118 row's notes cell names the proposed addendum. Status cell unchanged. |
| `specs/mailslurper-shim/spec.md` | Dated addendum and pointers at the header, R7 and AC5. Insertions only. |
| `docs/guide/compat-shims.md` | The item paragraph and a Known-differences bullet. |

## Approach

**The escape.** A private newtype over `&str` that implements
`fmt::Display` by scanning for the next of the five characters, writing
the clean run before it, then its entity, and finally the tail. Each
write is propagated with `?`. A sketch (the implementer may restructure it,
but must keep the single pass and the streaming):

```rust
/// `subject` as MailSlurper serves it: Go's `html.EscapeString` set,
/// one pass (0118 §4, 2026-11-04 addendum, proposed). Only `"` is
/// observed (Alkemio run 41808342506); `& < > '` are inferred from Go.
struct GoHtmlEscaped<'a>(&'a str);

impl fmt::Display for GoHtmlEscaped<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut rest = self.0;
        while let Some(at) = rest.find(['&', '<', '>', '"', '\'']) {
            f.write_str(&rest[..at])?;
            f.write_str(match rest.as_bytes()[at] {
                b'&' => "&amp;",
                b'<' => "&lt;",
                b'>' => "&gt;",
                b'"' => "&#34;",
                _ => "&#39;",
            })?;
            rest = &rest[at + 1..];
        }
        f.write_str(rest)
    }
}

impl Serialize for GoHtmlEscaped<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}
```

`SlurperView.subject: GoHtmlEscaped<'a>`, built from
`email.subject.as_deref().unwrap_or_default()`. The five targets are ASCII,
so a byte index taken from `find` is always a char boundary and slicing
cannot panic. No owned `String` of the subject exists on either route. On
`GET /mail` the escaped bytes go straight into `CappedBuffer`. On
`GET /mail/{id}` they go into the response `Vec`, which was uncapped before
this change and is unchanged in kind.

**Why not the alternatives** (spec § Rulings):

- The `replace` chain (simplicity): five whole-string copies, built before
  the cap. With a 50 MiB subject the transient is far past the 64 MiB page
  budget.
- The pre-check with `Cow` and an escaped-length helper (robustness): it
  bounds the page, but it adds a second refusal path, a length function
  that must agree with the escape, and their branches, all to restore a
  bound that streaming gives for free.
- `ui::esc`: wrong entity (`&quot;`), and it belongs to the UI's XSS guard.

**Tests.** As AC1–AC5 and AC7. The failing writer for AC5 is a small
inline `fmt::Write` with a byte budget. Like `CappedBuffer`, it refuses a
chunk whole when the chunk would cross the budget. It is driven with
`write!(w, "{}", GoHtmlEscaped("a\"b"))` for each budget `n` in `0..7`:
the prefix write fails at `n = 0`, the entity write at `1..=5`, the tail
write at `6`. The page-level half of AC5 extends
`the_slurper_page_budget_fits_exactly_and_refuses_the_next_byte`, or sits
beside it. That test gives the mail a subject such as `"q"`, finds the
offset of `&#34;` in the exact page, and checks that a budget ending two
bytes into that entity refuses.

**Records.** The design commit writes the 0118 addendum and the §4
pointer. The implementer writes the index notes, the shim-spec addendum
and the guide (T6–T7). They are separate commits so that the design commit
touches exactly one `specs/` directory. A design commit that touched
`specs/mailslurper-shim/` would also fail the design check, because that
spec declares `Decision: 0118`, which is now accepted.

## Research

- Go's `html.EscapeString` is `strings.NewReplacer("&", "&amp;", "'",
  "&#39;", "<", "&lt;", ">", "&gt;", `"`, "&#34;")`. That is the five
  characters, numeric entities for both quotes, and a single pass. This is
  the panel's recollection of the Go standard library, consistent with the
  observed `&#34;`. Upstream MailSlurper was not read.
- Whether upstream escapes at ingest or at render does not matter here.
  The wire form is the contract, and the view reproduces it.

## Data model

Unchanged. `Email.subject` stays the sender's decoded subject.

## Contracts

The MailSlurper `MailItem.subject` changes from "the stored subject" to
"the stored subject through Go's `html.EscapeString` set". Every other
field and every other surface is unchanged.

## Quickstart

```bash
swarmail serve --mailslurper-listen 127.0.0.1:4437 &
# send Subject: a "b" & <c>
curl -s 127.0.0.1:4437/mail | jq -r '.mailItems[0].subject'
# a &#34;b&#34; &amp; &lt;c&gt;
```

## Risks

- **The four inferred entities are wrong** (upstream escapes only `"`, or
  uses other spellings). *Mitigation:* the addendum and the guide mark them
  as inferred. The operator's Alkemio re-run checks the observed one. A
  correction is a one-line change to the table and a new dated addendum.
- **A consumer relied on raw subjects from this listener.** *Mitigation:*
  the only known consumer, Alkemio, compares with the escaped form, as real
  MailSlurper serves it. The guide states the change.
- **A page that used to fit is now refused.** A subject made mostly of
  escapable bytes grows up to 5×, so it reaches 64 MiB from about 13 MiB of
  stored subject. *Mitigation:* the refusal stays whole and by name, and
  points at `GET /mail/{id}`. The guide's Known differences says escaped
  bytes count. Truncating would corrupt the field the change exists to fix.
- **By-id response growth.** `GET /mail/{id}` is uncapped, as before. Its
  subject can grow up to 5× (≤ ~250 MiB for a pathological 50 MiB
  subject). *Mitigation:* streaming means no transient copy is allocated
  beside the response. The addendum names the residual. A by-id cap would
  be its own contract change. Review's low security residual (2026-11-04)
  was answered with a measurement: a control character already costs six
  JSON bytes on both routes, so a 50 MiB message could already reach about
  300 MiB by id and a page was already refused from about 10.7 MiB; the
  escape's five times is below both and raises no ceiling. The addendum
  records the measurement.
- **A shortcut implementation reintroduces the eager copy** (for example a
  `String` field, or `to_string()` on the escape). *Mitigation:* AC5's
  review check, plus the page test that refuses inside the subject. A test
  cannot observe peak memory, so review owns the structural half.
- **The design check or the docs tests trip on the records.** *Mitigation:*
  the addendum adds no `Status:` header line (`file_status` and the design
  check both count them). The design commit touches only
  `specs/mailslurper-subject-escape/` among spec directories. The index
  status cell is unchanged.
- **Merged on green against an accepted ruling.** *Mitigation:* AC9's
  first-line hold in the PR body, and the addendum's own text. Only the
  operator accepts.
