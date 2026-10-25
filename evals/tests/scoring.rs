//! The catalogue's shape and the pure scorer's precedence (decision 0055
//! §2), over hand-built snapshots: no servers, no agents.

use serde_json::json;
use swarmail_evals::state::*;
use swarmail_evals::tasks::{self, Check, Outcome, Scenario, Task, Tier, Verdict};

#[test]
fn the_catalogue_is_fifteen_tasks_in_three_tiers_plus_isolation() {
    let cat = tasks::catalogue();
    tasks::validate_catalogue(&cat).unwrap();
    assert_eq!(cat.len(), 16);
    let ids: Vec<&str> = cat.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "signup",
            "otp",
            "magic-login",
            "stale-otp",
            "confirm-decoys",
            "sibling-addresses",
            "entity-url",
            "i18n-subject",
            "slow-delivery",
            "smtp-retry",
            "digest-forty",
            "reset-login",
            "reset-reuse",
            "invite-time",
            "support-reply",
            "inbox-isolation"
        ]
    );
    for tier in [Tier::Straight, Tier::Traps, Tier::MultiStep] {
        assert!(cat.iter().any(|t| t.tier == tier));
    }
    assert_eq!(cat.iter().filter(|t| t.agent_count == 3).count(), 1);
    let expect = [
        ("signup", FailureClass::GaveUp, FailureClass::GaveUp),
        ("stale-otp", FailureClass::StaleCode, FailureClass::GaveUp),
        (
            "sibling-addresses",
            FailureClass::WrongMessage,
            FailureClass::GaveUp,
        ),
        (
            "slow-delivery",
            FailureClass::WaitTimeout,
            FailureClass::WaitTimeout,
        ),
        (
            "smtp-retry",
            FailureClass::WaitTimeout,
            FailureClass::WaitTimeout,
        ),
        (
            "reset-reuse",
            FailureClass::ToolMisuse,
            FailureClass::GaveUp,
        ),
    ];
    for (id, target, idle) in expect {
        let t = tasks::find(id).unwrap();
        assert_eq!((t.failure_class, t.idle_class), (target, idle), "{id}");
    }
}

fn base() -> Task {
    tasks::find("signup").unwrap()
}

#[test]
fn validation_refuses_every_malformed_declaration() {
    let mut cases: Vec<(Task, &str)> = Vec::new();
    let mut t = base();
    t.idle_class = FailureClass::WrongLink;
    cases.push((t, "idle_class"));
    let mut t = base();
    t.failure_class = FailureClass::HarnessError;
    cases.push((t, "failure_class"));
    let mut t = base();
    t.deadline_ms = 0;
    cases.push((t, "deadline_ms"));
    let mut t = base();
    t.goal_template = t.goal_template.replace("{inbox}", "inbox");
    cases.push((t, "{inbox}"));
    let mut t = base();
    t.goal.push_str(" {password}");
    cases.push((t, "unknown goal placeholder"));
    let mut t = base();
    t.goal_template.push_str(" {secret}");
    cases.push((t, "unknown template placeholder"));
    let mut t = base();
    t.goal.push_str(" {unclosed");
    cases.push((t, "unclosed"));
    let mut t = tasks::find("smtp-retry").unwrap();
    t.agent_count = 3;
    cases.push((t, "chaos"));
    let mut t = base();
    t.id = "Bad Id".into();
    cases.push((t, "kebab-case"));
    for (task, want) in cases {
        let err = tasks::validate_task(&task).unwrap_err();
        assert!(err.contains(want), "{want}: {err}");
    }
    let dup = vec![base(), base()];
    assert!(
        tasks::validate_catalogue(&dup)
            .unwrap_err()
            .contains("duplicate")
    );
}

#[test]
fn unknown_enum_values_and_fields_refuse_at_deserialization() {
    let good = serde_json::to_value(base()).unwrap();
    for (key, value) in [
        ("tier", json!("bonus")),
        ("failure_class", json!("wrong_mesage")),
        ("idle_class", json!("asleep")),
        ("scenario", json!("signup_v2")),
        ("check", json!("vibes")),
    ] {
        let mut bad = good.clone();
        bad[key] = value;
        assert!(serde_json::from_value::<Task>(bad).is_err(), "{key}");
    }
    let mut bad = good.clone();
    bad["extra"] = json!(1);
    assert!(serde_json::from_value::<Task>(bad).is_err());
    assert!(serde_json::from_value::<FailureClass>(json!("timeout")).is_err());
}

