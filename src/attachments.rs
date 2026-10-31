//! Attachments listed at ingest (decision 0110, accepted 0005): every
//! candidate part of the outer message is recorded once — label, decoded
//! size, SHA-256, whether its bytes are kept — and a calendar part's key
//! fields are read from its first `VEVENT`. Fetch, list and the ICS fields
//! are then served from what was stored; nothing re-parses `raw` at query
//! time.
//!
//! The walk is total. Nothing here returns an error, so no failure can sit
//! between the DATA terminator and the `250` (decision 0001): the work is
//! linear in the bytes the parser already decoded, and what it retains is
//! bounded by the first four constants below. The fifth bounds what the list
//! answers.

use crate::model::{Attachment, AttachmentList, IcsEvent};
use mail_parser::{Message, MimeHeaders, PartType};

/// Entries listed (and considered for storage) per mail. The exact candidate
/// total is recorded beside them, so truncation is never over-reported.
pub const MAX_ATTACHMENTS: usize = 250;

/// Decoded bytes retained per mail, summed over the stored parts; also the
/// largest fetchable part. Sized from the stdio bridge's 10 MiB response cap
/// (decision 0110 §3): a result carries its JSON twice and a worst-case text
/// payload escapes to four times its size inside the text block, so one
/// fetch answers in about 6 MiB at most.
pub const MAX_ATTACHMENT_STORED_BYTES: usize = 1 << 20;

/// Every stored string — filename, content type and each ICS field — is cut
/// to this many bytes at a character boundary.
pub const MAX_LABEL_BYTES: usize = 1024;

/// The largest calendar part the ICS scanner reads; a larger one yields no
/// fields.
pub const MAX_ICS_BYTES: usize = 1 << 20;

/// The entries of one list answer, as compact JSON; entries past it are cut
/// and `truncated` says so. The stored labels of 250 entries can escape to
/// 25 MiB of modern answer, far over the stdio bridge's 10 MiB response cap.
/// JSON at most doubles inside the text block (a `"` or `\` is the worst
/// case) and rides once more as `structuredContent`, so an answer is at most
/// three times this: 9 MiB, leaving the envelope and the HTTP head room
/// (decision 0110 §3). 250 entries of 1024-byte labels that need no escaping
/// are about 2.1 MiB, so only labels spelled to escape are ever cut.
pub const MAX_LIST_JSON_BYTES: usize = 3 << 20;

/// Lowercase-hex SHA-256 (decision 0110 §5). The digest comes from the ring
/// provider rustls already links, through the public hash provider of the
/// TLS 1.3 `AES_128_GCM_SHA256` suite — that suite's transcript hash *is*
/// SHA-256 — so no crate enters the tree. The known-answer tests below pin
/// that it is SHA-256; a hand-rolled digest would be a security regression.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let suite = rustls::crypto::ring::cipher_suite::TLS13_AES_128_GCM_SHA256
        .tls13()
        .expect("the TLS 1.3 suite is a TLS 1.3 suite");
    let digest = suite.common.hash_provider.hash(bytes);
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(64);
    for &byte in digest.as_ref() {
        hex.push(char::from(DIGITS[usize::from(byte >> 4)]));
        hex.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    hex
}

/// A stored label: at most [`MAX_LABEL_BYTES`], cut at a character boundary.
pub fn cap_label(label: &str) -> String {
    let mut end = label.len().min(MAX_LABEL_BYTES);
    while !label.is_char_boundary(end) {
        end -= 1;
    }
    label[..end].to_string()
}

/// `type/subtype`, ASCII-lowercased, or the bare type when the header has no
/// subtype; `None` when the part has no content type.
fn content_type(part: &mail_parser::MessagePart<'_>) -> Option<String> {
    let header = part.content_type()?;
    let mut label = header.ctype().to_ascii_lowercase();
    if let Some(subtype) = header.subtype() {
        label.push('/');
        label.push_str(&subtype.to_ascii_lowercase());
    }
    Some(cap_label(&label))
}

