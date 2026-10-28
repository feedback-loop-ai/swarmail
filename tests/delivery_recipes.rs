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
    for name in ["codex", "codex-careful"] {
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
        ("guarantees", "guarantees", vec!["astra"], Some("astra")),
        (
            "astra-correctness",
            "correctness",
            vec!["astra"],
            Some("astra"),
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

#[test]
fn operator_mixed_recipe_resolves_named_work_models_and_preserves_panel() {
    let recipe = read_json("bundles/mixed-opus-terra/bundle.json");
    assert_eq!(recipe["extends"], "codex-careful");
    assert!(recipe["seats"].get("review").is_none());
    assert_eq!(recipe["seats"]["intake"]["agent"], "mixed-terra-intake");
    assert_eq!(
        recipe["seats"]["implement"]["agent"],
        "mixed-opus-implementer"
    );
    let intake = office("mixed-terra-intake");
    let implement = office("mixed-opus-implementer");
    assert_eq!(intake["models"], json!(["terra61"]));
    assert_eq!(implement["models"], json!(["opus"]));
    assert_eq!(
        read_json("adapters/codex.json")["models"]["terra61"],
        "gpt-6.1-terra"
    );
    assert_eq!(
        read_json("adapters/claude.json")["models"]["opus"],
        "claude-opus-5-5"
    );
    let review = read_json("bundles/codex-careful/bundle.json");
    for role in [
        "correctness",
        "guarantees",
        "independent",
        "security",
        "tests",
    ] {
        assert_eq!(review["seats"]["review"]["panel"][role]["class"], "gate");
    }
    for (role, agent) in [
        ("correctness", "codex-review-astra-correctness"),
        ("guarantees", "codex-review-guarantees"),
        ("independent", "codex-review-independent"),
        ("security", "codex-review-security"),
        ("tests", "codex-review-tests"),
    ] {
        assert_eq!(review["seats"]["review"]["panel"][role]["agent"], agent);
    }
    assert_eq!(
        office("codex-review-independent")["models"],
        json!(["fable"])
    );
    assert_eq!(office("codex-review-security")["models"], json!(["flash"]));
    assert_eq!(office("codex-review-tests")["models"], json!(["glm-flash"]));
    assert_eq!(
        office("codex-review-astra-correctness")["models"],
        json!(["astra"])
    );
    assert_eq!(
        office("codex-review-guarantees")["models"],
        json!(["astra"])
    );
    assert_eq!(
        read_json("adapters/dsh.json")["models"]["glm-flash"],
        "spark-glm/GLM-5.3-Flash-EXL3"
    );
}