#[test]
fn rendering_fills_every_placeholder_without_rescanning_values() {
    let vars = tasks::PromptVars {
        app_url: "http://127.0.0.1:1",
        mcp_url: "http://127.0.0.1:2/mcp",
        identity: "ada.{goal}@example.test",
        inbox: "eval-x-ada",
    };
    let prompt = tasks::render_prompt(&base(), &vars).unwrap();
    assert!(prompt.contains("http://127.0.0.1:2/mcp"));
    assert!(
        prompt.contains("ada.{goal}@example.test"),
        "a value is never re-rendered"
    );
    assert!(prompt.contains("Task: You just signed up as ada.{goal}@example.test."));
}

fn identity(name: &str, role: Role, facts: Facts) -> IdentityState {
    IdentityState {
        identity: name.into(),
        inbox: "inbox".into(),
        role,
        facts,
    }
}

fn delivered(seq: u64, who: &str, flow: Flow) -> Delivery {
    Delivery {
        seq,
        identity: who.into(),
        flow,
        message: "m".into(),
        status: DeliveryStatus::Delivered,
        attempts: vec![Attempt {
            stage: "dot".into(),
            code: 250,
        }],
    }
}

fn event(
    seq: u64,
    who: &str,
    action: &str,
    accepted: bool,
    category: Option<FailureClass>,
) -> Event {
    Event {
        seq,
        claimed: Some(who.into()),
        flow: None,
        action: action.into(),
        accepted,
        category,
    }
}

fn snap(
    task: &str,
    identities: Vec<IdentityState>,
    deliveries: Vec<Delivery>,
    events: Vec<Event>,
) -> Snapshot {
    Snapshot {
        schema_version: SNAPSHOT_SCHEMA,
        trial_id: "t".into(),
        task: task.into(),
        frozen: true,
        identities,
        deliveries,
        events,
    }
}

fn confirmed() -> Facts {
    Facts {
        confirmed: true,
        ..Default::default()
    }
}

#[test]
fn a_satisfied_predicate_passes_even_after_wrong_actions() {
    let s = snap(
        "signup",
        vec![identity("a", Role::Agent, confirmed())],
        vec![delivered(1, "a", Flow::Signup)],
        vec![
            event(2, "a", "decoy_link", false, Some(FailureClass::WrongLink)),
            event(3, "a", action::CONFIRM, true, None),
        ],
    );
    assert_eq!(tasks::score(&base(), &s).unwrap(), Verdict::pass());
}

#[test]
fn the_first_refused_action_names_the_class_else_the_idle_class() {
    let s = snap(
        "signup",
        vec![identity("a", Role::Agent, Facts::default())],
        vec![delivered(1, "a", Flow::Signup)],
        vec![
            event(2, "a", "reset_page", true, None),
            event(
                3,
                "a",
                "superseded_code",
                false,
                Some(FailureClass::StaleCode),
            ),
            event(4, "a", "decoy_link", false, Some(FailureClass::WrongLink)),
        ],
    );
    assert_eq!(
        tasks::score(&base(), &s).unwrap(),
        Verdict::fail(FailureClass::StaleCode)
    );
    let idle = snap(
        "signup",
        vec![identity("a", Role::Agent, Facts::default())],
        vec![],
        vec![],
    );
    assert_eq!(
        tasks::score(&base(), &idle).unwrap(),
        Verdict::fail(FailureClass::GaveUp)
    );
    let slow = tasks::find("slow-delivery").unwrap();
    let idle = snap(
        "slow-delivery",
        vec![identity("a", Role::Agent, Facts::default())],
        vec![],
        vec![],
    );
    assert_eq!(
        tasks::score(&slow, &idle).unwrap(),
        Verdict::fail(FailureClass::WaitTimeout)
    );
}

#[test]
fn a_fact_without_its_delivery_receipt_does_not_pass() {
    let s = snap(
        "signup",
        vec![identity("a", Role::Agent, confirmed())],
        vec![],
        vec![event(1, "a", action::CONFIRM, true, None)],
    );
    assert_eq!(tasks::score(&base(), &s).unwrap().outcome, Outcome::Fail);
}

