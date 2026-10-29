//! Pins on the operator's Codex recipes: route independence and inherited
//! proof ownership. These structural guards complement `brokkr compile` and
//! live delivery; they do not claim to execute the engine's policy evaluator.

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;

fn read_json(path: &str) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn office(name: &str) -> Value {
    read_json(&format!("agents/{name}.json"))
}

fn work_offices(recipe: &Value) -> Vec<(String, &'static str)> {
    let seats = &recipe["seats"];
    let base = read_json("bundles/self/bundle.json");
    let design = base["seats"]["design"]["sequence"].clone();
    vec![
        (
            seats["intake"]["agent"].as_str().unwrap().to_owned(),
            "intake",
        ),
        (
            design[0]["panel"]["simplicity"]["agent"]
                .as_str()
                .unwrap()
                .to_owned(),
            "position-simplicity",
        ),
        (
            design[0]["panel"]["robustness"]["agent"]
                .as_str()
                .unwrap()
                .to_owned(),
            "position-robustness",
        ),
        (
            design[1]["agent"].as_str().unwrap().to_owned(),
            "chief-architect",
        ),
        (
            seats["implement"]["agent"].as_str().unwrap().to_owned(),
            "implementer",
        ),
    ]
}

#[test]
fn recipe_discovery_uses_the_existing_bundles_without_copied_definitions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        std::fs::read_link(root.join("recipes")).unwrap(),
        Path::new("bundles"),
        "the relative library alias survives clones and worktrees"
    );
    for name in [
        "self",
        "codex",
        "codex-careful",
        "codex-fast",
        "mixed-opus-terra",
        "claude-careful",
    ] {
        assert_eq!(
            root.join("recipes").join(name).canonicalize().unwrap(),
            root.join("bundles").join(name).canonicalize().unwrap(),
            "{name}: default discovery and explicit compilation must read the same bundle"
        );
    }
}

#[test]
fn recipes_cannot_replace_the_protected_policy_or_the_exec_gates() {
    for (name, parent) in [
        ("codex", "self"),
        ("codex-careful", "codex"),
        ("mixed-opus-terra", "codex-careful"),
    ] {
        let recipe = read_json(&format!("bundles/{name}/bundle.json"));
        assert_eq!(recipe["extends"], parent);
        for forbidden in ["policy", "protected_phase", "remove"] {
            assert!(recipe.get(forbidden).is_none(), "{name}: {forbidden}");
        }
        let overrides = recipe["override"].as_object().unwrap();
        assert!(
            overrides
                .keys()
                .all(|key| matches!(key.as_str(), "bundle" | "seats")),
            "{name}: no table, rule, or phase override may bypass self's policy"
        );
        assert_eq!(overrides["bundle"], json!(["description"]));
        for gate in ["design", "verify", "ship"] {
            assert!(recipe["seats"].get(gate).is_none(), "{name}: {gate}");
        }
        assert!(
            overrides["seats"]
                .as_array()
                .unwrap()
                .iter()
                .all(|seat| matches!(seat.as_str(), Some("intake" | "implement" | "review"))),
            "{name}: only model-seat assignments may change"
        );
    }
}

#[test]
fn both_designs_end_in_a_model_free_offline_validator() {
    let base = read_json("bundles/self/bundle.json");
    let design = base["seats"]["design"]["sequence"].as_array().unwrap();
    assert_eq!(design.len(), 3, "positions, chief, validator");
    assert_eq!(design[0]["aggregate"], "unanimous-pass");
    assert_eq!(design[1]["results"], json!(["drafted"]));
    let check = &design[2];
    assert_eq!(check["name"], "check");
    assert_eq!(check["class"], "gate");
    assert_eq!(check["hands"]["network"], false);
    assert!(check.get("agent").is_none());
    assert!(check.get("results").is_none());
    assert_eq!(
        check["driver"]["command"],
        json!([
            "{brokkr}",
            "driver",
            "exec",
            "--",
            "bash",
            "./scripts/design-check.sh",
            "{prompt_file}"
        ])
    );
    for name in ["codex", "codex-careful", "mixed-opus-terra"] {
        let recipe = read_json(&format!("bundles/{name}/bundle.json"));
        assert!(
            recipe["seats"].get("design").is_none(),
            "{name}: inherit the complete design seat so the exec script retains self's origin"
        );
    }
}

#[test]
fn codex_authors_keep_their_base_charters_and_get_boxed_workspace_tools() {
    for name in ["codex", "codex-careful"] {
        let recipe = read_json(&format!("bundles/{name}/bundle.json"));
        for (agent, base) in work_offices(&recipe) {
            let overlay = office(&agent);
            assert_eq!(overlay["extends"], base, "{agent}");
            assert!(
                overlay.get("charter").is_none(),
                "{agent}: preserve the office's charter"
            );
            assert!(
                office(base).get("extends").is_none(),
                "{agent}: no overlay of an overlay"
            );
            let models = overlay["models"].as_array().unwrap();
            assert_eq!(models.len(), 1, "{agent}: no provider fallback");
            assert!(
                matches!(models[0].as_str(), Some("sol" | "astra")),
                "{agent}"
            );
            assert_eq!(overlay["hands"]["kind"], "workspace", "{agent}");
            assert!(
                overlay["replaces"]["hands"]
                    .as_str()
                    .is_some_and(|s| !s.trim().is_empty())
            );
            let binds = overlay["hands"]["binds"].as_array().unwrap();
            let cargo = binds
                .iter()
                .find(|bind| bind["path"] == "~/.cargo")
                .unwrap();
            assert_eq!(cargo["mode"], "overlay");
            for credential in ["credentials", "credentials.toml"] {
                assert!(
                    cargo["mask"]
                        .as_array()
                        .unwrap()
                        .contains(&json!(credential))
                );
            }
            assert!(
                binds
                    .iter()
                    .any(|bind| bind["path"] == "~/.rustup" && bind["mode"] == "ro")
            );
        }
    }
}

#[test]
fn careful_work_keeps_the_directed_codex_efforts() {
    for (name, model, effort) in [
        ("codex-intake", "sol", "medium"),
        ("codex-position-simplicity", "sol", "high"),
        ("codex-position-robustness", "astra", "high"),
        ("codex-chief-architect", "astra", "high"),
        ("codex-implementer", "sol", "high"),
        ("codex-careful-intake", "sol", "high"),
        ("codex-careful-implementer", "sol", "xhigh"),
    ] {
        let overlay = office(name);
        assert_eq!(overlay["models"], json!([model]), "{name}");
        assert_eq!(overlay["efforts"], json!({model: effort}), "{name}");
    }
}

