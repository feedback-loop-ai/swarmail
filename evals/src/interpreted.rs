//! The interpreted-CLI pin (accepted 0137, 2026-11-23 amendment): dsh
//! 0.2.2-rc.1 ships as a node package, so its pin names the native node
//! runtime (absolute path, exact `--version` banner, SHA-256) and the SHA-256
//! of every file under the package root by relative path. Launch verifies
//! all of it before spend and runs the pinned runtime on the pinned entry
//! file directly, with no shim in between.
//!
//! A package entry is a regular file, a directory, or a symbolic link that
//! resolves inside the package root. A link is recorded, never followed: its
//! value is the SHA-256 of `symlink\0<target relative to the root>`, so a
//! retargeted link changes the manifest, and the file it names is covered
//! by its own entry. Anything else, or a link leaving the root, refuses.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    os::unix::fs::MetadataExt as _,
    path::{Path, PathBuf},
};

/// The most entries a package may hold, and the deepest directory: a bound
/// on the walk, far above an installed CLI's size.
pub const FILE_LIMIT: usize = 200_000;
pub const DEPTH_LIMIT: usize = 64;

/// One interpreted CLI's pin, as the paid selection names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    /// The native runtime binary, launched directly.
    pub runtime: PathBuf,
    /// The exact line `RUNTIME --version` answers (node: `v22.11.0`).
    pub runtime_version: String,
    pub runtime_sha256: String,
    /// The package root the manifest is relative to.
    pub package_root: PathBuf,
    /// The entry file the runtime is given, relative to the root.
    pub entry: String,
    /// Every package entry by relative path.
    pub files: BTreeMap<String, String>,
}

impl Pin {
    /// The absolute entry file the runtime runs.
    pub fn entry_path(&self) -> PathBuf {
        self.package_root.join(&self.entry)
    }
    /// The pin's identity, which a paid selection records as `cli_sha256`:
    /// the SHA-256 of `swarmail-interpreted-pin/1\n`, then
    /// `runtime <sha256> <version>\n`, `entry <path>\n`, then one
    /// `<sha256>  <path>\n` line per entry in byte order of path (the
    /// `sha256sum` line shape).
    pub fn identity(&self) -> String {
        let mut text = format!(
            "swarmail-interpreted-pin/1\nruntime {} {}\nentry {}\n",
            self.runtime_sha256, self.runtime_version, self.entry
        );
        for (path, sha) in &self.files {
            text.push_str(&format!("{sha}  {path}\n"));
        }
        let digest = ring::digest::digest(&ring::digest::SHA256, text.as_bytes());
        crate::provenance::hex(digest.as_ref())
    }
    /// The static shape: absolute runtime and root, a safe relative entry
    /// that the manifest names, well-formed paths and digests.
    pub fn validate(&self) -> Result<(), String> {
        let hashes = std::iter::once(&self.runtime_sha256).chain(self.files.values());
        if !self.runtime.is_absolute()
            || !self.package_root.is_absolute()
            || self.runtime_version.is_empty()
            || self.runtime_version.len() > 64
            || self.files.keys().any(|k| !relative(k))
            || hashes.into_iter().any(|h| !crate::provenance::is_hash(h))
        {
            return Err("interpreted_pin_invalid".into());
        }
        if !self.files.contains_key(&self.entry) {
            return Err("entry_invalid".into());
        }
        Ok(())
    }
}

/// A relative manifest path: non-empty `/`-separated normal components.
fn relative(path: &str) -> bool {
    !path.is_empty()
        && path
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != ".." && !c.contains('\0'))
}

fn link_value(target: &str) -> String {
    let mut bytes = b"symlink\0".to_vec();
    bytes.extend_from_slice(target.as_bytes());
    crate::provenance::hex(ring::digest::digest(&ring::digest::SHA256, &bytes).as_ref())
}

