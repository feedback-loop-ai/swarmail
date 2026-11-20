use super::*;
use crate::journal::ToolOutcome;

const BEARER: &str = "BearerCanary0123456789abcdefXYZ";
const SENTINEL: &str = "sentinel-canary-ÿ-42";

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
        // Whitespace and multibyte text without a literal keep their values.
        (
            "{ \"q\" : \"héllo ✓\" , \"n\" : 1.50 }".into(),
            r#"{"q":"héllo ✓","n":1.50}"#,
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
        // A literal in plain text beside a benign undecodable escape is
        // masked as text.
        (
            format!(r#"{{"s":"{BEARER} 100%FF"}}"#),
            r#"{"s":"[REDACTED] 100%FF"}"#,
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
    let escaped = format!(r#"{{"a":"\\u0025{}41"}}"#, "25".repeat(DEPTH_LIMIT + 1));
    assert_eq!(r.redact(escaped.as_bytes()), Err(Refusal::ResidualSecret));
    let settles = format!(r#"{{"a":"\\u0025{}41"}}"#, "25".repeat(DEPTH_LIMIT - 2));
    assert!(r.redact(settles.as_bytes()).is_ok());
    // A percent-encoded JSON escape: the literal is in no single decoding,
    // only in the alternation, and is refused rather than published.
    let mixed = format!(r#"{{"a":"%5Cu0042{}"}}"#, &BEARER[1..]);
    assert_eq!(r.redact(mixed.as_bytes()), Err(Refusal::ResidualSecret));
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
    let at = format!(r#"{{"s":"{}"}}"#, "a".repeat(VALUE_LIMIT - 8));
    assert_eq!(at.len(), VALUE_LIMIT);
    assert!(r.redact(at.as_bytes()).is_ok());
    let over = format!(r#"{{"s":"{}"}}"#, "a".repeat(VALUE_LIMIT - 7));
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
    assert_eq!(r.redact(deep.as_bytes()), Err(Refusal::ResidualSecret));
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
    for plain in [
        "%5BREDACTED",
        "[REDACTE D]",
        "%5BRED%ACTED%5D",
        "[redacted]",
    ] {
        let doc = format!(r#"{{"a":{}}}"#, quote(plain));
        assert_eq!(r.redact(doc.as_bytes()).as_deref(), Ok(doc.as_str()));
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
    std::fs::write(&good, serde_json::to_vec(&[SENTINEL]).unwrap()).unwrap();
    let built = registry(BEARER, Some(&good));
    assert!(built.is_complete() && built.len() == 2);
    for bad in [&b"[\"\"]"[..], b"{}", b"nope"] {
        std::fs::write(&good, bad).unwrap();
        assert!(!registry(BEARER, Some(&good)).is_complete());
    }
    let many: Vec<String> = (0..=REGISTRY_LITERALS).map(|i| format!("s{i}")).collect();
    std::fs::write(&good, serde_json::to_vec(&many).unwrap()).unwrap();
    assert!(!registry(BEARER, Some(&good)).is_complete());
    assert!(!registry(BEARER, Some(&dir.path().join("absent"))).is_complete());
    assert!(read_sentinels(Path::new("/")).is_err());
    assert!(registry(BEARER, None).is_complete());
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
    assert_eq!(comp.calls[1]["value"], Value::Null);
    assert_eq!(comp.calls[1]["owned"], false);
    assert_eq!(comp.calls[2]["value"]["path"], "/");
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
         "journal":{"tokens_per_success":null,"tokens_per_success_reason":"sum_overflow"}}
    ]}));
    assert!(
        page.contains("| signup | d | 2 | 2 | 0 | 0 | null (sum\\_overflow) | null |"),
        "{page}"
    );
    assert!(
        page.contains("| signup | a\\|b | 1 | 0 | 1 | 0 | null (incomplete\\_join) | null |"),
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
    Companion {
        trial_id: r.trial_id.clone(),
        row_sha256: Some(sha256(&row_bytes(r))),
        calls: (0..calls)
            .map(|i| json!({"ordinal": i + 1, "value": value}))
            .collect(),
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
    let big = format!(r#"{{"s":"{}"}}"#, "y".repeat(8000));
    let bigs: Vec<&[u8]> = (0..8).map(|_| big.as_bytes()).collect();
    setup(4, whole, &bigs);
    let calls: Vec<ToolCall> = (1..=8)
        .map(|i| call(i, "eval_http", big.as_bytes()))
        .collect();
    let over = publish_companion(&ev, &caps, &facts(4, calls), &entry);
    assert_eq!(reason(over), "trial_too_large");
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
