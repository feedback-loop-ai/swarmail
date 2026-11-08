//! The comparison's closed target set, the adapter's announcement line and
//! the competitor prompt (proposed decision 0129 §1–§2, spec R2 and R6).

use swarmail_evals::provenance::sha256;
use swarmail_evals::target::{self, Target};
use swarmail_evals::tasks::{self, COMPETITOR_PROMPT_TEMPLATE, PROMPT_TEMPLATE, PromptVars, Tier};

#[test]
fn the_target_set_is_closed_and_an_unknown_value_is_refused_by_name() {
    let names: Vec<_> = target::ALL.iter().map(|t| t.name()).collect();
    assert_eq!(names, ["swarmail", "mailpit", "mailhog", "mailslurper"]);
    for t in target::ALL {
        assert_eq!(Target::parse(t.name()), Ok(t));
        assert_eq!(t.comparison_name(), format!("{}-mcp", t.name()));
        assert!(t.tool_prefix().starts_with(t.name()));
    }
    let refused = Target::parse("bogus").unwrap_err();
    assert!(
        refused.contains("\"bogus\"") && refused.contains("mailhog"),
        "{refused}"
    );
    // A comparison name is not a --target value.
    assert!(Target::parse("mailhog-mcp").is_err());
    assert!(Target::parse("").is_err());
    assert!(!Target::Swarmail.is_competitor());
    assert!(Target::Mailpit.adapter_built() && Target::Mailhog.adapter_built());
    // MailSlurper's adapter is built from the source bytes the lock binds.
    assert!(Target::Mailslurper.is_competitor() && Target::Mailslurper.adapter_built());
    assert_eq!(Target::Mailslurper.adapter_mode(), Some("mailslurper-mcp"));
    for t in [Target::Mailpit, Target::Mailhog, Target::Mailslurper] {
        assert_eq!(t.adapter_mode(), Some(format!("{}-mcp", t.name()).as_str()));
    }
    assert_eq!(Target::Swarmail.adapter_mode(), None);
}

#[test]
fn the_announcement_is_one_loopback_address_and_one_digest() {
    let digest = "ab".repeat(32);
    let ok = format!("smtp=127.0.0.1:2525 tools_sha256={digest}\n");
    let a = target::parse_announcement(&ok).unwrap();
    assert_eq!(a.smtp.to_string(), "127.0.0.1:2525");
    assert_eq!(a.tools_sha256, digest);
    assert!(target::parse_announcement(&format!("smtp=[::1]:25 tools_sha256={digest}")).is_some());
    for bad in [
        format!("smtp=10.0.0.1:25 tools_sha256={digest}"),
        format!("smtp=127.0.0.1:0 tools_sha256={digest}"),
        format!("smtp=127.0.0.1:25 tools_sha256={}", "AB".repeat(32)),
        format!("smtp=127.0.0.1:25 tools_sha256={}", "a".repeat(63)),
        format!("smtp=127.0.0.1:25 tools_sha256={digest} extra=1"),
        format!("tools_sha256={digest} smtp=127.0.0.1:25"),
        format!("smtp=localhost:25 tools_sha256={digest}"),
        "smtp=127.0.0.1:25".to_string(),
        String::new(),
        "{\"jsonrpc\":\"2.0\"}".to_string(),
    ] {
        assert_eq!(target::parse_announcement(&bad), None, "{bad}");
    }
}

#[test]
fn tool_names_and_server_keys_are_shapes_a_wrapper_can_pass_on_a_command_line() {
    for ok in ["mailpit_list", "a", "mailhog_wait_2"] {
        assert!(target::valid_tool_name(ok), "{ok}");
    }
    for bad in [
        "",
        "Mail",
        "1a",
        "a-b",
        "a b",
        "a;b",
        "a/b",
        &"a".repeat(65),
    ] {
        assert!(!target::valid_tool_name(bad), "{bad}");
    }
    for ok in ["swarmail", "mail-target"] {
        assert!(target::valid_server_key(ok), "{ok}");
    }
    for bad in ["", "Mail", "a_b", "a b", "a__b", &"a".repeat(33)] {
        assert!(!target::valid_server_key(bad), "{bad}");
    }
}

#[test]
fn the_swarmail_prompt_template_is_the_one_main_froze() {
    // The digest of `PROMPT_TEMPLATE` at main c95386e.
    assert_eq!(
        sha256(PROMPT_TEMPLATE.as_bytes()),
        "5998e59e89bba23cca7207bc2cab8ecd5dcb00d009f2f2ec15e6ab0e6444b70a"
    );
}

#[test]
fn a_competitor_prompt_carries_the_goal_byte_for_byte_and_no_inbox_instruction() {
    let hard: Vec<_> = tasks::catalogue()
        .into_iter()
        .filter(|t| t.tier == Tier::Hard)
        .collect();
    assert_eq!(hard.len(), 3);
    for task in &hard {
        let vars = PromptVars {
            app_url: "http://127.0.0.1:1",
            mcp_url: "http://127.0.0.1:2/mcp",
            identity: "alice+t1@fixture.eval",
            inbox: "inbox-name-never-shown",
        };
        let swarmail = tasks::render_prompt(task, &vars).unwrap();
        let goal_line = format!("Task: {}", {
            let at = swarmail.find("Task: ").unwrap();
            swarmail[at + 6..].trim_end().to_string()
        });
        for t in [Target::Mailpit, Target::Mailhog, Target::Mailslurper] {
            let prompt = tasks::render_competitor_prompt(task, &vars, t.display()).unwrap();
            assert!(prompt.contains(&goal_line), "{}: the goal differs", task.id);
            assert!(prompt.contains(t.display()), "the header names the target");
            assert!(!prompt.contains("Swarmail"), "{prompt}");
            assert!(!prompt.contains("inbox-name-never-shown"));
            assert!(
                !prompt.contains("\"inbox\""),
                "no inbox argument instruction"
            );
            assert!(!prompt.contains("Your inbox:"), "no inbox line");
            assert!(prompt.contains("one shared mailbox"));
            assert!(
                !prompt.contains('{'),
                "every placeholder was filled: {prompt}"
            );
        }
        assert!(swarmail.contains("\"inbox\": \"inbox-name-never-shown\""));
    }
}

#[test]
fn the_competitor_header_is_everything_above_the_goal_and_has_a_stable_digest() {
    let header = tasks::competitor_header("MailHog");
    assert!(header.starts_with("You are an agent completing one task"));
    assert!(header.contains("MailHog MCP endpoint"));
    assert!(!header.contains("Task:") && !header.contains("{goal}"));
    assert_ne!(header, tasks::competitor_header("Mailpit"));
    assert!(COMPETITOR_PROMPT_TEMPLATE.contains("{display}"));
    // Frozen text: any edit to the header is a method change, which needs a
    // new product record (0129 §8). These are the digests that record freezes.
    for (display, want) in [
        (
            "Mailpit",
            "db409f4cae3bba3e2364fe0ee5d6ad4098305d08fee76e4ad58c12853fa16c4f",
        ),
        (
            "MailHog",
            "dbf3120882c0360c1336744389f0f2ebed1e2bc66a64d548c503cb49167f3399",
        ),
        (
            "MailSlurper",
            "34a5ad44de571677f66a8075c5690e0f08a24b02f0dab4b08bc6192d07ede521",
        ),
    ] {
        assert_eq!(
            sha256(tasks::competitor_header(display).as_bytes()),
            want,
            "{display}"
        );
    }
}