#[test]
fn standard_and_careful_keep_intake_without_triage() {
    for (recipe, intake) in [
        ("codex", "codex-intake"),
        ("codex-careful", "codex-careful-intake"),
        ("mixed-opus-terra", "mixed-terra-intake"),
    ] {
        let bundle = read_json(&format!("bundles/{recipe}/bundle.json"));
        assert_eq!(bundle["seats"]["intake"]["agent"], intake, "{recipe}");
        assert!(bundle["seats"].get("triage").is_none(), "{recipe}");
    }
    let base = read_json("bundles/self/bundle.json");
    assert!(base["seats"].get("triage").is_none());
}

fn standard_panel() -> Value {
    json!({
        "correctness": {"agent": "codex-review-correctness", "class": "gate"},
        "security": {"agent": "codex-review-security", "class": "gate"},
        "tests": {"agent": "codex-review-tests", "class": "gate"}
    })
}

fn full_panel() -> Value {
    let mut panel = standard_panel();
    panel["correctness"]["agent"] = json!("codex-review-astra-correctness");
    panel["guarantees"] = json!({"agent": "codex-review-guarantees", "class": "gate"});
    panel["independent"] = json!({"agent": "codex-review-independent", "class": "gate"});
    panel
}

#[test]
fn reviewers_keep_exact_models_charters_efforts_and_the_only_fallback() {
    for (name, base, models, effort) in [
        ("correctness", "correctness", vec!["fable"], Some("fable")),
        ("security", "security", vec!["flash"], Some("flash")),
        ("tests", "tests", vec!["glm-flash"], None),
        // Proposed 0098: both full-panel correctness/guarantees members take
        // security's official Flash route; the office names stay.
        ("guarantees", "guarantees", vec!["flash"], Some("flash")),
        (
            "astra-correctness",
            "correctness",
            vec!["flash"],
            Some("flash"),
        ),
        ("independent", "correctness", vec!["fable"], Some("fable")),
        (
            "feature-correctness",
            "correctness",
            vec!["qwen-max", "sonnet"],
            Some("sonnet"),
        ),
    ] {
        let overlay = office(&format!("codex-review-{name}"));
        assert_eq!(overlay["extends"], format!("review-{base}"));
        assert_eq!(overlay["models"], json!(models), "{name}");
        let efforts = match effort {
            Some(model) => json!({model: "high"}),
            None => json!({}),
        };
        assert_eq!(overlay["efforts"], efforts, "{name}");
        assert!(overlay.get("charter").is_none());
        assert!(overlay.get("hands").is_none(), "inherit read-only review");
        assert_eq!(
            office(&format!("review-{base}"))["charter"],
            format!("charters/review-{base}.md")
        );
    }
    let dsh = read_json("adapters/dsh.json");
    assert_eq!(dsh["models"]["glm-flash"], "spark-glm/GLM-5.3-Flash-EXL3");
    assert_eq!(dsh["routes"]["spark-glm"], "local");
    assert!(
        dsh["judges"]
            .as_array()
            .unwrap()
            .contains(&json!("glm-flash"))
    );
    assert!(
        dsh["effortless_routes"]["spark-glm"]
            .as_str()
            .is_some_and(|s| s.contains("not applicable"))
    );
    assert_eq!(dsh["tool_permissions"], "unsupported");
    assert_eq!(dsh["mcp"], "unsupported");
    assert_eq!(dsh["models"]["qwen-max"], "dashscope/qwen3.8-max");
    assert_eq!(dsh["models"]["flash"], "deepseek-flash");
    assert_eq!(dsh["credentials"]["deepseek-official"], "DEEPSEEK_API_KEY");
    assert_eq!(dsh["routes"]["deepseek-official"], "uncontracted");
    assert!(
        dsh["effortless_routes"]["dashscope"]
            .as_str()
            .is_some_and(|s| !s.trim().is_empty())
    );
    assert!(dsh["models"].as_object().unwrap().values().all(|v| {
        let route = v.as_str().unwrap();
        !(route.contains("deepseek") && route.contains("pro"))
    }));
    let claude = read_json("adapters/claude.json");
    for (model, route) in [
        ("fable", "claude-fable-5-1"),
        ("sonnet", "claude-sonnet-5-5"),
    ] {
        assert_eq!(claude["models"][model], route);
        assert!(claude["judges"].as_array().unwrap().contains(&json!(model)));
    }
    assert_eq!(
        read_json("adapters/codex.json")["models"]["astra"],
        "gpt-6-astra"
    );
}

/// Proposed 0098: the two full-panel members that left Astra take exactly
/// security's official DeepSeek Flash route and effort, as separate offices
/// with their own charters; the comparison refuses each one-fault drift.
fn flash_panel_member_violations(office: &dyn Fn(&str) -> Value) -> Vec<String> {
    let mut bad = Vec::new();
    let security = office("codex-review-security");
    let dsh = read_json("adapters/dsh.json");
    if dsh["models"]["flash"] != "deepseek-flash"
        || dsh["routes"]["deepseek-official"] != "uncontracted"
        || dsh["tool_permissions"] != "unsupported"
        || dsh["mcp"] != "unsupported"
    {
        bad.push("dsh flash route or declared capabilities".into());
    }
    for (name, base) in [
        ("codex-review-astra-correctness", "review-correctness"),
        ("codex-review-guarantees", "review-guarantees"),
    ] {
        let overlay = office(name);
        let keys: BTreeSet<&str> = overlay
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        if keys != BTreeSet::from(["efforts", "extends", "models"]) {
            bad.push(format!("{name}: keys {keys:?}"));
        }
        if overlay["extends"] != base {
            bad.push(format!("{name}: charter office"));
        }
        if overlay["models"] != json!(["flash"]) || overlay["models"] != security["models"] {
            bad.push(format!("{name}: model chain"));
        }
        if overlay["efforts"] != json!({"flash": "high"})
            || overlay["efforts"] != security["efforts"]
        {
            bad.push(format!("{name}: effort"));
        }
    }
    bad
}

#[test]
fn full_panel_correctness_and_guarantees_take_the_official_flash_route() {
    let on_disk = |name: &str| office(name);
    assert_eq!(
        flash_panel_member_violations(&on_disk),
        Vec::<String>::new()
    );
    type Drift = fn(&mut Value);
    let drifts: [(&str, &str, Drift); 7] = [
        ("codex-review-astra-correctness", "astra kept", |o| {
            o["models"] = json!(["astra"]);
            o["efforts"] = json!({"astra": "high"})
        }),
        ("codex-review-guarantees", "astra kept", |o| {
            o["models"] = json!(["astra"]);
            o["efforts"] = json!({"astra": "high"})
        }),
        ("codex-review-guarantees", "fallback", |o| {
            o["models"] = json!(["flash", "astra"])
        }),
        ("codex-review-astra-correctness", "effort", |o| {
            o["efforts"] = json!({"flash": "xhigh"})
        }),
        (
            "codex-review-astra-correctness",
            "hands",
            |o| o["hands"] = json!({"kind": "workspace", "network": false, "binds": []}),
        ),
        ("codex-review-guarantees", "merged charter", |o| {
            o["extends"] = json!("review-correctness")
        }),
        ("codex-review-security", "security drift", |o| {
            o["efforts"] = json!({"flash": "medium"})
        }),
    ];
    for (target, mutant, drift) in drifts {
        let drifted = |name: &str| {
            let mut value = office(name);
            if name == target {
                drift(&mut value);
            }
            value
        };
        assert!(
            !flash_panel_member_violations(&drifted).is_empty(),
            "{target}: {mutant} accepted"
        );
    }
}

