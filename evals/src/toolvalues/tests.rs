use super::*;
use crate::journal::ToolOutcome;

const BEARER: &str = "BearerCanary0123456789abcdefXYZ";
const SENTINEL: &str = "sentinel-canary-ÿ-42";
/// Sentinels read from a file are at least [`SENTINEL_MIN_BYTES`] long.
const FILE_SENTINEL: &str = "file sentinel canary 0123456789abcdef";
const LATE_SENTINEL: &str = "late sentinel canary 0123456789abcdef";

fn long(i: usize) -> String {
    format!("sentinel {i:032}")
}

fn reg() -> Registry {
    let mut r = Registry::new();
    r.register(BEARER);
    r.register(SENTINEL);
    r
}

fn catalogue() -> BTreeSet<String> {
    ["swarmail_wait_for_email", "swarmail_get_email"]
        .into_iter()
        .map(String::from)
        .collect()
}

#[test]
fn the_reader_is_strict_and_keeps_spans() {
    let text = br#" {"a" : [1, -2.5e+3, true, null], "b": "x\u00e9\ud83d\ude00\n"} "#;
    let node = parse(text).unwrap();
    assert_eq!(&text[node.span.clone()], &text[1..text.len() - 1]);
    let b = node.get("b").unwrap();
    assert_eq!(b.value, Json::Str("xé😀\n".into()));
    let a = node.get("a").unwrap();
    assert_eq!(&text[a.span.clone()], b"[1, -2.5e+3, true, null]");
    for bad in [
        &br#"{"a":1,"a":2}"#[..],
        br#"{"a":1"#,
        br#"[1,]"#,
        br#"{"a":01}"#,
        br#"{"a":1.}"#,
        br#"{"a":1e}"#,
        br#""\ud800""#,
        br#""\ud800A""#,
        br#""\x""#,
        b"\"tab\there\"",
        b"\"\xff\"",
        br#"tru"#,
        br#"{"a":1} x"#,
        br#"{1:2}"#,
        br#"-"#,
        br#""\u12g4""#,
        b"",
    ] {
        assert!(parse(bad).is_err(), "{:?}", String::from_utf8_lossy(bad));
    }
    let deep = format!(
        "{}{}",
        "[".repeat(DEPTH_LIMIT + 1),
        "]".repeat(DEPTH_LIMIT + 1)
    );
    assert!(parse(deep.as_bytes()).is_err());
    let ok = format!("{}{}", "[".repeat(DEPTH_LIMIT), "]".repeat(DEPTH_LIMIT));
    assert!(parse(ok.as_bytes()).is_ok());
    assert!(parse(br#"{"e":"\"\\\/\b\f\r\t"}"#).is_ok());
}

#[test]
fn redaction_masks_every_occurrence_in_every_form() {
    let r = reg();
    let cases: Vec<(String, &str)> = vec![
        // Plain, repeated and adjacent occurrences; a member name.
        (
            format!(r#"{{"path":"/a?t={BEARER}&u={BEARER}{BEARER}"}}"#),
            r#"{"path":"/a?t=[REDACTED]&u=[REDACTED]"}"#,
        ),
        (format!(r#"{{"{SENTINEL}":1}}"#), r#"{"[REDACTED]":1}"#),
        // An escaped spelling decodes to the literal.
        (
            r#"{"x":"sentinel-canary-\u00ff-42"}"#.into(),
            r#"{"x":"[REDACTED]"}"#,
        ),
        // Percent-encoded in a path: redacted in its decoded form.
        (
            "{\"path\":\"/a/sentinel-canary-%C3%BF-42\"}".into(),
            r#"{"path":"/a/[REDACTED]"}"#,
        ),
        // A structured body nested as a JSON string, escapes inside it.
        (
            format!(r#"{{"body":"{{\"k\":\"{BEARER}\",\"n\":[1, 2]}}"}}"#),
            r#"{"body":"{\"k\":\"[REDACTED]\",\"n\":[1,2]}"}"#,
        ),
        // Whitespace, and JSON escapes that decode to plain text, keep
        // their values.
        (
            "{ \"q\" : \"h\\u0065llo \\\"x\\\"\" , \"n\" : 1.50 }".into(),
            r#"{"q":"hello \"x\"","n":1.50}"#,
        ),
        // A nested-looking string without a literal is left as written.
        (r#"{"s":"[1, 2]"}"#.into(), r#"{"s":"[1, 2]"}"#),
        // Bad percent escapes are text.
        (r#"{"s":"100%zz"}"#.into(), r#"{"s":"100%zz"}"#),
        // A malformed escape beside an encoded literal does not hide it:
        // each valid escape is decoded wherever it stands.
        (
            format!(r#"{{"path":"/%42{}%zz"}}"#, &BEARER[1..]),
            r#"{"path":"/[REDACTED]%zz"}"#,
        ),
        // Double and triple encodings are followed to their fixed point.
        (
            format!(r#"{{"path":"/%2542{}"}}"#, &BEARER[1..]),
            r#"{"path":"/[REDACTED]"}"#,
        ),
        (
            "{\"path\":\"/a/sentinel-canary-%252525C3%25BF-42?x=%\"}".into(),
            r#"{"path":"/a/[REDACTED]?x=%"}"#,
        ),
    ];
    for (input, want) in cases {
        assert_eq!(r.redact(input.as_bytes()).as_deref(), Ok(want), "{input}");
    }
    // Overlapping literals mask as one run.
    let mut o = Registry::new();
    o.register("abcd");
    o.register("cdef");
    assert_eq!(o.redact(br#"["xabcdefy"]"#).unwrap(), r#"["x[REDACTED]y"]"#);
    assert!(o.is_complete() && o.len() == 2 && !o.is_empty());
    assert!(Registry::default().is_empty());
}

#[test]
fn values_that_cannot_be_proven_redacted_refuse_by_reason() {
    let r = reg();
    // A literal in a number's spelling cannot be masked.
    let mut digits = Registry::new();
    digits.register("424242");
    assert_eq!(
        digits.redact(b"{\"n\":4242420}"),
        Err(Refusal::ResidualSecret)
    );
    // A literal inside the mask or JSON syntax survives masking.
    let mut mask = Registry::new();
    mask.register("REDACT");
    assert_eq!(
        mask.redact(br#"{"a":"REDACT"}"#),
        Err(Refusal::ResidualSecret)
    );
    let mut syntax = Registry::new();
    syntax.register("ru");
    assert_eq!(
        syntax.redact(br#"{"a":true}"#),
        Err(Refusal::ResidualSecret)
    );
    // A value that already spells the mask could not be told from a
    // redaction, in any of its decoded forms.
    for spelled in [
        &br#"{"a":"x [REDACTED] y"}"#[..],
        br#"{"a":"x \u005bREDACTED] y"}"#,
        br#"{"[REDACTED]":1}"#,
        br#"{"a":"{\"b\":\"[REDACTED]\"}"}"#,
    ] {
        assert_eq!(
            r.redact(spelled),
            Err(Refusal::InvalidArgumentEncoding),
            "{}",
            String::from_utf8_lossy(spelled)
        );
    }
    let collides = format!(r#"{{"a":"%42{} %5BREDACTED%5D"}}"#, &BEARER[1..]);
    assert_eq!(
        r.redact(collides.as_bytes()),
        Err(Refusal::InvalidArgumentEncoding)
    );
    // A percent decoding that is not UTF-8 or never settles cannot be
    // masked as text.
    let not_utf8 = format!(r#"{{"a":"%FF%42{}"}}"#, &BEARER[1..]);
    assert_eq!(
        r.redact(not_utf8.as_bytes()),
        Err(Refusal::InvalidArgumentEncoding)
    );
    let unsettled = format!(
        r#"{{"a":"%{}42{}"}}"#,
        "25".repeat(DEPTH_LIMIT + 1),
        &BEARER[1..]
    );
    assert_eq!(
        r.redact(unsettled.as_bytes()),
        Err(Refusal::InvalidArgumentEncoding)
    );
    // Text whose layers never settle cannot be proven clean, literal or not.
    let benign = format!(r#"{{"a":"%{}41"}}"#, "25".repeat(DEPTH_LIMIT + 1));
    assert_eq!(
        r.redact(benign.as_bytes()),
        Err(Refusal::InvalidArgumentEncoding)
    );
    // The same without a percent sign: a JSON-escaped one starts the chain.
    // Text that still holds a backslash is not plain, settled or not.
    for n in [DEPTH_LIMIT + 1, DEPTH_LIMIT - 2] {
        let escaped = format!(r#"{{"a":"\\u0025{}41"}}"#, "25".repeat(n));
        assert_eq!(
            r.redact(escaped.as_bytes()),
            Err(Refusal::InvalidArgumentEncoding)
        );
    }
    // A percent-encoded JSON escape: not plain either.
    let mixed = format!(r#"{{"a":"%5Cu0042{}"}}"#, &BEARER[1..]);
    assert_eq!(
        r.redact(mixed.as_bytes()),
        Err(Refusal::InvalidArgumentEncoding)
    );
    // A literal in text beside an undecodable %FF: masked, but a complete
    // escape left in the published text is not plain.
    assert_eq!(
        r.redact(format!(r#"{{"s":"{BEARER} 100%FF"}}"#).as_bytes()),
        Err(Refusal::InvalidArgumentEncoding)
    );
    // A literal hiding behind a malformed escape in a member name.
    let key = format!(r#"{{"%42{}%zz":1,"b":2}}"#, &BEARER[1..]);
    assert_eq!(
        r.redact(key.as_bytes()).as_deref(),
        Ok(r#"{"[REDACTED]%zz":1,"b":2}"#)
    );
    assert_eq!(percent_lenient(b"%4%41%g1%"), b"%4A%g1%");
    assert_eq!(percent_fixed("%2541").as_deref(), Some("A"));
    assert_eq!(percent_fixed("%FF"), None);
    // Two names that collide once masked are ambiguous.
    assert_eq!(
        r.redact(format!(r#"{{"{BEARER}":1,"{SENTINEL}":2}}"#).as_bytes()),
        Err(Refusal::InvalidArgumentEncoding)
    );
    for bad in [
        &br#"{"a":1,"a":2}"#[..],
        b"{\"a\":\"\xff\"}",
        br#"not json"#,
    ] {
        assert_eq!(r.redact(bad), Err(Refusal::InvalidArgumentEncoding));
    }
    // Exactly at the cap is kept; one byte over is refused.
    let at = format!(r#"{{"s":"{}"}}"#, "x ".repeat((VALUE_LIMIT - 8) / 2));
    assert_eq!(at.len(), VALUE_LIMIT);
    assert!(r.redact(at.as_bytes()).is_ok());
    let over = format!(r#"{{"s":"{}x"}}"#, "x ".repeat((VALUE_LIMIT - 8) / 2));
    assert_eq!(r.redact(over.as_bytes()), Err(Refusal::ValueTooLarge));
    // A redacted encoding over the cap: a short literal under a long mask.
    let mut short = Registry::new();
    short.register("a");
    let grows = format!(r#"{{"s":"{}"}}"#, "ab".repeat(3000));
    assert!(grows.len() <= VALUE_LIMIT);
    assert_eq!(short.redact(grows.as_bytes()), Err(Refusal::ValueTooLarge));
    // Nesting too deep to parse inside a nested string: the literal's
    // escaped spelling there cannot be masked as text and is refused.
    let escaped = format!("\\\\u0042{}", &BEARER[1..]);
    let deep = format!(
        "{{\"s\":\"{}\\\"{}\\\"{}\"}}",
        "[".repeat(DEPTH_LIMIT),
        escaped,
        "]".repeat(DEPTH_LIMIT)
    );
    assert_eq!(
        r.redact(deep.as_bytes()),
        Err(Refusal::InvalidArgumentEncoding)
    );
    // Shallow enough to parse: redacted structurally instead.
    let shallow = deep.replacen('[', "", 2).replacen(']', "", 2);
    assert!(r.redact(shallow.as_bytes()).unwrap().contains(MASK));
    assert_eq!(
        unescape_lenient("a\\q\\u00zz\\ud83d\\ude00\\udc00\\ud800x\\u0041"),
        "a\\q\\u00zz\u{1f600}\\udc00\\ud800xA"
    );
}

#[test]
fn a_decoded_mask_is_refused_without_a_literal() {
    // No registered literal anywhere: the mask alone, in any supported
    // decoding, in a value or in a member name, refuses the value.
    let r = reg();
    let spellings = [
        "%5BREDACTED%5D",
        "x %5BREDACTED%5D y",
        "%255BREDACTED%255D",
        "%5bREDACTED%5d",
        "\\u005bREDACTED\\u005d",
        "%5Cu005bREDACTED]",
        r#"{"b":"%5BREDACTED%5D"}"#,
        r#"[{"%5BREDACTED%5D":1}]"#,
    ];
    for spelled in spellings {
        let quoted = quote(spelled);
        for doc in [
            format!(r#"{{"a":{quoted}}}"#),
            format!(r#"{{{quoted}:1}}"#),
            format!(r#"{{"n":{{"k":[{quoted}]}}}}"#),
        ] {
            assert_eq!(
                r.redact(doc.as_bytes()),
                Err(Refusal::InvalidArgumentEncoding),
                "{doc}"
            );
        }
    }
    // Near misses are ordinary text and pass through unchanged.
    for plain in ["[REDACTE D]", "[redacted]", "REDACTED"] {
        let doc = format!(r#"{{"a":{}}}"#, quote(plain));
        assert_eq!(r.redact(doc.as_bytes()).as_deref(), Ok(doc.as_str()));
    }
}

#[test]
fn a_numeric_sentinel_is_found_by_value_in_any_spelling() {
    let mut r = reg();
    r.register("150");
    // As a JSON number, nested or not, and in a nested JSON string.
    for spelling in [
        "150", "1.5e2", "1.50e2", "15.0e1", "150.0", "1500e-1", "0.15E3", "1.5e+2",
    ] {
        for doc in [
            format!(r#"{{"n":{spelling}}}"#),
            format!(r#"{{"a":[{{"n":{spelling}}}]}}"#),
            format!(r#"{{"s":"{{\"n\":{spelling}}}"}}"#),
            // In text: the decision is that a numeric sentinel is found by
            // value in any string, in any layer, and refused; where its own
            // spelling stands in the text it is masked as usual.
            format!(r#"{{"s":"limit={spelling}&x"}}"#),
        ] {
            let got = r.redact(doc.as_bytes());
            if spelling.contains("150") && doc.contains("limit=") {
                // The registered spelling inside another one is masked.
                let masked = doc.replace("150", MASK);
                assert_eq!(got.as_deref(), Ok(masked.as_str()));
            } else {
                assert_eq!(got, Err(Refusal::ResidualSecret), "{doc}");
            }
        }
    }
    // With a sign written before it in text, and percent-encoded (not
    // plain, so refused before the value is even read).
    assert_eq!(
        r.redact(br#"{"s":"id-1.5e2"}"#),
        Err(Refusal::ResidualSecret)
    );
    assert_eq!(
        r.redact(br#"{"s":"%31.5e2"}"#),
        Err(Refusal::InvalidArgumentEncoding)
    );
    // Other values are kept, whatever their spelling.
    for doc in [
        r#"{"n":151}"#,
        r#"{"n":1.5e1}"#,
        r#"{"n":-1.5e2}"#,
        r#"{"n":0}"#,
        r#"{"s":"1.5e1 or 151"}"#,
    ] {
        assert_eq!(r.redact(doc.as_bytes()).as_deref(), Ok(doc), "{doc}");
    }
    // A negative sentinel, and zero in any sign.
    let mut n = Registry::new();
    n.register("-2.5");
    n.register("0");
    assert_eq!(n.redact(br#"{"n":-25e-1}"#), Err(Refusal::ResidualSecret));
    assert_eq!(n.redact(br#"{"n":-0.0e5}"#), Err(Refusal::ResidualSecret));
    assert_eq!(n.redact(br#"{"n":3.5}"#).as_deref(), Ok(r#"{"n":3.5}"#));
    // A number whose digits hold a numeric sentinel's is refused whatever
    // its value: no spelling question remains.
    let mut d = Registry::new();
    d.register("1.5");
    for doc in [&br#"{"n":2.15}"#[..], br#"{"n":-1150e9}"#, br#"{"n":15}"#] {
        assert_eq!(d.redact(doc), Err(Refusal::ResidualSecret));
    }
    assert_eq!(d.redact(br#"{"n":1.6}"#).as_deref(), Ok(r#"{"n":1.6}"#));
    // A leading or trailing dot is read in text: 0.5 as .5, -.5, .5e1 (5,
    // not 0.5, so kept apart), 5.e-1, and after another fraction.
    let mut h = Registry::new();
    h.register("0.5");
    for text in [".5", "x -.5", "5.e-1", "7.1.5", "id=.05e1"] {
        let doc = format!(r#"{{"s":"{text}"}}"#);
        assert_eq!(
            h.redact(doc.as_bytes()),
            Err(Refusal::ResidualSecret),
            "{doc}"
        );
    }
    assert!(h.redact(br#"{"s":".6"}"#).is_ok());
    // Every literal that reads as a decimal number is matched by value
    // and digit string, not only a strict JSON number: leading zeros,
    // surrounding whitespace, a leading '+' and a trailing dot.
    for (literal, doc) in [
        ("0150", &br#"{"n":150}"#[..]),
        ("0150", br#"{"s":"limit 1.5e2"}"#),
        (" 7", br#"{"n":7.0}"#),
        ("+42", br#"{"n":4.2e1}"#),
        ("5.", br#"{"n":5}"#),
        ("0150", br#"{"n":31500}"#),
    ] {
        let mut t = Registry::new();
        t.register(literal);
        assert_eq!(t.redact(doc), Err(Refusal::ResidualSecret), "{literal:?}");
    }
    // A sentinel of the file minimum, all digits with a leading zero, is
    // caught spelled without it, as a number and in text.
    let zeroed = format!("0{}", "1234567890".repeat(4).get(..31).unwrap());
    assert_eq!(zeroed.len(), SENTINEL_MIN_BYTES);
    let mut z = Registry::new();
    z.register(&zeroed);
    let bare = &zeroed[1..];
    for doc in [
        format!(r#"{{"n":{bare}}}"#),
        format!(r#"{{"s":"id {bare}"}}"#),
        format!(r#"{{"n":{bare}.0}}"#),
    ] {
        assert_eq!(
            z.redact(doc.as_bytes()),
            Err(Refusal::ResidualSecret),
            "{doc}"
        );
    }
    // Embedded in a longer number in text, with digits before and after:
    // the digit string is checked there too, signed and unsigned.
    for doc in [
        format!(r#"{{"s":"id 3{bare}9"}}"#),
        format!(r#"{{"s":"id 3{bare}"}}"#),
        format!(r#"{{"s":"id {bare}9"}}"#),
        format!(r#"{{"s":"id -3{bare}9.5"}}"#),
        format!(r#"{{"s":"n=7.{bare}e3"}}"#),
    ] {
        assert_eq!(
            z.redact(doc.as_bytes()),
            Err(Refusal::ResidualSecret),
            "{doc}"
        );
    }
    // Text that is not a number is matched by spelling only.
    let mut w = Registry::new();
    w.register("15 apples");
    assert_eq!(w.redact(br#"{"n":15}"#).as_deref(), Ok(r#"{"n":15}"#));
    assert_eq!(numeric_reading("  "), None);
    assert_eq!(numeric_reading("0").map(|(_, d)| d), Some(b"0".to_vec()));
    // The row is checked by value too.
    assert!(r.found_in_document(br#"{"wall_time_ms":1.5e2}"#));
    assert_eq!(
        Decimal::of(b"1e99999999999999999999"),
        Decimal::of(b"1e99999999999999999998"),
        "a saturated exponent can only make values equal"
    );
    assert_eq!(Decimal::of(b".5e1"), Decimal::of(b"5"));
    assert_eq!(Decimal::of(b"5.e-1"), Decimal::of(b"0.5"));
    for bad in [&b""[..], b"-", b".", b".e1", b"1e", b"1e+", b"1x", b"--1"] {
        assert_eq!(Decimal::of(bad), None, "{:?}", String::from_utf8_lossy(bad));
    }
}

#[test]
fn a_form_encoded_space_never_hides_a_sentinel() {
    let mut r = reg();
    r.register("my secret");
    // The '+' reading of a form or query value: in a path, a query, a
    // form body, a nested JSON body, and percent-encoded on top.
    for doc in [
        r#"{"method":"GET","path":"/files/my+secret/x"}"#,
        r#"{"method":"GET","path":"/search?q=my+secret&page=2"}"#,
        r#"{"method":"POST","path":"/login","body":"user=a&pass=my+secret"}"#,
        r#"{"method":"POST","path":"/","body":"{\"q\":\"my+secret\"}"}"#,
        r#"{"my+secret":1}"#,
    ] {
        assert_eq!(
            r.redact(doc.as_bytes()),
            Err(Refusal::ResidualSecret),
            "{doc}"
        );
    }
    // Percent-encoded on top: a %XX left in published text is not plain.
    for doc in [
        r#"{"method":"GET","path":"/?q=my%2Bsecret"}"#,
        r#"{"method":"GET","path":"/?q=my%252Bsecret"}"#,
    ] {
        assert_eq!(
            r.redact(doc.as_bytes()),
            Err(Refusal::InvalidArgumentEncoding),
            "{doc}"
        );
    }
    // The literal and its %20 spelling are masked as before; a '+' that
    // reveals nothing is kept.
    assert_eq!(
        r.redact(br#"{"path":"/?q=my%20secret&r=my secret"}"#)
            .as_deref(),
        Ok(r#"{"path":"/?q=[REDACTED]&r=[REDACTED]"}"#)
    );
    let kept = r#"{"path":"/?q=a+b&c=1+1"}"#;
    assert_eq!(r.redact(kept.as_bytes()).as_deref(), Ok(kept));
    // A row with the form spelling is not approved for a transcript.
    assert!(r.found_in_document(br#"{"agent":"my+secret"}"#));
}

/// A trial-shaped bearer (32 alphanumerics) and a file-length sentinel.
const TRIAL_BEARER: &str = "Qz7pL2mXv9KdR4tYw8NcB1sHf6JgU3eA";

fn every_spelling(literal: &str) -> Vec<(&'static str, String)> {
    use base64::Engine as _;
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    let decimal: String = literal.bytes().map(|b| format!("&#{b};")).collect();
    let hex: String = literal.bytes().map(|b| format!("&#x{b:x};")).collect();
    let mixed: String = literal
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i % 2 == 0 {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect();
    let escaped: String = literal.bytes().map(|b| format!("\\\\u{b:04x}")).collect();
    vec![
        ("base64", STANDARD.encode(literal)),
        (
            "base64 in text",
            format!("Basic {}", STANDARD.encode(format!("u:{literal}"))),
        ),
        ("base64url", URL_SAFE_NO_PAD.encode(literal)),
        ("hex", literal.bytes().map(|b| format!("{b:02x}")).collect()),
        ("named entity", format!("&lt;{literal}&gt;")),
        ("decimal entity", decimal),
        ("hex entity", hex),
        ("upper case", literal.to_ascii_uppercase()),
        ("lower case", literal.to_ascii_lowercase()),
        ("mixed case", mixed),
        ("text-level \\u escape", escaped),
        (
            "fullwidth",
            literal
                .chars()
                .map(|c| char::from_u32(c as u32 + 0xFEE0).unwrap_or(c))
                .collect(),
        ),
    ]
}

#[test]
fn deny_by_default_refuses_every_unreadable_spelling() {
    let mut r = Registry::new();
    r.register(TRIAL_BEARER);
    r.register(FILE_SENTINEL);
    for literal in [TRIAL_BEARER, FILE_SENTINEL] {
        for (how, spelled) in every_spelling(literal) {
            for doc in [
                json!({"path": format!("/?t={spelled}")}),
                json!({"body": {"note": spelled}}),
                json!({ spelled.clone(): 1 }),
            ] {
                let bytes = doc.to_string();
                let got = r.redact(bytes.as_bytes());
                // A spelling that is the literal itself is masked; every
                // other one is refused.
                let ok = match &got {
                    Err(Refusal::InvalidArgumentEncoding | Refusal::ResidualSecret) => true,
                    Ok(out) => spelled == literal && !r.found_folded(out.as_bytes()),
                    Err(_) => false,
                };
                assert!(ok, "{how} of {literal:?} was {got:?}");
            }
        }
    }
    // The case-folded spelling is a residual literal, not a shape.
    let upper = json!({"q": TRIAL_BEARER.to_ascii_uppercase()}).to_string();
    assert_eq!(r.redact(upper.as_bytes()), Err(Refusal::ResidualSecret));
    // A leading-dot numeric spelling of a numeric sentinel.
    let mut n = Registry::new();
    n.register("0.5");
    assert_eq!(
        n.redact(br#"{"s":"ratio .5e0"}"#),
        Err(Refusal::ResidualSecret)
    );
    // Each shape rule on its own, with no literal in sight.
    for bad in [
        "caf\u{e9}",
        "tab\there",
        "\u{2713}",
        "a\\u0041",
        "a\\x41",
        "100%41",
        "&amp;",
        "&#65;",
        "&#x41;",
        &"A".repeat(OPAQUE_RUN_LIMIT),
        &format!("{}+-_", "a".repeat(OPAQUE_RUN_LIMIT - 3)),
    ] {
        assert!(!plain(bad), "{bad:?}");
        let doc = json!({ "s": bad }).to_string();
        assert_eq!(
            r.redact(doc.as_bytes()),
            Err(Refusal::InvalidArgumentEncoding),
            "{bad:?}"
        );
    }
    // The fixture's synthetic data is plain and publishes unchanged: an
    // inbox, a subject, a mailed link's 24-character token, a Swarmail
    // message id (a UUID, the longest run: 36), a code, a 16-character
    // password, a form body, a query with '&' that is no entity.
    let synthetic = json!({
        "inbox": "ada.b57b4@example.test",
        "subject": "Invitation: [Ticket #4821] Autumn sale!",
        "path": "/a/Xk3P9qLm2Vn8RtY4Wz6Bc1Df",
        "link": "http://127.0.0.1:41235/a/Xk3P9qLm2Vn8RtY4Wz6Bc1Df",
        "id": "0193a5f4-1c2b-7def-8abc-0123456789ab",
        "code": "482913",
        "password": "Hq4ZtP8mWc2LxN7v",
        "body": {"token": "Xk3P9qLm2Vn8RtY4Wz6Bc1Df", "uid": 3, "start": 0.5},
        "form": "user=ada&pass=hunter2",
        "reply": "The export button does nothing when I click it."
    })
    .to_string();
    assert_eq!(
        r.redact(synthetic.as_bytes()).as_deref(),
        Ok(synthetic.as_str())
    );
    assert!(plain(&"A".repeat(OPAQUE_RUN_LIMIT - 1)));
    // One well-formed eval_http argument per route the fixture advertises
    // (fixture.rs `index`, and the invitation's `ics_url`), with the real
    // shapes: 24-character tokens, a 16-character password, a 12-character
    // ICS UID, a 5-digit ticket, a word-and-number challenge.
    let token = "Xk3P9qLm2Vn8RtY4Wz6Bc1Df";
    for route in [
        json!({"method": "GET", "path": format!("/a/{token}")}),
        json!({"method": "POST", "path": "/otp/verify", "body": {"code": "482913"}}),
        json!({"method": "POST", "path": "/otp/resend", "body": {}}),
        json!({"method": "POST", "path": "/reset/complete", "body": {"token": token, "password": "Hq4ZtP8mWc2LxN7v"}}),
        json!({"method": "POST", "path": "/login", "body": {"password": "Hq4ZtP8mWc2LxN7v"}}),
        json!({"method": "GET", "path": format!("/invite/{token}/event.ics")}),
        json!({"method": "GET", "path": format!("http://127.0.0.1:41235/invite/{token}/event.ics")}),
        json!({"method": "POST", "path": "/invite/accept", "body": {"token": token, "uid": "Ab3dEf6hIj9k@fixture.eval", "start": "20261026T153000Z"}}),
        json!({"method": "POST", "path": "/support/reply", "body": {"ticket": "48213", "challenge": "harbor-4821", "reply": "The export button does nothing when I click it."}}),
        json!({"method": "POST", "path": "/deliveries/release", "body": {}}),
        json!({"method": "GET", "path": "/newsletter"}),
        json!({"method": "POST", "path": "/newsletter", "body": {"subject": "Autumn sale", "html": "<p>Hello <a href=\"https://example.test/x\">there</a></p>"}}),
    ] {
        let doc = route.to_string();
        assert_eq!(
            r.redact(doc.as_bytes()).as_deref(),
            Ok(doc.as_str()),
            "{doc}"
        );
    }
}

/// Quoted-printable, every byte escaped (`=73=65...`), in either case.
fn qp_all(bytes: &[u8], upper: bool) -> String {
    bytes
        .iter()
        .map(|b| {
            if upper {
                format!("={b:02X}")
            } else {
                format!("={b:02x}")
            }
        })
        .collect()
}

/// Hex pairs joined by `sep`.
fn hex_joined(bytes: &[u8], sep: &str) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(sep)
}

/// `text` cut into chunks of `width` characters joined by `sep`.
fn chunked(text: &str, width: usize, sep: &str) -> String {
    text.as_bytes()
        .chunks(width)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(sep)
}

/// The base64, base64url and base32 spellings a chunked search decodes.
fn base_spellings(bytes: &[u8]) -> Vec<String> {
    use base64::Engine as _;
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    vec![
        STANDARD.encode(bytes),
        STANDARD_NO_PAD.encode(bytes),
        URL_SAFE.encode(bytes),
        URL_SAFE_NO_PAD.encode(bytes),
        base32(bytes),
        base32(bytes).to_ascii_lowercase(),
    ]
}

#[test]
fn chunked_base64_and_base32_are_found_by_decoding() {
    let mut r = Registry::new();
    r.register(TRIAL_BEARER);
    r.register(FILE_SENTINEL);
    let mut checked = 0;
    for literal in [TRIAL_BEARER, FILE_SENTINEL] {
        for prefix in [&b""[..], b"user:", b"\xff"] {
            let bytes = [prefix, literal.as_bytes()].concat();
            for encoded in base_spellings(&bytes) {
                for width in [4, 20, 33] {
                    for sep in [
                        " ", ".", ",", ", ", " | ", ":", ";", "/", "!", "#", "?", "&", "'", "(",
                        ")", "\"", "*", "~", "@", "$", "^",
                    ] {
                        let spelled = chunked(&encoded, width, sep);
                        // Every joiner as a value; a space and a slash also
                        // as a key, in a path and after `Basic`.
                        let mut docs = vec![json!({"note": spelled})];
                        if sep == " " || sep == "/" {
                            docs.extend([
                                json!({ spelled.clone(): 1 }),
                                json!({"path": format!("/x/{spelled}/y")}),
                                json!({"auth": format!("Basic {spelled}")}),
                            ]);
                        }
                        for doc in docs {
                            let got = r.redact(doc.to_string().as_bytes());
                            assert_eq!(got, Err(Refusal::ResidualSecret), "{doc}");
                            checked += 1;
                        }
                    }
                    // A newline between chunks is a control character: not
                    // plain, refused before the decoding is even read.
                    let lines = chunked(&encoded, width, "\n");
                    assert_eq!(
                        r.redact(json!({"note": lines}).to_string().as_bytes()),
                        Err(Refusal::InvalidArgumentEncoding)
                    );
                }
            }
        }
    }
    assert_eq!(checked, 2 * 3 * 6 * 3 * (21 + 2 * 3));
    // Joiners that are themselves base64 characters, mixed: '/' and '-'
    // together break both the standard and the URL reading (a '+' would be
    // read as a space), and only the letters-and-digits candidate decodes
    // them.
    use base64::Engine as _;
    let std64 = |b: &[u8]| base64::engine::general_purpose::STANDARD.encode(b);
    for literal in [TRIAL_BEARER, FILE_SENTINEL] {
        let mixed = chunked(&chunked(&std64(literal.as_bytes()), 4, "/"), 9, " -");
        let doc = json!({ "note": mixed }).to_string();
        assert_eq!(
            r.redact(doc.as_bytes()),
            Err(Refusal::ResidualSecret),
            "{doc}"
        );
    }
    // A suffix byte that turns the seam character (the one holding both
    // the literal's last bits and the suffix's first) into a '/': a '/'
    // joiner then deletes it with those bits, and only the literal's core
    // (its edge bytes dropped) is left to find. The sentinel is chosen so
    // its last byte ('o', low bits 11) can reach the seam.
    let edge = "file sentinel canary 0123456789abcdeo";
    let mut e = Registry::new();
    e.register(edge);
    let bits = edge.len() * 8;
    assert_ne!(bits % 6, 0, "the last character is shared with the suffix");
    let mut seams = 0;
    for suffix in 0u8..=255 {
        let encoded = std64(&[edge.as_bytes(), &[suffix, 0xff]].concat());
        seams += usize::from(encoded.as_bytes()[(bits - 1) / 6] == b'/');
        let doc = json!({ "note": chunked(&encoded, 4, "/") }).to_string();
        assert_eq!(
            e.redact(doc.as_bytes()),
            Err(Refusal::ResidualSecret),
            "{doc}"
        );
    }
    assert_eq!(seams, 16, "suffixes 0xf0..=0xff make the seam a '/'");
    // Near misses publish: prose, URLs, a UUID, encodings of other bytes,
    // and the fixture's routes (in deny_by_default_refuses_every_...).
    let other = base64::engine::general_purpose::STANDARD.encode([7u8; 48]);
    for kept in [
        json!({"reply": "The export button does nothing when I click it, and the settings page never finishes loading either."}),
        json!({"link": "http://127.0.0.1:41235/a/Xk3P9qLm2Vn8RtY4Wz6Bc1Df"}),
        json!({"id": "0193a5f4-1c2b-7def-8abc-0123456789ab"}),
        json!({"note": chunked(&other, 20, " ")}),
        json!({"note": chunked(&base32(&[7u8; 40]), 8, ".")}),
    ] {
        let doc = kept.to_string();
        assert_eq!(
            r.redact(doc.as_bytes()).as_deref(),
            Ok(doc.as_str()),
            "{doc}"
        );
    }
    // The decoder itself, and its bound: past it a text counts as holding
    // a literal rather than being published unread.
    assert_eq!(pack(&[16, 22, 20], 6, 0), b"Ae");
    // Skipping the first two bits of "QWU" ("Ae") realigns on what follows.
    assert_eq!(pack(&[16, 22, 20], 6, 2), [0b0000_0101, 0b1001_0100]);
    assert!(r.found_in_encodings(&vec![b'A'; ENCODED_SCAN_LIMIT + 1]));
    assert!(!r.found_in_encodings(&vec![b'A'; ENCODED_SCAN_LIMIT]));
    assert!(!Registry::new().found_in_encodings(b"anything"));
    // No false positive on real prose: every task's goal and template, the
    // oracle's replies and every line of this harness's own README decode
    // to bytes that hold no literal.
    let mut prose: Vec<String> = crate::tasks::catalogue()
        .into_iter()
        .flat_map(|t| [t.goal, t.goal_template])
        .collect();
    prose.push("The export button does nothing when I click it.".into());
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap();
    prose.extend(readme.lines().map(str::to_string));
    assert!(prose.len() > 1000, "{}", prose.len());
    for text in &prose {
        assert!(!r.found_in_encodings(text.as_bytes()), "{text}");
    }
    // A whole README is one long candidate too, past the per-string cap:
    // it counts as holding a literal rather than being decoded unread.
    assert!(readme.len() > ENCODED_SCAN_LIMIT);
    assert!(r.found_in_encodings(readme.as_bytes()));
}

#[test]
fn quoted_printable_and_separator_joined_hex_are_refused() {
    let mut r = Registry::new();
    r.register(TRIAL_BEARER);
    r.register(FILE_SENTINEL);
    for literal in [TRIAL_BEARER, FILE_SENTINEL] {
        let b = literal.as_bytes();
        let mut spellings = vec![
            ("qp", qp_all(b, true)),
            ("qp lower", qp_all(b, false)),
            (
                "qp soft breaks",
                qp_all(b, true)
                    .as_bytes()
                    .chunks(12)
                    .map(|c| String::from_utf8_lossy(c).into_owned())
                    .collect::<Vec<_>>()
                    .join("=\n"),
            ),
        ];
        for sep in ["/", ":", " ", ",", "\n", "-", ".", "=", ", "] {
            spellings.push(("hex joined", hex_joined(b, sep)));
            spellings.push(("HEX joined", hex_joined(b, sep).to_ascii_uppercase()));
        }
        for (how, spelled) in spellings {
            for doc in [
                json!({"note": spelled}),
                json!({ spelled.clone(): 1 }),
                json!({"path": format!("/x/{spelled}/y")}),
            ] {
                let got = r.redact(doc.to_string().as_bytes());
                assert!(
                    matches!(
                        got,
                        Err(Refusal::InvalidArgumentEncoding | Refusal::ResidualSecret)
                    ),
                    "{how} {spelled:?} of {literal:?}: {got:?}"
                );
            }
        }
    }
    // Each rule on its own, where no other one would catch the spelling.
    // The '=' counts in a run: base64-like text joined by one '='.
    let joined = "abcdefghijklmnopqr=stuvwxyzGHIJKLMNOP";
    assert_eq!(joined.len(), OPAQUE_RUN_LIMIT);
    assert!(!plain(joined));
    // Two quoted-printable escapes in a row.
    assert!(!plain("x=41=42y"));
    // Separator-joined hex: 64 digits refused, 63 kept, a non-hex letter
    // breaks the count.
    let digits = "0123456789abcdef".repeat(4);
    let spaced = |d: &str| {
        d.as_bytes()
            .chunks(2)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect::<Vec<_>>()
            .join(":")
    };
    assert!(!plain(&spaced(&digits)));
    assert!(plain(&spaced(&digits[..63])));
    assert!(plain(&format!(
        "{} g {}",
        spaced(&digits[..40]),
        spaced(&digits[..40])
    )));
    // A sparse quoted-printable escape inside a sentinel with spaces: no
    // shape rule sees it; the quoted-printable decoding finds the literal.
    let sparse = FILE_SENTINEL.replacen("canary", "=63anary", 1);
    assert!(plain(&sparse));
    assert_eq!(
        r.redact(json!({"s": sparse}).to_string().as_bytes()),
        Err(Refusal::ResidualSecret)
    );
    assert_eq!(quoted_printable_lenient(b"a=41=\nb=\r\nc=zz"), b"aAbc=zz");
    // Near misses publish: one UUID, a form with a value that starts with
    // two hex digits, a token query, prose of hex-letter words.
    for kept in [
        json!({"id": "0193a5f4-1c2b-7def-8abc-0123456789ab"}),
        json!({"path": "/search?page=10&start=20261026T153000Z"}),
        json!({"path": "/a/?token=4BXk3P9qLm2Vn8RtY4Wz6Bc1"}),
        json!({"form": "token=4BXk3P9qLm2Vn8RtY4Wz6Bc1&code=482913"}),
        json!({"reply": "deadbeef cafe is a fine name; add a bead to the face of a babe"}),
        json!({"digits": spaced(&digits[..63])}),
    ] {
        let doc = kept.to_string();
        assert_eq!(
            r.redact(doc.as_bytes()).as_deref(),
            Ok(doc.as_str()),
            "{doc}"
        );
    }
    // Two UUIDs side by side are 64 hex digits: refused, the accepted cost.
    let two =
        json!({"ids": "0193a5f4-1c2b-7def-8abc-0123456789ab 0193a5f4-1c2b-7def-8abc-0123456789ab"});
    assert_eq!(
        r.redact(two.to_string().as_bytes()),
        Err(Refusal::InvalidArgumentEncoding)
    );
}

/// RFC 4648 base32, padded: the base64 crate has no base32 engine.
fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    for chunk in bytes.chunks(5) {
        let mut block = [0u8; 5];
        block[..chunk.len()].copy_from_slice(chunk);
        let bits = block.iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b));
        let chars = (chunk.len() * 8).div_ceil(5);
        for i in 0..8 {
            if i < chars {
                out.push(ALPHABET[((bits >> (35 - 5 * i)) & 31) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[test]
fn every_long_encoding_of_a_sentinel_holds_one_unbroken_run() {
    use base64::Engine as _;
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    assert_eq!(base32(b"foobar"), "MZXW6YTBOI======");
    // Every byte a sentinel may hold, in pseudo-random 32-byte literals
    // (a fixed xorshift, so the run is the same every time), and the
    // extremes; each behind every alignment, with prefix and suffix bytes
    // an agent chooses freely (0xff makes '/' and '+' at the seams).
    let admitted: Vec<u8> = (0x20..=0x7eu8).filter(|b| !b"?>~".contains(b)).collect();
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    let mut literals: Vec<Vec<u8>> = (0..48)
        .map(|_| {
            (0..SENTINEL_MIN_BYTES)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    admitted[(state % admitted.len() as u64) as usize]
                })
                .collect()
        })
        .collect();
    for b in *b" ~>/z0" {
        literals.push(vec![b; SENTINEL_MIN_BYTES]);
    }
    literals.push(TRIAL_BEARER.as_bytes().to_vec());
    let mut checked = 0;
    for literal in &literals {
        for prefix in [
            &b""[..],
            b"\xff",
            b"\xff\xff",
            b"\x00",
            b"ab",
            b"\xff\x00\xff\x00",
        ] {
            for suffix in [&b""[..], b"\xff", b"\xff\xff\xff"] {
                let bytes: Vec<u8> = [prefix, literal.as_slice(), suffix].concat();
                for encoded in [
                    STANDARD.encode(&bytes),
                    STANDARD_NO_PAD.encode(&bytes),
                    URL_SAFE.encode(&bytes),
                    URL_SAFE_NO_PAD.encode(&bytes),
                    base32(&bytes),
                    bytes.iter().map(|b| format!("{b:02x}")).collect(),
                    bytes.iter().map(|b| format!("{b:02X}")).collect(),
                    qp_all(&bytes, true),
                    qp_all(&bytes, false),
                    hex_joined(&bytes, ":"),
                    hex_joined(&bytes, "/"),
                    hex_joined(&bytes, " "),
                    hex_joined(&bytes, ","),
                ] {
                    for text in [encoded.clone(), format!("/a/{encoded}/x.ics?t={encoded}")] {
                        assert!(!plain(&text), "{text}");
                        checked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(checked, literals.len() * 6 * 3 * 13 * 2);
    // Chunked by separators, every encoding still holds the literal once
    // decoded: refused as a residual.
    // Over the literals a sentinel file admits (the extremes `~` and `>`
    // are refused there: they make standard base64 write `+`).
    let admitted_bytes = |l: &[u8]| l.iter().all(|b| !b"?>~".contains(b));
    let admitted_literals = literals.iter().filter(|l| admitted_bytes(l)).count();
    assert!(admitted_literals >= 50, "{admitted_literals}");
    let mut decoded = 0;
    for literal in literals.iter().filter(|l| admitted_bytes(l)) {
        let mut r = Registry::new();
        r.register(std::str::from_utf8(literal).unwrap());
        for prefix in [&b""[..], b"\xff"] {
            let bytes = [prefix, literal.as_slice()].concat();
            for encoded in base_spellings(&bytes) {
                for (width, sep) in [(5, " "), (9, "/"), (11, "!"), (13, "(")] {
                    let doc = json!({ "s": chunked(&encoded, width, sep) }).to_string();
                    assert_eq!(
                        r.redact(doc.as_bytes()),
                        Err(Refusal::ResidualSecret),
                        "{doc}"
                    );
                    decoded += 1;
                }
            }
        }
    }
    assert_eq!(decoded, admitted_literals * 2 * 6 * 4);
    // Why '?' and non-ASCII bytes are refused in a sentinel: standard
    // base64 then writes '/' inside the literal's own characters, and the
    // separators break every run short of the limit.
    assert!(plain(&STANDARD.encode("?".repeat(SENTINEL_MIN_BYTES))));
    assert!(plain(
        &STANDARD.encode("\u{ff}".repeat(SENTINEL_MIN_BYTES / 2))
    ));
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("s.json");
    for refused in [
        format!("{}?", FILE_SENTINEL),
        format!("{}>", FILE_SENTINEL),
        format!("{}~", FILE_SENTINEL),
        format!("{}\u{ff}", FILE_SENTINEL),
        format!("{}\t", FILE_SENTINEL),
    ] {
        std::fs::write(&file, serde_json::to_vec(&[&refused]).unwrap()).unwrap();
        assert_eq!(read_sentinels(&file), Err(Invalid), "{refused:?}");
    }
}

#[test]
fn the_registry_is_bounded_and_fails_closed() {
    let mut r = Registry::new();
    r.register("");
    r.register("a");
    r.register("a");
    assert_eq!(r.len(), 1);
    for i in 1..REGISTRY_LITERALS {
        r.register(&format!("lit{i}"));
    }
    assert!(r.is_complete() && r.len() == REGISTRY_LITERALS);
    r.register("one-too-many");
    assert!(!r.is_complete());
    let mut big = Registry::new();
    big.register(&"x".repeat(REGISTRY_BYTES));
    assert!(big.is_complete());
    big.register("y");
    assert!(!big.is_complete());
    assert!(format!("{big:?}").contains("complete: false"));
    assert!(
        !format!("{r:?}").contains("lit1"),
        "Debug never prints a literal"
    );

    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("s.json");
    std::fs::write(&good, serde_json::to_vec(&[FILE_SENTINEL]).unwrap()).unwrap();
    let snapshot = read_sentinels(&good).ok();
    let built = sealed_registry(BEARER, snapshot.as_deref(), &good);
    assert!(built.is_complete() && built.len() == 2);
    // A file that cannot be read when the trial starts: no snapshot, so
    // the registry is unusable.
    for bad in [&b"[\"\"]"[..], b"{}", b"nope"] {
        std::fs::write(&good, bad).unwrap();
        assert_eq!(read_sentinels(&good), Err(Invalid));
        assert!(!sealed_registry(BEARER, None, &good).is_complete());
    }
    // The file holds at most 255 sentinels: with the bearer that fills the
    // registry exactly, and a trial still seals complete. 256 are refused.
    let full: Vec<String> = (0..SENTINEL_LIMIT).map(long).collect();
    std::fs::write(&good, serde_json::to_vec(&full).unwrap()).unwrap();
    let at = read_sentinels(&good).unwrap();
    assert_eq!(at.len(), 255);
    let sealed = sealed_registry(BEARER, Some(&at), &good);
    assert!(sealed.is_complete() && sealed.len() == REGISTRY_LITERALS);
    let many: Vec<String> = (0..=SENTINEL_LIMIT).map(long).collect();
    assert_eq!(many.len(), 256);
    std::fs::write(&good, serde_json::to_vec(&many).unwrap()).unwrap();
    assert!(read_sentinels(&good).is_err());
    // A sentinel shorter than the minimum is refused with the file: its
    // base64 spelling would be too short to be told from fixture data.
    let short = "x".repeat(SENTINEL_MIN_BYTES - 1);
    std::fs::write(&good, serde_json::to_vec(&[short]).unwrap()).unwrap();
    assert!(read_sentinels(&good).is_err());
    std::fs::write(&good, serde_json::to_vec(&[long(1)]).unwrap()).unwrap();
    assert_eq!(read_sentinels(&good), Ok(vec![long(1)]));
    // A numeric sentinel needs 16 significant digits: one digit padded
    // with zeros, or 15 padded with spaces, is refused; 16 are admitted.
    for (sentinel, ok) in [
        (format!("{}1", "0".repeat(31)), false),
        (format!("{:>32}", "123456789012345"), false),
        // Trailing zeros are not significant: the value is 1.
        (format!("1.{}", "0".repeat(30)), false),
        (format!("{}0000000000000000", "1234567890123456"), true),
        (format!("{}1234567890123456", "0".repeat(16)), true),
        (format!("{:>32}", "1234567890123456"), true),
    ] {
        assert_eq!(sentinel.len(), SENTINEL_MIN_BYTES);
        std::fs::write(&good, serde_json::to_vec(&[&sentinel]).unwrap()).unwrap();
        assert_eq!(read_sentinels(&good).is_ok(), ok, "{sentinel:?}");
    }
    assert!(read_sentinels(&dir.path().join("absent")).is_err());
    assert!(read_sentinels(Path::new("/")).is_err());
}

#[test]
fn the_registry_only_grows_during_a_trial() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("s.json");
    std::fs::write(&file, serde_json::to_vec(&[FILE_SENTINEL]).unwrap()).unwrap();
    let snapshot = read_sentinels(&file).unwrap();
    let late = LATE_SENTINEL;
    let leak = format!(r#"{{"a":"{FILE_SENTINEL}","b":"{late}"}}"#);
    // Rewritten to [] before seal: the snapshot keeps the sentinel.
    std::fs::write(&file, b"[]").unwrap();
    let r = sealed_registry(BEARER, Some(&snapshot), &file);
    assert!(r.is_complete() && r.len() == 2);
    assert_eq!(
        r.redact(leak.as_bytes()).as_deref(),
        Ok(format!(r#"{{"a":"[REDACTED]","b":"{late}"}}"#).as_str())
    );
    // A sentinel registered mid-trial is honoured beside the snapshot.
    std::fs::write(&file, serde_json::to_vec(&[late]).unwrap()).unwrap();
    let r = sealed_registry(BEARER, Some(&snapshot), &file);
    assert_eq!(r.len(), 3);
    assert_eq!(
        r.redact(leak.as_bytes()).as_deref(),
        Ok(r#"{"a":"[REDACTED]","b":"[REDACTED]"}"#)
    );
    // Unreadable or malformed at seal: the registry is unusable, never the
    // snapshot alone (a late sentinel may be in the broken file).
    let c = Capture::new(catalogue());
    http(&c, leak.as_bytes());
    for broken in [
        format!(r#"["{late}""#).into_bytes(),
        format!(r#"["{late}", 1]"#).into_bytes(),
        format!(r#"["{late}", ""]"#).into_bytes(),
        b"nope".to_vec(),
    ] {
        std::fs::write(&file, &broken).unwrap();
        let r = sealed_registry(BEARER, Some(&snapshot), &file);
        assert!(!r.is_complete(), "{}", String::from_utf8_lossy(&broken));
        let rec = c.seal(&id(1), &r, b"{}");
        assert_eq!(reason(&rec), Some(Refusal::RegistryUnavailable));
        assert_eq!(rec.row_sha256, None);
    }
    std::fs::remove_file(&file).unwrap();
    assert!(!sealed_registry(BEARER, Some(&snapshot), &file).is_complete());
    // No snapshot at trial start: unusable whatever the file holds now.
    std::fs::write(&file, serde_json::to_vec(&[FILE_SENTINEL]).unwrap()).unwrap();
    assert!(!sealed_registry(BEARER, None, &file).is_complete());
    // The union is still capped: past it the registry is unusable.
    let many: Vec<String> = (0..SENTINEL_LIMIT).map(long).collect();
    std::fs::write(&file, serde_json::to_vec(&many).unwrap()).unwrap();
    assert!(!sealed_registry(BEARER, Some(&snapshot), &file).is_complete());
}

fn id(n: u32) -> String {
    format!("{n:032x}")
}

fn seal_one(capture: &Capture) -> CaptureRecord {
    capture.seal(&id(1), &reg(), b"{\"row\":1}")
}

fn reason(record: &CaptureRecord) -> Option<Refusal> {
    match record.outcome {
        CaptureOutcome::Refused { reason } => Some(reason),
        CaptureOutcome::Complete { .. } => None,
    }
}

fn http(c: &Capture, args: &[u8]) {
    drop(c.observe(Server::Fixture, "eval_http", args));
}

#[test]
fn capture_latches_one_reason_and_keeps_nothing_after_it() {
    // An ordinary sequence across both servers, in order.
    let c = Capture::new(catalogue());
    drop(c.observe(
        Server::Swarmail,
        "swarmail_wait_for_email",
        br#"{"inbox":"a"}"#,
    ));
    http(&c, br#"{"method":"GET","path":"/"}"#);
    let rec = seal_one(&c);
    let CaptureOutcome::Complete { calls } = &rec.outcome else {
        panic!("{rec:?}")
    };
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].argument_sha256, sha256(br#"{"inbox":"a"}"#));
    assert_eq!(
        rec.row_sha256.as_deref(),
        Some(sha256(b"{\"row\":1}").as_str())
    );

    type Make = Box<dyn Fn(&Capture)>;
    let cases: Vec<(Make, Refusal)> = vec![
        (
            Box::new(|c| drop(c.observe(Server::Swarmail, "not_in_catalogue", b"{}"))),
            Refusal::UnknownTool,
        ),
        (
            Box::new(|c| drop(c.observe(Server::Fixture, "other", b"{}"))),
            Refusal::UnknownTool,
        ),
        (
            Box::new(|c| http(c, &vec![b' '; VALUE_LIMIT + 1])),
            Refusal::ValueTooLarge,
        ),
        (
            Box::new(|c| (0..=CALL_LIMIT).for_each(|_| http(c, b"{}"))),
            Refusal::TrialTooLarge,
        ),
        (
            Box::new(|c| (0..9).for_each(|_| http(c, &vec![b' '; VALUE_LIMIT]))),
            Refusal::TrialTooLarge,
        ),
        (
            Box::new(|c| {
                let _open = c.observe(Server::Fixture, "eval_http", b"{}");
                drop(c.observe(Server::Swarmail, "swarmail_get_email", b"{}"));
            }),
            Refusal::BindingMismatch,
        ),
        // The last call is the bad one: the whole record is withheld.
        (
            Box::new(|c| {
                http(c, b"{}");
                http(c, b"{\"a\":1,\"a\":1}");
            }),
            Refusal::InvalidArgumentEncoding,
        ),
        (
            Box::new(|c| {
                http(c, b"{}");
                c.refuse(Refusal::ResidualSecret);
            }),
            Refusal::ResidualSecret,
        ),
    ];
    for (i, (make, want)) in cases.into_iter().enumerate() {
        let c = Capture::new(catalogue());
        make(&c);
        assert_eq!(reason(&seal_one(&c)), Some(want), "case {i}");
    }
    // Exactly at the call and byte limits is kept.
    let c = Capture::new(catalogue());
    (0..CALL_LIMIT).for_each(|_| http(&c, b"{}"));
    assert_eq!(reason(&seal_one(&c)), None);
    let c = Capture::new(catalogue());
    let mut padded = vec![b' '; VALUE_LIMIT - 2];
    padded.extend_from_slice(b"{}");
    (0..8).for_each(|_| http(&c, &padded));
    assert_eq!(reason(&seal_one(&c)), None);
    // The lowest reason wins when several hold.
    let c = Capture::new(catalogue());
    c.refuse(Refusal::BindingMismatch);
    drop(c.observe(Server::Fixture, "nope", b"{}"));
    assert_eq!(reason(&seal_one(&c)), Some(Refusal::UnknownTool));
    // A residual found while sealing, and an incomplete registry first.
    let c = Capture::new(catalogue());
    let mut digits = reg();
    digits.register("777");
    http(&c, b"{\"n\":7770}");
    http(&c, b"{\"a\":1,\"a\":1}");
    assert_eq!(
        reason(&c.seal(&id(1), &digits, b"{}")),
        Some(Refusal::InvalidArgumentEncoding),
        "the earlier reason in the check order wins"
    );
    let c = Capture::new(catalogue());
    http(&c, b"{\"n\":7770}");
    assert_eq!(
        reason(&c.seal(&id(1), &digits, b"{}")),
        Some(Refusal::ResidualSecret)
    );
    digits.poison();
    let rec = c.seal(&id(1), &digits, b"{}");
    assert_eq!(reason(&rec), Some(Refusal::RegistryUnavailable));
    assert_eq!(rec.row_sha256, None);
    // A row holding a literal is never approved for a transcript.
    let c = Capture::new(catalogue());
    assert_eq!(c.seal(&id(1), &reg(), BEARER.as_bytes()).row_sha256, None);
}

/// An object of `n` distinct members, `{"k0":0,"k1":0,...}`.
fn wide_object(n: usize) -> String {
    let members: Vec<String> = (0..n).map(|i| format!(r#""k{i}":0"#)).collect();
    format!("{{{}}}", members.join(","))
}

#[test]
fn a_wide_or_long_body_is_read_in_linear_time() {
    // At the member cap an object is read; one member more is refused.
    assert!(parse(wide_object(MEMBER_LIMIT).as_bytes()).is_ok());
    assert_eq!(
        parse(wide_object(MEMBER_LIMIT + 1).as_bytes()),
        Err(Invalid)
    );
    // A duplicate name is still refused, found by the set.
    assert_eq!(parse(br#"{"a":1,"b":2,"a":3}"#), Err(Invalid));
    // A body near the forwarder's 2 MiB limit holding one object of some
    // 180 000 members, as a tools/call's arguments: refused as soon as the
    // cap is passed. The quadratic duplicate scan this replaces took 47.7 s
    // in a release build on such a body. The bound below is generous (a
    // debug build on a loaded shared host) and exists only to fail rather
    // than hang if the read ever turns quadratic again; the assertions
    // are on the results.
    let started = std::time::Instant::now();
    let wide = wide_object(180_000);
    assert!(
        wide.len() > 1_900_000 && wide.len() <= 2_097_152,
        "{}",
        wide.len()
    );
    let call = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"swarmail_get_email","arguments":{wide}}}}}"#
    );
    let c = Capture::new(catalogue());
    assert!(c.observe_mcp_body(call.as_bytes()).is_empty());
    assert_eq!(
        reason(&seal_one(&c)),
        Some(Refusal::InvalidArgumentEncoding)
    );
    // The same width without a tools/call is not a call: ignored.
    let c = Capture::new(catalogue());
    assert!(c.observe_mcp_body(wide.as_bytes()).is_empty());
    assert_eq!(reason(&seal_one(&c)), None);
    // A 200 000-digit run through the residual search, with a numeric
    // sentinel registered: the number scanner reads the run once (it once
    // started a number at every digit), and a text that long counts as
    // holding a literal rather than being decoded whole.
    let mut r = reg();
    r.register("1234567890123456");
    let digits = format!(r#"{{"s":"{}"}}"#, "7".repeat(200_000));
    assert!(r.found_in_document(digits.as_bytes()));
    assert!(!r.number_in_text("7".repeat(200_000).as_bytes()));
    let elapsed = started.elapsed();
    assert!(elapsed < std::time::Duration::from_secs(10), "{elapsed:?}");
}

#[test]
fn mcp_bodies_are_scanned_in_order_including_batches() {
    let c = Capture::new(catalogue());
    let body = br#"[{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_get_email","arguments": { "id" : "1" }}},{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","id":2,"method":"tools/list"}]"#;
    let started = c.observe_mcp_body(body);
    assert_eq!(started.len(), 1);
    drop(started);
    let rec = seal_one(&c);
    let CaptureOutcome::Complete { calls } = &rec.outcome else {
        panic!()
    };
    assert_eq!(calls[0].argument_bytes, br#"{ "id" : "1" }"#.len() as u64);
    assert_eq!(calls[0].value, r#"{"id":"1"}"#);
    // Two calls in one batch overlap: the trial is withheld.
    let c = Capture::new(catalogue());
    let two = br#"[{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_get_email","arguments":{"id":"1"}}},{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"swarmail_get_email","arguments":{"id":"2"}}}]"#;
    let started = c.observe_mcp_body(two);
    assert_eq!(started.len(), 2);
    drop(started);
    assert_eq!(reason(&seal_one(&c)), Some(Refusal::BindingMismatch));
    // Not a call, not JSON: ignored.
    let c = Capture::new(catalogue());
    assert!(c.observe_mcp_body(b"garbage").is_empty());
    assert!(
        c.observe_mcp_body(br#"{"method":"ping","id":1}"#)
            .is_empty()
    );
    assert_eq!(reason(&seal_one(&c)), None);
    // A call this reader cannot bind refuses the trial.
    for body in [
        &br#"{"method":"tools/call","id":1,"params":{"name":"swarmail_get_email"}}"#[..],
        br#"{"method":"tools/call","id":1,"params":{"name":"x","arguments":{},"arguments":{}}}"#,
    ] {
        let c = Capture::new(catalogue());
        drop(c.observe_mcp_body(body));
        assert_eq!(
            reason(&seal_one(&c)),
            Some(Refusal::InvalidArgumentEncoding)
        );
    }
}

#[test]
fn captures_round_trip_and_substitution_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let root = artifacts::resolve_root(dir.path()).unwrap();
    let c = Capture::new(catalogue());
    http(&c, b"{}");
    let rec = seal_one(&c);
    write_capture(&root, &rec).unwrap();
    assert_eq!(read_capture(&root, &id(1)), Some(rec.clone()));
    assert!(write_capture(&root, &rec).is_err(), "never clobbers");
    assert_eq!(read_capture(&root, &id(2)), None);
    assert_eq!(read_capture(&root, "../x"), None);
    // A changed byte no longer matches the manifest.
    let file = root.join(id(1)).join(CAPTURE_FILE);
    let mut bytes = std::fs::read(&file).unwrap();
    bytes[10] ^= 1;
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, &bytes).unwrap();
    assert_eq!(read_capture(&root, &id(1)), None);
}

fn call(i: u64, tool: &str, args: &[u8]) -> ToolCall {
    ToolCall {
        ordinal: i,
        tool: tool.into(),
        outcome: ToolOutcome::Ok,
        argument_bytes: args.len() as u64,
        argument_sha256: sha256(args),
        result_bytes: 1,
        result_sha256: sha256(b"r"),
    }
}

#[test]
fn binding_follows_the_structural_sequence_exactly() {
    let owned = |n| owned_tool(n);
    assert_eq!(
        owned("mcp__swarmail__swarmail_get_email"),
        Some((Server::Swarmail, "swarmail_get_email"))
    );
    assert_eq!(
        owned("fixture.eval_http"),
        Some((Server::Fixture, "eval_http"))
    );
    assert_eq!(owned("eval_http"), Some((Server::Fixture, "eval_http")));
    assert_eq!(owned("swarmail:x"), Some((Server::Swarmail, "x")));
    assert_eq!(owned("mcp__swarmail__"), None);
    assert_eq!(owned("Bash"), None);
    assert_eq!(owned("swarmailx__y"), None);

    let a = br#"{"inbox":"a"}"#;
    let b = br#"{"method":"GET","path":"/"}"#;
    let c = Capture::new(catalogue());
    drop(c.observe(Server::Swarmail, "swarmail_get_email", a));
    http(&c, b);
    let rec = seal_one(&c);
    let structural = vec![
        call(1, "mcp__swarmail__swarmail_get_email", a),
        call(2, "Bash", b"ls"),
        call(3, "mcp__fixture__eval_http", b),
    ];
    let comp = bind(&structural, &rec).unwrap();
    assert_eq!(comp.calls.len(), 3);
    let calls: Vec<Value> = comp
        .calls
        .iter()
        .map(|c| serde_json::from_str(c).unwrap())
        .collect();
    assert_eq!(calls[1]["value"], Value::Null);
    assert_eq!(calls[1]["owned"], false);
    // A name the journal alone supplies is published only as its digest.
    assert_eq!(calls[1]["tool"], format!("sha256:{}", sha256(b"Bash")));
    assert_eq!(calls[0]["tool"], "mcp__swarmail__swarmail_get_email");
    assert_eq!(calls[2]["value"]["path"], "/");
    assert_eq!(comp.digests[1], None);
    assert_eq!(
        comp.digests[2],
        Some(sha256(br#"{"method":"GET","path":"/"}"#))
    );
    // Order, count, digest, length and identity each refuse.
    let mut bad: Vec<Vec<ToolCall>> = vec![
        vec![structural[2].clone(), structural[0].clone()],
        structural[..1].to_vec(),
    ];
    let mut more = structural.clone();
    more.push(call(4, "eval_http", b));
    bad.push(more);
    for edit in [
        |c: &mut ToolCall| c.argument_sha256 = sha256(b"other"),
        |c: &mut ToolCall| c.argument_bytes += 1,
        |c: &mut ToolCall| c.tool = "mcp__fixture__swarmail_get_email".into(),
        |c: &mut ToolCall| c.tool = "mcp__swarmail__swarmail_wait_for_email".into(),
    ] {
        let mut s = structural.clone();
        edit(&mut s[0]);
        bad.push(s);
    }
    for s in bad {
        assert_eq!(bind(&s, &rec).unwrap_err(), Refusal::BindingMismatch);
    }
    // A proven empty sequence binds to an empty capture.
    let empty = Capture::new(catalogue());
    assert!(bind(&[], &seal_one(&empty)).unwrap().calls.is_empty());
    // A latched refusal is carried.
    let refused = Capture::new(catalogue());
    refused.refuse(Refusal::UnknownTool);
    assert_eq!(
        bind(&[], &seal_one(&refused)).unwrap_err(),
        Refusal::UnknownTool
    );
}

#[test]
fn every_reason_has_its_name_in_check_order() {
    let names: Vec<&str> = Refusal::ALL.iter().map(|r| r.as_str()).collect();
    assert_eq!(
        names,
        [
            "capture_missing",
            "registry_unavailable",
            "unknown_tool",
            "value_too_large",
            "trial_too_large",
            "invalid_argument_encoding",
            "residual_secret",
            "binding_mismatch",
            "evidence_write_failed"
        ]
    );
    for r in Refusal::ALL {
        assert_eq!(serde_json::to_value(r).unwrap(), json!(r.as_str()));
    }
    let mut sorted = Refusal::ALL;
    sorted.sort();
    assert_eq!(sorted, Refusal::ALL);
}

#[test]
fn cells_are_escaped_and_figures_never_dash() {
    assert_eq!(escape_cell("a|b*c\n"), "a\\|b\\*c\u{fffd}");
    assert_eq!(escape_cell("a\u{2028}b\u{2029}c"), "a\u{fffd}b\u{fffd}c");
    let paid = json!({"tool_surfaces":[{"kind":"wrapper"}],"scored":2});
    assert_eq!(tokens_figure(&paid), (None, "incomplete_join"));
    let empty = json!({"tool_surfaces":[{"kind":"wrapper"}],"scored":0});
    assert_eq!(tokens_figure(&empty), (None, "no_passes"));
    let builtin = json!({"tool_surfaces":[{"kind":"builtin_not_applicable"}],"scored":2});
    assert_eq!(tokens_figure(&builtin), (None, "builtin_agent"));
    // A journal joined beside a built-in group does not turn it into a
    // missing join: it never had accounting to join.
    let joined = json!({"tool_surfaces":[{"kind":"builtin_not_applicable"}],"scored":2,
        "journal":{"tokens_per_success":null,"tokens_per_success_reason":"incomplete_join"}});
    assert_eq!(tokens_figure(&joined), (None, "builtin_agent"));
    for (j, want) in [
        (json!({"tokens_per_success": 12.5}), (Some(12.5), "")),
        (
            json!({"tokens_per_success": null, "tokens_per_success_reason": "no_passes"}),
            (None, "no_passes"),
        ),
        (
            json!({"tokens_per_success": null, "tokens_per_success_reason": "sum_overflow"}),
            (None, "sum_overflow"),
        ),
        (
            json!({"tokens_per_success": null, "tokens_per_success_reason": "incomplete_join"}),
            (None, "incomplete_join"),
        ),
    ] {
        assert_eq!(tokens_figure(&json!({ "journal": j })), want);
    }
    let page = render_page(&json!({"groups":[
        {"task":"signup","agent":"a|b","attempts":1,"passes":0,"fails":1,"harness_errors":0,"scored":1,"tool_surfaces":[{"kind":"wrapper"}]},
        {"task":"signup","agent":"c","attempts":1,"passes":1,"fails":0,"harness_errors":0,"scored":1,"tool_surfaces":[{"kind":"wrapper"}],
         "journal":{"tokens_per_success":10.0,"tokens":{"cached":{"sum":7}}}},
        {"task":"signup","agent":"d","attempts":2,"passes":2,"fails":0,"harness_errors":0,"scored":2,"tool_surfaces":[{"kind":"wrapper"}],
         "journal":{"tokens_per_success":null,"tokens_per_success_reason":"sum_overflow"}},
        {"task":"signup","agent":"e","attempts":2,"passes":2,"fails":0,"harness_errors":0,"scored":2,"tool_surfaces":[{"kind":"wrapper"}],
         "journal":{"tokens_per_success":null,"tokens_per_success_reason":"sum_overflow","tokens":{"cached":{"sum":7}}}},
        {"task":"signup","agent":"f","attempts":2,"passes":1,"fails":1,"harness_errors":0,"scored":2,"tool_surfaces":[{"kind":"wrapper"}],
         "journal":{"tokens_per_success":null,"tokens_per_success_reason":"incomplete_join","tokens":{"cached":{"sum":3}}}}
    ]}));
    assert!(
        page.contains(
            "| signup | d | 2 | 2 | 0 | 0 | null (sum\\_overflow) | null (sum\\_overflow) |"
        ),
        "{page}"
    );
    assert!(
        page.contains(
            "| signup | a\\|b | 1 | 0 | 1 | 0 | null (incomplete\\_join) | null (incomplete\\_join) |"
        ),
        "{page}"
    );
    // An overflowed total keeps its separate cached count; a partial one
    // beside incomplete_join is withheld with that reason.
    assert!(
        page.contains("| signup | e | 2 | 2 | 0 | 0 | null (sum\\_overflow) | 7 |"),
        "{page}"
    );
    assert!(
        page.contains(
            "| signup | f | 2 | 1 | 1 | 0 | null (incomplete\\_join) | null (incomplete\\_join) |"
        ),
        "{page}"
    );
    assert!(
        page.contains("| signup | c | 1 | 1 | 0 | 0 | 10 | 7 |"),
        "{page}"
    );
}

/// One real row, cloned per trial: building each from scratch hashes the
/// harness executable every time.
fn template() -> &'static TrialResult {
    static ROW: std::sync::OnceLock<TrialResult> = std::sync::OnceLock::new();
    ROW.get_or_init(|| {
        let req = crate::runner::TrialRequest {
            task: Some("signup".into()),
            agent: Some("agent".into()),
            argv: vec!["/bin/agent".into()],
            ..Default::default()
        };
        crate::runner::refused_row(&req, &id(0))
    })
}

fn row(n: u32, class: Option<FailureClass>) -> TrialResult {
    let mut row = template().clone();
    row.trial_id = id(n);
    match class {
        None => {
            row.outcome = Outcome::Pass;
            row.failure_class = None;
            row.agent_exit_status = Some(0);
        }
        Some(FailureClass::HarnessError) => {}
        Some(c) => {
            row.outcome = Outcome::Fail;
            row.failure_class = Some(c);
            row.agent_exit_status = Some(0);
        }
    }
    row
}

fn companion(r: &TrialResult, calls: usize, value: &str) -> Companion {
    let fragment = quote(value);
    Companion {
        trial_id: r.trial_id.clone(),
        row_sha256: Some(sha256(&row_bytes(r))),
        calls: (0..calls)
            .map(|i| format!(r#"{{"ordinal":{},"value":{fragment}}}"#, i + 1))
            .collect(),
        digests: vec![Some(sha256(fragment.as_bytes())); calls],
    }
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(root).unwrap() {
        let p = e.unwrap().path();
        out.insert(
            p.file_name().unwrap().to_str().unwrap().to_string(),
            std::fs::read(&p).unwrap(),
        );
    }
    out
}

#[test]
fn transcript_selection_is_deterministic_bounded_and_refills() {
    use FailureClass::*;
    // gave_up and wrong_link tie at four; harness errors count; passes never.
    let spec = [
        (1, Some(GaveUp)),
        (2, Some(GaveUp)),
        (3, Some(GaveUp)),
        (4, Some(GaveUp)),
        (5, Some(WrongLink)),
        (6, Some(WrongLink)),
        (7, Some(WrongLink)),
        (8, Some(WrongLink)),
        (9, Some(HarnessError)),
        (10, Some(Budget)),
        (11, None),
    ];
    let rows: Vec<TrialResult> = spec.iter().map(|(n, c)| row(*n, *c)).collect();
    let mut comps = BTreeMap::new();
    for r in &rows {
        comps.insert(r.trial_id.clone(), companion(r, 2, "v"));
    }
    comps.remove(&id(2)); // no verified companion: not eligible
    comps.get_mut(&id(3)).unwrap().row_sha256 = None; // row not approved
    // One leaf too large: it is skipped by size and the class refills.
    comps.insert(id(5), companion(&rows[4], 40, &"x".repeat(4000)));
    comps.remove(&id(10)); // budget: counted, nothing eligible
    let run = |rows: &[TrialResult]| {
        let dir = tempfile::tempdir().unwrap();
        let root = artifacts::resolve_root(dir.path()).unwrap();
        write_transcripts(&root, rows, &comps).unwrap();
        (dir, tree(&root.join(TRANSCRIPTS_DIR)))
    };
    let (_a, forward) = run(&rows);
    let mut shuffled = rows.clone();
    shuffled.reverse();
    shuffled.rotate_left(4);
    let (_b, backward) = run(&shuffled);
    assert_eq!(forward, backward, "a permutation yields the same bytes");
    let index: Value = serde_json::from_slice(&forward["index.json"]).unwrap();
    assert!(forward["index.json"].len() <= INDEX_LIMIT);
    let classes = index["classes"].as_array().unwrap();
    let names: Vec<&str> = classes
        .iter()
        .map(|c| c["class"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["gave_up", "wrong_link", "budget", "harness_error"]);
    let picked = |i: usize| -> Vec<String> {
        classes[i]["transcripts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["trial_id"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(classes[0]["total"], 4);
    assert_eq!(classes[0]["eligible"], 2);
    assert_eq!(classes[0]["skipped"], json!({"row_unverified": 1}));
    assert_eq!(picked(0), [id(1), id(4)]);
    assert_eq!(classes[1]["eligible"], 4);
    assert_eq!(classes[1]["skipped"], json!({"leaf_too_large": 1}));
    assert_eq!(
        picked(1),
        [id(6), id(7), id(8)],
        "refilled past the oversized leaf"
    );
    assert_eq!(
        (classes[2]["total"].clone(), classes[2]["selected"].clone()),
        (json!(1), json!(0))
    );
    assert_eq!(picked(3), [id(9)]);
    assert_eq!(forward.len(), 1 + 2 + 3 + 1);
    for (name, bytes) in &forward {
        assert!(bytes.len() <= LEAF_LIMIT, "{name}");
    }
    let leaf: Value = serde_json::from_slice(&forward[&format!("{}.json", id(6))]).unwrap();
    assert_eq!(leaf["failure_class"], "wrong_link");
    assert_eq!(leaf["model_text"], "not_retained");
    assert_eq!(leaf["row"], serde_json::to_value(&rows[5]).unwrap());
    // At most three per class, so at most 24 leaves over the eight classes.
    let many: Vec<TrialResult> = (1..=40)
        .map(|n| row(n, Some(FailureClass::ALL[n as usize % 7])))
        .collect();
    let all: BTreeMap<String, Companion> = many
        .iter()
        .map(|r| (r.trial_id.clone(), companion(r, 1, "v")))
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let root = artifacts::resolve_root(dir.path()).unwrap();
    write_transcripts(&root, &many, &all).unwrap();
    assert_eq!(tree(&root.join(TRANSCRIPTS_DIR)).len(), 7 * PER_CLASS + 1);
    // Zero eligible rows still yields the class counts; an existing section
    // is never replaced.
    let empty = tempfile::tempdir().unwrap();
    let root = artifacts::resolve_root(empty.path()).unwrap();
    write_transcripts(&root, &rows, &BTreeMap::new()).unwrap();
    let index: Value =
        serde_json::from_slice(&tree(&root.join(TRANSCRIPTS_DIR))["index.json"]).unwrap();
    assert_eq!(index["classes"][0]["total"], 4);
    assert_eq!(index["classes"][0]["selected"], 0);
    assert!(write_transcripts(&root, &rows, &BTreeMap::new()).is_err());
}

#[test]
fn a_row_holding_an_escaped_or_encoded_literal_never_reaches_a_transcript() {
    // A sentinel the row's serializer escapes (quote, backslash, control
    // character) is not in the row's bytes as written, and neither is a
    // percent-encoded one: the decoded check finds both.
    let escaped = "sen\"ti\\nel\u{7}-q";
    let mut registry = reg();
    registry.register(escaped);
    let labelled = |n: u32, label: &str| {
        let mut r = row(n, Some(FailureClass::GaveUp));
        r.agent = Some(label.to_string());
        r
    };
    let rows = vec![
        labelled(1, &format!("agent {escaped}")),
        labelled(2, "agent sentinel-canary-%C3%BF-42"),
        labelled(3, "agent clean"),
    ];
    assert!(!registry.found_in(&row_bytes(&rows[0])), "escaped in bytes");
    assert!(!registry.found_in(&row_bytes(&rows[1])), "encoded in bytes");
    let mut comps = BTreeMap::new();
    for r in &rows {
        let c = Capture::new(catalogue());
        http(&c, b"{}");
        let rec = c.seal(&r.trial_id, &registry, &row_bytes(r));
        let CaptureOutcome::Complete { calls } = &rec.outcome else {
            panic!("{rec:?}")
        };
        assert_eq!(calls.len(), 1, "the values themselves are clean");
        let mut companion = companion(r, 1, "v");
        companion.row_sha256 = rec.row_sha256;
        comps.insert(r.trial_id.clone(), companion);
    }
    assert_eq!(comps[&id(1)].row_sha256, None);
    assert_eq!(comps[&id(2)].row_sha256, None);
    assert!(comps[&id(3)].row_sha256.is_some());
    let dir = tempfile::tempdir().unwrap();
    let root = artifacts::resolve_root(dir.path()).unwrap();
    write_transcripts(&root, &rows, &comps).unwrap();
    let written = tree(&root.join(TRANSCRIPTS_DIR));
    let index: Value = serde_json::from_slice(&written["index.json"]).unwrap();
    assert_eq!(index["classes"][0]["skipped"], json!({"row_unverified": 2}));
    assert_eq!(
        written.keys().cloned().collect::<Vec<_>>(),
        [format!("{}.json", id(3)), "index.json".to_string()]
    );
    // A row that is not one strict JSON document cannot be checked.
    assert!(registry.found_in_document(b"{\"a\":1,\"a\":2}"));
    assert!(!registry.found_in_document(b"{\"a\":1}"));
}

/// Seal `values` as one trial's `eval_http` calls against `registry`,
/// publish the companion, and return its disposition and bytes.
fn publish_values(
    registry: &Registry,
    values: &[&[u8]],
) -> (Disposition, Option<Vec<u8>>, CaptureRecord) {
    let dir = tempfile::tempdir().unwrap();
    let root = artifacts::resolve_root(dir.path()).unwrap();
    let (ev, caps) = (root.join("ev"), root.join("caps"));
    std::fs::create_dir(&ev).unwrap();
    std::fs::create_dir(&caps).unwrap();
    let leaf = artifacts::create_private_dir(&ev, &id(1)).unwrap();
    artifacts::publish(&leaf, "tool-calls.json", br#"{"truncated":false}"#).unwrap();
    let c = Capture::new(catalogue());
    for v in values {
        http(&c, v);
    }
    let record = c.seal(&id(1), registry, b"{}");
    write_capture(&caps, &record).unwrap();
    let facts = journal::TrialFacts {
        trial_id: id(1),
        run_id: "run-1".into(),
        seat: "trial".into(),
        tokens: Fact::Present {
            value: journal::Tokens {
                input: 1,
                output: 1,
                cached: 0,
            },
        },
        turns: Fact::Present { value: 1 },
        wall_time_ms: Fact::Present { value: 1 },
        deadline_killed: Fact::Present { value: false },
        tool_calls: Fact::Present {
            value: values
                .iter()
                .enumerate()
                .map(|(i, v)| call(i as u64 + 1, "eval_http", v))
                .collect(),
        },
    };
    let d = publish_companion(&ev, &caps, &facts, &json!({"path": "x"}));
    let bytes = std::fs::read(leaf.join(COMPANION_FILE)).ok();
    (d, bytes, record)
}

fn checked_values(record: &CaptureRecord) -> Vec<String> {
    match &record.outcome {
        CaptureOutcome::Complete { calls } => calls.iter().map(|c| c.value.clone()).collect(),
        CaptureOutcome::Refused { reason } => panic!("{reason:?}"),
    }
}

#[test]
fn a_published_value_is_the_checked_bytes() {
    // The admitted sentinel whose value a double would round to: the
    // checked spelling 1234567890123456.99 is not it, and is what is
    // published, never re-spelled as 1234567890123457.0.
    let mut r = Registry::new();
    r.register("00000000000000001234567890123457");
    let close = br#"{"n":1234567890123456.99}"#;
    assert_eq!(
        r.redact(close).as_deref(),
        Ok(r#"{"n":1234567890123456.99}"#)
    );
    // A number's spelling, the member order and an integer past u64 are
    // kept as written.
    let kept = br#"{"z":1.50e2,"a":123456789012345678901234567890,"m":-0.0}"#;
    let (d, bytes, record) = publish_values(&r, &[close, kept]);
    assert!(matches!(d, Disposition::Published(..)), "{d:?}");
    let bytes = bytes.unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    let checked = checked_values(&record);
    assert_eq!(
        checked[1],
        r#"{"z":1.50e2,"a":123456789012345678901234567890,"m":-0.0}"#
    );
    for value in &checked {
        assert!(text.contains(&format!(r#""value":{value}}}"#)), "{text}");
    }
    assert!(!text.contains("1234567890123457"), "{text}");
    // Each published value's own bytes hash to the digest recorded at seal.
    let CaptureOutcome::Complete { calls } = &record.outcome else {
        unreachable!()
    };
    let digests: Vec<Option<String>> = calls.iter().map(|c| Some(c.value_sha256.clone())).collect();
    assert!(fragments_verified(&bytes, &digests));
    // A fragment that differs from its checked bytes, even by its spelling
    // alone, does not verify.
    let respelled = text.replacen("1234567890123456.99", "1234567890123457.0", 1);
    assert!(!fragments_verified(respelled.as_bytes(), &digests));
    assert!(!fragments_verified(bytes.as_slice(), &digests[..1]));
    // A capture whose stored value no longer matches the digest recorded
    // at seal is withheld whole.
    let c = Capture::new(catalogue());
    http(&c, close);
    let mut tampered = c.seal(&id(1), &r, b"{}");
    if let CaptureOutcome::Complete { calls } = &mut tampered.outcome {
        calls[0].value = r#"{"n":1234567890123457}"#.into();
    }
    let dir = tempfile::tempdir().unwrap();
    let root = artifacts::resolve_root(dir.path()).unwrap();
    write_capture(&root, &tampered).unwrap();
    let back = read_capture(&root, &id(1)).unwrap();
    assert_eq!(
        bind(&[call(1, "eval_http", close)], &back).unwrap_err(),
        Refusal::BindingMismatch
    );
    // A number no double can hold is refused at seal, never left for a
    // reader to fail on; the largest finite one is kept.
    assert_eq!(
        r.redact(br#"{"n":1e999}"#),
        Err(Refusal::InvalidArgumentEncoding)
    );
    assert_eq!(
        r.redact(br#"{"n":-1e999}"#),
        Err(Refusal::InvalidArgumentEncoding)
    );
    assert!(r.redact(br#"{"n":1.7e308}"#).is_ok());
    // The transcript leaf carries the same bytes, and the row's own.
    let Disposition::Published(_, companion) = d else {
        unreachable!()
    };
    let mut row = row(1, Some(FailureClass::GaveUp));
    row.trial_id = id(1);
    let companion = Companion {
        row_sha256: Some(sha256(&row_bytes(&row))),
        ..*companion
    };
    let tdir = tempfile::tempdir().unwrap();
    let troot = artifacts::resolve_root(tdir.path()).unwrap();
    write_transcripts(
        &troot,
        &[row.clone()],
        &BTreeMap::from([(id(1), companion)]),
    )
    .unwrap();
    let leaf = std::fs::read(troot.join(TRANSCRIPTS_DIR).join(format!("{}.json", id(1)))).unwrap();
    let leaf = String::from_utf8(leaf).unwrap();
    for value in &checked {
        assert!(leaf.contains(&format!(r#""value":{value}}}"#)), "{leaf}");
    }
    let row_text = String::from_utf8(row_bytes(&row)).unwrap();
    assert!(leaf.contains(&format!(r#""row":{row_text}"#)));
}

#[test]
fn a_companion_is_published_whole_or_absent_with_its_reason() {
    let dir = tempfile::tempdir().unwrap();
    let root = artifacts::resolve_root(dir.path()).unwrap();
    let ev = root.join("ev");
    let caps = root.join("caps");
    std::fs::create_dir(&ev).unwrap();
    std::fs::create_dir(&caps).unwrap();
    let args = br#"{"method":"GET","path":"/"}"#;
    let facts = |n: u32, calls: Vec<ToolCall>| journal::TrialFacts {
        trial_id: id(n),
        run_id: format!("run-{n}"),
        seat: "trial".into(),
        tokens: Fact::Present {
            value: journal::Tokens {
                input: 1,
                output: 1,
                cached: 0,
            },
        },
        turns: Fact::Present { value: 1 },
        wall_time_ms: Fact::Present { value: 1 },
        deadline_killed: Fact::Present { value: false },
        tool_calls: Fact::Present { value: calls },
    };
    let setup = |n: u32, structural: &[u8], values: &[&[u8]]| {
        let leaf = artifacts::create_private_dir(&ev, &id(n)).unwrap();
        artifacts::publish(&leaf, "tool-calls.json", structural).unwrap();
        let c = Capture::new(catalogue());
        for v in values {
            http(&c, v);
        }
        write_capture(&caps, &c.seal(&id(n), &reg(), b"{}")).unwrap();
        leaf
    };
    let whole = br#"{"truncated":false}"#;
    let entry = json!({"path": "x"});
    // Published, and bound.
    setup(1, whole, &[args]);
    let d = publish_companion(
        &ev,
        &caps,
        &facts(1, vec![call(1, "eval_http", args)]),
        &entry,
    );
    assert!(
        matches!(&d, Disposition::Published(b, _) if b.path == format!("{}/{COMPANION_FILE}", id(1)))
    );
    assert_eq!(
        d.index_entry()["length"],
        std::fs::read(ev.join(id(1)).join(COMPANION_FILE))
            .unwrap()
            .len()
    );
    let reason = |d: Disposition| match d {
        Disposition::Absent(r) => r,
        Disposition::Published(..) => panic!("published"),
    };
    // A companion that would already exist is never replaced.
    let again = publish_companion(
        &ev,
        &caps,
        &facts(1, vec![call(1, "eval_http", args)]),
        &entry,
    );
    assert_eq!(reason(again), "evidence_write_failed");
    // No capture at all.
    let none = publish_companion(&ev, &caps, &facts(9, vec![]), &entry);
    assert_eq!(reason(none), "capture_missing");
    // A structural record that is cut, or absent, carries no companion.
    setup(2, br#"{"truncated":true}"#, &[args]);
    let cut = publish_companion(
        &ev,
        &caps,
        &facts(2, vec![call(1, "eval_http", args)]),
        &entry,
    );
    assert_eq!(reason(cut), "trial_too_large");
    setup(3, whole, &[args]);
    let gone = json!({"absent": "evidence_write_failed"});
    let absent = publish_companion(
        &ev,
        &caps,
        &facts(3, vec![call(1, "eval_http", args)]),
        &gone,
    );
    assert_eq!(reason(absent), "evidence_write_failed");
    // Values that fit one by one but not as one companion.
    let big = format!(r#"{{"s":"{}"}}"#, "y ".repeat(4000));
    let bigs: Vec<&[u8]> = (0..8).map(|_| big.as_bytes()).collect();
    setup(4, whole, &bigs);
    let calls: Vec<ToolCall> = (1..=8)
        .map(|i| call(i, "eval_http", big.as_bytes()))
        .collect();
    let over = publish_companion(&ev, &caps, &facts(4, calls), &entry);
    assert_eq!(reason(over), "trial_too_large");
    assert_eq!(
        serde_json::from_slice::<Value>(
            &std::fs::read(ev.join(id(1)).join(COMPANION_FILE)).unwrap()
        )
        .unwrap()["run_id"],
        "run-1"
    );
    // A run id that is not plain is withheld, the companion kept.
    setup(7, whole, &[args]);
    let mut odd = facts(7, vec![call(1, "eval_http", args)]);
    odd.run_id = format!("run-{}", "Q".repeat(OPAQUE_RUN_LIMIT));
    assert!(matches!(
        publish_companion(&ev, &caps, &odd, &entry),
        Disposition::Published(..)
    ));
    let published: Value =
        serde_json::from_slice(&std::fs::read(ev.join(id(7)).join(COMPANION_FILE)).unwrap())
            .unwrap();
    assert_eq!(published["run_id"], Value::Null);
    // A present, verified zero-call sequence: a nonempty envelope, bound.
    setup(6, whole, &[]);
    let zero = publish_companion(&ev, &caps, &facts(6, vec![]), &entry);
    let Disposition::Published(binding, _) = &zero else {
        panic!("{zero:?}")
    };
    let bytes = std::fs::read(ev.join(id(6)).join(COMPANION_FILE)).unwrap();
    assert_eq!(binding.length, bytes.len() as u64);
    assert_eq!(binding.sha256, sha256(&bytes));
    let envelope: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(envelope["tool_calls"], json!([]));
    assert_eq!(envelope["trial_id"], id(6));
    assert_eq!(envelope["structural"], entry);
    assert_eq!(envelope["model_text"], "not_retained");
    // An unavailable fact is the caller's upstream absence, never values.
    let mut f = facts(5, vec![]);
    f.tool_calls = Fact::Unavailable {
        reason: journal::Reason::JournalAmbiguous,
    };
    assert_eq!(
        reason(publish_companion(&ev, &caps, &f, &entry)),
        "capture_missing"
    );
    for n in [2, 3, 4] {
        assert!(
            !ev.join(id(n)).join(COMPANION_FILE).exists(),
            "no partial file"
        );
    }
}
