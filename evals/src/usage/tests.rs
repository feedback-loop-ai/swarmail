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

mod groups {
    use super::*;
    use crate::provenance::WrapperProfile;
    use crate::runner::{self, TrialRequest};
    use crate::state::FailureClass;

    fn counts(i: u64, o: u64) -> Facts {
        Facts {
            input_tokens: present(i),
            output_tokens: present(o),
            cache_read_tokens: present(i / 2),
            cache_write_tokens: present(1),
            num_turns: present(2),
            total_cost_usd: Fact::Present { value: 0.5 },
        }
    }

    fn row(n: u32, outcome: Outcome, facts: Option<Facts>) -> TrialResult {
        let req = TrialRequest {
            task: Some("signup".into()),
            agent: Some("claude-nested".into()),
            argv: vec!["/bin/agent".into()],
            agent_model: Some("claude-sonnet-5-5".into()),
            wrapper_profile: Some(WrapperProfile::Claude2_1_329ViaBrokkr),
            ..Default::default()
        };
        let trial_id = format!("{n:032x}");
        let mut row = runner::refused_row(&req, &trial_id);
        row.outcome = outcome;
        if outcome != Outcome::HarnessError {
            row.agent_exit_status = Some(0);
            row.failure_class = (outcome == Outcome::Fail).then_some(FailureClass::GaveUp);
        }
        row.nested_usage = facts.map(|facts| NestedUsage {
            source: SOURCE.into(),
            trial_id: trial_id.clone(),
            attempt_id: format!("a{n}"),
            family: "claude".into(),
            facts,
        });
        row.validate().unwrap();
        row
    }

