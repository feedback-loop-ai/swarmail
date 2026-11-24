//! Hold 3's open proof (the panel's third review, part of #404): a release
//! build carries no stand-in admission. The debug suites can only show the
//! seam's presence; its absence is a property of the release profile, so
//! this test runs there alone, by the dispatcher, outside CI:
//!
//! ```bash
//! cd evals && cargo test --release --locked --test wrappers -- --ignored release_standin_is_not_admitted
//! ```
//!
//! In that build the real adjacent stand-in, with the retired override
//! variable naming its digest, is refused by the production verifier
//! before anything launches; the published digest is still admitted; and
//! the debug-only `grant-standin` subcommand does not exist.
use super::*;

#[test]
#[ignore = "host proof: run in the release profile (cargo test --release), outside CI"]
fn release_standin_is_not_admitted() {
    // Read at run time, so the profile that built this test decides.
    let debug = std::hint::black_box(cfg!(debug_assertions));
    assert!(
        !debug,
        "this proof means nothing in a debug build: run it with --release"
    );
    assert!(!std::hint::black_box(wrappers::SEAM_BUILD));
    let n = nested::Nested::new("claude", "claude-sonnet-5-5");
    // The driver here is the stand-in image built beside the release
    // binary; its digest is admitted by nothing but a debug build.
    let standin = swarmail_evals::identity::file_hash(&n.driver).unwrap();
    assert!(!wrappers::driver_admitted(&standin));
    assert!(wrappers::driver_admitted(wrappers::DRIVER_RELEASE_SHA256));
    // The production verifier, through the real wrapper, with the retired
    // variable set: refused before the grant, any read or any launch.
    let mut cmd = n.c.command("claude", "claude-sonnet-5-5");
    cmd.arg("--via-brokkr")
        .arg(&n.driver)
        .arg("--paid-launch")
        .arg(n.binding())
        .env("EVAL_TEST_STANDIN_DRIVER_SHA256", &standin);
    let out = cmd.output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{err}");
    assert!(err.contains("driver_release_unrecognized"), "{err}");
    assert!(std::fs::read_dir(&n.c.home).unwrap().next().is_none());
    // No grant stand-in in a release build: clap knows no such command.
    let out = std::process::Command::new(common::exe())
        .args([
            "grant-standin",
            "--endpoints",
            "1",
            "--grant",
            "x:1",
            "--",
            "/bin/true",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("arguments_invalid"));
}
