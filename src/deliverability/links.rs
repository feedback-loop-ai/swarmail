//! Per-link findings (0121): the static rules over the visible `<a href>` tags
//! of the eligible HTML parts. This is a pure function of the stored bytes. It
//! reuses #154's lexical walk and label rules from `extract`, adds no parser,
//! and opens no connection; reachability lives apart in `reach` (0121 §6).

use super::html::Part;
use super::{AnalysisError, Collector, Finding, Location, Severity};
use crate::extract::{
    AnchorAttrs, LABEL_BREAKS, RefScanStats, Regions, TagScanStats, decode_refs, space,
};
use reqwest::Url;
use std::net::IpAddr;

/// The built-in shortener list's version. It is Swarmail-authored, basis date
/// 2026-11-03, and was not fetched. Editing `SHORTENERS` moves this and
/// `RULE_SET_ID` together; a snapshot test pins all three.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const SHORTENERS_VERSION: &str = "swarmail-shorteners/1";

/// Sorted, lowercase, unique, no trailing dot. Matching is exact on the host.
pub(crate) const SHORTENERS: [&str; 17] = [
    "bit.ly",
    "bl.ink",
    "buff.ly",
    "cutt.ly",
    "goo.gl",
    "is.gd",
    "lnkd.in",
    "ow.ly",
    "rb.gy",
    "rebrand.ly",
    "s.id",
    "shorturl.at",
    "t.co",
    "t.ly",
    "tiny.cc",
    "tinyurl.com",
    "v.gd",
];

/// A static rule: stable id, severity and literal fix hint (0121 §1).
pub(crate) struct Rule {
    pub(crate) id: &'static str,
    severity: Severity,
    hint: &'static str,
}

static INVALID: Rule = Rule {
    id: "link.url.invalid.v1",
    severity: Severity::Warning,
    hint: "Use an absolute https URL; mail has no base URL for relative links.",
};
static UNSAFE: Rule = Rule {
    id: "link.scheme.unsafe.v1",
    severity: Severity::Error,
    hint: "Replace the script, data or file link with an https link.",
};
static HTTP: Rule = Rule {
    id: "link.scheme.http.v1",
    severity: Severity::Warning,
    hint: "Serve the link over https.",
};
static SHORTENER: Rule = Rule {
    id: "link.shortener.v1",
    severity: Severity::Warning,
    hint: "Link to the destination directly instead of through a URL shortener.",
};
static MISMATCH: Rule = Rule {
    id: "link.anchor-host-mismatch.v1",
    severity: Severity::Warning,
    hint: "Make the visible link text name the host the link goes to.",
};

/// One visible `<a>` start tag with an `href`, as the report sees it.
pub(crate) struct Occurrence {
    pub(crate) location: Location,
    /// The source value with character references decoded once and ASCII
    /// whitespace trimmed.
    pub(crate) href: String,
    /// The #154 label: collapsed, block-separated, visible text only.
    pub(crate) label: String,
}

/// The #154 anchor consumer of the shared HTML walk: it keeps, for every
/// visible `a` start tag with an `href`, the value and the label that ends at
/// the next `a` start or end tag. Each finished anchor goes to `sink`; after
/// the sink's first error the walk still completes but nothing more is built.
struct Anchors<'a, F> {
    part: &'a Part<'a>,
    html: &'a str,
    sink: F,
    failed: Option<AnalysisError>,
    /// The open anchor: the href's source span and the label so far.
    open: Option<((usize, usize), String)>,
}

impl<F: FnMut(Occurrence) -> Result<(), AnalysisError>> Anchors<'_, F> {
    fn finish(&mut self) {
        let Some(((start, end), label)) = self.open.take() else {
            return;
        };
        if self.failed.is_some() {
            return;
        }
        let source = &self.html[start..end];
        let lead = source.len()
            - source
                .trim_start_matches(|c: char| c.is_ascii_whitespace())
                .len();
        let location = if self.part.exact {
            Location::Link {
                part_path: self.part.path.clone(),
                byte_offset: (start + lead) as u64,
            }
        } else {
            Location::MimePart {
                path: self.part.path.clone(),
            }
        };
        let href = decode_refs(source)
            .trim_matches(|c: char| c.is_ascii_whitespace())
            .to_string();
        let label = label.trim_end_matches(' ').to_string();
        if let Err(error) = (self.sink)(Occurrence {
            location,
            href,
            label,
        }) {
            self.failed = Some(error);
        }
    }
}