#[test]
fn the_expected_replay_refusal_is_evidence_in_reset_reuse_only() {
    let facts = Facts {
        reset_count: 1,
        reset_replay_rejected: true,
        post_reset_login: true,
        logged_in: true,
        ..Default::default()
    };
    let events = vec![
        event(2, "a", action::RESET, true, None),
        event(
            3,
            "a",
            action::RESET_REPLAY,
            false,
            Some(FailureClass::ToolMisuse),
        ),
        event(4, "a", action::LOGIN, true, None),
    ];
    let reuse = tasks::find("reset-reuse").unwrap();
    let s = snap(
        "reset-reuse",
        vec![identity("a", Role::Agent, facts.clone())],
        vec![delivered(1, "a", Flow::Reset)],
        events.clone(),
    );
    assert_eq!(tasks::score(&reuse, &s).unwrap(), Verdict::pass());
    // Without the login the replay is excluded, so the idle class stands.
    let mut f = facts;
    f.post_reset_login = false;
    f.logged_in = false;
    let s = snap(
        "reset-reuse",
        vec![identity("a", Role::Agent, f)],
        vec![delivered(1, "a", Flow::Reset)],
        events[..2].to_vec(),
    );
    assert_eq!(
        tasks::score(&reuse, &s).unwrap(),
        Verdict::fail(FailureClass::GaveUp)
    );
    // In reset-login the same replay is misuse.
    let login = tasks::find("reset-login").unwrap();
    let f = Facts {
        reset_count: 1,
        reset_replay_rejected: true,
        ..Default::default()
    };
    let s = snap(
        "reset-login",
        vec![identity("a", Role::Agent, f)],
        vec![delivered(1, "a", Flow::Reset)],
        events[..2].to_vec(),
    );
    assert_eq!(
        tasks::score(&login, &s).unwrap(),
        Verdict::fail(FailureClass::ToolMisuse)
    );
}

#[test]
fn isolation_forbids_a_foreign_presentation_even_when_all_confirmed() {
    let iso = tasks::find("inbox-isolation").unwrap();
    let names = ["a", "b", "c"];
    let ids = names
        .iter()
        .map(|n| identity(n, Role::Agent, confirmed()))
        .collect();
    let deliveries = names
        .iter()
        .enumerate()
        .map(|(i, n)| delivered(i as u64 + 1, n, Flow::Signup))
        .collect::<Vec<_>>();
    let mut events: Vec<Event> = names
        .iter()
        .enumerate()
        .map(|(i, n)| event(10 + i as u64, n, action::CONFIRM, true, None))
        .collect();
    let clean = snap("inbox-isolation", ids, deliveries, events.clone());
    assert_eq!(tasks::score(&iso, &clean).unwrap(), Verdict::pass());
    events.insert(
        0,
        event(
            5,
            "a",
            action::FOREIGN_TOKEN,
            false,
            Some(FailureClass::WrongMessage),
        ),
    );
    let dirty = Snapshot { events, ..clean };
    assert_eq!(
        tasks::score(&iso, &dirty).unwrap(),
        Verdict::fail(FailureClass::WrongMessage)
    );
}

#[test]
fn malformed_or_mismatched_snapshots_refuse() {
    let good = snap(
        "signup",
        vec![identity("a", Role::Agent, confirmed())],
        vec![delivered(1, "a", Flow::Signup)],
        vec![event(2, "a", action::CONFIRM, true, None)],
    );
    assert!(tasks::score(&base(), &good).is_ok());
    let cases: Vec<(Snapshot, &str)> = vec![
        (
            Snapshot {
                schema_version: 3,
                ..good.clone()
            },
            "schema",
        ),
        (
            Snapshot {
                task: "otp".into(),
                ..good.clone()
            },
            "task",
        ),
        (
            Snapshot {
                frozen: false,
                ..good.clone()
            },
            "frozen",
        ),
        (
            Snapshot {
                events: vec![],
                ..good.clone()
            },
            "no accepted event",
        ),
        (
            Snapshot {
                identities: vec![],
                ..good.clone()
            },
            "agent identities",
        ),
        (
            Snapshot {
                identities: vec![
                    identity("a", Role::Agent, confirmed()),
                    identity("d", Role::Decoy, confirmed()),
                ],
                ..good.clone()
            },
            "decoy",
        ),
        (
            Snapshot {
                events: vec![
                    event(3, "a", action::CONFIRM, true, None),
                    event(2, "a", "x", true, None),
                ],
                ..good.clone()
            },
            "increasing",
        ),
    ];
    for (s, want) in cases {
        let err = tasks::score(&base(), &s).unwrap_err();
        assert!(err.contains(want), "{want}: {err}");
    }
    assert!(
        serde_json::from_value::<Snapshot>(json!({"schema_version": 1, "success": true})).is_err()
    );
}