#[test]
fn codex_review_seats_allow_four_hours_and_other_limits_stay() {
    // Proposed 0098: an explicit seat limit wins over the office's, and the
    // panel shares one deadline, so the review seat itself carries 14400 s.
    let review = json!({"max_attempts": 3, "timeout_seconds": 14400});
    let implement = json!({"max_attempts": 3, "timeout_seconds": 7200});
    for recipe in ["codex", "codex-careful", "codex-fast"] {
        let bundle = read_json(&format!("bundles/{recipe}/bundle.json"));
        assert_eq!(bundle["seats"]["review"]["limits"], review, "{recipe}");
        assert_eq!(
            bundle["seats"]["implement"]["limits"], implement,
            "{recipe}"
        );
    }
    let fast = read_json("bundles/codex-fast/bundle.json");
    assert_eq!(
        fast["seats"]["triage"]["limits"],
        json!({"max_attempts": 1, "timeout_seconds": 1800})
    );
    // mixed-opus-terra inherits careful's review seat rather than copying it.
    let mixed = read_json("bundles/mixed-opus-terra/bundle.json");
    assert!(mixed["seats"].get("review").is_none());
    // The legacy self constitution keeps its original review deadline.
    let base = read_json("bundles/self/bundle.json");
    assert_eq!(
        base["seats"]["review"]["limits"],
        json!({"max_attempts": 3, "timeout_seconds": 3600})
    );
    assert_eq!(
        base["seats"]["implement"]["limits"],
        json!({"max_attempts": 3, "timeout_seconds": 7200})
    );
}

#[test]
fn standard_and_careful_keep_exact_roles_and_identical_review_contracts() {
    let standard = read_json("bundles/codex/bundle.json");
    let careful = read_json("bundles/codex-careful/bundle.json");
    let mut review = standard["seats"]["review"].clone();
    assert_eq!(review["panel"], standard_panel());
    assert_eq!(review["aggregate"], "review-panel");
    review["panel"] = full_panel();
    assert_eq!(careful["seats"]["review"], review);
    assert!(
        careful["override"]["seats"]
            .as_array()
            .unwrap()
            .contains(&json!("review"))
    );
}

#[test]
fn fast_triage_routes_only_bounded_bugs_directly_to_implementation() {
    let recipe = read_json("bundles/codex-fast/bundle.json");
    let policy = read_json("bundles/codex-fast/policy.json");
    assert_eq!(recipe["extends"], "codex");
    assert_eq!(policy["initial"], "triage");
    assert_eq!(recipe["seats"]["triage"]["agent"], "codex-triage");
    let triage = office("codex-triage");
    assert_eq!(triage["extends"], "intake");
    assert_eq!(triage["models"], json!(["opus"]));
    assert_eq!(triage["efforts"], json!({"opus": "medium"}));
    assert_eq!(triage["charter"], "charters/codex-triage.md");
    assert!(triage.get("limits").is_none(), "inherit intake's budget");
    assert_eq!(
        office("intake")["limits"],
        json!({"max_attempts": 3, "timeout_seconds": 3600})
    );
    assert_eq!(
        triage["hands"],
        json!({"kind": "workspace", "network": true, "binds": [
            {"path": "~/.cargo", "mode": "overlay", "mask": ["credentials.toml", "credentials"]},
            {"path": "~/.rustup", "mode": "ro"}
        ]})
    );
    let claude = read_json("adapters/claude.json");
    assert_eq!(claude["provider"], "claude");
    assert_eq!(claude["models"]["opus"], "claude-opus-5-5");
    assert!(
        claude["efforts"]
            .as_array()
            .unwrap()
            .contains(&json!("medium"))
    );
    let results = recipe["seats"]["triage"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 5);
    let rules = policy["rules"].as_array().unwrap();
    for (class, destination) in [
        ("chore", "implement"),
        ("feature", "implement"),
        ("design", "design"),
        ("engine", "design"),
        ("escalate", "stop"),
    ] {
        assert!(results.contains(&json!(class)));
        let routes: Vec<_> = rules
            .iter()
            .filter(|rule| {
                rule["from"] == "triage" && rule["result"] == class && rule.get("when").is_none()
            })
            .collect();
        assert_eq!(routes.len(), 1, "{class}: one unconditional routing rule");
        assert_eq!(routes[0]["next"], destination, "{class}");
        if class == "escalate" {
            assert_eq!(routes[0]["severity"], "hard");
        }
    }
}