impl<F: FnMut(Occurrence) -> Result<(), AnalysisError>> Regions for Anchors<'_, F> {
    fn text(&mut self, run: &str, _refs: &mut RefScanStats) {
        let Some((_, label)) = &mut self.open else {
            return;
        };
        for c in decode_refs(run).chars() {
            if c.is_ascii_whitespace() {
                space(label);
            } else {
                label.push(c);
            }
        }
    }

    fn value(&mut self, _start: usize, _end: usize) {}

    fn element(
        &mut self,
        name: &[u8],
        closing: bool,
        attrs: AnchorAttrs,
        _end: usize,
        visible: bool,
    ) {
        if visible && name.eq_ignore_ascii_case(b"a") {
            self.finish();
            if let (false, Some(href)) = (closing, attrs.href) {
                self.open = Some((href, String::new()));
            }
        } else if visible
            && (LABEL_BREAKS
                .iter()
                .any(|tag| name.eq_ignore_ascii_case(tag))
                || (!closing && name.eq_ignore_ascii_case(b"br")))
            && let Some((_, label)) = &mut self.open
        {
            space(label);
        }
    }
}

/// Every occurrence of the part, in source order, to `sink`. The first error
/// the sink returns is returned after the walk.
pub(super) fn scan(
    part: &Part<'_>,
    sink: impl FnMut(Occurrence) -> Result<(), AnalysisError>,
) -> Result<(), AnalysisError> {
    let mut anchors = Anchors {
        part,
        html: part.body,
        sink,
        failed: None,
        open: None,
    };
    crate::extract::walk(part.body, &mut TagScanStats::default(), &mut anchors);
    anchors.finish();
    anchors.failed.map_or(Ok(()), Err)
}

/// The static rules for one link, in table order.
pub(crate) fn judge(href: &str, label: &str) -> Vec<&'static Rule> {
    if href.starts_with('#') {
        return Vec::new();
    }
    let Ok(url) = Url::parse(href) else {
        return vec![&INVALID];
    };
    let mut hit = Vec::new();
    let scheme = url.scheme();
    if matches!(scheme, "javascript" | "vbscript" | "data" | "file") {
        hit.push(&UNSAFE);
    }
    if matches!(scheme, "http" | "https") {
        let text = url.host_str().unwrap_or("");
        let host = text.strip_suffix('.').unwrap_or(text);
        let loopback = text
            .trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
            || host == "localhost"
            || host.ends_with(".localhost");
        if scheme == "http" && !loopback {
            hit.push(&HTTP);
        }
        if SHORTENERS.binary_search(&host).is_ok() {
            hit.push(&SHORTENER);
        }
        let tokens = host_tokens(label);
        let target = host.strip_prefix("www.").unwrap_or(host);
        if !tokens.is_empty()
            && !tokens.iter().any(|token| {
                let token = token.strip_prefix("www.").unwrap_or(token);
                target == token
                    || target
                        .strip_suffix(token)
                        .is_some_and(|rest| rest.ends_with('.'))
            })
        {
            hit.push(&MISMATCH);
        }
    }
    hit
}

/// The host-like tokens of a label (0121 §1, spec § Host-like tokens).
pub(crate) fn host_tokens(label: &str) -> Vec<String> {
    label.split_ascii_whitespace().filter_map(token).collect()
}

fn token(raw: &str) -> Option<String> {
    if !raw.is_ascii() || raw.contains('@') {
        return None;
    }
    let lower = raw.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or(&lower);
    let rest = &rest[..rest.find(['/', '?', '#', ':']).unwrap_or(rest.len())];
    let rest = rest
        .trim_start_matches(['(', '[', '<', '"', '\''])
        .trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '>', '"', '\'']);
    let (head, last) = rest.rsplit_once('.')?;
    let label = |l: &str| {
        !l.is_empty()
            && l.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    (head.split('.').all(label) && last.len() >= 2 && last.bytes().all(|b| b.is_ascii_lowercase()))
        .then(|| rest.to_string())
}