/// Measure every entry under `root`: the manifest the pin must equal.
pub fn measure(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let meta = std::fs::symlink_metadata(root).map_err(|_| "package_unavailable")?;
    if !meta.file_type().is_dir() {
        return Err("package_root_invalid".into());
    }
    let canonical = std::fs::canonicalize(root).map_err(|_| "package_unavailable")?;
    let uid = unsafe { libc::geteuid() };
    let mut out = BTreeMap::new();
    let mut stack = vec![(root.to_path_buf(), String::new(), 0usize)];
    while let Some((dir, prefix, depth)) = stack.pop() {
        if depth > DEPTH_LIMIT {
            return Err("package_too_large".into());
        }
        for entry in std::fs::read_dir(&dir).map_err(|_| "package_unavailable")? {
            let entry = entry.map_err(|_| "package_unavailable")?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "package_name_invalid")?;
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let path = entry.path();
            let m = std::fs::symlink_metadata(&path).map_err(|_| "package_unavailable")?;
            let kind = m.file_type();
            if !kind.is_symlink() && (m.mode() & 0o022 != 0 || (m.uid() != uid && m.uid() != 0)) {
                return Err("package_unsafe".into());
            }
            if kind.is_dir() {
                stack.push((path, rel, depth + 1));
                continue;
            }
            let value = if kind.is_file() {
                crate::identity::file_hash(&path).map_err(|_| "package_unavailable")?
            } else if kind.is_symlink() {
                let target = std::fs::canonicalize(&path).map_err(|_| "package_symlink_invalid")?;
                let inside = target
                    .strip_prefix(&canonical)
                    .map_err(|_| "package_symlink_escapes")?;
                link_value(inside.to_str().ok_or("package_name_invalid")?)
            } else {
                return Err("package_special_file".into());
            };
            out.insert(rel, value);
            if out.len() > FILE_LIMIT {
                return Err("package_too_large".into());
            }
        }
    }
    Ok(out)
}

/// The measured package against the pin: a missing, an extra and a changed
/// entry each refuse with their own reason.
pub fn compare(
    pinned: &BTreeMap<String, String>,
    measured: &BTreeMap<String, String>,
) -> Result<(), String> {
    if pinned.keys().any(|k| !measured.contains_key(k)) {
        return Err("package_file_missing".into());
    }
    if measured.keys().any(|k| !pinned.contains_key(k)) {
        return Err("package_file_extra".into());
    }
    if pinned.iter().any(|(k, v)| measured[k] != *v) {
        return Err("package_file_changed".into());
    }
    Ok(())
}

