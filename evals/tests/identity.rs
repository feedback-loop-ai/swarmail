//! Disposable real builds prove that source identity is captured at build time.
mod common;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use swarmail_evals::{
    identity,
    provenance::{Availability, Reason, Source},
    runner::{self, TrialResult},
};
fn git(root: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.name=Eval proof",
            "-c",
            "user.email=eval@example.test",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap().trim().to_string()
}
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let e = entry.unwrap();
        let name = e.file_name();
        if matches!(name.to_str(), Some("target" | ".git")) {
            continue;
        }
        if e.path().is_dir() {
            copy_tree(&e.path(), &to.join(name));
        } else {
            std::fs::copy(e.path(), to.join(name)).unwrap();
        }
    }
}
fn build(root: &Path, target: &Path) -> PathBuf {
    let out = Command::new("cargo")
        .current_dir(root.join("evals"))
        .args([
            "build",
            "--bin",
            "swarmail-evals",
            "--offline",
            "--locked",
            "--target-dir",
        ])
        .arg(target)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    target.join("debug/swarmail-evals")
}
fn facts(exe: &Path, cwd: &Path) -> TrialResult {
    let out = Command::new(exe)
        .current_dir(cwd)
        .env("SWARMAIL_BIN", "/unavailable-in-build-proof")
        .env_remove("EVAL_MATRIX_CONTEXT")
        .args(["trial", "--task", "signup", "--agent", "null"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let row: TrialResult = serde_json::from_slice(&out.stdout).unwrap();
    row.validate().unwrap();
    row
}
#[test]
fn runtime_checkout_cannot_supply_harness_source_or_external_target_source() {
    let _trial_scope = common::trial_scope();
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-b", "unrelated"]);
    git(
        dir.path(),
        &["commit", "--allow-empty", "-m", "Unrelated checkout"],
    );
    let a = facts(
        Path::new(common::exe()),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    );
    let b = facts(Path::new(common::exe()), dir.path());
    assert_eq!(a.provenance.harness, b.provenance.harness);
    assert_eq!(
        a.provenance.harness.executable_sha256,
        Availability::present(
            identity::file_hash(Path::new(common::exe())).unwrap(),
            Source::MeasuredBytes
        )
    );
    let out = common::cli()
        .env("SWARMAIL_BIN", "/bin/false")
        .args(["trial", "--task", "signup", "--agent", "null"])
        .output()
        .unwrap();
    let r = common::one_row(&out);
    assert_eq!(
        r.provenance.target.build_commit,
        Availability::unavailable(Reason::ExternalSourceUnknown)
    );
    assert_eq!(
        r.provenance.target.source_dirty,
        Availability::unavailable(Reason::ExternalSourceUnknown)
    );
    assert_eq!(
        r.provenance.target.executable_sha256,
        Availability::present(
            identity::file_hash(Path::new("/bin/false")).unwrap(),
            Source::MeasuredBytes
        )
    );
}
#[test]
fn source_content_fingerprint_detects_dirty_to_dirty_changes_and_executable_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"), "manifest").unwrap();
    std::fs::write(root.join("Cargo.lock"), "lock").unwrap();
    std::fs::write(root.join("src/main.rs"), "first").unwrap();
    git(root, &["init", "-b", "proof"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "Initial source"]);
    let clean = identity::SourceContext::capture(root).unwrap();
    std::fs::write(root.join("src/main.rs"), "dirty-one").unwrap();
    let dirty = identity::SourceContext::capture(root).unwrap();
    std::fs::write(root.join("src/main.rs"), "dirty-two").unwrap();
    let changed = identity::SourceContext::capture(root).unwrap();
    assert!(clean != dirty && dirty != changed);
    let exe = root.join("program");
    std::fs::write(&exe, b"original").unwrap();
    let measured = identity::TargetBuild::external(exe.clone());
    measured.recheck().unwrap();
    std::fs::write(exe, b"replacement").unwrap();
    assert!(measured.recheck().is_err());
}
#[test]
fn an_owned_real_cargo_build_that_changes_source_refuses_association() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='swarmail'\nversion='0.1.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(root.join("build.rs"),"fn main(){std::fs::write(\"src/main.rs\",\"fn main() {println!(\\\"changed\\\");}\\n\").unwrap();}\n").unwrap();
    let out = Command::new("cargo")
        .current_dir(root)
        .args(["generate-lockfile", "--offline"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let result = runner::resolve_owned_target(root);
    assert!(result.err().unwrap().contains("build_context_changed"));
}
#[test]
fn rebuilds_follow_source_git_indirection_packed_refs_and_archive_unavailability() {
    let _trial_scope = common::trial_scope();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    let actual = Path::new(env!("CARGO_MANIFEST_DIR"));
    copy_tree(actual, &root.join("evals"));
    std::fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
    std::fs::write(root.join("Cargo.lock"), "").unwrap();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/source.rs"), "// watched source\n").unwrap();
    git(&root, &["init", "-b", "proof"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "Capture clean build"]);
    let first = git(&root, &["rev-parse", "HEAD"]);
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/identity-proof");
    let exe = build(&root, &target);
    let clean = facts(&exe, dir.path());
    assert_eq!(
        clean.provenance.harness.build_commit,
        Availability::present(first, Source::BuildGit)
    );
    assert_eq!(
        clean.provenance.harness.source_dirty,
        Availability::present(false, Source::BuildGit)
    );
    std::fs::write(root.join("src/source.rs"), "// changed while dirty\n").unwrap();
    let exe = build(&root, &target);
    let dirty = facts(&exe, dir.path());
    assert_eq!(
        dirty.provenance.harness.source_dirty,
        Availability::present(true, Source::BuildGit)
    );
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "Change source"]);
    let second = git(&root, &["rev-parse", "HEAD"]);
    let exe = build(&root, &target);
    assert_eq!(
        facts(&exe, dir.path()).provenance.harness.build_commit,
        Availability::present(second.clone(), Source::BuildGit)
    );
    git(
        &root,
        &["commit", "--allow-empty", "-m", "Advance revision"],
    );
    git(&root, &["pack-refs", "--all"]);
    let exe = build(&root, &target);
    let packed = facts(&exe, dir.path());
    assert_ne!(
        packed.provenance.harness.build_commit,
        Availability::present(second.clone(), Source::BuildGit)
    );
    git(&root, &["update-ref", "refs/heads/proof", &second]);
    let exe = build(&root, &target);
    assert_eq!(
        facts(&exe, dir.path()).provenance.harness.build_commit,
        Availability::present(second.clone(), Source::BuildGit)
    );
    let linked = dir.path().join("linked");
    git(
        &root,
        &[
            "worktree",
            "add",
            "--detach",
            linked.to_str().unwrap(),
            "HEAD",
        ],
    );
    let exe = build(&linked, &target);
    assert_eq!(
        facts(&exe, dir.path()).provenance.harness.build_commit,
        Availability::present(second, Source::BuildGit)
    );
    let archive = dir.path().join("archive");
    copy_tree(&root, &archive);
    let exe = build(&archive, &target);
    let archived = facts(&exe, dir.path());
    assert_eq!(
        archived.provenance.harness.build_commit,
        Availability::unavailable(Reason::NoGit)
    );
    assert_eq!(
        archived.provenance.harness.source_dirty,
        Availability::unavailable(Reason::NoGit)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn runtime_git_helpers_have_closed_maps_and_private_source_locators_never_publish() {
    let _trial_scope = common::trial_scope();
    use std::{collections::BTreeMap, os::unix::fs::PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    let discovery = dir.path().join("private-discovery-canary-200");
    let captures = dir.path().join("captures");
    let caller = dir.path().join("private-cwd-canary-200");
    std::fs::create_dir(&discovery).unwrap();
    std::fs::create_dir(&captures).unwrap();
    std::fs::create_dir(&caller).unwrap();
    let wrapper = discovery.join("git");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\n/bin/cat /proc/$$/environ > '{}/map.'$$\nexec /usr/bin/git \"$@\"\n",
            captures.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = std::env::join_paths(
        std::iter::once(discovery.clone())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let evidence = dir.path().join("evidence");
    let out = common::cli()
        .env_remove("SWARMAIL_BIN")
        .env("PATH", path)
        .env("HOME", dir.path().join("private-home-canary-200"))
        .env("PWD", "private-pwd-canary-200")
        .env("OPENAI_API_KEY", "private-key-canary-200")
        .env("CARGO_HOME", std::env::var_os("CARGO_HOME").unwrap())
        .env("RUSTUP_HOME", std::env::var_os("RUSTUP_HOME").unwrap())
        .current_dir(&caller)
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "null",
            "--artifacts-dir",
        ])
        .arg(&evidence)
        .output()
        .unwrap();
    let row = common::one_row(&out);
    assert_eq!(
        row.outcome,
        swarmail_evals::tasks::Outcome::Fail,
        "{}",
        common::stderr(&out)
    );
    assert!(matches!(
        row.provenance.target.build_commit,
        Availability::Present {
            source: Source::OwnedBuildContext,
            ..
        }
    ));
    let maps: Vec<_> = std::fs::read_dir(&captures)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        maps.len(),
        4,
        "before/after revision and dirty observations"
    );
    for entry in maps {
        let bytes = std::fs::read(entry.path()).unwrap();
        let map: BTreeMap<_, _> = bytes
            .split(|b| *b == 0)
            .filter(|e| !e.is_empty())
            .map(|entry| {
                let at = entry.iter().position(|b| *b == b'=').unwrap();
                (
                    String::from_utf8(entry[..at].to_vec()).unwrap(),
                    String::from_utf8(entry[at + 1..].to_vec()).unwrap(),
                )
            })
            .collect();
        assert_eq!(
            map.keys().map(String::as_str).collect::<Vec<_>>(),
            vec![
                "GIT_CONFIG_GLOBAL",
                "GIT_CONFIG_NOSYSTEM",
                "HOME",
                "LANG",
                "LC_ALL",
                "PATH",
                "TMPDIR",
                "XDG_CACHE_HOME",
                "XDG_CONFIG_HOME",
                "XDG_DATA_HOME",
                "XDG_STATE_HOME"
            ]
        );
        assert_eq!(map["PATH"], "/usr/bin:/bin");
        assert_eq!(map["GIT_CONFIG_GLOBAL"], "/dev/null");
        assert_eq!(map["GIT_CONFIG_NOSYSTEM"], "1");
        for key in [
            "HOME",
            "TMPDIR",
            "XDG_CACHE_HOME",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_STATE_HOME",
        ] {
            assert!(map[key].starts_with("/tmp/swarmail-eval-git-"));
            assert!(
                !Path::new(&map[key]).exists(),
                "owned helper directories are removed after cleanup"
            );
        }
    }
    use std::io::Write;
    let mut report = common::cli()
        .args(["report", "--input", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    report.stdin.take().unwrap().write_all(&out.stdout).unwrap();
    let report = report.wait_with_output().unwrap();
    assert!(report.status.success());
    let replay = common::cli()
        .env("SWARMAIL_BIN", "/unavailable-during-replay")
        .args(["check", "--artifacts-dir"])
        .arg(evidence.join(&row.trial_id))
        .output()
        .unwrap();
    assert_eq!(replay.status.code(), Some(1));
    let mut public = out.stdout.clone();
    public.extend_from_slice(&out.stderr);
    public.extend_from_slice(&report.stdout);
    public.extend_from_slice(&report.stderr);
    public.extend_from_slice(&replay.stdout);
    public.extend_from_slice(&replay.stderr);
    for entry in std::fs::read_dir(evidence.join(&row.trial_id)).unwrap() {
        public.extend(std::fs::read(entry.unwrap().path()).unwrap());
    }
    for canary in [
        "private-discovery-canary-200",
        "private-cwd-canary-200",
        "private-home-canary-200",
        "private-pwd-canary-200",
        "private-key-canary-200",
    ] {
        assert!(
            !public.windows(canary.len()).any(|w| w == canary.as_bytes()),
            "private source inputs never publish"
        );
    }
}