#[test]
fn each_scenario_maps_to_its_flow_and_only_retry_uses_chaos() {
    assert_eq!(Scenario::Invite.flow(), Flow::Invite);
    assert!(Scenario::SignupRetry.uses_chaos());
    assert_eq!(
        tasks::catalogue()
            .iter()
            .filter(|t| t.scenario.uses_chaos())
            .count(),
        1
    );
    assert_eq!(
        tasks::find("reset-reuse").unwrap().check,
        Check::ResetReuseRefused
    );
}

fn receipt(seq: u64, who: &str, status: DeliveryStatus, attempts: &[(&str, u16)]) -> Delivery {
    Delivery {
        status,
        attempts: attempts
            .iter()
            .map(|&(stage, code)| Attempt {
                stage: stage.into(),
                code,
            })
            .collect(),
        ..delivered(seq, who, Flow::Signup)
    }
}

/// Precedence step 1 (0055 §2): a final failed or parked fixture delivery
/// makes the snapshot unscorable however the business check would read —
/// satisfied, idle, or the failed receipt a decoy's.
#[test]
fn a_failed_or_parked_fixture_delivery_is_unscorable() {
    let satisfied = snap(
        "signup",
        vec![
            identity("a", Role::Agent, confirmed()),
            identity("d", Role::Decoy, Facts::default()),
        ],
        vec![delivered(1, "a", Flow::Signup)],
        vec![event(3, "a", action::CONFIRM, true, None)],
    );
    let idle = Snapshot {
        identities: vec![identity("a", Role::Agent, Facts::default())],
        events: vec![],
        ..satisfied.clone()
    };
    assert_eq!(tasks::score(&base(), &satisfied), Ok(Verdict::pass()));
    assert_eq!(
        tasks::score(&base(), &idle),
        Ok(Verdict::fail(base().idle_class))
    );
    let failed = receipt(2, "d", DeliveryStatus::Failed, &[("connect", 0)]);
    let parked = receipt(2, "a", DeliveryStatus::Parked, &[("data", 451)]);
    let bare = receipt(2, "a", DeliveryStatus::Failed, &[]);
    for base_snap in [&satisfied, &idle] {
        for (bad, says) in [
            (&failed, "Failed after connect 0"),
            (&parked, "Parked after data 451"),
            (&bare, "Failed after no attempt"),
        ] {
            let mut s = base_snap.clone();
            s.deliveries.push(bad.clone());
            let err = tasks::score(&base(), &s).unwrap_err();
            assert!(err.contains(says), "{err}");
            assert_eq!(tasks::validate_snapshot(&base(), &s), Err(err));
        }
    }
}

/// The rule reads the final status, never a non-250 attempt: a delivery
/// retried to 250 is eligible, and held (unreleased) mail is scored as an
/// ordinary business outcome.
#[test]
fn held_mail_and_a_retried_delivery_score_normally() {
    let retried = snap(
        "signup",
        vec![identity("a", Role::Agent, confirmed())],
        vec![receipt(
            1,
            "a",
            DeliveryStatus::Delivered,
            &[("data", 451), ("dot", 250)],
        )],
        vec![event(2, "a", action::CONFIRM, true, None)],
    );
    assert_eq!(tasks::score(&base(), &retried), Ok(Verdict::pass()));
    let slow = tasks::find("slow-delivery").unwrap();
    let held = Snapshot {
        task: slow.id.clone(),
        identities: vec![identity("a", Role::Agent, Facts::default())],
        deliveries: vec![receipt(1, "a", DeliveryStatus::Held, &[])],
        events: vec![],
        ..retried.clone()
    };
    assert_eq!(
        tasks::score(&slow, &held),
        Ok(Verdict::fail(FailureClass::WaitTimeout))
    );
}

