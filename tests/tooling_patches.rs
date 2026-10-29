//! Execute the tooling patch helper against real Git checkouts and patches.

use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

const ONE_TWO: &[u8] = b"diff --git a/value.txt b/value.txt\n--- a/value.txt\n+++ b/value.txt\n@@ -1 +1 @@\n-one\n+two\n";
const TWO_THREE: &[u8] = b"diff --git a/value.txt b/value.txt\n--- a/value.txt\n+++ b/value.txt\n@@ -1 +1 @@\n-two\n+three\n";

struct Fixture {
    _temporary: TempDir,
    helper: PathBuf,
    bundle: PathBuf,
    source: PathBuf,
    sha: String,
}

fn command_output(command: &mut Command) -> Output {
    command.output().expect("the fixture command starts")
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn digest(bytes: &[u8]) -> String {
    let mut child = Command::new("python3")
        .args([
            "-c",
            "import hashlib,sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    success(child.wait_with_output().unwrap())
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let helper = temporary.path().join("scripts/apply-brokkr-patches.py");
        let bundle = temporary.path().join(".brokkr/recipes/self");
        let source = temporary.path().join("engine");
        std::fs::create_dir_all(helper.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            &helper,
            include_bytes!("../scripts/apply-brokkr-patches.py"),
        )
        .unwrap();
        let mut fixture = Self {
            _temporary: temporary,
            helper,
            bundle,
            source,
            sha: String::new(),
        };
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "user.name", "tooling fixture"]);
        fixture.git(&["config", "user.email", "tooling@example.invalid"]);
        fixture.git(&["config", "commit.gpgsign", "false"]);
        fixture.write_value("one\n");
        fixture.git(&["add", "value.txt"]);
        fixture.git(&["commit", "-qm", "Create the patch fixture"]);
        fixture.sha = fixture.git(&["rev-parse", "HEAD"]);
        fixture.record(json!([]));
        fixture
    }

    fn git(&self, arguments: &[&str]) -> String {
        success(command_output(
            Command::new("git")
                .arg("-C")
                .arg(&self.source)
                .args(["-c", "core.hooksPath=/dev/null"])
                .args(arguments),
        ))
    }

    fn write_value(&self, value: &str) {
        std::fs::write(self.source.join("value.txt"), value).unwrap();
    }

    fn value(&self) -> String {
        std::fs::read_to_string(self.source.join("value.txt")).unwrap()
    }

    fn record(&self, patches: Value) {
        self.raw_record(&json!({"brokkr": {"sha": self.sha, "patches": patches}}).to_string());
    }

    fn raw_record(&self, record: &str) {
        std::fs::write(self.bundle.join("tooling.json"), record).unwrap();
    }

    fn patch(&self, name: &str, bytes: &[u8]) -> Value {
        let relative = format!("tooling-patches/{name}");
        let path = self.bundle.join(&relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
        json!({"path": relative, "sha256": digest(bytes)})
    }

    fn run(&self) -> Output {
        command_output(Command::new("python3").arg(&self.helper).arg(&self.source))
    }

    fn refuses(&self, reason: &str) {
        let output = self.run();
        assert!(!output.status.success(), "the helper must refuse");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(reason), "expected {reason:?}: {error}");
    }
}

#[test]
fn applies_verified_patches_in_recorded_order_and_reports_their_hashes() {
    let fixture = Fixture::new();
    let patches = json!([
        fixture.patch("first.patch", ONE_TWO),
        fixture.patch("second.patch", TWO_THREE)
    ]);
    fixture.record(patches.clone());
    std::fs::write(fixture.source.join("untracked-build-output"), "allowed").unwrap();
    let receipt: Value = serde_json::from_str(&success(fixture.run())).unwrap();
    assert_eq!(fixture.value(), "three\n");
    assert_eq!(receipt["base_sha"], fixture.sha);
    assert_eq!(receipt["applied_patches"], patches);
    assert_eq!(fixture.git(&["rev-parse", "HEAD"]), fixture.sha);
    assert!(fixture.git(&["diff", "--", "value.txt"]).contains("+three"));
}

#[test]
fn an_absent_patch_list_is_an_explicit_unpatched_receipt() {
    let fixture = Fixture::new();
    fixture.raw_record(&json!({"brokkr": {"sha": fixture.sha}}).to_string());
    let receipt: Value = serde_json::from_str(&success(fixture.run())).unwrap();
    assert_eq!(receipt["applied_patches"], json!([]));
    assert_eq!(fixture.value(), "one\n");
    assert!(fixture.git(&["status", "--porcelain"]).is_empty());
}

#[test]
fn every_digest_is_checked_before_the_first_mutation() {
    let fixture = Fixture::new();
    let first = fixture.patch("first.patch", ONE_TWO);
    let mut second = fixture.patch("second.patch", TWO_THREE);
    second["sha256"] = json!("0".repeat(64));
    fixture.record(json!([first, second]));
    fixture.refuses("sha256 does not match");
    assert_eq!(fixture.value(), "one\n");
    assert!(fixture.git(&["status", "--porcelain"]).is_empty());
}

