//! Route (b) through the real brokkr v0.15.2 driver (local proof). The
//! release is not in the tree, so these run only with `EVAL_BROKKR_DRIVER`
//! naming the extracted executable; they are ignored otherwise, never
//! silently passed. Inert stand-in CLIs play each harness: no credential,
//! no network beyond loopback, no paid session.
use super::nested::Nested;
use super::*;
use swarmail_evals::runner::{LaunchReport, launch_report_of};
use swarmail_evals::usage::{Fact, Reason};

fn release() -> PathBuf {
    let path = PathBuf::from(std::env::var_os("EVAL_BROKKR_DRIVER").expect("EVAL_BROKKR_DRIVER"));
    // The pin: the executable extracted from the archive whose SHA256SUMS
    // line wrappers::DRIVER_ARCHIVE_SHA256 records.
    let sha = swarmail_evals::identity::file_hash(&path).unwrap();
    assert_eq!(
        sha,
        wrappers::DRIVER_RELEASE_SHA256,
        "not the v0.15.2 release"
    );
    path
}

/// Ignored on every platform unless asked for: the release asset is Linux
/// x86_64 only, and no hard `cfg` leaves its helpers dead elsewhere.
#[test]
#[ignore = "needs the staged brokkr v0.15.2 release in EVAL_BROKKR_DRIVER (Linux x86_64)"]
fn the_release_driver_launches_each_inert_cli_on_the_host_route() {
    let driver = release();
    for (family, model) in super::nested::CASES {
        let n = Nested::new(family, model);
        let record = n.c.root.path().join("cli-record.json");
        let body = format!(
            "#!{}\nFAMILY={}\nRECORD={}\n{}",
            standin_python(),
            json!(family),
            json!(record),
            include_str!("inert_cli.py")
        );
        std::fs::remove_file(&n.cli).unwrap();
        let hop = PathBuf::from(format!("{}.hop", n.cli.display()));
        std::fs::remove_file(&hop).unwrap();
        common::paid::install_native(&n.cli, body.as_bytes());
        // The release itself, copied under the case so its pin's ancestors
        // are the case's own.
        std::fs::remove_file(&n.driver).unwrap();
        std::fs::copy(&driver, &n.driver).unwrap();
        std::fs::set_permissions(&n.driver, std::fs::Permissions::from_mode(0o700)).unwrap();
        n.reselect();
        let (code, err, report) = n.launch(json!({"mode": "real"}));
        assert_eq!(code, Some(0), "{family}: {err}");
        let LaunchReport::Nested(receipt) = launch_report_of(&report) else {
            panic!("{family}: {}", String::from_utf8_lossy(&report));
        };
        assert_eq!(receipt.disposition, "completed", "{family}");
        let f = receipt.facts;
        match family {
            // Input is inclusive of the cache read (10 + 3).
            "claude" => {
                assert_eq!(f.input_tokens, Fact::Present { value: 13 });
                assert_eq!(f.output_tokens, Fact::Present { value: 5 });
                assert_eq!(f.cache_write_tokens, Fact::Present { value: 2 });
                assert_eq!(f.num_turns, Fact::Present { value: 1 });
                assert_eq!(f.total_cost_usd, Fact::Present { value: 0.0125 });
            }
            "codex" => {
                assert_eq!(f.input_tokens, Fact::Present { value: 20 });
                assert_eq!(f.output_tokens, Fact::Present { value: 6 });
                assert_eq!(f.cache_read_tokens, Fact::Present { value: 8 });
            }
            // The inert dsh writes no session file: its final step is
            // recognised and carries no count.
            _ => assert_eq!(
                f.input_tokens,
                Fact::Unavailable {
                    reason: Reason::FieldAbsent
                }
            ),
        }
        let rec: Value = serde_json::from_slice(&std::fs::read(&record).unwrap()).unwrap();
        let env = &rec["env"];
        // Resolved through the controlled PATH and the driver's override,
        // the pinned program ran: dsh's runtime was given the pinned entry
        // first, with no script of ours between them.
        let cli_dir = n.cli.parent().unwrap().display().to_string();
        assert!(
            env["PATH"].as_str().unwrap().starts_with(&cli_dir),
            "{family}"
        );
        if family == "dsh" {
            assert_eq!(rec["argv"][0], json!(n.dsh_pin().entry_path()));
            assert_eq!(env["SPARK_API_KEY"], "<present>");
            assert!(env.get("DASHSCOPE_API_KEY").is_none());
        }
        assert_eq!(env["HOME"], json!(n.c.home), "{family}");
        assert_eq!(rec["fd3"], Value::Null, "{family}: the launch pipe leaked");
        assert!(!env.to_string().contains(CANARY));
        // The result was written inside the work directory (codex under
        // its selected sandbox class), and the DNS inputs were readable.
        // Whether a write is refused is not read from `os.access` on the
        // host's files: the recipe route's read-only binds are proven by a
        // real mount (`evals/tests/wrappers_launch/mount_proof.rs`).
        let result = rec["result_path"].as_str().unwrap();
        assert!(result.starts_with(n.c.work.to_str().unwrap()), "{family}");
        for (path, access) in rec["dns"].as_object().unwrap() {
            assert_eq!(access[0], json!(true), "{family}: {path}");
        }
    }
}