#[test]
fn fast_strategy_selectors_have_no_default_and_keep_one_two_or_five_reviewers() {
    let recipe = read_json("bundles/codex-fast/bundle.json");
    let expected: BTreeSet<_> = ["chore", "feature", "design", "engine"]
        .into_iter()
        .collect();
    for seat in ["implement", "review"] {
        let selector = &recipe["seats"][seat]["select"];
        assert_eq!(selector["on"], "strategy");
        assert!(
            selector
                .as_object()
                .unwrap()
                .keys()
                .all(|key| matches!(key.as_str(), "on" | "cases")),
            "{seat}: an absent or unknown strategy must not select a default"
        );
        let cases: BTreeSet<_> = selector["cases"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(cases, expected, "{seat}");
    }
    let cases = &recipe["seats"]["review"]["select"]["cases"];
    let single = &cases["chore"];
    assert_eq!(single["agent"], "codex-bug-reviewer");
    assert_eq!(single["class"], "gate");
    assert!(single.get("panel").is_none());
    let reviewer = office("codex-bug-reviewer");
    assert_eq!(reviewer["extends"], "reviewer");
    assert_eq!(reviewer["models"], json!(["flash"]));
    let mut feature = standard_panel();
    feature.as_object_mut().unwrap().remove("tests");
    feature["correctness"]["agent"] = json!("codex-review-feature-correctness");
    for (class, expected) in [
        ("feature", feature),
        ("design", full_panel()),
        ("engine", full_panel()),
    ] {
        assert_eq!(cases[class]["aggregate"], "review-panel");
        assert_eq!(cases[class]["panel"], expected, "{class}");
    }
    let implement = &recipe["seats"]["implement"]["select"]["cases"];
    for class in ["chore", "feature", "design", "engine"] {
        assert_eq!(
            implement[class]["agent"], "codex-fast-implementer",
            "{class}: settled fast implementer office"
        );
        let overlay = office(implement[class]["agent"].as_str().unwrap());
        assert_eq!(overlay["models"], json!(["luna"]), "{class}: Codex only");
        assert_eq!(overlay["efforts"]["luna"], "xhigh", "{class}: effort");
    }
}

#[test]
fn fast_implementation_uses_luna_at_xhigh_without_provider_fallback() {
    let implementer = office("codex-fast-implementer");
    assert_eq!(implementer["extends"], "implementer");
    assert_eq!(implementer["models"], json!(["luna"]));
    assert_eq!(implementer["efforts"]["luna"], "xhigh");
    assert_eq!(
        read_json("adapters/codex.json")["models"]["luna"],
        "gpt-6-luna"
    );
    assert_eq!(
        read_json("adapters/codex.json")["efforts"],
        json!(["none", "minimal", "low", "medium", "high", "xhigh", "max"])
    );
}

#[test]
fn fast_recipe_preserves_shell_proofs_design_origin_and_hard_review_refusals() {
    let recipe = read_json("bundles/codex-fast/bundle.json");
    for seat in ["design", "verify", "ship"] {
        assert!(recipe["seats"].get(seat).is_none(), "inherit {seat}");
    }
    assert!(recipe.get("protected_phase").is_none());
    assert_eq!(recipe["remove"]["seats"], json!(["intake"]));
    assert_eq!(recipe["remove"]["phases"], json!(["intake"]));
    assert_eq!(recipe["remove"]["rules"], json!(["INTAKE-OK"]));
    let overridden = recipe["override"]["rules"].as_array().unwrap();
    let policy = read_json("bundles/codex-fast/policy.json");
    let rules = policy["rules"].as_array().unwrap();
    for hard_stop in [
        "REVIEW-SECURITY-HOLD",
        "REVIEW-RESIDUAL-ABOVE-MEDIUM",
        "REVIEW-RESIDUAL-UNSAFE",
    ] {
        assert!(!overridden.contains(&json!(hard_stop)), "{hard_stop}");
        assert!(rules.iter().all(|rule| rule["id"] != hard_stop));
    }
    assert!(
        rules
            .iter()
            .all(|rule| { !(rule["from"] == "review" && rule["result"] == "security-hold") })
    );
    let residual_routes: Vec<_> = rules
        .iter()
        .filter(|rule| {
            rule["from"] == "review"
                && rule["result"] == "residual"
                && matches!(rule["next"].as_str(), Some("ship" | "triage"))
        })
        .collect();
    assert_eq!(
        residual_routes.len(),
        3,
        "small-bug reclassification and the small/broader ship routes must exist"
    );
    for rule in residual_routes {
        assert_eq!(rule["when"]["has_security_residual"], false, "{rule}");
        assert_eq!(
            rule["when"]["max_residual_severity_at_most"], "medium",
            "{rule}: security, high severity and unknown facts cannot take a cheap route"
        );
    }
}

#[test]
fn fast_reclassification_has_a_finite_budget_and_explicit_scope_facts() {
    let policy = read_json("bundles/codex-fast/policy.json");
    let rules = policy["rules"].as_array().unwrap();
    for class in ["chore", "feature", "design", "engine"] {
        let cap = rules
            .iter()
            .position(|rule| {
                rule["from"] == "triage"
                    && rule["result"] == class
                    && rule["when"]["visits_triage_gte"] == 3
            })
            .unwrap();
        let route = rules
            .iter()
            .position(|rule| {
                rule["from"] == "triage" && rule["result"] == class && rule.get("when").is_none()
            })
            .unwrap();
        assert!(
            cap < route,
            "{class}: stop a second return before routing it"
        );
        assert_eq!(rules[cap]["next"], "stop");
        assert_eq!(rules[cap]["severity"], "hard");
    }
    let oversize: Vec<_> = rules
        .iter()
        .filter(|rule| rule["from"] == "implement" && rule["result"] == "oversized")
        .collect();
    assert_eq!(oversize.len(), 2);
    assert_eq!(oversize[0]["when"]["visits_triage_gte"], 2);
    assert_eq!(oversize[0]["next"], "stop");
    assert_eq!(oversize[0]["severity"], "hard");
    assert!(oversize[1].get("when").is_none());
    assert_eq!(oversize[1]["next"], "triage");
    let reclassify: Vec<_> = rules
        .iter()
        .filter(|rule| rule["from"] == "review" && rule["next"] == "triage")
        .collect();
    let reclassified_results: BTreeSet<_> = reclassify
        .iter()
        .map(|rule| rule["result"].as_str().unwrap())
        .collect();
    assert_eq!(reclassified_results, BTreeSet::from(["clean", "residual"]));
    for rule in reclassify {
        assert_eq!(
            rule["when"]["strategy_in"],
            json!(["chore", "feature", "design", "engine"])
        );
        assert_eq!(rule["when"]["spec_defect"], true);
        assert_eq!(rule["when"]["fixes_applied"], false);
    }
    let known_scope_ship: Vec<_> = rules
        .iter()
        .filter(|rule| rule["from"] == "review" && rule["next"] == "ship")
        .collect();
    let shippable_results: BTreeSet<_> = known_scope_ship
        .iter()
        .map(|rule| rule["result"].as_str().unwrap())
        .collect();
    assert_eq!(shippable_results, BTreeSet::from(["clean", "residual"]));
    for rule in known_scope_ship {
        assert_eq!(rule["when"]["spec_defect"], false);
        assert_eq!(rule["when"]["fixes_applied"], false);
    }
    assert!(rules.iter().any(|rule| {
        rule["from"] == "review"
            && rule["result"] == "clean"
            && rule["when"] == json!({"strategy_in": ["chore"]})
            && rule["next"] == "stop"
            && rule["severity"] == "hard"
    }));
}

#[test]
fn full_review_reverification_requires_known_scope_and_unknown_clean_facts_stop() {
    let base = read_json("bundles/self/policy.json");
    let base_rules = base["rules"].as_array().unwrap();
    for id in ["REVIEW-CLEAN-NO-FIXES", "REVIEW-RESIDUAL-OK"] {
        let rule = base_rules.iter().find(|rule| rule["id"] == id).unwrap();
        assert_eq!(
            rule["when"]["spec_defect"], false,
            "{id}: inherited full recipes require explicit scope"
        );
        assert_eq!(
            rule["when"]["fixes_applied"], false,
            "{id}: inherited full recipes require unchanged code"
        );
    }
    let base_reverify = base_rules
        .iter()
        .find(|rule| rule["id"] == "REVIEW-CLEAN")
        .unwrap();
    assert_eq!(
        base_reverify["when"],
        json!({"spec_defect": false, "fixes_applied": true})
    );
    let base_unknown = base_rules
        .iter()
        .find(|rule| rule["id"] == "REVIEW-CLEAN-UNSAFE")
        .unwrap();
    assert_eq!(base_unknown["next"], "stop");
    assert!(base_unknown.get("when").is_none());
    let policy = read_json("bundles/codex-fast/policy.json");
    let rules = policy["rules"].as_array().unwrap();
    let reverify = rules
        .iter()
        .find(|rule| rule["id"] == "REVIEW-CLEAN")
        .unwrap();
    assert_eq!(reverify["next"], "verify");
    assert_eq!(reverify["when"]["fixes_applied"], true);
    assert_eq!(reverify["when"]["spec_defect"], false);
    assert_eq!(
        reverify["when"]["strategy_in"],
        json!(["feature", "design", "engine"])
    );
    let fallback = rules
        .iter()
        .position(|rule| rule["id"] == "REVIEW-CLEAN-UNSAFE")
        .unwrap();
    assert_eq!(rules[fallback]["result"], "clean");
    assert_eq!(rules[fallback]["next"], "stop");
    assert_eq!(rules[fallback]["severity"], "hard");
    assert!(rules[fallback].get("when").is_none());
    assert!(
        rules
            .iter()
            .position(|rule| rule["id"] == "REVIEW-CLEAN")
            .unwrap()
            < fallback
    );
}

#[test]
fn ship_closeout_refuses_missing_empty_and_mismatched_ledgers() {
    use std::process::Command;
    let repo = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q"]);
    std::fs::write(repo.path().join(".gitignore"), ".forge/\n").unwrap();
    git(&["add", ".gitignore"]);
    git(&[
        "-c",
        "user.name=Ship proof",
        "-c",
        "user.email=proof@invalid",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "Record a clean close-out candidate",
    ]);
    let head = git(&["rev-parse", "HEAD"]);
    let state = repo.path().join(".forge");
    std::fs::create_dir_all(state.join("ledger")).unwrap();
    std::fs::create_dir_all(state.join("results")).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("bundles/self/scripts/ship-seat.sh");
    let ledger = state.join("ledger/probe.md");
    for case in ["valid", "missing", "empty", "mismatch"] {
        match case {
            "valid" => std::fs::write(&ledger, format!("Repository head: `{head}`.\n")).unwrap(),
            "missing" => std::fs::remove_file(&ledger).unwrap(),
            "empty" => std::fs::write(&ledger, "").unwrap(),
            "mismatch" => std::fs::write(
                &ledger,
                "Repository head: `0000000000000000000000000000000000000000`.\n",
            )
            .unwrap(),
            _ => unreachable!(),
        }
        let result = state.join("results").join(format!("{case}.json"));
        let prompt = state.join(format!("{case}.md"));
        let context = format!(
            "    {}\n\n```json\n{{\n  \"run_id\": \"probe\",\n  \"last_decision\": {{\n    \"rule_id\": \"SHIP-READY\"\n  }}\n}}\n```\n",
            result.display()
        );
        std::fs::write(&prompt, context).unwrap();
        let output = Command::new("bash")
            .arg(&script)
            .arg(&prompt)
            .current_dir(repo.path())
            .output()
            .unwrap();
        if case == "valid" {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let result: Value = serde_json::from_slice(&std::fs::read(&result).unwrap()).unwrap();
            assert_eq!(result["result"], "shipped");
        } else {
            assert!(
                !output.status.success(),
                "{case}: a discrepancy must refuse, not report shipped"
            );
            assert!(
                !result.exists(),
                "{case}: no shippable result may be delivered"
            );
            let diagnostic = String::from_utf8(output.stderr).unwrap();
            assert!(
                diagnostic.contains("close-out discrepancy"),
                "{case}: {diagnostic}"
            );
        }
    }
}

fn mixed_work_route_violations(
    recipe: &Value,
    careful: &Value,
    codex: &Value,
    office: &dyn Fn(&str) -> Value,
) -> Vec<String> {
    let mut bad = Vec::new();
    let keys: BTreeSet<&str> = recipe
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    if keys != BTreeSet::from(["description", "extends", "name", "override", "seats"]) {
        bad.push(format!("recipe keys {keys:?}"));
    }
    if recipe["extends"] != "codex-careful" {
        bad.push("extends".into());
    }
    if recipe["override"] != json!({"bundle": ["description"], "seats": ["intake", "implement"]}) {
        bad.push("override".into());
    }
    let seats: BTreeSet<&str> = recipe["seats"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    if seats != BTreeSet::from(["implement", "intake"]) {
        bad.push(format!("seats {seats:?}"));
    }
    for (seat, agent, model, concrete) in [
        ("intake", "mixed-terra-intake", "terra61", "gpt-6.1-terra"),
        (
            "implement",
            "mixed-opus-implementer",
            "opus",
            "claude-opus-5-5",
        ),
    ] {
        // Only the agent differs from careful's seat record: results and limits are inherited.
        let mut inherited = careful["seats"][seat].clone();
        inherited["agent"] = json!(agent);
        if recipe["seats"][seat] != inherited {
            bad.push(format!("{seat}: seat record"));
        }
        let overlay = office(agent);
        let base = office(careful["seats"][seat]["agent"].as_str().unwrap());
        let overlay_keys: BTreeSet<&str> = overlay
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        if overlay_keys != BTreeSet::from(["efforts", "extends", "hands", "models", "replaces"]) {
            bad.push(format!("{agent}: keys {overlay_keys:?}"));
        }
        if overlay["extends"] != base["extends"] {
            bad.push(format!("{agent}: charter office"));
        }
        if overlay["models"] != json!([model]) || overlay["efforts"] != json!({model: "high"}) {
            bad.push(format!("{agent}: model or effort"));
        }
        if overlay["hands"] != base["hands"] {
            bad.push(format!("{agent}: hands differ from careful"));
        }
        let adapter = if model == "opus" {
            read_json("adapters/claude.json")
        } else {
            codex.clone()
        };
        if adapter["models"][model] != concrete {
            bad.push(format!("{agent}: {model} pin"));
        }
    }
    if codex["models"]["terra"] != "gpt-5.6-terra" {
        bad.push("historical terra alias".into());
    }
    bad
}

#[test]
fn operator_mixed_recipe_overrides_only_its_two_work_seats() {
    let recipe = read_json("bundles/mixed-opus-terra/bundle.json");
    let careful = read_json("bundles/codex-careful/bundle.json");
    let codex = read_json("adapters/codex.json");
    let on_disk = |name: &str| office(name);
    assert_eq!(
        mixed_work_route_violations(&recipe, &careful, &codex, &on_disk),
        Vec::<String>::new()
    );
    // Overlays keep the base offices' charters and limits: 14400s per office,
    // under the recipe seat's own 7200s and three attempts.
    for (agent, base) in [
        ("mixed-terra-intake", "intake"),
        ("mixed-opus-implementer", "implementer"),
    ] {
        assert_eq!(office(agent)["extends"], base);
        assert!(office(base).get("extends").is_none(), "{agent}");
        assert!(office(base)["charter"].is_string(), "{agent}");
    }
    assert_eq!(office("implementer")["limits"]["timeout_seconds"], 14400);
    assert_eq!(
        recipe["seats"]["implement"]["limits"],
        json!({"max_attempts": 3, "timeout_seconds": 7200})
    );

    // The same comparison refuses each one-fault drift.
    let mut extra = recipe.clone();
    extra["override"]["seats"]
        .as_array_mut()
        .unwrap()
        .push(json!("review"));
    let mut review = recipe.clone();
    review["seats"]["review"] = careful["seats"]["review"].clone();
    let mut limits = recipe.clone();
    limits["seats"]["implement"]["limits"]["timeout_seconds"] = json!(14400);
    let mut agent = recipe.clone();
    agent["seats"]["intake"]["agent"] = json!("codex-careful-intake");
    let mut parent = recipe.clone();
    parent["extends"] = json!("codex");
    let mut policy = recipe.clone();
    policy["policy"] = json!("policy.json");
    let mut alias = codex.clone();
    alias["models"]["terra"] = json!("gpt-6.1-terra");
    let mut pin = codex.clone();
    pin["models"]["terra61"] = json!("gpt-5.6-terra");
    for (mutant, recipe, codex) in [
        ("extra override", &extra, &codex),
        ("copied review", &review, &codex),
        ("seat limits", &limits, &codex),
        ("careful intake", &agent, &codex),
        ("parent", &parent, &codex),
        ("copied policy", &policy, &codex),
        ("historical alias", &recipe, &alias),
        ("terra61 pin", &recipe, &pin),
    ] {
        assert!(
            !mixed_work_route_violations(recipe, &careful, codex, &on_disk).is_empty(),
            "{mutant} accepted"
        );
    }
    type Drift = fn(&mut Value);
    let drifts: [(&str, &str, Drift); 6] = [
        ("mixed-opus-implementer", "network", |o| {
            o["hands"]["network"] = json!(false)
        }),
        ("mixed-opus-implementer", "credential mask", |o| {
            o["hands"]["binds"][0]["mask"] = json!(["credentials.toml"])
        }),
        ("mixed-terra-intake", "writable rustup", |o| {
            o["hands"]["binds"][1]["mode"] = json!("overlay")
        }),
        ("mixed-terra-intake", "fallback", |o| {
            o["models"] = json!(["terra61", "terra"])
        }),
        ("mixed-opus-implementer", "effort", |o| {
            o["efforts"] = json!({"opus": "xhigh"})
        }),
        ("mixed-opus-implementer", "charter", |o| {
            o["charter"] = json!("charters/review-correctness.md")
        }),
    ];
    for (target, mutant, drift) in drifts {
        let drifted = |name: &str| {
            let mut value = office(name);
            if name == target {
                drift(&mut value);
            }
            value
        };
        assert!(
            !mixed_work_route_violations(&recipe, &careful, &codex, &drifted).is_empty(),
            "{mutant} accepted"
        );
    }
}

#[test]
fn operator_mixed_recipe_inherits_the_careful_panel_unchanged() {
    let recipe = read_json("bundles/mixed-opus-terra/bundle.json");
    let careful = read_json("bundles/codex-careful/bundle.json");
    assert!(recipe["seats"].get("review").is_none());
    assert_eq!(careful["seats"]["review"]["panel"], full_panel());
    for (role, model) in [
        ("astra-correctness", "flash"),
        ("guarantees", "flash"),
        ("independent", "fable"),
        ("security", "flash"),
        ("tests", "glm-flash"),
    ] {
        assert_eq!(
            office(&format!("codex-review-{role}"))["models"],
            json!([model])
        );
    }
    assert_eq!(
        read_json("adapters/dsh.json")["models"]["glm-flash"],
        "spark-glm/GLM-5.3-Flash-EXL3"
    );
}

// Proposed 0099: the operator's zero-Codex careful lane. One row per model
// seat: compiled key, office, base office (charter), adapter, model key,
// concrete pin, office effort (`None` = not applicable) and workspace hands.
type RouteRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
    bool,
);
const CLAUDE_CAREFUL_ROUTE: [RouteRow; 10] = [
    (
        "intake",
        "claude-intake",
        "intake",
        "claude",
        "haiku55",
        "claude-haiku-5-5",
        Some("high"),
        true,
    ),
    (
        "design:positions:simplicity",
        "claude-position-simplicity",
        "position-simplicity",
        "claude",
        "opus",
        "claude-opus-5-5",
        Some("high"),
        true,
    ),
    (
        "design:positions:robustness",
        "dsh-position-robustness",
        "position-robustness",
        "dsh",
        "flash",
        "deepseek-flash",
        Some("high"),
        false,
    ),
    (
        "design:chief",
        "claude-chief-architect",
        "chief-architect",
        "claude",
        "fable",
        "claude-fable-5-1",
        Some("high"),
        true,
    ),
    (
        "implement",
        "claude-implementer",
        "implementer",
        "claude",
        "sonnet",
        "claude-sonnet-5-5",
        Some("high"),
        true,
    ),
    (
        "review:correctness",
        "codex-review-correctness",
        "review-correctness",
        "claude",
        "fable",
        "claude-fable-5-1",
        Some("high"),
        false,
    ),
    (
        "review:security",
        "codex-review-security",
        "review-security",
        "dsh",
        "flash",
        "deepseek-flash",
        Some("high"),
        false,
    ),
    (
        "review:guarantees",
        "codex-review-guarantees",
        "review-guarantees",
        "dsh",
        "flash",
        "deepseek-flash",
        Some("high"),
        false,
    ),
    (
        "review:tests",
        "codex-review-tests",
        "review-tests",
        "dsh",
        "glm-flash",
        "spark-glm/GLM-5.3-Flash-EXL3",
        None,
        false,
    ),
    (
        "review:independent",
        "claude-review-independent",
        "review-correctness",
        "claude",
        "opus",
        "claude-opus-5-5",
        Some("high"),
        false,
    ),
];

/// The office each model seat of a recipe names, keyed as the compiled
/// manifest keys them.
fn claude_careful_seat_agents(recipe: &Value) -> BTreeSet<(String, String)> {
    let seats = &recipe["seats"];
    let design = &seats["design"]["sequence"];
    let mut agents = BTreeSet::new();
    let mut put = |key: &str, agent: &Value| {
        agents.insert((key.to_owned(), agent.as_str().unwrap_or("").to_owned()));
    };
    put("intake", &seats["intake"]["agent"]);
    put("implement", &seats["implement"]["agent"]);
    for role in ["simplicity", "robustness"] {
        put(
            &format!("design:positions:{role}"),
            &design[0]["panel"][role]["agent"],
        );
    }
    put("design:chief", &design[1]["agent"]);
    if let Some(panel) = seats["review"]["panel"].as_object() {
        for (role, member) in panel {
            put(&format!("review:{role}"), &member["agent"]);
        }
    }
    agents
}

/// Every office the composed recipe hires, walking the extends chain with
/// whole-seat overrides (the leaf's seat wins), as the resolver composes it.
fn composed_offices(leaf: &Value) -> Vec<String> {
    fn collect(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, member) in map {
                    match (key.as_str(), member.as_str()) {
                        ("agent", Some(agent)) => out.push(agent.to_owned()),
                        _ => collect(member, out),
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|item| collect(item, out)),
            _ => {}
        }
    }
    let mut seats = serde_json::Map::new();
    let mut layer = leaf.clone();
    loop {
        for (name, seat) in layer["seats"].as_object().unwrap() {
            seats.entry(name.clone()).or_insert_with(|| seat.clone());
        }
        match layer["extends"].as_str() {
            Some(parent) => layer = read_json(&format!("bundles/{parent}/bundle.json")),
            None => break,
        }
    }
    let mut out = Vec::new();
    collect(&Value::Object(seats), &mut out);
    out
}