#[test]
fn wrong_source_and_dirty_worktree_or_index_are_refused() {
    let fixture = Fixture::new();
    fixture.raw_record(&json!({"brokkr": {"sha": "0".repeat(40)}}).to_string());
    fixture.refuses("does not match recorded");
    fixture.record(json!([fixture.patch("change.patch", ONE_TWO)]));
    fixture.write_value("dirty\n");
    fixture.refuses("tracked tree or index is dirty");
    fixture.git(&["add", "value.txt"]);
    fixture.refuses("tracked tree or index is dirty");
    assert_eq!(fixture.value(), "dirty\n");
}

#[test]
fn malformed_records_paths_and_missing_patches_fail_closed() {
    let fixture = Fixture::new();
    for record in ["null", "{}", "{\"brokkr\":[]}"] {
        fixture.raw_record(record);
        fixture.refuses("brokkr object");
    }
    fixture.raw_record("{\"brokkr\":{\"sha\":\"BAD\"}}");
    fixture.refuses("40-character commit SHA");
    for entries in [Value::Null, json!({}), json!(false)] {
        fixture.record(entries);
        fixture.refuses("patches must be an array");
    }
    for entry in [
        json!(12),
        json!({"path":"x"}),
        json!({"path":"x","sha256":"x","extra":true}),
    ] {
        fixture.record(json!([entry]));
        fixture.refuses("exactly path and sha256");
    }
    for path in [
        "/tmp/change.patch",
        "../change.patch",
        "tooling-patches/../change.patch",
        "tooling-patches\\change.patch",
        "tooling-patches//change.patch",
        "tooling-patches",
    ] {
        fixture.record(json!([{"path":path,"sha256":"0".repeat(64)}]));
        fixture.refuses("patch path must stay beneath");
    }
    for sha in [
        json!(null),
        json!(42),
        json!("ABCDEF".repeat(11)),
        json!("A".repeat(64)),
    ] {
        fixture.record(json!([{"path":"tooling-patches/change.patch","sha256":sha}]));
        fixture.refuses("lowercase sha256");
    }
    fixture.record(json!([{"path":"tooling-patches/missing.patch","sha256":"0".repeat(64)}]));
    fixture.refuses("No such file or directory");
    assert_eq!(fixture.value(), "one\n");
}

#[test]
fn duplicate_json_keys_and_duplicate_patch_paths_are_refused() {
    let fixture = Fixture::new();
    fixture.raw_record(&format!(
        "{{\"brokkr\":{{\"sha\":\"{}\",\"patches\":[],\"patches\":[]}}}}",
        fixture.sha
    ));
    fixture.refuses("duplicate JSON key");
    fixture.raw_record(&format!(
        "{{\"brokkr\":{{\"sha\":\"{}\",\"patches\":[],\"invalid\":NaN}}}}",
        fixture.sha
    ));
    fixture.refuses("non-JSON constant");
    let patch = fixture.patch("change.patch", ONE_TWO);
    fixture.record(json!([patch.clone(), patch]));
    fixture.refuses("duplicate patch path");
    assert_eq!(fixture.value(), "one\n");
}

#[test]
fn actual_git_check_rejects_wrong_order_and_malformed_patch_bytes() {
    let fixture = Fixture::new();
    fixture.record(json!([
        fixture.patch("second.patch", TWO_THREE),
        fixture.patch("first.patch", ONE_TWO)
    ]));
    fixture.refuses("git apply --check - failed");
    assert_eq!(fixture.value(), "one\n");
    fixture.record(json!([fixture.patch("bad.patch", b"not a Git patch\n")]));
    fixture.refuses("git apply --check - failed");
    assert_eq!(fixture.value(), "one\n");
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_the_pinned_patch_directory() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let outside = fixture.bundle.parent().unwrap().join("outside.patch");
    std::fs::write(&outside, ONE_TWO).unwrap();
    let area = fixture.bundle.join("tooling-patches");
    std::fs::create_dir_all(&area).unwrap();
    symlink(&outside, area.join("escape.patch")).unwrap();
    fixture.record(json!([{"path":"tooling-patches/escape.patch","sha256":digest(ONE_TWO)}]));
    fixture.refuses("escapes tooling-patches");
    assert_eq!(fixture.value(), "one\n");
    std::fs::remove_file(area.join("escape.patch")).unwrap();
    std::fs::remove_dir(&area).unwrap();
    symlink(outside.parent().unwrap(), &area).unwrap();
    fixture.refuses("patch directory must not escape");
}

#[test]
fn a_source_subdirectory_cannot_silently_skip_the_patch() {
    let fixture = Fixture::new();
    fixture.record(json!([fixture.patch("change.patch", ONE_TWO)]));
    let nested = fixture.source.join("nested");
    std::fs::create_dir(&nested).unwrap();
    let output = command_output(Command::new("python3").arg(&fixture.helper).arg(&nested));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("checkout root"));
    assert_eq!(fixture.value(), "one\n");
}
