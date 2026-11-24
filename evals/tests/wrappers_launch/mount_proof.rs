//! F5's read-only binds, proven by a real mount (the operator's 2026-11-24
//! ruling on #404): inside a bubblewrap namespace carrying the recipe's
//! binds, a write to an owner-writable source through its read-only target
//! fails with EROFS, the private result directory accepts a write, the
//! source is unchanged after teardown, and the namespace's own mount table
//! passes `binds::verify` while the host's does not. Creating a user
//! namespace needs a privilege a boxed seat may lack, so this is a host
//! proof the dispatcher runs, ignored by default like the release proof:
//!
//! ```bash
//! cd evals && cargo test --locked --test wrappers mount_proof -- --ignored
//! ```
use super::*;

#[test]
#[ignore = "host proof: needs bubblewrap and an unprivileged user namespace (Linux)"]
fn a_read_only_bind_rejects_a_write_and_verifies_in_its_namespace() {
    let base = tempfile::tempdir().unwrap();
    let root = swarmail_evals::artifacts::resolve_root(base.path()).unwrap();
    let source = root.join("source");
    let tree = source.join("tree");
    std::fs::create_dir_all(&tree).unwrap();
    // Owner-writable fixtures: only the mount can refuse the write.
    let auth = tree.join("auth.json");
    std::fs::write(&auth, b"{\"inert\":true}\n").unwrap();
    std::fs::set_permissions(&auth, std::fs::Permissions::from_mode(0o600)).unwrap();
    let dns: Vec<(PathBuf, PathBuf)> = ["resolv.conf", "nsswitch.conf", "hosts"]
        .iter()
        .map(|name| {
            let fixture = source.join(name);
            std::fs::write(&fixture, format!("# inert {name}\n")).unwrap();
            std::fs::set_permissions(&fixture, std::fs::Permissions::from_mode(0o600)).unwrap();
            (fixture, PathBuf::from("/etc").join(name))
        })
        .collect();
    let out = root.join("out");
    std::fs::create_dir(&out).unwrap();
    let script = format!(
        "import errno, os, shutil\n\
         shutil.copyfile('/proc/self/mountinfo', {mi:?})\n\
         def attempt(path):\n\
         \x20   try:\n\
         \x20       with open(path, 'a') as f: f.write('tampered\\n')\n\
         \x20       return 'written'\n\
         \x20   except OSError as e:\n\
         \x20       return errno.errorcode.get(e.errno, str(e.errno))\n\
         codes = [attempt(p) for p in ({auth:?}, '/etc/hosts', '/etc/resolv.conf', '/etc/nsswitch.conf')]\n\
         open({result:?}, 'w').write(' '.join(codes) + '\\n')\n",
        mi = out.join("mountinfo").to_str().unwrap(),
        auth = auth.to_str().unwrap(),
        result = out.join("result").to_str().unwrap(),
    );
    let mut cmd = std::process::Command::new("bwrap");
    cmd.args([
        "--die-with-parent",
        "--unshare-user",
        "--dev-bind",
        "/",
        "/",
    ])
    .args(["--proc", "/proc"]);
    // The resolver directory first (where the host has it), so a
    // resolv.conf linked into it is bound over inside the read-only bind.
    let systemd = Path::new("/run/systemd/resolve");
    if systemd.exists() {
        cmd.arg("--ro-bind").arg(systemd).arg(systemd);
    }
    for (fixture, target) in &dns {
        cmd.arg("--ro-bind").arg(fixture).arg(target);
    }
    cmd.arg("--ro-bind").arg(&tree).arg(&tree);
    cmd.arg("--bind").arg(&out).arg(&out);
    cmd.args(["python3", "-c", &script]);
    let run = cmd.output().expect("bwrap");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    // Every write through a read-only target failed with EROFS; the
    // private result directory took a write.
    let result = std::fs::read_to_string(out.join("result")).unwrap();
    assert_eq!(result, "EROFS EROFS EROFS EROFS\n");
    // After teardown each source still holds its own bytes.
    assert_eq!(std::fs::read(&auth).unwrap(), b"{\"inert\":true}\n");
    for (fixture, target) in &dns {
        let name = target.file_name().unwrap().to_str().unwrap();
        assert_eq!(
            std::fs::read_to_string(fixture).unwrap(),
            format!("# inert {name}\n")
        );
    }
    // The namespace's mount table verifies; the host's does not.
    let inside = std::fs::read_to_string(out.join("mountinfo")).unwrap();
    // The namespace shares the host's tree, links included.
    let resolve = |p: &Path| std::fs::canonicalize(p).ok();
    assert_eq!(
        swarmail_evals::binds::verify(&inside, &[auth.as_path()], resolve),
        Ok(())
    );
    let host = std::fs::read_to_string("/proc/self/mountinfo").unwrap();
    assert!(swarmail_evals::binds::verify(&host, &[auth.as_path()], resolve).is_err());
}