fn claude_careful_violations(
    recipe: &Value,
    office: &dyn Fn(&str) -> Value,
    adapter: &dyn Fn(&str) -> Value,
) -> Vec<String> {
    let mut bad = Vec::new();
    let keys: BTreeSet<&str> = recipe
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    if keys != BTreeSet::from(["description", "extends", "name", "override", "seats"]) {
        bad.push(format!("recipe keys {keys:?}"));
    }
    if recipe["extends"] != "codex-careful" {
        bad.push("extends".into());
    }
    if recipe["override"]
        != json!({"bundle": ["description"], "seats": ["intake", "design", "implement", "review"]})
    {
        bad.push("override".into());
    }
    let seats: BTreeSet<&str> = recipe["seats"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    if seats != BTreeSet::from(["design", "implement", "intake", "review"]) {
        bad.push(format!("seats {seats:?}"));
    }
    let wanted: BTreeSet<(String, String)> = CLAUDE_CAREFUL_ROUTE
        .iter()
        .map(|row| (row.0.to_owned(), row.1.to_owned()))
        .collect();
    if claude_careful_seat_agents(recipe) != wanted {
        bad.push("seat offices".into());
    }

    // Outside the offices, each seat is the record it replaces: careful's
    // intake, implement and review, self's design with its exec check.
    let careful = read_json("bundles/codex-careful/bundle.json");
    let base = read_json("bundles/self/bundle.json");
    let mut intake = careful["seats"]["intake"].clone();
    intake["agent"] = json!("claude-intake");
    let mut implement = careful["seats"]["implement"].clone();
    implement["agent"] = json!("claude-implementer");
    implement["limits"] = json!({"max_attempts": 3, "timeout_seconds": 14400});
    let mut review = careful["seats"]["review"].clone();
    review["panel"]["correctness"]["agent"] = json!("codex-review-correctness");
    review["panel"]["independent"]["agent"] = json!("claude-review-independent");
    let mut design = base["seats"]["design"].clone();
    design["sequence"][0]["panel"]["simplicity"]["agent"] = json!("claude-position-simplicity");
    design["sequence"][0]["panel"]["robustness"]["agent"] = json!("dsh-position-robustness");
    design["sequence"][1]["agent"] = json!("claude-chief-architect");
    for (seat, expected) in [
        ("intake", intake),
        ("implement", implement),
        ("review", review),
        ("design", design),
    ] {
        if recipe["seats"][seat] != expected {
            bad.push(format!("{seat}: seat record"));
        }
    }
    for seat in ["implement", "review"] {
        if recipe["seats"][seat]["limits"] != json!({"max_attempts": 3, "timeout_seconds": 14400}) {
            bad.push(format!("{seat}: limits"));
        }
    }

    let work_hands = office("codex-careful-intake")["hands"].clone();
    for (key, agent, base, provider, model, concrete, effort, hands) in CLAUDE_CAREFUL_ROUTE {
        let overlay = office(agent);
        let mut expected_keys = BTreeSet::from(["efforts", "extends", "models"]);
        if hands {
            expected_keys.extend(["hands", "replaces"]);
        }
        let overlay_keys: BTreeSet<&str> = overlay
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        if overlay_keys != expected_keys {
            bad.push(format!("{key}: keys {overlay_keys:?}"));
        }
        if overlay["extends"] != base
            || office(base).get("extends").is_some()
            || !office(base)["charter"].is_string()
        {
            bad.push(format!("{key}: charter office"));
        }
        if overlay["models"] != json!([model]) {
            bad.push(format!("{key}: model chain"));
        }
        let efforts = effort.map_or(json!({}), |effort| json!({model: effort}));
        if overlay["efforts"] != efforts {
            bad.push(format!("{key}: effort"));
        }
        if hands
            && (overlay["hands"] != work_hands
                || !overlay["replaces"]["hands"]
                    .as_str()
                    .is_some_and(|s| !s.trim().is_empty()))
        {
            bad.push(format!("{key}: hands"));
        }
        if adapter(provider)["models"][model] != concrete {
            bad.push(format!("{key}: {model} pin"));
        }
    }

    // No Codex model anywhere in the composed route.
    let codex = adapter("codex");
    for agent in composed_offices(recipe) {
        let mut models = office(&agent)["models"].clone();
        let mut current = office(&agent);
        while models.is_null() {
            current = office(current["extends"].as_str().unwrap());
            models = current["models"].clone();
        }
        for model in models.as_array().unwrap() {
            let model = model.as_str().unwrap();
            if codex["models"].get(model).is_some() {
                bad.push(format!("{agent}: Codex model {model}"));
            }
            let routed = ["claude", "dsh"]
                .iter()
                .filter(|name| adapter(name)["models"].get(model).is_some())
                .count();
            if routed != 1 {
                bad.push(format!("{agent}: {model} names {routed} providers"));
            }
        }
    }

    let claude = adapter("claude");
    if claude["models"]["haiku"] != "claude-haiku-4-5-20251001" {
        bad.push("historical haiku alias".into());
    }
    let dsh = adapter("dsh");
    if dsh["routes"]["deepseek-official"] != "uncontracted"
        || dsh["routes"]["spark-glm"] != "local"
        || dsh["tool_permissions"] != "unsupported"
        || dsh["mcp"] != "unsupported"
        || !dsh["effortless_routes"]["spark-glm"].is_string()
    {
        bad.push("dsh routes or declared capabilities".into());
    }
    if dsh["models"].as_object().unwrap().values().any(|v| {
        let route = v.as_str().unwrap();
        route.contains("deepseek") && route.contains("pro")
    }) {
        bad.push("DeepSeek Pro".into());
    }
    bad
}

#[test]
fn claude_careful_routes_every_model_seat_without_codex() {
    let recipe = read_json("bundles/claude-careful/bundle.json");
    let on_disk = |name: &str| office(name);
    let adapters = |name: &str| read_json(&format!("adapters/{name}.json"));
    assert_eq!(
        claude_careful_violations(&recipe, &on_disk, &adapters),
        Vec::<String>::new()
    );
    // The recipe keeps careful's protected policy: no table, rule or phase
    // override, and the exec gates verify and ship stay inherited.
    for forbidden in ["policy", "protected_phase", "remove"] {
        assert!(recipe.get(forbidden).is_none(), "{forbidden}");
    }
    for gate in ["verify", "ship"] {
        assert!(recipe["seats"].get(gate).is_none(), "{gate}");
    }

    // The same comparison refuses each one-fault drift of the recipe ...
    let mut inherited_design = recipe.clone();
    inherited_design["seats"]
        .as_object_mut()
        .unwrap()
        .remove("design");
    inherited_design["override"]["seats"] = json!(["intake", "implement", "review"]);
    let mut fable_independent = recipe.clone();
    fable_independent["seats"]["review"]["panel"]["independent"]["agent"] =
        json!("codex-review-independent");
    let mut flash_correctness = recipe.clone();
    flash_correctness["seats"]["review"]["panel"]["correctness"]["agent"] =
        json!("codex-review-astra-correctness");
    let mut implement_limit = recipe.clone();
    implement_limit["seats"]["implement"]["limits"]["timeout_seconds"] = json!(7200);
    let mut review_limit = recipe.clone();
    review_limit["seats"]["review"]["limits"]["timeout_seconds"] = json!(3600);
    let mut candidate_check = recipe.clone();
    candidate_check["seats"]["design"]["sequence"][2]["driver"]["command"][5] =
        json!("scripts/design-check.sh");
    let mut codex_position = recipe.clone();
    codex_position["seats"]["design"]["sequence"][0]["panel"]["simplicity"]["agent"] =
        json!("codex-position-simplicity");
    let mut policy = recipe.clone();
    policy["policy"] = json!("policy.json");
    for (mutant, recipe) in [
        ("inherited Codex design", &inherited_design),
        ("Fable independent", &fable_independent),
        ("Flash correctness", &flash_correctness),
        ("implement limit", &implement_limit),
        ("review limit", &review_limit),
        ("candidate design check", &candidate_check),
        ("Codex position", &codex_position),
        ("copied policy", &policy),
    ] {
        assert!(
            !claude_careful_violations(recipe, &on_disk, &adapters).is_empty(),
            "{mutant} accepted"
        );
    }

    // ... of an office ...
    type Drift = fn(&mut Value);
    let drifts: [(&str, &str, Drift); 11] = [
        ("claude-implementer", "Codex model", |o| {
            o["models"] = json!(["sol"]);
            o["efforts"] = json!({"sol": "high"})
        }),
        ("claude-intake", "Haiku 4.5", |o| {
            o["models"] = json!(["haiku"]);
            o["efforts"] = json!({"haiku": "high"})
        }),
        ("claude-chief-architect", "fallback", |o| {
            o["models"] = json!(["fable", "opus"])
        }),
        ("claude-review-independent", "fallback", |o| {
            o["models"] = json!(["opus", "glm-flash"])
        }),
        ("codex-review-tests", "effort on GLM", |o| {
            o["efforts"] = json!({"glm-flash": "high"})
        }),
        (
            "dsh-position-robustness",
            "dsh hands",
            |o| o["hands"] = json!({"kind": "workspace", "network": false, "binds": []}),
        ),
        ("dsh-position-robustness", "GLM robustness", |o| {
            o["models"] = json!(["glm-flash"]);
            o["efforts"] = json!({})
        }),
        ("claude-position-simplicity", "no hands", |o| {
            let o = o.as_object_mut().unwrap();
            o.remove("hands");
            o.remove("replaces");
        }),
        ("claude-implementer", "unmasked credentials", |o| {
            o["hands"]["binds"][0]["mask"] = json!(["credentials.toml"])
        }),
        ("claude-intake", "charter", |o| {
            o["charter"] = json!("charters/review-correctness.md")
        }),
        ("codex-review-security", "effort", |o| {
            o["efforts"] = json!({"flash": "medium"})
        }),
    ];
    for (target, mutant, drift) in drifts {
        let drifted = |name: &str| {
            let mut value = office(name);
            if name == target {
                drift(&mut value);
            }
            value
        };
        assert!(
            !claude_careful_violations(&recipe, &drifted, &adapters).is_empty(),
            "{target}: {mutant} accepted"
        );
    }

    // ... and of an adapter alias.
    let adapter_drifts: [(&str, &str, Drift); 4] = [
        ("claude", "haiku55 pinned to 4.5", |a| {
            a["models"]["haiku55"] = json!("claude-haiku-4-5-20251001")
        }),
        ("claude", "historical haiku moved", |a| {
            a["models"]["haiku"] = json!("claude-haiku-5-5")
        }),
        ("codex", "Codex alias for opus", |a| {
            a["models"]["opus"] = json!("gpt-6-astra")
        }),
        ("dsh", "DeepSeek Pro", |a| {
            a["models"]["flash"] = json!("deepseek-pro")
        }),
    ];
    for (target, mutant, drift) in adapter_drifts {
        let drifted = |name: &str| {
            let mut value = read_json(&format!("adapters/{name}.json"));
            if name == target {
                drift(&mut value);
            }
            value
        };
        assert!(
            !claude_careful_violations(&recipe, &on_disk, &drifted).is_empty(),
            "{target}: {mutant} accepted"
        );
    }
}

/// An overridden seat's `./` command resolves against the layer that wrote
/// it, and the exec box binds only that layer: claude-careful's design check
/// runs its own pinned copy, which must stay byte-identical to self's.
#[test]
fn claude_careful_design_check_is_self_validator_byte_for_byte() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let leaf = root.join("bundles/claude-careful");
    let mut files = Vec::new();
    let mut pending = vec![leaf.clone()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(
                    path.strip_prefix(&leaf)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    files.sort();
    assert_eq!(
        files,
        [
            "bundle.json",
            "scripts/design-check.sh",
            "scripts/speckit/check-prerequisites.sh",
            "scripts/speckit/common.sh",
        ],
        "the leaf carries only its seats and the design validator"
    );
    for script in &files[1..] {
        assert_eq!(
            std::fs::read(leaf.join(script)).unwrap(),
            std::fs::read(
                root.join("bundles/self/scripts")
                    .join(script.trim_start_matches("scripts/"))
            )
            .unwrap(),
            "{script} drifted from self's pinned copy"
        );
    }
    let recipe = read_json("bundles/claude-careful/bundle.json");
    let base = read_json("bundles/self/bundle.json");
    assert_eq!(
        recipe["seats"]["design"]["sequence"][2], base["seats"]["design"]["sequence"][2],
        "the model-free validator step is self's, field for field"
    );
}
