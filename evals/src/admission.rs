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
        Self::at(&PathBuf::from(format!("/tmp/swarmail-evals-{uid}")), uid)
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
}
