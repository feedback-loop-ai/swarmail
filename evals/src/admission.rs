//! Nonblocking admission shared by cooperating same-uid runners (0076).
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub(crate) struct Admission {
    _file: File,
}

fn private(metadata: &std::fs::Metadata, uid: u32, directory: bool) -> bool {
    metadata.uid() == uid
        && metadata.mode() & 0o7777 == if directory { 0o700 } else { 0o600 }
        && if directory {
            metadata.is_dir()
        } else {
            metadata.is_file() && metadata.nlink() == 1
        }
}

impl Admission {
    pub(crate) fn acquire() -> Result<Self, &'static str> {
        // SAFETY: geteuid has no preconditions.
        let uid = unsafe { libc::geteuid() };
        Self::at(&Self::root_for(uid), uid)
    }

    /// Keep the host admission for the rest of the process (0076 §3): used
    /// when the controller could not prove bwrap was reaped. The lock is
    /// released only by the process exit, so a later acquisition in this
    /// process refuses. The limit is 0076's own: an advisory `flock` is
    /// process-scoped, so that exit releases it while an unreaped bwrap may
    /// still hold a live namespace.
    pub(crate) fn retain(self) {
        std::mem::forget(self);
    }

    /// The private per-uid directory the lock lives in, on the host `/tmp`.
    pub(crate) fn root_for(uid: u32) -> PathBuf {
        PathBuf::from(format!("/tmp/swarmail-evals-{uid}"))
    }

    fn at(root: &Path, uid: u32) -> Result<Self, &'static str> {
        let unavailable = "trial_admission_unavailable";
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        match builder.create(root) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(unavailable),
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
            .open(root)
            .map_err(|_| unavailable)?;
        let before = directory.metadata().map_err(|_| unavailable)?;
        if !private(&before, uid, true) {
            return Err(unavailable);
        }
        let path = root.join("trial.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(&path)
            .map_err(|_| unavailable)?;
        let held = file.metadata().map_err(|_| unavailable)?;
        if !private(&held, uid, false) {
            return Err(unavailable);
        }
        file.try_lock().map_err(|e| match e {
            std::fs::TryLockError::WouldBlock => "concurrent_trials_refused",
            std::fs::TryLockError::Error(_) => unavailable,
        })?;
        let after = std::fs::symlink_metadata(root).map_err(|_| unavailable)?;
        let named = std::fs::symlink_metadata(path).map_err(|_| unavailable)?;
        if !private(&after, uid, true)
            || !private(&named, uid, false)
            || (before.dev(), before.ino()) != (after.dev(), after.ino())
            || (held.dev(), held.ino()) != (named.dev(), named.ino())
        {
            return Err(unavailable);
        }
        // Release by closing only. The stable inode must never be unlinked.
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn sequential_admission_rejects_unsafe_rendezvous() {
        // A fork elsewhere in the test process briefly holds every
        // descriptor without close-on-exec yet; the scope keeps it out.
        #[cfg(target_os = "linux")]
        let _scope = crate::test_support::trial_scope();
        let owner = tempfile::tempdir().unwrap();
        let root = owner.path().join("reservation");
        let uid = unsafe { libc::geteuid() };
        let first = Admission::at(&root, uid).unwrap();
        assert!(matches!(
            Admission::at(&root, uid),
            Err("concurrent_trials_refused")
        ));
        drop(first);
        drop(Admission::at(&root, uid).unwrap());
        assert!(Admission::at(&root, uid.wrapping_add(1)).is_err());
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(Admission::at(&root, uid).is_err());
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let lock = root.join("trial.lock");
        std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Admission::at(&root, uid).is_err());
        std::fs::remove_file(&lock).unwrap();
        symlink(owner.path().join("missing"), &lock).unwrap();
        assert!(Admission::at(&root, uid).is_err());
        std::fs::remove_file(&lock).unwrap();
        std::fs::create_dir(&lock).unwrap();
        assert!(Admission::at(&root, uid).is_err());
        std::fs::remove_dir(&lock).unwrap();
        std::fs::write(&lock, b"").unwrap();
        std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::hard_link(&lock, owner.path().join("alias")).unwrap();
        assert!(Admission::at(&root, uid).is_err());
        let alias = owner.path().join("root-alias");
        symlink(&root, &alias).unwrap();
        assert!(Admission::at(&alias, uid).is_err());
    }

    /// The child half of the retain test: only acts when the parent test
    /// re-executes this binary for it.
    #[test]
    fn retain_child() {
        let Some(root) = std::env::var_os("ADMISSION_TEST_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let uid = unsafe { libc::geteuid() };
        Admission::at(&root, uid).unwrap().retain();
        assert!(matches!(
            Admission::at(&root, uid),
            Err("concurrent_trials_refused")
        ));
    }

    #[test]
    fn a_retained_admission_is_released_only_by_the_process_exit() {
        #[cfg(target_os = "linux")]
        let _scope = crate::test_support::trial_scope();
        let owner = tempfile::tempdir().unwrap();
        let root = owner.path().join("reservation");
        let uid = unsafe { libc::geteuid() };
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "admission::tests::retain_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("ADMISSION_TEST_ROOT", &root)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the retain child never exited");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert!(status.success(), "the child failed: {status:?}");
        drop(Admission::at(&root, uid).unwrap());
    }
}
