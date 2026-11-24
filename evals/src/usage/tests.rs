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
    // A session keeps a receipt's attempt and facts, or none at all.
    let session = Session::from_receipt(0, &r);
    assert_eq!(session.validate(), Ok(()));
    for lost in [RECEIPT_MISSING, UNSTARTED] {
        assert_eq!(Session::without_receipt(1, lost).validate(), Ok(()));
        let mut forged = Session::without_receipt(1, lost);
        forged.attempt_id = "a".into();
        assert_eq!(forged.validate(), Err("usage_attempt_invalid".into()));
    }
    for edit in [
        (|s: &mut Session| s.attempt_id = String::new()) as fn(&mut Session),
        |s| s.disposition = RECEIPT_MISSING.into(),
        |s| s.disposition = "Completed!".into(),
        |s| s.attempt_id = "a b".into(),
    ] {
        let mut forged = session.clone();
        edit(&mut forged);
        assert_eq!(forged.validate(), Err("usage_attempt_invalid".into()));
    }
    let mut forged = session;
    forged.facts.total_cost_usd = Fact::Present { value: f64::NAN };
    assert_eq!(forged.validate(), Err("usage_cost_invalid".into()));
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

    pub(super) fn bare(n: u32, outcome: Outcome, task: &str) -> TrialResult {
        let req = TrialRequest {
            task: Some(task.into()),
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
        row.validate().unwrap();
        row
    }

    /// A one-agent nested row and, with facts, its companion.
    fn row(n: u32, outcome: Outcome, facts: Option<Facts>) -> (TrialResult, Option<Companion>) {
        let row = bare(n, outcome, "signup");
        let usage = facts.map(|facts| {
            let session = Session {
                agent: 0,
                attempt_id: format!("a{n}"),
                disposition: "completed".into(),
                facts,
            };
            let c = Companion::new(&row, vec![session]).unwrap();
            c.validate(&row).unwrap();
            c
        });
        (row, usage)
    }

    fn summary(rows: &[(TrialResult, Option<Companion>)]) -> Value {
        let usage: Companions = rows
            .iter()
            .filter_map(|(r, u)| u.clone().map(|u| (r.trial_id.clone(), u)))
            .collect();
        let rows: Vec<&TrialResult> = rows.iter().map(|(r, _)| r).collect();
        summarize(&rows, &usage).unwrap()
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
        assert_eq!(
            (s["scored_trials"].clone(), s["passes"].clone()),
            (json!(2), json!(1))
        );
        assert_eq!(s["total_tokens"], json!({"status": "present", "value": 40}));
        assert_eq!(
            s["tokens_per_success"],
            json!({"status": "present", "value": 40.0})
        );
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
        let reason = |rows: &[(TrialResult, Option<Companion>)]| {
            summary(rows)["tokens_per_success"]["reason"].clone()
        };
        assert_eq!(
            reason(&[row(1, Outcome::Fail, Some(counts(1, 1)))]),
            "no_passes"
        );
        // A scored row without its companion in a nested group.
        let rows = [
            row(1, Outcome::Pass, Some(counts(1, 1))),
            row(2, Outcome::Pass, None),
        ];
        assert_eq!(reason(&rows), "incomplete_attribution");
        assert_eq!(
            summary(&rows)["total_cost_usd"]["reason"],
            "incomplete_attribution"
        );
        // A required count missing, the other facts standing on their own.
        let mut lost = counts(1, 1);
        lost.output_tokens = Fact::Unavailable {
            reason: Reason::FieldAbsent,
        };
        lost.total_cost_usd = Fact::Unavailable {
            reason: Reason::FieldAbsent,
        };
        let rows = [
            row(1, Outcome::Pass, Some(lost)),
            row(2, Outcome::Pass, Some(counts(1, 1))),
        ];
        let s = summary(&rows);
        assert_eq!(s["tokens_per_success"]["reason"], "missing_required_count");
        assert_eq!(s["total_cost_usd"]["reason"], "missing_required_count");
        assert_eq!(s["num_turns"]["value"], 4);
        // A sum past the exact range is withheld, never wrapped.
        let big = |n| row(n, Outcome::Pass, Some(counts(COUNT_LIMIT, 0)));
        assert_eq!(reason(&[big(1), big(2)]), "sum_overflow");
        assert_eq!(
            reason(&[row(1, Outcome::Pass, Some(counts(COUNT_LIMIT, 1)))]),
            "sum_overflow"
        );
        let huge = |n| {
            let mut f = counts(1, 1);
            f.total_cost_usd = Fact::Present { value: f64::MAX };
            row(n, Outcome::Pass, Some(f))
        };
        assert_eq!(
            summary(&[huge(1), huge(2)])["total_cost_usd"]["reason"],
            "sum_overflow"
        );
        // A group with no nested row has no nested figures at all.
        let req = TrialRequest {
            task: Some("signup".into()),
            agent: Some("claude".into()),
            argv: vec!["/bin/agent".into()],
            agent_model: Some("claude-sonnet-5-5".into()),
            wrapper_profile: Some(WrapperProfile::Claude2_1_329),
            ..Default::default()
        };
        let direct = runner::refused_row(&req, &"9".repeat(32));
        assert_eq!(summarize(&[&direct], &Companions::new()), None);
        assert_eq!(Companion::new(&direct, Vec::new()), None);
        // Nor has a superseded via-brokkr row, which never ran nested.
        let legacy = runner::refused_row(
            &TrialRequest {
                wrapper_profile: Some(WrapperProfile::Claude2_1_310ViaBrokkr),
                ..req
            },
            &"8".repeat(32),
        );
        assert_eq!(nested_profile(&legacy), None);
    }

    /// The schema-2 row is unchanged (accepted 0119 §3): no usage field, and
    /// the strict historical reader decodes a nested row's bytes.
    #[test]
    fn a_nested_row_carries_no_usage_and_reads_strictly() {
        let (r, usage) = row(1, Outcome::Pass, Some(counts(1, 1)));
        let text = serde_json::to_string(&r).unwrap();
        assert!(!text.contains("usage"), "{text}");
        assert_eq!(crate::results::read(text.as_bytes()).unwrap(), r);
        // The companion is bound to its own row only.
        let usage = usage.unwrap();
        let other = bare(2, Outcome::Pass, "signup");
        assert_eq!(usage.validate(&other), Err("usage_binding_mismatch".into()));
    }

    /// A three-agent trial: every session listed by assignment, the trial's
    /// figure the checked per-field sum, any unavailable member making that
    /// field unavailable, and an overflow unavailable too.
    #[test]
    fn a_multi_agent_companion_sums_every_session_field_by_field() {
        let row = bare(7, Outcome::Pass, "inbox-isolation");
        let session = |i: u8, facts: Facts| Session {
            agent: i,
            attempt_id: format!("a{i}"),
            disposition: "completed".into(),
            facts,
        };
        let c = Companion::new(
            &row,
            vec![
                session(0, counts(10, 1)),
                session(1, counts(20, 2)),
                session(2, counts(0, 0)),
            ],
        )
        .unwrap();
        assert_eq!(c.validate(&row), Ok(()));
        let t = c.totals();
        assert_eq!(t.input_tokens, present(30));
        assert_eq!(t.output_tokens, present(3));
        assert_eq!(t.cache_write_tokens, present(3));
        assert_eq!(t.num_turns, present(6));
        assert_eq!(t.total_cost_usd, Fact::Present { value: 1.5 });
        assert_eq!(c.attempts().collect::<Vec<_>>(), ["a0", "a1", "a2"]);
        // One lost receipt: every field unavailable for that reason, never
        // a partial sum; a member missing one field withholds that field.
        let mut lost = c.clone();
        lost.sessions[1] = Session::without_receipt(1, RECEIPT_MISSING);
        assert_eq!(lost.validate(&row), Ok(()));
        assert_eq!(lost.totals(), Facts::unavailable(Reason::ReceiptMissing));
        let mut absent = c.clone();
        absent.sessions[2].facts.output_tokens = Fact::Unavailable {
            reason: Reason::FieldAbsent,
        };
        let t = absent.totals();
        assert_eq!(t.input_tokens, present(30));
        assert_eq!(
            t.output_tokens,
            Fact::Unavailable {
                reason: Reason::FieldAbsent
            }
        );
        // Overflow and a nonfinite cost sum are unavailable.
        let mut big = c.clone();
        big.sessions[0].facts.input_tokens = present(COUNT_LIMIT);
        let over = Fact::Unavailable {
            reason: Reason::SumOverflow,
        };
        assert_eq!(big.totals().input_tokens, over);
        big.sessions[0].facts.total_cost_usd = Fact::Present { value: f64::MAX };
        big.sessions[1].facts.total_cost_usd = Fact::Present { value: f64::MAX };
        assert_eq!(
            big.totals().total_cost_usd,
            Fact::Unavailable {
                reason: Reason::SumOverflow
            }
        );
        // Ownership: roster, order, duplicate attempts, version and profile.
        let mut short = c.clone();
        short.sessions.pop();
        assert_eq!(short.validate(&row), Err("usage_roster_mismatch".into()));
        let mut swapped = c.clone();
        swapped.sessions.swap(0, 1);
        assert_eq!(swapped.validate(&row), Err("usage_roster_mismatch".into()));
        let mut twice = c.clone();
        twice.sessions[1].attempt_id = "a0".into();
        assert_eq!(twice.validate(&row), Err("usage_attempt_duplicate".into()));
        let mut version = c.clone();
        version.version = 2;
        assert_eq!(
            version.validate(&row),
            Err("usage_version_unsupported".into())
        );
        let mut profile = c.clone();
        profile.profile = WrapperProfile::Codex0_166_0ViaBrokkr;
        assert_eq!(profile.validate(&row), Err("usage_profile_mismatch".into()));
        let mut slot = c.clone();
        slot.slot = Some(crate::provenance::MatrixCoordinates {
            matrix_id: "m".into(),
            master_seed: "s".into(),
            task_index: 0,
            agent_index: 0,
            repetition_index: 0,
        });
        assert_eq!(slot.validate(&row), Err("usage_binding_mismatch".into()));
        let mut bad = c;
        bad.sessions[0].facts.num_turns = present(COUNT_LIMIT + 1);
        assert_eq!(bad.validate(&row), Err("usage_count_invalid".into()));
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
            assert!(
                page.contains(r"writes null (missing\_required\_count)"),
                "{page}"
            );
        }
    }
}