/// Build the pin of an installed package, for the operator's selection: the
/// runtime's measured digest, the given banner and every entry's digest.
pub fn pin(
    runtime: &Path,
    runtime_version: &str,
    package_root: &Path,
    entry: &str,
) -> Result<Pin, String> {
    let pin = Pin {
        runtime: runtime.to_path_buf(),
        runtime_version: runtime_version.to_string(),
        runtime_sha256: crate::identity::file_hash(runtime).map_err(|_| "runtime_unavailable")?,
        package_root: package_root.to_path_buf(),
        entry: entry.to_string(),
        files: measure(package_root)?,
    };
    pin.validate()?;
    Ok(pin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt as _, symlink};

    /// A package with a hidden file, a nested module and an in-root link,
    /// under a root resolved before use (macOS temporaries sit under `/var`).
    fn package() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap().join("pkg");
        for (rel, body) in [
            ("package.json", "{}"),
            ("dist/cli.js", "console.log(1)"),
            (".npmrc", ""),
            ("node_modules/m/index.js", "module.exports=1"),
        ] {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        std::fs::create_dir_all(root.join("node_modules/.bin")).unwrap();
        symlink("../../dist/cli.js", root.join("node_modules/.bin/dsh")).unwrap();
        for d in [
            "",
            "dist",
            "node_modules",
            "node_modules/m",
            "node_modules/.bin",
        ] {
            std::fs::set_permissions(root.join(d), std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        (dir, root)
    }
    fn pinned(root: &Path) -> Pin {
        let runtime = std::env::current_exe().unwrap();
        pin(&runtime, "v22.11.0", root, "dist/cli.js").unwrap()
    }

    #[test]
    fn a_pin_covers_every_entry_hidden_nested_and_linked() {
        let (_d, root) = package();
        let pin = pinned(&root);
        let keys: Vec<_> = pin.files.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                ".npmrc",
                "dist/cli.js",
                "node_modules/.bin/dsh",
                "node_modules/m/index.js",
                "package.json"
            ]
        );
        assert_eq!(
            pin.files["node_modules/.bin/dsh"],
            link_value("dist/cli.js")
        );
        assert_eq!(pin.entry_path(), root.join("dist/cli.js"));
        assert!(compare(&pin.files, &measure(&root).unwrap()).is_ok());
        // The identity moves with the runtime, its banner, the entry and any file.
        let base = pin.identity();
        assert!(crate::provenance::is_hash(&base));
        let mut other = pin.clone();
        other.runtime_version = "v22.11.1".into();
        assert_ne!(other.identity(), base);
        let mut other = pin.clone();
        other.entry = "package.json".into();
        assert_ne!(other.identity(), base);
        let mut other = pin.clone();
        other.files.insert("package.json".into(), "0".repeat(64));
        assert_ne!(other.identity(), base);
    }

    #[test]
    fn each_package_mutation_refuses_with_its_own_reason() {
        let (_d, root) = package();
        let pin = pinned(&root);
        let verdict = || compare(&pin.files, &measure(&root)?);
        std::fs::write(root.join("dist/cli.js"), "changed").unwrap();
        assert_eq!(verdict(), Err("package_file_changed".into()));
        std::fs::write(root.join("dist/cli.js"), "console.log(1)").unwrap();
        std::fs::write(root.join("dist/extra.js"), "").unwrap();
        std::fs::set_permissions(
            root.join("dist/extra.js"),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert_eq!(verdict(), Err("package_file_extra".into()));
        std::fs::remove_file(root.join("dist/extra.js")).unwrap();
        std::fs::remove_file(root.join(".npmrc")).unwrap();
        assert_eq!(verdict(), Err("package_file_missing".into()));
        std::fs::write(root.join(".npmrc"), "").unwrap();
        std::fs::set_permissions(root.join(".npmrc"), std::fs::Permissions::from_mode(0o644))
            .unwrap();
        assert_eq!(verdict(), Ok(()));
        // A link retargeted inside the root changes its entry.
        std::fs::remove_file(root.join("node_modules/.bin/dsh")).unwrap();
        symlink("../../package.json", root.join("node_modules/.bin/dsh")).unwrap();
        assert_eq!(verdict(), Err("package_file_changed".into()));
        // A link leaving the root, a dangling link, a group-writable file and
        // a special file refuse while walking, before any comparison.
        std::fs::remove_file(root.join("node_modules/.bin/dsh")).unwrap();
        let outside = root.with_file_name("outside.js");
        std::fs::write(&outside, "").unwrap();
        symlink(&outside, root.join("node_modules/.bin/dsh")).unwrap();
        assert_eq!(measure(&root), Err("package_symlink_escapes".into()));
        std::fs::remove_file(root.join("node_modules/.bin/dsh")).unwrap();
        symlink("../../missing.js", root.join("node_modules/.bin/dsh")).unwrap();
        assert_eq!(measure(&root), Err("package_symlink_invalid".into()));
        std::fs::remove_file(root.join("node_modules/.bin/dsh")).unwrap();
        std::fs::set_permissions(
            root.join("package.json"),
            std::fs::Permissions::from_mode(0o664),
        )
        .unwrap();
        assert_eq!(measure(&root), Err("package_unsafe".into()));
        std::fs::set_permissions(
            root.join("package.json"),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        let fifo = std::ffi::CString::new(root.join("fifo").into_os_string().into_encoded_bytes())
            .unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        assert_eq!(measure(&root), Err("package_special_file".into()));
        std::fs::remove_file(root.join("fifo")).unwrap();
        // A root that is itself a link, or absent, refuses.
        let link = root.with_file_name("pkg-link");
        symlink(&root, &link).unwrap();
        assert_eq!(measure(&link), Err("package_root_invalid".into()));
        assert_eq!(
            measure(&root.join("absent")),
            Err("package_unavailable".into())
        );
    }

    #[test]
    fn the_walk_is_bounded_and_names_must_be_utf8() {
        let (_d, root) = package();
        let mut deep = root.join("deep");
        for _ in 0..=DEPTH_LIMIT {
            std::fs::create_dir(&deep).unwrap();
            std::fs::set_permissions(&deep, std::fs::Permissions::from_mode(0o755)).unwrap();
            deep = deep.join("d");
        }
        assert_eq!(measure(&root), Err("package_too_large".into()));
        std::fs::remove_dir_all(root.join("deep")).unwrap();
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::ffi::OsStrExt as _;
            let bad = root.join(std::ffi::OsStr::from_bytes(b"\xff"));
            std::fs::write(&bad, "").unwrap();
            std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert_eq!(measure(&root), Err("package_name_invalid".into()));
            std::fs::remove_file(&bad).unwrap();
            // A link whose in-root target is not UTF-8: the link is walked
            // with the root's entries, before the directory holding the
            // target, so its own arm refuses.
            let sub = root.join("sub");
            std::fs::create_dir(&sub).unwrap();
            std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755)).unwrap();
            let target = sub.join(std::ffi::OsStr::from_bytes(b"\xff"));
            std::fs::write(&target, "").unwrap();
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
            std::os::unix::fs::symlink(&target, root.join("link")).unwrap();
            assert_eq!(measure(&root), Err("package_name_invalid".into()));
            std::fs::remove_file(root.join("link")).unwrap();
            std::fs::remove_dir_all(&sub).unwrap();
        }
        assert!(measure(&root).is_ok());
    }

    #[test]
    fn a_pin_refuses_unsafe_paths_digests_and_an_unlisted_entry() {
        let (_d, root) = package();
        let good = pinned(&root);
        assert!(good.validate().is_ok());
        type Edit = Box<dyn Fn(&mut Pin)>;
        let cases: Vec<(Edit, &str)> = vec![
            (
                Box::new(|p| p.runtime = "node".into()),
                "interpreted_pin_invalid",
            ),
            (
                Box::new(|p| p.package_root = "pkg".into()),
                "interpreted_pin_invalid",
            ),
            (
                Box::new(|p| p.runtime_version.clear()),
                "interpreted_pin_invalid",
            ),
            (
                Box::new(|p| p.runtime_version = "v".repeat(65)),
                "interpreted_pin_invalid",
            ),
            (
                Box::new(|p| p.runtime_sha256 = "A".repeat(64)),
                "interpreted_pin_invalid",
            ),
            (
                Box::new(|p| {
                    p.files.insert("../escape".into(), "0".repeat(64));
                }),
                "interpreted_pin_invalid",
            ),
            (
                Box::new(|p| {
                    p.files.insert("a//b".into(), "0".repeat(64));
                }),
                "interpreted_pin_invalid",
            ),
            (
                Box::new(|p| {
                    p.files.insert("ok".into(), "short".into());
                }),
                "interpreted_pin_invalid",
            ),
            (
                Box::new(|p| p.entry = "dist/absent.js".into()),
                "entry_invalid",
            ),
        ];
        for (edit, reason) in cases {
            let mut pin = good.clone();
            edit(&mut pin);
            assert_eq!(pin.validate(), Err(reason.into()));
        }
        assert!(!relative("") && !relative("./a") && !relative("/a") && relative("a/.b"));
        assert_eq!(
            pin(Path::new("/nonexistent/node"), "v1", &root, "dist/cli.js"),
            Err("runtime_unavailable".into())
        );
    }
}