/// Whether the part is calendar-shaped: `text/calendar`, `application/ics`
/// or a filename ending `.ics`, ASCII case-insensitively.
fn calendar_shaped(part: &mail_parser::MessagePart<'_>) -> bool {
    part.is_content_type("text", "calendar")
        || part.is_content_type("application", "ics")
        || part.attachment_name().is_some_and(|name| {
            name.len() >= 4 && name.as_bytes()[name.len() - 4..].eq_ignore_ascii_case(b".ics")
        })
}

/// Walk the outer message once (decision 0110 §1). A candidate is a leaf
/// part in the parser's attachment list or any calendar-shaped leaf — a bare
/// `text/calendar` body is a real invite shape that never reaches the
/// attachment list. An attached message is one entry; its inner parts are
/// not walked. The first [`MAX_ATTACHMENTS`] candidates are hashed and
/// scanned and the first-fit budget decides which keep their bytes; beyond
/// them only the count grows.
pub fn extract(message: &Message<'_>) -> AttachmentList {
    // Sized to cover every listed id, so a list that named a part the
    // parser did not keep could never index out of range.
    let size = message
        .attachments
        .iter()
        .map(|&id| id as usize + 1)
        .fold(message.parts.len(), usize::max);
    let mut listed = vec![false; size];
    for &id in &message.attachments {
        listed[id as usize] = true;
    }
    let mut list = AttachmentList::default();
    let mut remaining = MAX_ATTACHMENT_STORED_BYTES;
    for (id, part) in message.parts.iter().enumerate() {
        let calendar = calendar_shaped(part);
        // Bitwise, so the test is one decision: a container part is never a
        // candidate, whatever name it carries.
        let candidate = (listed[id] | calendar) & !matches!(part.body, PartType::Multipart(_));
        if !candidate {
            continue;
        }
        list.total += 1;
        if list.parts.len() == MAX_ATTACHMENTS {
            continue;
        }
        let bytes = part.contents();
        let stored = bytes.len() <= remaining;
        if stored {
            remaining -= bytes.len();
        }
        list.parts.push(Attachment {
            filename: part.attachment_name().map(cap_label),
            content_type: content_type(part),
            size: bytes.len() as u64,
            sha256: sha256_hex(bytes),
            stored,
            ics: if calendar { scan_ics(bytes) } else { None },
            content: stored.then(|| bytes.to_vec()),
        });
    }
    list
}

/// The first `VEVENT`'s key fields (decision 0110 §4), or `None` when the
/// content is not UTF-8, is over [`MAX_ICS_BYTES`] or holds no `VEVENT`.
///
/// After RFC 5545 unfolding the scan starts at the first `BEGIN:VEVENT`,
/// reads properties only at depth one — every nested `BEGIN:` raises the
/// depth and its `END:` lowers it, so a `VTIMEZONE`'s `DTSTART` and a
/// `VALARM`'s `SUMMARY` are never the event's — and stops at `END:VEVENT`.
/// The first occurrence of a property wins. Total on any input.
pub fn scan_ics(content: &[u8]) -> Option<IcsEvent> {
    if content.len() > MAX_ICS_BYTES {
        return None;
    }
    let text = unfold(std::str::from_utf8(content).ok()?);
    let mut lines = text.split('\n');
    lines.find(|line| {
        line.trim_end_matches('\r')
            .eq_ignore_ascii_case("BEGIN:VEVENT")
    })?;
    let mut event = IcsEvent::default();
    let mut depth = 1usize;
    for line in lines {
        let line = line.trim_end_matches('\r');
        if strip_prefix_ci(line, "BEGIN:").is_some() {
            depth += 1;
        } else if let Some(name) = strip_prefix_ci(line, "END:") {
            if name.eq_ignore_ascii_case("VEVENT") {
                break;
            }
            if depth > 1 {
                depth -= 1;
            }
        } else if depth == 1 {
            property(line, &mut event);
        }
    }
    Some(event)
}