fn recovered(task: &Task) -> Snapshot {
    let (facts, actions) = match task.check {
        Check::Confirmed => (confirmed(), vec![action::CONFIRM]),
        Check::OtpLoggedIn => (
            Facts {
                otp_verified: true,
                logged_in: true,
                ..Default::default()
            },
            vec![action::OTP_VERIFY],
        ),
        Check::MagicLoggedIn => (
            Facts {
                magic_login: true,
                logged_in: true,
                ..Default::default()
            },
            vec![action::MAGIC_LOGIN],
        ),
        Check::MarketingCompleted => (
            Facts {
                marketing_completed: true,
                ..Default::default()
            },
            vec![action::MARKETING],
        ),
        Check::DigestSelected => (
            Facts {
                digest_selected: true,
                ..Default::default()
            },
            vec![action::DIGEST_SELECT],
        ),
        _ => unreachable!(),
    };
    snap(
        &task.id,
        vec![
            identity("a", Role::Agent, facts),
            identity("d", Role::Decoy, Facts::default()),
        ],
        vec![delivered(1, "a", task.scenario.flow())],
        actions
            .into_iter()
            .enumerate()
            .map(|(i, a)| event(10 + i as u64, "a", a, true, None))
            .collect(),
    )
}

fn trap_cases() -> Vec<(&'static str, &'static str, FailureClass)> {
    vec![
        (
            "confirm-decoys",
            action::DECOY_LINK,
            FailureClass::WrongLink,
        ),
        (
            "sibling-addresses",
            action::FOREIGN_TOKEN,
            FailureClass::WrongMessage,
        ),
        (
            "i18n-subject",
            action::DECOY_MESSAGE,
            FailureClass::WrongMessage,
        ),
        (
            "stale-otp",
            action::SUPERSEDED_CODE,
            FailureClass::StaleCode,
        ),
        ("magic-login", action::DECOY_LINK, FailureClass::WrongLink),
        ("entity-url", action::DECOY_LINK, FailureClass::WrongLink),
        (
            "entity-url",
            action::MARKETING_MANGLED,
            FailureClass::WrongLink,
        ),
        (
            "digest-forty",
            action::DIGEST_DECOY,
            FailureClass::WrongLink,
        ),
    ]
}

#[test]
fn every_named_trap_precedes_goals_in_both_orders_and_unrelated_refusals() {
    for (id, name, class) in trap_cases() {
        let task = tasks::find(id).unwrap();
        assert!(task.goal.contains("disqualifies the trial"), "{id}");
        assert!(task.goal.contains("already completed it"), "{id}");
        let clean = recovered(&task);
        assert_eq!(tasks::score(&task, &clean), Ok(Verdict::pass()), "{id}");
        for seq in [5, 20] {
            let mut s = clean.clone();
            let mut trap = event(seq, "a", name, false, Some(class));
            trap.flow = Some(task.scenario.flow());
            s.events.push(trap.clone());
            s.events.sort_by_key(|e| e.seq);
            assert_eq!(
                tasks::score(&task, &s),
                Ok(Verdict::fail(class)),
                "{id}/{seq}"
            );
            // A prior generic refusal cannot mask a later named trap; repeated
            // matching events are consumed in the validated sequence order.
            s.events.insert(
                0,
                event(2, "a", "unrelated", false, Some(FailureClass::ToolMisuse)),
            );
            trap.seq = 30;
            s.events.push(trap);
            assert_eq!(tasks::score(&task, &s), Ok(Verdict::fail(class)), "{id}");
            s.identities[0].facts = Facts::default();
            s.events.retain(|e| !e.accepted);
            assert_eq!(
                tasks::score(&task, &s),
                Ok(Verdict::fail(class)),
                "trap only {id}"
            );
        }
    }
}

#[test]
fn exact_match_dimensions_and_sibling_claimant_exception() {
    for (id, name, class) in trap_cases() {
        let task = tasks::find(id).unwrap();
        let clean = recovered(&task);
        let mut trap = event(20, "a", name, false, Some(class));
        trap.flow = Some(task.scenario.flow());
        let mut mismatches = Vec::new();
        let mut e = trap.clone();
        e.accepted = true;
        mismatches.push(e);
        let mut e = trap.clone();
        e.action = "other".into();
        mismatches.push(e);
        let mut e = trap.clone();
        e.category = None;
        mismatches.push(e);
        let mut e = trap.clone();
        e.category = Some(FailureClass::ToolMisuse);
        mismatches.push(e);
        let mut e = trap.clone();
        e.flow = None;
        mismatches.push(e);
        let mut e = trap.clone();
        e.flow = Some(Flow::Support);
        mismatches.push(e);
        let mut e = trap.clone();
        e.claimed = None;
        mismatches.push(e);
        let mut e = trap.clone();
        e.claimed = Some("unknown".into());
        mismatches.push(e);
        if id != "sibling-addresses" {
            let mut e = trap.clone();
            e.claimed = Some("d".into());
            mismatches.push(e);
        }
        for e in mismatches {
            let mut s = clean.clone();
            s.events.push(e);
            assert_eq!(
                tasks::score(&task, &s),
                Ok(Verdict::pass()),
                "{id}: {:?}",
                s.events.last()
            );
        }
        if id == "sibling-addresses" {
            for flow in [None, Some(Flow::Signup), Some(Flow::Support)] {
                let mut s = clean.clone();
                let mut e = trap.clone();
                e.claimed = Some("d".into());
                e.flow = flow;
                s.events.push(e.clone());
                assert_eq!(
                    tasks::score(&task, &s),
                    Ok(Verdict::fail(class)),
                    "{flow:?}"
                );
                for (accepted, category, action) in [
                    (true, Some(class), name),
                    (false, Some(FailureClass::ToolMisuse), name),
                    (false, Some(class), "other"),
                ] {
                    let mut nonmatch = s.clone();
                    let last = nonmatch.events.last_mut().unwrap();
                    last.accepted = accepted;
                    last.category = category;
                    last.action = action.into();
                    assert_eq!(tasks::score(&task, &nonmatch), Ok(Verdict::pass()));
                }
            }
        }
    }
}

