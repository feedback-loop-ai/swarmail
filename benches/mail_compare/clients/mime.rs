//! A strict reader of the one MIME shape the API corpus authors, for the
//! families that serve only a raw message (MailHog's `Raw.Data`, GreenMail's
//! `mimeMessage`, smtp4dev's and Inbucket's source routes) or decoded fields
//! re-wrapped as one (MailSlurper). The eval crate holds no MIME package, so
//! the shared clients read MIME here: unfolded headers, a single text or
//! HTML part or a `multipart/alternative` of them, and the `7bit`, `8bit`,
//! `quoted-printable` and `base64` transfer encodings. Anything else
//! refuses: a shape this reader does not know is never read as a shorter
//! message.

use base64::Engine as _;

/// The fields a `get` compares.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parsed {
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Vec<String>,
    pub text: Option<String>,
    pub html: Option<String>,
}

/// Unfolded `(lowercased name, value)` header pairs.
pub type Headers = Vec<(String, String)>;

/// The headers and the body after the first empty line.
pub fn split(raw: &[u8]) -> Option<(Headers, &[u8])> {
    let (head, body) = match find(raw, b"\r\n\r\n") {
        Some(i) => (&raw[..i], &raw[i + 4..]),
        None => {
            let i = find(raw, b"\n\n")?;
            (&raw[..i], &raw[i + 2..])
        }
    };
    let head = std::str::from_utf8(head).ok()?;
    let mut out: Vec<(String, String)> = Vec::new();
    for line in head.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.starts_with([' ', '\t']) {
            let (_, v) = out.last_mut()?;
            v.push(' ');
            v.push_str(line.trim());
            continue;
        }
        let (name, value) = line.split_once(':')?;
        out.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }
    Some((out, body))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn header<'a>(h: &'a [(String, String)], name: &str) -> Option<&'a str> {
    h.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
}

/// `type/subtype` (lowercased) and its `boundary` parameter.
pub fn content_type(v: &str) -> (String, Option<String>) {
    let mut parts = v.split(';');
    let mime = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    let boundary = parts.find_map(|p| {
        let (k, v) = p.split_once('=')?;
        (k.trim().eq_ignore_ascii_case("boundary")).then(|| v.trim().trim_matches('"').to_owned())
    });
    (mime, boundary)
}

/// Decode a part body by its transfer encoding.
pub fn decode(body: &[u8], encoding: Option<&str>) -> Option<String> {
    let enc = encoding.unwrap_or("7bit").trim().to_ascii_lowercase();
    let bytes = match enc.as_str() {
        "7bit" | "8bit" | "binary" => body.to_vec(),
        "base64" => {
            let clean: Vec<u8> = body
                .iter()
                .copied()
                .filter(|b| !b.is_ascii_whitespace())
                .collect();
            base64::engine::general_purpose::STANDARD
                .decode(clean)
                .ok()?
        }
        "quoted-printable" => quoted_printable(body)?,
        _ => return None,
    };
    String::from_utf8(bytes).ok()
}

fn quoted_printable(body: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(body.len());
    let mut i = 0;
    while i < body.len() {
        match body[i] {
            b'=' if body.get(i + 1) == Some(&b'\r') && body.get(i + 2) == Some(&b'\n') => i += 3,
            b'=' if body.get(i + 1) == Some(&b'\n') => i += 2,
            b'=' => {
                let hex = std::str::from_utf8(body.get(i + 1..i + 3)?).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    Some(out)
}

/// One message: its headers and its text and HTML parts.
pub fn parse(raw: &[u8]) -> Option<Parsed> {
    let (h, body) = split(raw)?;
    let mut p = Parsed {
        subject: header(&h, "subject").map(str::to_owned),
        from: header(&h, "from").map(super::bare),
        to: header(&h, "to")
            .map(|t| {
                t.split(',')
                    .map(super::bare)
                    .filter(|a| !a.is_empty())
                    .collect()
            })
            .unwrap_or_default(),
        ..Parsed::default()
    };
    let (mime, boundary) = content_type(header(&h, "content-type").unwrap_or("text/plain"));
    let encoding = header(&h, "content-transfer-encoding");
    match mime.as_str() {
        "text/plain" => p.text = Some(decode(body, encoding)?),
        "text/html" => p.html = Some(decode(body, encoding)?),
        "multipart/alternative" | "multipart/mixed" => {
            for (ph, pb) in parts(body, &boundary?)? {
                let (pm, _) = content_type(header(&ph, "content-type").unwrap_or("text/plain"));
                let pe = header(&ph, "content-transfer-encoding");
                let slot = match pm.as_str() {
                    "text/plain" => &mut p.text,
                    "text/html" => &mut p.html,
                    _ => continue,
                };
                // A second part of one kind is a shape the corpus never
                // authors: refuse rather than choose.
                if slot.is_some() {
                    return None;
                }
                *slot = Some(decode(pb, pe)?);
            }
        }
        _ => return None,
    }
    Some(p)
}

type Part<'a> = (Headers, &'a [u8]);

/// The parts of a multipart body. A delimiter must start a line, and the
/// closing delimiter must be present: a cut body refuses.
fn parts<'a>(body: &'a [u8], boundary: &str) -> Option<Vec<Part<'a>>> {
    let delim = format!("--{boundary}");
    let mut starts = Vec::new();
    let mut from = 0;
    while let Some(at) = find(&body[from..], delim.as_bytes()).map(|i| i + from) {
        if at > 0 && body[at - 1] != b'\n' {
            return None;
        }
        starts.push(at);
        from = at + delim.len();
    }
    let last = *starts.last()?;
    if !body[last + delim.len()..].starts_with(b"--") {
        return None;
    }
    let mut out = Vec::new();
    for w in starts.windows(2) {
        let mut seg = &body[w[0] + delim.len()..w[1]];
        seg = seg
            .strip_prefix(b"\r\n")
            .or_else(|| seg.strip_prefix(b"\n"))?;
        // The line break before the next delimiter belongs to it.
        let seg = seg
            .strip_suffix(b"\r\n")
            .or_else(|| seg.strip_suffix(b"\n"))
            .unwrap_or(seg);
        out.push(split(seg).or_else(|| {
            // A part with no headers at all: an empty block, then its body.
            Some((
                Vec::new(),
                seg.strip_prefix(b"\r\n")
                    .or_else(|| seg.strip_prefix(b"\n"))?,
            ))
        })?);
    }
    Some(out)
}