/// RFC 5545 unfolding: CRLF to LF, then every `LF SP` and `LF HT` removed.
fn unfold(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(at) = rest.find('\n') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        rest = if after.starts_with([' ', '\t']) {
            &after[1..]
        } else {
            out.push('\n');
            after
        };
    }
    out.push_str(rest);
    out
}

fn strip_prefix_ci<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let head = line.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &line[prefix.len()..])
}

/// The first `target` byte outside a double-quoted parameter value.
fn unquoted(text: &str, target: u8) -> Option<usize> {
    let mut quoted = false;
    for (at, byte) in text.bytes().enumerate() {
        if byte == b'"' {
            quoted = !quoted;
        } else if byte == target && !quoted {
            return Some(at);
        }
    }
    None
}

/// One property line at depth one: `NAME[;PARAM=value]…:value`, the value
/// starting after the first `:` outside a quoted parameter value.
fn property(line: &str, event: &mut IcsEvent) {
    let Some(colon) = unquoted(line, b':') else {
        return;
    };
    let (head, value) = (&line[..colon], &line[colon + 1..]);
    let (name, params) = match unquoted(head, b';') {
        Some(semicolon) => (&head[..semicolon], &head[semicolon + 1..]),
        None => (head, ""),
    };
    let (field, tzid) = if name.eq_ignore_ascii_case("DTSTART") {
        (&mut event.dtstart, Some(&mut event.dtstart_tzid))
    } else if name.eq_ignore_ascii_case("DTEND") {
        (&mut event.dtend, Some(&mut event.dtend_tzid))
    } else if name.eq_ignore_ascii_case("UID") {
        (&mut event.uid, None)
    } else if name.eq_ignore_ascii_case("SUMMARY") {
        (&mut event.summary, None)
    } else {
        return;
    };
    if field.is_some() {
        return;
    }
    *field = Some(if name.eq_ignore_ascii_case("SUMMARY") {
        cap_label(&unescape(value))
    } else {
        cap_label(value)
    });
    if let Some(tzid) = tzid {
        *tzid = tzid_param(params).map(|value| cap_label(&value));
    }
}

/// The `TZID` parameter among `;`-separated parameters, quotes stripped.
fn tzid_param(mut params: &str) -> Option<String> {
    while !params.is_empty() {
        let (param, rest) = match unquoted(params, b';') {
            Some(semicolon) => (&params[..semicolon], &params[semicolon + 1..]),
            None => (params, ""),
        };
        if let Some((key, value)) = param.split_once('=')
            && key.eq_ignore_ascii_case("TZID")
        {
            return Some(value.trim_matches('"').to_string());
        }
        params = rest;
    }
    None
}

/// RFC 5545 text escapes: `\\`, `\;`, `\,`, `\n` and `\N`. Any other
/// backslash stays as sent.
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.clone().next() {
            Some('n' | 'N') => out.push('\n'),
            Some(next @ ('\\' | ';' | ',')) => out.push(next),
            _ => {
                out.push('\\');
                continue;
            }
        }
        chars.next();
    }
    out
}