#[test]
fn expiry_and_nontarget_recovery_remain_recoverable_and_isolation_keeps_its_fallback() {
    let task = tasks::find("stale-otp").unwrap();
    let mut s = recovered(&task);
    let mut expired = event(2, "a", "expired_code", false, Some(FailureClass::StaleCode));
    expired.flow = Some(Flow::Otp);
    s.events.insert(0, expired);
    assert_eq!(tasks::score(&task, &s), Ok(Verdict::pass()));
    for id in ["signup", "otp"] {
        let task = tasks::find(id).unwrap();
        let mut s = recovered(&task);
        let mut e = event(
            2,
            "a",
            if id == "otp" {
                action::SUPERSEDED_CODE
            } else {
                action::DECOY_LINK
            },
            false,
            Some(FailureClass::StaleCode),
        );
        e.flow = Some(task.scenario.flow());
        s.events.insert(0, e);
        assert_eq!(tasks::score(&task, &s), Ok(Verdict::pass()));
    }
    let task = tasks::find("inbox-isolation").unwrap();
    let mut s = recovered(&task);
    for (i, name) in ["b", "c"].into_iter().enumerate() {
        s.identities.push(identity(name, Role::Agent, confirmed()));
        s.deliveries
            .push(delivered(2 + i as u64, name, Flow::Signup));
        s.events
            .push(event(11 + i as u64, name, action::CONFIRM, true, None));
    }
    s.events.insert(
        0,
        event(5, "a", "unrelated", false, Some(FailureClass::ToolMisuse)),
    );
    s.events.push(event(
        20,
        "a",
        action::FOREIGN_TOKEN,
        false,
        Some(FailureClass::WrongMessage),
    ));
    assert_eq!(
        tasks::score(&task, &s),
        Ok(Verdict::fail(FailureClass::ToolMisuse))
    );
}

#[test]
fn invalid_or_failed_trap_bearing_snapshots_still_refuse_before_scoring() {
    let task = tasks::find("confirm-decoys").unwrap();
    let mut good = recovered(&task);
    let mut e = event(
        20,
        "a",
        action::DECOY_LINK,
        false,
        Some(FailureClass::WrongLink),
    );
    e.flow = Some(Flow::Signup);
    good.events.push(e);
    for status in [DeliveryStatus::Failed, DeliveryStatus::Parked] {
        let mut bad = good.clone();
        bad.deliveries
            .push(receipt(2, "d", status, &[("dot", 451)]));
        assert!(
            tasks::score(&task, &bad)
                .unwrap_err()
                .contains("fixture delivery")
        );
    }
    for version in [1, 3] {
        let mut bad = good.clone();
        bad.schema_version = version;
        assert!(tasks::score(&task, &bad).unwrap_err().contains("schema"));
    }
    let mut bad = good.clone();
    bad.events.reverse();
    assert!(
        tasks::score(&task, &bad)
            .unwrap_err()
            .contains("increasing")
    );
    let mut bad = good.clone();
    bad.events.retain(|e| !e.accepted);
    assert!(
        tasks::score(&task, &bad)
            .unwrap_err()
            .contains("no accepted event")
    );
    let mut bad = good;
    bad.frozen = false;
    assert!(tasks::score(&task, &bad).unwrap_err().contains("frozen"));
}