    fn summary(rows: &[TrialResult]) -> Value {
        summarize(&rows.iter().collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn tokens_per_success_charges_every_scored_trial_to_the_passes() {
        // A pass costing 10 and a scored failure costing 30: 40 per success.
        let rows = [
            row(1, Outcome::Pass, Some(counts(4, 6))),
            row(2, Outcome::Fail, Some(counts(20, 10))),
            // A harness error is not scored, whatever it spent.
            row(3, Outcome::HarnessError, Some(counts(500, 500))),
        ];
        let s = summary(&rows);
        assert_eq!(s["source"], SOURCE);
        assert_eq!((s["scored_trials"].clone(), s["passes"].clone()), (json!(2), json!(1)));
        assert_eq!(s["total_tokens"], json!({"status": "present", "value": 40}));
        assert_eq!(s["tokens_per_success"], json!({"status": "present", "value": 40.0}));
        // Caches, turns and cost stand beside the figure, never added.
        assert_eq!(s["cache_read_tokens"]["value"], 12);
        assert_eq!(s["cache_write_tokens"]["value"], 2);
        assert_eq!(s["num_turns"]["value"], 4);
        assert_eq!(s["total_cost_usd"]["value"], 1.0);
        // An explicit zero is a measurement.
        let zero = summary(&[row(1, Outcome::Pass, Some(counts(0, 0)))]);
        assert_eq!(zero["tokens_per_success"]["value"], 0.0);
    }

    #[test]
    fn every_gap_is_a_typed_unavailability_never_a_journal_fill_or_zero() {
        let reason = |rows: &[TrialResult]| summary(rows)["tokens_per_success"]["reason"].clone();
        assert_eq!(reason(&[row(1, Outcome::Fail, Some(counts(1, 1)))]), "no_passes");
        // A scored row without the record in a nested group.
        let bare = row(2, Outcome::Pass, None);
        let rows = [row(1, Outcome::Pass, Some(counts(1, 1))), bare.clone()];
        assert_eq!(reason(&rows), "incomplete_attribution");
        assert_eq!(summary(&rows)["total_cost_usd"]["reason"], "incomplete_attribution");
        // A required count missing, the other facts standing on their own.
        let mut lost = counts(1, 1);
        lost.output_tokens = Fact::Unavailable { reason: Reason::FieldAbsent };
        lost.total_cost_usd = Fact::Unavailable { reason: Reason::FieldAbsent };
        let rows = [row(1, Outcome::Pass, Some(lost)), row(2, Outcome::Pass, Some(counts(1, 1)))];
        let s = summary(&rows);
        assert_eq!(s["tokens_per_success"]["reason"], "missing_required_count");
        assert_eq!(s["total_cost_usd"]["reason"], "missing_required_count");
        assert_eq!(s["num_turns"]["value"], 4);
        // A sum past the exact range is withheld, never wrapped.
        let big = |n| row(n, Outcome::Pass, Some(counts(COUNT_LIMIT, 0)));
        assert_eq!(reason(&[big(1), big(2)]), "sum_overflow");
        assert_eq!(reason(&[row(1, Outcome::Pass, Some(counts(COUNT_LIMIT, 1)))]), "sum_overflow");
        let huge = |n| {
            let mut f = counts(1, 1);
            f.total_cost_usd = Fact::Present { value: f64::MAX };
            row(n, Outcome::Pass, Some(f))
        };
        assert_eq!(summary(&[huge(1), huge(2)])["total_cost_usd"]["reason"], "sum_overflow");
        // A group with no nested record has no nested figures at all.
        let mut direct = bare;
        direct.nested_usage = None;
        assert_eq!(summarize(&[&direct]), None);
    }

    #[test]
    fn the_record_binds_to_a_nested_row_and_a_legacy_row_reads_unchanged() {
        let r = row(1, Outcome::Pass, Some(counts(1, 1)));
        let text = serde_json::to_string(&r).unwrap();
        assert!(text.contains("\"nested_usage\""));
        let back: TrialResult = serde_json::from_str(&text).unwrap();
        assert_eq!(back, r);
        let mut legacy = r.clone();
        legacy.nested_usage = None;
        let text = serde_json::to_string(&legacy).unwrap();
        assert!(!text.contains("nested_usage"));
        assert_eq!(serde_json::from_str::<TrialResult>(&text).unwrap(), legacy);
        // The record is the row's own and only a nested row's.
        let mut other = r.clone();
        other.nested_usage.as_mut().unwrap().trial_id = format!("{:032x}", 9);
        assert_eq!(other.validate(), Err("usage_binding_mismatch".into()));
        let mut family = r.clone();
        family.nested_usage.as_mut().unwrap().family = "codex".into();
        assert_eq!(family.validate(), Err("usage_without_nested_driver".into()));
        let req = TrialRequest {
            task: Some("signup".into()),
            agent: Some("claude".into()),
            argv: vec!["/bin/agent".into()],
            agent_model: Some("claude-sonnet-5-5".into()),
            wrapper_profile: Some(WrapperProfile::Claude2_1_329),
            ..Default::default()
        };
        let mut direct = runner::refused_row(&req, &r.trial_id);
        direct.nested_usage = r.nested_usage.clone();
        assert_eq!(direct.validate(), Err("usage_without_nested_driver".into()));
    }

    #[test]
    fn the_page_takes_a_nested_groups_figure_from_its_usage_alone() {
        use crate::toolvalues::{render_page, tokens_figure};
        let rows = [
            row(1, Outcome::Pass, Some(counts(4, 6))),
            row(2, Outcome::Fail, Some(counts(20, 10))),
        ];
        let mut group = json!({
            "task": "signup", "agent": "claude-nested", "attempts": 2, "passes": 1,
            "fails": 1, "harness_errors": 0, "scored": 2,
            "nested_usage": summary(&rows),
            // A journal figure is never used for a nested group.
            "journal": {"tokens_per_success": 1.0, "tokens": {"cached": {"sum": 5}}},
        });
        assert_eq!(tokens_figure(&group), (Some(40.0), ""));
        let page = render_page(&json!({"groups": [group.clone()]}));
        assert!(page.contains("| 40 | reads 12, writes 2 |"), "{page}");
        for (reason, want) in [
            ("no_passes", "no_passes"),
            ("sum_overflow", "sum_overflow"),
            ("missing_required_count", "missing_required_count"),
            ("incomplete_attribution", "incomplete_attribution"),
            ("anything else", "incomplete_attribution"),
        ] {
            group["nested_usage"]["tokens_per_success"] =
                json!({"status": "unavailable", "reason": reason});
            group["nested_usage"]["cache_write_tokens"] =
                json!({"status": "unavailable", "reason": "missing_required_count"});
            assert_eq!(tokens_figure(&group), (None, want));
            let page = render_page(&json!({"groups": [group.clone()]}));
            assert!(page.contains(r"writes null (missing\_required\_count)"), "{page}");
        }
    }
}