/// `utf-8` for bytes that are valid UTF-8 with no control byte beyond TAB,
/// LF and CR (DEL is a control byte), else `base64` (decision 0110 §7).
pub fn encode_content(bytes: &[u8]) -> (&'static str, String) {
    let plain = std::str::from_utf8(bytes).ok().filter(|text| {
        text.bytes()
            .all(|b| !(b < 0x20 && !matches!(b, b'\t' | b'\n' | b'\r') || b == 0x7f))
    });
    match plain {
        Some(text) => ("utf-8", text.to_string()),
        None => {
            use base64::Engine as _;
            (
                "base64",
                base64::engine::general_purpose::STANDARD.encode(bytes),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use mail_parser::MessageParser;

    /// The NIST known answers pin that the rustls provider's hash really is
    /// SHA-256 (decision 0110 §5): the empty message, `abc`, and the
    /// 448-bit two-block message.
    #[test]
    fn the_provider_digest_is_sha256() {
        for (input, digest) in [
            (
                &b""[..],
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
        ] {
            assert_eq!(sha256_hex(input), digest);
        }
        // Every nibble is lowercase hex, zero-padded.
        let digest = sha256_hex(&[0, 1, 2]);
        assert_eq!(digest.len(), 64);
        assert!(
            digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
    }

    /// The five constants are pinned in one place (decision 0110 §3).
    #[test]
    fn the_caps_are_pinned() {
        assert_eq!(MAX_ATTACHMENTS, 250);
        assert_eq!(MAX_ATTACHMENT_STORED_BYTES, 1_048_576);
        assert_eq!(MAX_LABEL_BYTES, 1024);
        assert_eq!(MAX_ICS_BYTES, 1_048_576);
        assert_eq!(MAX_LIST_JSON_BYTES, 3_145_728);
    }

    #[test]
    fn labels_are_cut_at_a_character_boundary() {
        assert_eq!(cap_label("short"), "short");
        assert_eq!(cap_label(&"a".repeat(1024)).len(), 1024);
        assert_eq!(cap_label(&"a".repeat(1025)).len(), 1024);
        // 'é' is two bytes: 512 fit exactly; 513 cut at 1024, not 1025.
        assert_eq!(cap_label(&"é".repeat(513)), "é".repeat(512));
        // A three-byte character straddling the cap steps back to its start.
        let straddle = format!("{}日", "a".repeat(1023));
        assert_eq!(cap_label(&straddle), "a".repeat(1023));
    }

    #[test]
    fn content_is_utf8_only_when_it_is_plain_text() {
        assert_eq!(encode_content(b""), ("utf-8", String::new()));
        assert_eq!(
            encode_content("tab\there\r\nline é 日".as_bytes()),
            ("utf-8", "tab\there\r\nline é 日".to_string())
        );
        for bytes in [
            &b"nul\0byte"[..],
            b"bell\x07",
            b"del\x7f",
            b"\xff\xfe",
            b"\x1b[0m",
        ] {
            let (encoding, content) = encode_content(bytes);
            assert_eq!(encoding, "base64", "{bytes:?}");
            assert_eq!(
                base64::engine::general_purpose::STANDARD
                    .decode(content)
                    .unwrap(),
                bytes
            );
        }
    }

    fn event(body: &str) -> Vec<u8> {
        format!(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\n{body}END:VEVENT\r\nEND:VCALENDAR\r\n"
        )
        .into_bytes()
    }

    #[test]
    fn the_first_event_is_read_verbatim() {
        let ics = scan_ics(&event(
            "UID:evt-1@x.io\r\n\
             DTSTART;TZID=Europe/Berlin:20261102T150000\r\n\
             DTEND;TZID=\"America/New_York\":20261102T160000Z\r\n\
             SUMMARY:Plan\\, review\\; part\\nTwo \\N\\\\ ok\r\n",
        ))
        .unwrap();
        assert_eq!(
            ics,
            IcsEvent {
                dtstart: Some("20261102T150000".into()),
                dtstart_tzid: Some("Europe/Berlin".into()),
                dtend: Some("20261102T160000Z".into()),
                dtend_tzid: Some("America/New_York".into()),
                summary: Some("Plan, review; part\nTwo \n\\ ok".into()),
                uid: Some("evt-1@x.io".into()),
            }
        );
    }

    /// Folding, nesting and the first-event rule: a folded line is joined, a
    /// `VTIMEZONE`'s `DTSTART` and a `VALARM`'s `SUMMARY` are never the
    /// event's, the second event is never read, and a stray `END:` at depth
    /// one cannot lift the depth below the event.
    #[test]
    fn nesting_folding_and_the_first_event() {
        let ics = scan_ics(
            b"BEGIN:VCALENDAR\r\n\
              BEGIN:VTIMEZONE\r\nTZID:Europe/Berlin\r\nBEGIN:STANDARD\r\nDTSTART:19701025T030000\r\nEND:STANDARD\r\nDTSTART:19700101T000000\r\nEND:VTIMEZONE\r\n\
              BEGIN:VEVENT\r\n\
              SUMMARY:Quarterly\r\n  planning\r\n\tsession\r\n\
              END:VALARM\r\n\
              BEGIN:VALARM\r\nSUMMARY:alarm text\r\nDTSTART:19990101T000000\r\nEND:VALARM\r\n\
              DTSTART:20261102T150000\r\n\
              DTSTART:20991231T000000\r\n\
              UID:first\r\n\
              END:VEVENT\r\n\
              BEGIN:VEVENT\r\nSUMMARY:second\r\nUID:second\r\nEND:VEVENT\r\n\
              END:VCALENDAR\r\n",
        )
        .unwrap();
        assert_eq!(ics.summary.as_deref(), Some("Quarterly planningsession"));
        assert_eq!(ics.dtstart.as_deref(), Some("20261102T150000"));
        assert_eq!(ics.uid.as_deref(), Some("first"));
        assert_eq!(ics.dtend, None);
        assert_eq!(ics.dtstart_tzid, None);
    }

    /// Property parsing: names compare case-insensitively, a quoted
    /// parameter value may hold `:` and `;`, parameters without `=` and
    /// other keys are skipped, the first occurrence wins, and a line with
    /// no colon or an unknown name is ignored.
    #[test]
    fn property_lines_follow_rfc_5545_quoting() {
        let ics = scan_ics(&event(
            "no colon here\r\n\
             X-OTHER:ignored\r\n\
             dtstart;X-Q=\"a;b:c\";FLAG;tzid=\"Zone:One\":20261102T150000\r\n\
             DTSTART;TZID=Later:20300101T000000\r\n\
             DtEnd;VALUE=DATE-TIME:20261102T160000\r\n\
             uid:lower\r\n\
             summary:\r\n",
        ))
        .unwrap();
        assert_eq!(ics.dtstart.as_deref(), Some("20261102T150000"));
        assert_eq!(ics.dtstart_tzid.as_deref(), Some("Zone:One"));
        assert_eq!(ics.dtend.as_deref(), Some("20261102T160000"));
        assert_eq!(ics.dtend_tzid, None);
        assert_eq!(ics.uid.as_deref(), Some("lower"));
        assert_eq!(ics.summary.as_deref(), Some(""));
    }

    #[test]
    fn unknown_escapes_and_a_trailing_backslash_stay_as_sent() {
        assert_eq!(unescape(r"a\xb\"), r"a\xb\");
        assert_eq!(unescape(r"\,\;\\\n\N"), ",;\\\n\n");
        assert_eq!(unescape(""), "");
    }

    /// Hostile or odd calendars never panic and never lie: no event, no
    /// fields; an unterminated event still reads what it has; a line that
    /// splits a multi-byte character cannot be mistaken for a keyword.
    #[test]
    fn malformed_calendars_degrade_to_what_they_say() {
        assert_eq!(scan_ics(b""), None);
        assert_eq!(scan_ics(b"BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n"), None);
        assert_eq!(scan_ics(b"BEGIN:VEVEN\r\n"), None);
        assert_eq!(scan_ics(b"\xff\xfeBEGIN:VEVENT\r\n"), None);
        let open =
            scan_ics(b"begin:vevent\nSUMMARY:no end\nBEGIN:VALARM\nEND:VALARM\nEND:X").unwrap();
        assert_eq!(open.summary.as_deref(), Some("no end"));
        let odd = scan_ics("BEGIN:VEVENT\naaaaa\u{e9}\naaa\u{e9}\ne\nUID:u\n".as_bytes()).unwrap();
        assert_eq!(odd.uid.as_deref(), Some("u"));
        // At exactly the cap the scan runs; one byte more yields nothing.
        let mut at_cap = b"BEGIN:VEVENT\nUID:cap\nEND:VEVENT\n".to_vec();
        at_cap.resize(MAX_ICS_BYTES, b' ');
        assert_eq!(scan_ics(&at_cap).unwrap().uid.as_deref(), Some("cap"));
        at_cap.push(b' ');
        assert_eq!(scan_ics(&at_cap), None);
    }

    #[test]
    fn every_ics_field_is_capped_at_a_character_boundary() {
        let long = "é".repeat(600);
        let ics = scan_ics(&event(&format!(
            "SUMMARY:{long}\r\nUID:{long}\r\nDTSTART;TZID={long}:{long}\r\n"
        )))
        .unwrap();
        for field in [&ics.summary, &ics.uid, &ics.dtstart, &ics.dtstart_tzid] {
            assert_eq!(field.as_deref().unwrap(), "é".repeat(512));
        }
    }

    /// One multipart/mixed message from `(headers, body)` pairs.
    fn mixed(parts: &[(&str, &str)]) -> Vec<u8> {
        let mut raw = String::from(
            "From: a@x.io\r\nTo: b@x.io\r\nSubject: s\r\nMIME-Version: 1.0\r\n\
             Content-Type: multipart/mixed; boundary=\"bd\"\r\n\r\n--bd\r\n\
             Content-Type: text/plain\r\n\r\nhello\r\n",
        );
        for (headers, body) in parts {
            raw.push_str(&format!("--bd\r\n{headers}\r\n\r\n{body}\r\n"));
        }
        raw.push_str("--bd--\r\n");
        raw.into_bytes()
    }

    fn list_of(raw: &[u8]) -> AttachmentList {
        extract(&MessageParser::default().parse(raw).unwrap())
    }

    fn base64_of(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD
            .encode(bytes)
            .as_bytes()
            .chunks(76)
            .map(|line| std::str::from_utf8(line).unwrap())
            .collect::<Vec<_>>()
            .join("\r\n")
    }

    fn binary_part(name: &str, bytes: &[u8]) -> (String, String) {
        (
            format!(
                "Content-Type: application/octet-stream\r\n\
                 Content-Transfer-Encoding: base64\r\n\
                 Content-Disposition: attachment; filename=\"{name}\""
            ),
            base64_of(bytes),
        )
    }

    fn built(parts: &[(String, String)]) -> AttachmentList {
        let borrowed: Vec<(&str, &str)> = parts
            .iter()
            .map(|(h, b)| (h.as_str(), b.as_str()))
            .collect();
        list_of(&mixed(&borrowed))
    }

    #[test]
    fn a_mail_without_attachments_lists_known_empty() {
        let plain = list_of(b"From: a@x.io\r\nSubject: s\r\n\r\nbody\r\n");
        assert_eq!(plain, AttachmentList::default());
        let text_only = list_of(&mixed(&[]));
        assert_eq!((text_only.total, text_only.parts.len()), (0, 0));
    }

    /// Label, type, size and hash of a base64 and a quoted-printable part:
    /// the size and hash describe the decoded bytes, the type is lowercased
    /// and a part without a Content-Type has none.
    #[test]
    fn parts_are_labelled_sized_and_hashed_from_their_decoded_bytes() {
        let list = list_of(&mixed(&[
            (
                "Content-Type: Application/PDF; name=\"ignored.pdf\"\r\n\
                 Content-Transfer-Encoding: base64\r\n\
                 Content-Disposition: attachment; filename=\"Invoice.PDF\"",
                &base64_of(b"%PDF-1.4 abc"),
            ),
            (
                "Content-Type: text/csv; charset=utf-8\r\n\
                 Content-Transfer-Encoding: quoted-printable\r\n\
                 Content-Disposition: attachment; filename=\"a.csv\"",
                "caf=C3=A9,1=\r\n2",
            ),
            ("Content-Disposition: attachment; filename=\"x.bin\"", "raw"),
            (
                "Content-Type: image\r\nContent-Disposition: attachment",
                "i",
            ),
        ]));
        assert_eq!(list.total, 4);
        let [pdf, csv, bare, typeonly]: &[Attachment; 4] =
            list.parts.as_slice().try_into().unwrap();
        assert_eq!(pdf.filename.as_deref(), Some("Invoice.PDF"));
        assert_eq!(pdf.content_type.as_deref(), Some("application/pdf"));
        assert_eq!((pdf.size, pdf.stored), (12, true));
        assert_eq!(pdf.sha256, sha256_hex(b"%PDF-1.4 abc"));
        assert_eq!(pdf.content.as_deref(), Some(&b"%PDF-1.4 abc"[..]));
        assert_eq!(pdf.ics, None);
        assert_eq!(csv.content.as_deref(), Some("café,12".as_bytes()));
        assert_eq!(csv.size, "café,12".len() as u64);
        assert_eq!(csv.sha256, sha256_hex("café,12".as_bytes()));
        assert_eq!(bare.content_type, None);
        assert_eq!(bare.filename.as_deref(), Some("x.bin"));
        assert_eq!(typeonly.content_type.as_deref(), Some("image"));
        assert_eq!(typeonly.filename, None);
    }

    /// A bare `text/calendar` body with no disposition, `application/ics`
    /// and a `.ics`-named octet-stream are all listed and scanned; a name
    /// of `ics` alone or `.txt` is not a calendar.
    #[test]
    fn calendar_shaped_parts_are_candidates_even_without_a_disposition() {
        let body = "BEGIN:VEVENT\r\nUID:u1\r\nSUMMARY:Hi\r\nEND:VEVENT\r\n";
        let list = list_of(&mixed(&[
            ("Content-Type: text/calendar; method=REQUEST", body),
            ("Content-Type: application/ics", body),
            (
                "Content-Type: application/octet-stream; name=\"Cal.ICS\"",
                body,
            ),
            ("Content-Type: text/plain; name=\"ics\"", body),
            ("Content-Type: text/plain; name=\"notes.txt\"", body),
        ]));
        let types: Vec<_> = list
            .parts
            .iter()
            .map(|part| (part.content_type.as_deref().unwrap(), part.ics.is_some()))
            .collect();
        assert_eq!(
            types,
            [
                ("text/calendar", true),
                ("application/ics", true),
                ("application/octet-stream", true),
                // Listed by the parser (a named text part), never scanned.
                ("text/plain", false),
                ("text/plain", false),
            ]
        );
        assert_eq!(list.total, 5);
        let ics = list.parts[0].ics.as_ref().unwrap();
        assert_eq!(ics.uid.as_deref(), Some("u1"));
        assert_eq!(ics.summary.as_deref(), Some("Hi"));
        assert_eq!(list.parts[0].filename, None);
        assert_eq!(list.parts[2].filename.as_deref(), Some("Cal.ICS"));
    }

    /// A message that is itself one text/calendar body (the invite shape).
    #[test]
    fn a_whole_message_that_is_a_calendar_is_one_entry() {
        let raw = b"From: a@x.io\r\nSubject: s\r\nContent-Type: text/calendar\r\n\r\n\
                    BEGIN:VEVENT\r\nUID:solo\r\nEND:VEVENT\r\n";
        let list = list_of(raw);
        assert_eq!(list.total, 1);
        assert_eq!(
            list.parts[0].ics.as_ref().unwrap().uid.as_deref(),
            Some("solo")
        );
    }

    /// An attached message is one entry and its inner parts — an inner
    /// attachment included — are never walked; a multipart that merely
    /// carries an ".ics" name is not a candidate.
    #[test]
    fn an_attached_message_is_one_entry() {
        let inner = "From: i@x.io\r\nSubject: inner\r\nMIME-Version: 1.0\r\n\
                     Content-Type: multipart/mixed; boundary=\"in\"\r\n\r\n--in\r\n\
                     Content-Type: text/plain\r\n\r\nx\r\n--in\r\n\
                     Content-Type: application/pdf\r\nContent-Disposition: attachment; filename=\"deep.pdf\"\r\n\r\npdf\r\n--in--\r\n";
        let list = list_of(&mixed(&[
            ("Content-Type: message/rfc822", inner),
            (
                "Content-Type: multipart/mixed; boundary=\"zz\"; name=\"x.ics\"",
                "--zz\r\nContent-Type: text/plain\r\n\r\nq\r\n--zz--",
            ),
        ]));
        assert_eq!(list.total, 1);
        let message = &list.parts[0];
        assert_eq!(message.content_type.as_deref(), Some("message/rfc822"));
        assert_eq!(message.filename, None);
        assert_eq!(
            message.size as usize,
            message.content.as_ref().unwrap().len()
        );
        assert!(
            message
                .content
                .as_ref()
                .unwrap()
                .starts_with(b"From: i@x.io")
        );
    }

    /// The count cap: 250 entries listed with the exact total beside them.
    #[test]
    fn the_list_is_capped_and_the_total_is_exact() {
        let part = |i: usize| binary_part(&format!("f{i}"), b"x");
        let exactly: Vec<_> = (0..250).map(part).collect();
        let list = built(&exactly);
        assert_eq!((list.total, list.parts.len()), (250, 250));
        let over: Vec<_> = (0..251).map(part).collect();
        let list = built(&over);
        assert_eq!((list.total, list.parts.len()), (251, 250));
        assert_eq!(list.parts[249].filename.as_deref(), Some("f249"));
    }

    /// The first-fit budget at its edges (decision 0110 §2).
    #[test]
    fn the_budget_is_first_fit_and_exact() {
        let stored =
            |list: &AttachmentList| list.parts.iter().map(|p| p.stored).collect::<Vec<_>>();
        let whole = vec![7u8; MAX_ATTACHMENT_STORED_BYTES];
        let list = built(&[binary_part("whole", &whole)]);
        assert_eq!(stored(&list), [true]);
        assert_eq!(
            list.parts[0].content.as_ref().unwrap().len(),
            MAX_ATTACHMENT_STORED_BYTES
        );

        let over = vec![7u8; MAX_ATTACHMENT_STORED_BYTES + 1];
        let list = built(&[binary_part("over", &over)]);
        assert_eq!(stored(&list), [false]);
        let part = &list.parts[0];
        assert_eq!(part.size, 1_048_577);
        assert_eq!(part.sha256, sha256_hex(&over));
        assert_eq!(part.content, None);

        let half = vec![1u8; 524_288];
        let list = built(&[
            binary_part("a", &half),
            binary_part("b", &half),
            binary_part("c", b"x"),
        ]);
        assert_eq!(stored(&list), [true, true, false]);

        // A part that does not fit leaves the budget for a later small one.
        let list = built(&[
            binary_part("big", &half),
            binary_part("too-big", &[2u8; 600_000]),
            binary_part("small", &half),
            binary_part("late", b"y"),
        ]);
        assert_eq!(stored(&list), [true, false, true, false]);

        // A 1 MiB part before a small calendar: the calendar is unstored but
        // its fields are present.
        let ics = "BEGIN:VEVENT\r\nUID:late-ics\r\nEND:VEVENT\r\n";
        let blob = binary_part("blob", &whole);
        let list = list_of(&mixed(&[
            (&blob.0, &blob.1),
            ("Content-Type: text/calendar", ics),
        ]));
        assert_eq!(stored(&list), [true, false]);
        assert_eq!(list.parts[1].content, None);
        assert_eq!(
            list.parts[1].ics.as_ref().unwrap().uid.as_deref(),
            Some("late-ics")
        );
        // A zero-length part always fits.
        let list = built(&[binary_part("empty", b"")]);
        assert_eq!(stored(&list), [true]);
        assert_eq!(list.parts[0].sha256, sha256_hex(b""));
    }

    /// Filenames and types are cut to the label cap at a boundary.
    #[test]
    fn long_labels_are_capped() {
        let name = "é".repeat(600);
        let list = built(&[binary_part(&name, b"x")]);
        assert_eq!(list.parts[0].filename.as_deref().unwrap(), "é".repeat(512));
        let kind = format!("application/{}", "x".repeat(1100));
        let list = list_of(&mixed(&[(
            &format!("Content-Type: {kind}\r\nContent-Disposition: attachment"),
            "b",
        )]));
        assert_eq!(list.parts[0].content_type.as_deref().unwrap().len(), 1024);
    }
}