/// The static findings of one eligible part, into the shared collector.
pub(super) fn collect(part: &Part<'_>, out: &mut Collector) -> Result<(), AnalysisError> {
    scan(part, |occurrence| {
        for rule in judge(&occurrence.href, &occurrence.label) {
            out.push(Finding {
                rule_id: rule.id.into(),
                location: occurrence.location.clone(),
                severity: rule.severity,
                fix_hint: rule.hint.into(),
            })?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::super::{Inputs, Report, evaluate};
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};

    fn ids(href: &str, label: &str) -> Vec<&'static str> {
        judge(href, label).iter().map(|rule| rule.id).collect()
    }
    fn fires(href: &str, label: &str, id: &str) -> bool {
        ids(href, label).contains(&id)
    }
    fn occurrences(html: &str) -> Vec<(Location, String, String)> {
        let part = Part {
            path: vec![0],
            body: html,
            exact: true,
        };
        let mut out = Vec::new();
        scan(&part, |o| {
            out.push((o.location, o.href, o.label));
            Ok(())
        })
        .unwrap();
        out
    }
    fn message(encoding: &str, charset: &str, body: &[u8]) -> Vec<u8> {
        let mut raw = format!(
            "Content-Type: text/html; charset={charset}\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n"
        )
        .into_bytes();
        raw.extend_from_slice(body);
        raw
    }
    fn links(raw: &[u8]) -> Vec<Finding> {
        let mut report = evaluate(raw, &Inputs::default()).unwrap();
        report.findings.retain(|f| f.rule_id.starts_with("link."));
        report.findings
    }
    fn report(raw: &[u8]) -> Result<Report, AnalysisError> {
        evaluate(raw, &Inputs::default())
    }

    #[test]
    fn invalid_url_pair() {
        for href in [
            "",
            "/relative/path",
            "relative.html",
            "//host.test/x",
            "{{unsubscribe_url}}",
            "https://exa mple.test/",
            "http://",
        ] {
            assert_eq!(ids(href, ""), ["link.url.invalid.v1"], "{href:?}");
        }
        for href in ["#top", "#", "https://example.test/", "mailto:a@b.test"] {
            assert!(!fires(href, "", "link.url.invalid.v1"), "{href:?}");
        }
    }

    #[test]
    fn unsafe_scheme_pair() {
        for href in [
            "javascript:alert(1)",
            "JAVASCRIPT:alert(1)",
            "java\tscript:alert(1)",
            "vbscript:msgbox",
            "data:text/html,x",
            "file:///etc/passwd",
        ] {
            assert!(fires(href, "", "link.scheme.unsafe.v1"), "{href:?}");
        }
        for href in [
            "https://example.test/",
            "mailto:a@b.test",
            "tel:+15550100",
            "cid:logo@x",
            "sms:+15550100",
            "ftp://example.test/",
        ] {
            assert!(!fires(href, "", "link.scheme.unsafe.v1"), "{href:?}");
        }
    }

    #[test]
    fn http_scheme_pair() {
        for href in [
            "http://example.test/",
            "HTTP://EXAMPLE.TEST/",
            "http://1.2.3.4/",
        ] {
            assert!(fires(href, "", "link.scheme.http.v1"), "{href:?}");
        }
        for href in [
            "https://example.test/",
            "http://localhost:3000/x",
            "http://localhost./x",
            "http://app.localhost/",
            "http://127.0.0.1/",
            "http://127.9.9.9/",
            "http://[::1]/",
            "mailto:a@b.test",
        ] {
            assert!(!fires(href, "", "link.scheme.http.v1"), "{href:?}");
        }
    }

    #[test]
    fn shortener_pair() {
        for href in [
            "https://bit.ly/x",
            "http://t.co./x",
            "https://TINYURL.com/a",
        ] {
            assert!(fires(href, "", "link.shortener.v1"), "{href:?}");
        }
        for href in [
            "https://notbit.ly/x",
            "https://bit.ly.evil.test/x",
            "https://www.bit.ly/x",
            "https://example.test/",
            "https://1.2.3.4/",
            "javascript:bit.ly",
        ] {
            assert!(!fires(href, "", "link.shortener.v1"), "{href:?}");
        }
    }

    #[test]
    fn shortener_list_is_canonical_and_pinned() {
        let mut sorted = SHORTENERS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, SHORTENERS);
        for host in SHORTENERS {
            assert_eq!(host, host.to_ascii_lowercase());
            assert!(!host.ends_with('.'));
            assert!(!host.starts_with('.'));
        }
        // FNV-1a over the newline-joined list: editing the list fails here until
        // the version, the rule-set id and this digest move together (0121 §4).
        let digest = SHORTENERS
            .join("\n")
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
            });
        assert_eq!(
            (SHORTENERS_VERSION, super::super::RULE_SET_ID, digest),
            (
                "swarmail-shorteners/1",
                "swarmail-html/4",
                0xd850a45b7e464f79
            )
        );
    }

    #[test]
    fn host_token_table() {
        for (label, tokens) in [
            ("paypal.com", vec!["paypal.com"]),
            ("PayPal.COM", vec!["paypal.com"]),
            ("https://paypal.com/login", vec!["paypal.com"]),
            ("HTTP://paypal.com?x=1", vec!["paypal.com"]),
            ("see (paypal.com), now", vec!["paypal.com"]),
            ("<paypal.com>", vec!["paypal.com"]),
            (
                "\"a.b.example\"! x.test:8080",
                vec!["a.b.example", "x.test"],
            ),
            ("www.example.com.", vec!["www.example.com"]),
            ("Version 1.2", vec![]),
            ("pi is 3.14", vec![]),
            ("e.g. this", vec![]),
            ("Node.js", vec!["node.js"]),
            ("a@b.test", vec![]),
            ("café.test", vec![]),
            ("a..b.test", vec![]),
            ("-.test", vec!["-.test"]),
            ("x.t", vec![]),
            ("x.t1", vec![]),
            ("under_score.test", vec![]),
            ("plain words only", vec![]),
            ("", vec![]),
        ] {
            assert_eq!(host_tokens(label), tokens, "{label:?}");
        }
    }

    #[test]
    fn anchor_mismatch_pair() {
        let m = "link.anchor-host-mismatch.v1";
        for (label, href) in [
            ("paypal.com", "https://paypal.com.evil.test/"),
            ("example.com", "https://1.2.3.4/"),
            ("example.com", "https://[::1]/"),
            ("trusted.test", "https://trusted.test@evil.test/"),
            ("Visit a.test or b.test", "https://c.test/"),
            ("example.com", "http://notexample.com/"),
        ] {
            assert!(fires(href, label, m), "{label:?} {href:?}");
        }
        for (label, href) in [
            ("example.com", "https://www.example.com/"),
            ("example.com", "https://click.example.com/x"),
            ("www.example.com", "https://example.com/"),
            ("www.example.com", "https://example.com./"),
            ("Visit a.test or b.test", "https://b.test/"),
            ("", "https://example.com/"),
            ("Click here", "https://example.com/"),
            ("Version 1.2", "https://example.com/"),
            ("example.com", "mailto:a@example.com"),
            ("example.com", "javascript:void(0)"),
        ] {
            assert!(!fires(href, label, m), "{label:?} {href:?}");
        }
    }

    #[test]
    fn several_rules_fire_on_one_link() {
        assert_eq!(
            ids("http://bit.ly/x", "paypal.com"),
            [
                "link.scheme.http.v1",
                "link.shortener.v1",
                "link.anchor-host-mismatch.v1"
            ]
        );
    }

    #[test]
    fn hidden_anchors_yield_no_occurrence() {
        for html in [
            "<head><a href='https://x.test/1'>hidden</a></head>",
            "<template><a href='https://x.test/1'>hidden</a></template>",
            "<!--[if mso]><a href='https://x.test/1'>hidden</a><![endif]-->",
            "<script>var a = '<a href=\"https://x.test/1\">';</script>",
            "<style>/* <a href=https://x.test/1> */</style>",
            "<a>no href</a>",
        ] {
            assert!(occurrences(html).is_empty(), "{html}");
        }
    }

    #[test]
    fn anchor_labels_match_ingest_anchor_text() {
        for html in [
            "<a href='https://x.io/1'> &nbsp; É\t雪 &nbsp; </a>",
            "<a href='https://x.io/1'>one<a href='https://x.io/2'/>two",
            "<a href='https://x.io/1'>one</a></a><a href='https://x.io/3'>",
            "<head><a href='https://x.io/1'>hidden</a></head><a href=https://x.io/2>two",
            "<a href='https://x.io/1'> a <i> b </i><style>x</style> c<title>x</title> d</a>",
            "<a href=https://x.io/1><p>one</p><li>two<tr>three<h1>4</h1><h2>5</h2><h3>6</h3><h4>7</h4><h5>8</h5><h6>9</h6>x<br>y</br>z<span>w</span></a>",
            "<p><br><a href=https://x.io/1><br> <div>one</div><br></a>",
            "<a href=https://x.io/1>a<template><p>b</p></template>c<head><br></head>d</a>",
            "<A HREF=\"https://x.io/1\">Up</A> <a href=\"https://x.io/2?a=1&amp;b=2\">&lt;b&gt; &amp; c</a>",
            "<a href='https://x.io/1' rel=nofollow>x</a>",
        ] {
            let (_, metadata) = crate::extract::extract_link_metadata(None, Some(html));
            let ingest: Vec<_> = metadata
                .iter()
                .filter_map(|m| m.anchor_text.clone().map(|t| (m.url.clone(), t)))
                .collect();
            let mine: Vec<_> = occurrences(html)
                .into_iter()
                .map(|(_, href, label)| (href, label))
                .collect();
            assert_eq!(mine, ingest, "{html}");
        }
    }

    #[test]
    fn non_http_hrefs_have_labels_ingest_does_not_store() {
        let found = occurrences("<a href='javascript:x()'>Win</a><a href=\"\">e</a><a href>v</a>");
        assert_eq!(
            found
                .iter()
                .map(|(_, h, l)| (h.as_str(), l.as_str()))
                .collect::<Vec<_>>(),
            [("javascript:x()", "Win"), ("", "e"), ("", "v")]
        );
    }

    #[test]
    fn link_offsets_index_the_source_bytes() {
        let html = "<p>x</p><a href=\"https://bit.ly/q\">bit.ly</a><a href=  http://t.co/u >u</a>\
                    <a href='  javascript:z '>j</a><a href=\"&#106;avascript:y\">e</a>";
        let found = occurrences(html);
        let at = |needle: &str| Location::Link {
            part_path: vec![0],
            byte_offset: html.find(needle).unwrap() as u64,
        };
        assert_eq!(
            found
                .iter()
                .map(|(loc, _, _)| loc.clone())
                .collect::<Vec<_>>(),
            [
                at("https://bit.ly/q"),
                at("http://t.co/u"),
                at("javascript:z"),
                at("&#106;"),
            ]
        );
        assert_eq!(found[2].1, "javascript:z");
        assert_eq!(found[3].1, "javascript:y");
    }

    fn quoted_printable(text: &str) -> String {
        text.replace('=', "=3D").replace('\n', "\r\n")
    }

    #[test]
    fn link_offsets_index_transfer_decoded_source() {
        let html = "<p>intro</p>\n<a href=\"https://bit.ly/q\">bit.ly</a>\n\
                    <a href=\"javascript:z()\">go</a>\n";
        let b64 = STANDARD
            .encode(html)
            .as_bytes()
            .chunks(20)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect::<Vec<_>>()
            .join("\r\n");
        let crlf = html.replace('\n', "\r\n");
        for (encoding, body, source) in [
            ("7bit", crlf.clone(), crlf.clone()),
            ("base64", b64, html.to_string()),
            ("quoted-printable", quoted_printable(html), crlf),
        ] {
            let found = links(&message(encoding, "utf-8", body.as_bytes()));
            let at = |rule: &str| -> Vec<Location> {
                found
                    .iter()
                    .filter(|f| f.rule_id == rule)
                    .map(|f| f.location.clone())
                    .collect()
            };
            let link = |needle: &str| Location::Link {
                part_path: vec![],
                byte_offset: source.find(needle).unwrap() as u64,
            };
            assert_eq!(
                at("link.scheme.unsafe.v1"),
                [link("javascript:z()")],
                "{encoding}"
            );
            assert_eq!(
                at("link.shortener.v1"),
                [link("https://bit.ly/q")],
                "{encoding}"
            );
        }
    }

    #[test]
    fn link_offsets_name_the_part_path_in_multipart_mail() {
        let html = "<a href=\"javascript:z()\">go</a>";
        let raw = format!(
            "Content-Type: multipart/alternative; boundary=b\r\n\r\n--b\r\n\
             Content-Type: text/plain\r\n\r\nplain\r\n--b\r\n\
             Content-Type: text/html\r\n\r\n{html}\r\n--b--\r\n"
        );
        assert_eq!(
            links(raw.as_bytes())[0].location,
            Location::Link {
                part_path: vec![1],
                byte_offset: html.find("javascript").unwrap() as u64
            }
        );
    }

    #[test]
    fn non_identity_charset_falls_back_to_mime_part() {
        // One 8-bit Latin-1 byte: the decoded text is longer than the payload,
        // so no source offset can be promised.
        let body = b"<p>caf\xe9</p><a href=\"javascript:z()\">go</a>";
        let found = links(&message("8bit", "iso-8859-1", body));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location, Location::MimePart { path: vec![] });
        // The same link with only ASCII bytes keeps its exact offset.
        let body = b"<p>cafe</p><a href=\"javascript:z()\">go</a>";
        let found = links(&message("8bit", "iso-8859-1", body));
        assert_eq!(
            found[0].location,
            Location::Link {
                part_path: vec![],
                byte_offset: 20
            }
        );
    }

    #[test]
    fn a_refused_transfer_decode_fails_the_report() {
        // `!` is not base64: the parser flags the part, and the 0106 walk
        // already fails the whole report on that; links change nothing.
        let body = b"<a href=\"javascript:z()\">go</a>!!";
        let raw = message("base64", "utf-8", body);
        assert_eq!(report(&raw).unwrap_err(), AnalysisError::Failed);
    }

    #[test]
    fn link_findings_respect_report_limits() {
        let bad = |n: usize| "<a href=\"javascript:z()\">x</a>".repeat(n);
        let raw = |n| message("7bit", "utf-8", bad(n).as_bytes());
        // The other rule families share the same 256 findings.
        let others = report(&raw(0)).unwrap().findings.len();
        assert_eq!(report(&raw(256 - others)).unwrap().findings.len(), 256);
        for n in [257 - others, 300] {
            assert_eq!(report(&raw(n)).unwrap_err(), AnalysisError::Failed);
        }
    }

    #[test]
    fn link_findings_over_the_byte_limit_fail_without_truncation() {
        // Deep part paths make each finding long: three findings per link.
        let leaf = "<a href=\"http://bit.ly/x\">paypal.com</a>".repeat(85);
        let mut part = format!("Content-Type: text/html\r\n\r\n{leaf}");
        for depth in (0..30).rev() {
            part = format!(
                "Content-Type: multipart/mixed; boundary=b{depth}\r\n\r\n--b{depth}\r\n{part}\r\n--b{depth}--\r\n"
            );
        }
        assert_eq!(report(part.as_bytes()).unwrap_err(), AnalysisError::Failed);
        let few = part.replace(&leaf, &leaf[..leaf.len() / 85 * 20]);
        assert_eq!(links(few.as_bytes()).len(), 60);
    }

    #[test]
    fn link_report_is_deterministic_cold_warm_and_across_services() {
        let raw = message(
            "7bit",
            "utf-8",
            b"<a href=\"http://bit.ly/x\">paypal.com</a><a href=\"{{u}}\">u</a>",
        );
        let first = report(&raw).unwrap();
        assert_eq!(first, report(&raw).unwrap());
        assert!(
            first
                .findings
                .iter()
                .any(|f| f.rule_id == "link.shortener.v1")
        );
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&report(&raw).unwrap()).unwrap()
        );
    }

    #[test]
    fn link_ingest_is_byte_identical() {
        let html = "<a href=\"https://x.io/a?b=1&amp;c=2\">Go</a> <a href=javascript:x()>no</a> \
                    <a href='mailto:a@x.io'>m</a> https://x.io/text.";
        let (links, metadata) = crate::extract::extract_link_metadata(None, Some(html));
        assert_eq!(links, ["https://x.io/a?b=1&c=2", "https://x.io/text"]);
        assert_eq!(
            metadata
                .iter()
                .map(|m| (m.url.as_str(), m.anchor_text.as_deref()))
                .collect::<Vec<_>>(),
            [
                ("https://x.io/a?b=1&c=2", Some("Go")),
                ("https://x.io/text", None)
            ]
        );
    }
}
