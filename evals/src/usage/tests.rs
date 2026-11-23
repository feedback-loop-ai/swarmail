use super::*;
use serde_json::json;

fn data(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

/// The 2026-11-23 smoke's projected checkpoints, reconstructed as `data`
/// objects (the 0.12.0 projection omits the envelope; v0.15.2 puts the same
/// keys under `checkpoint.data`).
fn smoke(family: &str) -> Vec<Map<String, Value>> {
    let turn = |step: &str, i: u64, o: u64| {
        data(json!({"step": step, "input_tokens": i, "output_tokens": o, "model": "m"}))
    };
    let common = [
        data(json!({"step": "transcript", "effort": "e", "model": "m"})),
        data(json!({"step": "harness-started", "harness": "h", "launch": "cold"})),
    ];
    let mut out: Vec<_> = common.to_vec();
    match family {
        "claude" => {
            out.push(turn("seat-turn", 11844, 16));
            out.push(turn("seat-turn", 30027, 4));
            out.push(data(json!({"step": "claude-code-session-finished", "input_tokens": 41871, "output_tokens": 180, "cache_read_tokens": 41867, "cache_write_tokens": 18455, "num_turns": 2, "total_cost_usd": 0.0798147, "exit_code": 0})));
        }
        "codex" => {
            out.push(turn("turn-completed", 45729, 244));
            out.push(data(json!({"step": "codex-session-finished", "input_tokens": 45729, "output_tokens": 244, "cache_read_tokens": 41600, "num_turns": 1, "exit_code": 0})));
        }
        _ => {
            out.push(turn("seat-turn", 11497, 323));
            out.push(data(json!({"step": "deepseek-harness-session-finished", "input_tokens": 11497, "output_tokens": 323, "cache_read_tokens": 4096, "num_turns": 2, "exit_code": 0})));
        }
    }
    out
}

fn present(v: u64) -> Fact<u64> {
    Fact::Present { value: v }
}
fn missing(reason: Reason) -> Fact<u64> {
    Fact::Unavailable { reason }
}

#[test]
fn the_smokes_final_totals_project_and_per_turn_rows_never_count() {
    let claude = project("claude", &smoke("claude"));
    assert_eq!(claude.input_tokens, present(41871));
    assert_eq!(claude.output_tokens, present(180));
    assert_eq!(claude.cache_read_tokens, present(41867));
    assert_eq!(claude.cache_write_tokens, present(18455));
    assert_eq!(claude.num_turns, present(2));
    assert_eq!(claude.total_cost_usd, Fact::Present { value: 0.0798147 });
    let codex = project("codex", &smoke("codex"));
    assert_eq!(
        (codex.input_tokens, codex.output_tokens),
        (present(45729), present(244))
    );
    assert_eq!(codex.cache_write_tokens, missing(Reason::FieldAbsent));
    assert_eq!(
        codex.total_cost_usd,
        Fact::Unavailable {
            reason: Reason::FieldAbsent
        }
    );
    let dsh = project("dsh", &smoke("dsh"));
    assert_eq!(
        (dsh.input_tokens, dsh.output_tokens),
        (present(11497), present(323))
    );
    assert_eq!(dsh.cache_read_tokens, present(4096));
    // Turn rows alone establish nothing, and another family's final is not
    // this family's.
    let turns: Vec<_> = smoke("claude")
        .into_iter()
        .filter(|d| d["step"] == "seat-turn")
        .collect();
    assert_eq!(
        project("claude", &turns),
        Facts::unavailable(Reason::NoSessionFinal)
    );
    assert_eq!(
        project("codex", &smoke("claude")),
        Facts::unavailable(Reason::NoSessionFinal)
    );
    assert_eq!(
        project("lanetally", &smoke("claude")),
        Facts::unavailable(Reason::NoSessionFinal)
    );
    assert_eq!(final_step("exec"), None);
}

#[test]
fn repeats_conflicts_zero_and_malformed_values_are_typed_per_field() {
    let fin = |v: Value| {
        let mut d = data(v);
        d.insert("step".into(), "codex-session-finished".into());
        d
    };
    let a = fin(json!({"input_tokens": 5, "output_tokens": 0, "num_turns": 1}));
    // An identical repeated final counts once; an explicit zero is kept.
    let twice = project("codex", &[a.clone(), a.clone()]);
    assert_eq!(twice.input_tokens, present(5));
    assert_eq!(twice.output_tokens, present(0));
    // A disagreeing repeat makes only the disagreeing field ambiguous.
    let b = fin(json!({"input_tokens": 6, "output_tokens": 0, "num_turns": 1}));
    let both = project("codex", &[a.clone(), b]);
    assert_eq!(both.input_tokens, missing(Reason::FinalConflicting));
    assert_eq!(both.output_tokens, present(0));
    // Present in one final and absent in another is a conflict too.
    let c = fin(json!({"output_tokens": 0, "num_turns": 1}));
    assert_eq!(
        project("codex", &[a, c]).input_tokens,
        missing(Reason::FinalConflicting)
    );
    for bad in [
        json!(-1),
        json!(1.5),
        json!("7"),
        json!(null),
        json!(COUNT_LIMIT + 1),
        json!([1]),
    ] {
        let f = project("codex", &[fin(json!({"input_tokens": bad}))]);
        assert_eq!(f.input_tokens, missing(Reason::FieldMalformed), "{bad}");
        assert_eq!(f.output_tokens, missing(Reason::FieldAbsent));
    }
    for bad in [json!(-0.5), json!("0.1"), json!(null), json!(true)] {
        let f = project("codex", &[fin(json!({"total_cost_usd": bad}))]);
        assert_eq!(
            f.total_cost_usd,
            Fact::Unavailable {
                reason: Reason::FieldMalformed
            },
            "{bad}"
        );
    }
    assert_eq!(
        project("codex", &[fin(json!({"total_cost_usd": 0}))]).total_cost_usd,
        Fact::Present { value: 0.0 }
    );
    assert_eq!(
        project("codex", &[fin(json!({"num_turns": COUNT_LIMIT}))]).num_turns,
        present(COUNT_LIMIT)
    );
}

#[test]
fn receipts_round_trip_and_refuse_anything_untyped() {
    let r = Receipt {
        disposition: "completed".into(),
        attempt_id: "a-1".into(),
        family: "claude".into(),
        facts: project("claude", &smoke("claude")),
    };
    let line = receipt_line(&r);
    assert!(line.len() < RECEIPT_LIMIT && line.ends_with(b"\n"));
    assert_eq!(parse_receipt(&line[..line.len() - 1]), Some(r.clone()));
    let u = NestedUsage::from_receipt("t1", &r);
    assert_eq!(u.validate("t1"), Ok(()));
    assert_eq!(u.validate("t2"), Err("usage_binding_mismatch".into()));
    for (bad, why) in [
        (
            Receipt {
                attempt_id: String::new(),
                ..r.clone()
            },
            "empty attempt",
        ),
        (
            Receipt {
                attempt_id: "a b".into(),
                ..r.clone()
            },
            "unplain attempt",
        ),
        (
            Receipt {
                family: "exec".into(),
                ..r.clone()
            },
            "family",
        ),
        (
            Receipt {
                disposition: "Completed!".into(),
                ..r.clone()
            },
            "code",
        ),
        (
            Receipt {
                facts: Facts {
                    input_tokens: present(COUNT_LIMIT + 1),
                    ..r.facts
                },
                ..r.clone()
            },
            "count",
        ),
        (
            Receipt {
                facts: Facts {
                    total_cost_usd: Fact::Present { value: -1.0 },
                    ..r.facts
                },
                ..r.clone()
            },
            "cost",
        ),
    ] {
        let line = receipt_line(&bad);
        assert_eq!(parse_receipt(&line[..line.len() - 1]), None, "{why}");
    }
    assert_eq!(parse_receipt(b"ready"), None);
    assert_eq!(parse_receipt(b"nested {"), None);
    assert_eq!(
        parse_receipt(b"nested {\"disposition\":\"completed\"}"),
        None
    );
    let lost = NestedUsage::missing("t1", "dsh");
    assert_eq!(lost.validate("t1"), Ok(()));
    let mut forged = lost.clone();
    forged.attempt_id = "a".into();
    assert_eq!(forged.validate("t1"), Err("usage_attempt_invalid".into()));
    let mut forged = u.clone();
    forged.attempt_id = String::new();
    assert_eq!(forged.validate("t1"), Err("usage_attempt_invalid".into()));
    let mut forged = u.clone();
    forged.family = "exec".into();
    assert_eq!(forged.validate("t1"), Err("usage_family_invalid".into()));
    let mut forged = u.clone();
    forged.source = "journal".into();
    assert_eq!(forged.validate("t1"), Err("usage_binding_mismatch".into()));
    let mut forged = u;
    forged.facts.total_cost_usd = Fact::Present { value: f64::NAN };
    assert_eq!(forged.validate("t1"), Err("usage_cost_invalid".into()));
}
