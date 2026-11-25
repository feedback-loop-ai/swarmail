//! The recipe route's F5 binding (the operator's 2026-11-24 ruling on #404):
//! a nested paid wrapper with no host paid-launch binding may launch only
//! when the trial namespace it runs in holds the eval recipe's read-only
//! binds. The proof is the effective mount table, never the bundle text and
//! never an access probe on host files: every protected path must resolve
//! under a mount whose per-mount options say `ro`, with no writable mount
//! stacked inside a protected tree.
use std::path::{Path, PathBuf};

/// The four DNS inputs the trial seat binds read-only. Each must exist
/// inside the namespace: a missing source refuses by name
/// (`recipe_bind_missing`), never silently drops.
pub const DNS: [&str; 4] = [
    "/etc/resolv.conf",
    "/etc/nsswitch.conf",
    "/etc/hosts",
    "/run/systemd/resolve",
];

/// The trial seat's declared binds in `.brokkr/recipes/eval/bundle.json`,
/// path and mode, that a paid agent under `matrix --brokkr` requires. The
/// wrapper re-proves the effect in its own namespace; this is the matrix's
/// refusal before any slot spends, so a recipe without them never starts
/// a paid agent.
pub const RECIPE_BINDS: [(&str, &str); 14] = [
    ("/etc/resolv.conf", "ro"),
    ("/run/systemd/resolve", "ro"),
    ("/etc/nsswitch.conf", "ro"),
    ("/etc/hosts", "ro"),
    ("~/.config/swarmail-evals", "ro"),
    ("~/.local/share/brokkr-0.15.2", "ro"),
    ("~/.local/share/claude", "ro"),
    ("~/.volta", "ro"),
    ("~/.claude", "ro"),
    ("~/.codex", "ro"),
    ("~/.local/bin", "ro"),
    ("~/.claude.json", "ro"),
    ("~/.dsh/spark.env", "ro"),
    ("~/.dsh/profiles", "overlay"),
];

/// Whether a bundle's `trial` seat declares every [`RECIPE_BINDS`] entry
/// with its mode. Extra binds are the bundle's business.
pub fn declared(bundle: &str) -> Result<(), &'static str> {
    let value: serde_json::Value =
        serde_json::from_str(bundle).map_err(|_| "brokkr_realm_invalid")?;
    let binds = value["seats"]["trial"]["hands"]["binds"]
        .as_array()
        .ok_or("brokkr_paid_unbound")?;
    let has = |path: &str, mode: &str| {
        binds
            .iter()
            .any(|b| b["path"] == path && b["mode"] == mode && b.get("mask").is_none())
    };
    if RECIPE_BINDS.iter().all(|(path, mode)| has(path, mode)) {
        Ok(())
    } else {
        Err("brokkr_paid_unbound")
    }
}

/// One mount-table entry: its mount point and whether it is read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Mount {
    point: PathBuf,
    read_only: bool,
    fstype: String,
}

/// Undo mountinfo's octal escapes (`\040` space, `\011` tab, `\012`
/// newline, `\134` backslash). Anything else is kept as written.
fn unescape(field: &str) -> PathBuf {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let octal = bytes
            .get(i + 1..i + 4)
            .filter(|d| bytes[i] == b'\\' && d.iter().all(|c| (b'0'..=b'7').contains(c)))
            .map(|d| d.iter().fold(0u32, |n, c| n * 8 + u32::from(c - b'0')));
        match octal.and_then(|n| u8::try_from(n).ok()) {
            Some(b) => {
                out.push(b);
                i += 4;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    PathBuf::from(<std::ffi::OsString as std::os::unix::ffi::OsStringExt>::from_vec(out))
}

/// Parse `/proc/self/mountinfo`: field 5 is the mount point, field 6 the
/// per-mount options, and the first field after the ` - ` separator the
/// filesystem type. A line that does not have them is skipped.
fn mounts(mountinfo: &str) -> Vec<Mount> {
    mountinfo
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(' ').skip(4);
            let point = unescape(fields.next()?);
            let options = fields.next()?;
            let fstype = fields.skip_while(|f| *f != "-").nth(1)?;
            Some(Mount {
                point,
                read_only: options.split(',').any(|o| o == "ro"),
                fstype: fstype.to_string(),
            })
        })
        .collect()
}

/// The mount a path lives on: the deepest mount point that is an ancestor
/// of (or equal to) it, the later entry winning on a stacked point.
fn owner<'a>(table: &'a [Mount], path: &Path) -> Option<&'a Mount> {
    let mut best: Option<&Mount> = None;
    for m in table {
        if path.starts_with(&m.point)
            && best.is_none_or(|b| m.point.components().count() >= b.point.components().count())
        {
            best = Some(m);
        }
    }
    best
}

/// Whether `path` (already resolved) and everything mounted beneath it is
/// read-only in this table.
fn protected(table: &[Mount], path: &Path) -> bool {
    owner(table, path).is_some_and(|m| m.read_only)
        && table
            .iter()
            .all(|m| m.read_only || !m.point.starts_with(path) || m.point == path)
}

/// Check a mount table against the DNS inputs and the selection's sources.
/// `resolve` maps a path to its canonical form inside the namespace (`None`
/// when it is absent there). Each refusal is a fixed reason.
pub fn verify(
    mountinfo: &str,
    sources: &[&Path],
    resolve: impl Fn(&Path) -> Option<PathBuf>,
) -> Result<(), String> {
    let table = mounts(mountinfo);
    // Presence first, over every source, then protection: a missing
    // declared source names itself even where an earlier input (a host's
    // writable DNS file) would also refuse, so the wrapper's wiring of the
    // HOME inventory is observable outside a recipe namespace.
    let resolved = DNS
        .iter()
        .map(Path::new)
        .chain(sources.iter().copied())
        .map(&resolve)
        .collect::<Option<Vec<_>>>()
        .ok_or("recipe_bind_missing")?;
    if resolved.iter().all(|real| protected(&table, real)) {
        Ok(())
    } else {
        Err("recipe_bind_writable".into())
    }
}

/// The recipe seat's source HOME, read from where the operator's selection
/// lies: under the declared `~/.config/swarmail-evals` tree, so `config`'s
/// nearest `.config/swarmail-evals` ancestor names it. It only locates the
/// declared mounts, each of which must then prove read-only; it confers no
/// authority. A selection elsewhere has no recipe HOME (`None`).
pub fn source_home(config: &Path) -> Option<PathBuf> {
    config
        .ancestors()
        .find(|a| a.ends_with(".config/swarmail-evals"))
        .and_then(|a| a.parent()?.parent())
        .filter(|h| h.is_absolute())
        .map(Path::to_path_buf)
}

/// Every read-only [`RECIPE_BINDS`] entry under HOME, resolved against the
/// seat's source `home`: the declared inventory the wrapper re-proves.
pub fn inventory(home: &Path) -> Vec<PathBuf> {
    RECIPE_BINDS
        .iter()
        .filter(|(_, mode)| *mode == "ro")
        .filter_map(|(path, _)| path.strip_prefix("~/"))
        .map(|rest| home.join(rest))
        .collect()
}

/// The declared overlay's target under `home` (dsh's profiles).
pub fn overlay_target(home: &Path) -> PathBuf {
    let (path, _) = RECIPE_BINDS[RECIPE_BINDS.len() - 1];
    home.join(path.trim_start_matches("~/"))
}

/// The whole recipe check: the DNS inputs, every declared read-only source
/// under `home` and the selection's own `selected` paths are protected (a
/// missing one refuses `recipe_bind_missing`, a writable one or a writable
/// mount beneath it `recipe_bind_writable`), and the profiles target is
/// itself a writable overlay mount, so the seat's writes stay private
/// (`recipe_overlay_missing` when the target is absent,
/// `recipe_overlay_not_private` when it is anything but an overlay mount).
pub fn verify_recipe(
    mountinfo: &str,
    home: &Path,
    selected: &[&Path],
    resolve: impl Fn(&Path) -> Option<PathBuf>,
) -> Result<(), String> {
    let declared = inventory(home);
    let sources: Vec<&Path> = declared
        .iter()
        .map(PathBuf::as_path)
        .chain(selected.iter().copied())
        .collect();
    verify(mountinfo, &sources, &resolve)?;
    let real = resolve(&overlay_target(home)).ok_or("recipe_overlay_missing")?;
    let table = mounts(mountinfo);
    match owner(&table, &real) {
        Some(m) if m.point == real && m.fstype == "overlay" && !m.read_only => Ok(()),
        _ => Err("recipe_overlay_not_private".into()),
    }
}

/// The live check a paid recipe wrapper makes: this process's own mount
/// table. A host outside any recipe seat has writable DNS inputs, so an
/// unbound paid selection refuses here before any credential is read.
pub fn verify_live(home: &Path, selected: &[&Path]) -> Result<(), String> {
    let mountinfo = std::fs::read_to_string("/proc/self/mountinfo")
        .map_err(|_| "recipe_bind_unverifiable".to_string())?;
    verify_recipe(&mountinfo, home, selected, |p| {
        std::fs::canonicalize(p).ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(point: &str, options: &str) -> String {
        format!("36 35 98:0 / {point} {options} master:1 - ext4 /dev/root rw")
    }
    fn table(rows: &[(&str, &str)]) -> String {
        rows.iter()
            .map(|(p, o)| line(p, o))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn present(p: &Path) -> Option<PathBuf> {
        Some(p.to_path_buf())
    }
    const BOUND: [(&str, &str); 6] = [
        ("/", "rw,relatime"),
        ("/etc/resolv.conf", "ro,nosuid"),
        ("/etc/nsswitch.conf", "ro"),
        ("/etc/hosts", "ro"),
        ("/run/systemd/resolve", "ro"),
        ("/opt/cli", "ro"),
    ];

    #[test]
    fn the_recipe_binds_verify_only_when_every_input_is_read_only() {
        let bound = table(&BOUND);
        let cli = Path::new("/opt/cli/bin/claude");
        assert_eq!(verify(&bound, &[cli], present), Ok(()));
        // The host itself: one writable root holds everything.
        let host = table(&[("/", "rw,relatime")]);
        assert_eq!(
            verify(&host, &[cli], present),
            Err("recipe_bind_writable".into())
        );
        // A source outside the binds, a writable mount stacked inside a
        // protected tree, and a source absent from the namespace.
        assert_eq!(
            verify(&bound, &[Path::new("/home/op/auth.json")], present),
            Err("recipe_bind_writable".into())
        );
        let mut stacked = BOUND.to_vec();
        stacked.push(("/opt/cli/bin", "rw"));
        assert_eq!(
            verify(&table(&stacked), &[Path::new("/opt/cli")], present),
            Err("recipe_bind_writable".into())
        );
        let gone = |p: &Path| (p != Path::new("/opt/cli/bin/claude")).then(|| p.to_path_buf());
        assert_eq!(
            verify(&bound, &[cli], gone),
            Err("recipe_bind_missing".into())
        );
        // A stacked point: the later entry wins, so ro over rw protects and
        // rw over ro does not.
        let mut over = BOUND.to_vec();
        over.push(("/etc/hosts", "rw"));
        assert_eq!(
            verify(&table(&over), &[], present),
            Err("recipe_bind_writable".into())
        );
        // Every DNS source must be there, the systemd resolver included.
        let no_systemd =
            |p: &Path| (p != Path::new("/run/systemd/resolve")).then(|| p.to_path_buf());
        assert_eq!(
            verify(&bound, &[], no_systemd),
            Err("recipe_bind_missing".into())
        );
        let no_hosts = |p: &Path| (p != Path::new("/etc/hosts")).then(|| p.to_path_buf());
        assert_eq!(
            verify(&bound, &[], no_hosts),
            Err("recipe_bind_missing".into())
        );
        // resolv.conf resolving into the bound resolver directory is
        // protected by that bind.
        let linked = |p: &Path| {
            Some(if p == Path::new("/etc/resolv.conf") {
                PathBuf::from("/run/systemd/resolve/stub-resolv.conf")
            } else {
                p.to_path_buf()
            })
        };
        assert_eq!(verify(&bound, &[], linked), Ok(()));
    }

    /// The whole declared inventory under the source HOME, not only the
    /// selected files, and dsh's profiles as a private overlay (guarantees 1
    /// on 6074b83e).
    #[test]
    fn the_recipe_check_proves_every_declared_source_and_the_overlay() {
        let home = Path::new("/home/op");
        assert_eq!(
            source_home(Path::new("/home/op/.config/swarmail-evals/agents/sel.json")),
            Some(home.to_path_buf())
        );
        assert_eq!(source_home(Path::new("/srv/sel.json")), None);
        assert_eq!(
            source_home(Path::new(".config/swarmail-evals/s.json")),
            None
        );
        let declared = inventory(home);
        assert_eq!(declared.len(), 9);
        assert!(declared.contains(&home.join(".volta")));
        assert!(declared.contains(&home.join(".dsh/spark.env")));
        assert_eq!(overlay_target(home), home.join(".dsh/profiles"));
        // Every declared tree read-only, the overlay a writable overlay.
        let mut rows: Vec<(String, &str, &str)> = BOUND
            .iter()
            .map(|(p, o)| (p.to_string(), *o, "ext4"))
            .collect();
        for path in &declared {
            rows.push((path.to_str().unwrap().to_string(), "ro", "ext4"));
        }
        let profiles = overlay_target(home).to_str().unwrap().to_string();
        let render = |rows: &[(String, &str, &str)]| {
            rows.iter()
                .map(|(p, o, t)| format!("36 35 98:0 / {p} {o} master:1 - {t} src rw"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let mut good = rows.clone();
        good.push((profiles.clone(), "rw,relatime", "overlay"));
        let sel = Path::new("/opt/cli/bin/dsh");
        assert_eq!(verify_recipe(&render(&good), home, &[sel], present), Ok(()));
        // A writable sibling mounted under a declared read-only tree.
        let mut sibling = good.clone();
        sibling.push(("/home/op/.volta/tools/node_modules".into(), "rw", "ext4"));
        assert_eq!(
            verify_recipe(&render(&sibling), home, &[sel], present),
            Err("recipe_bind_writable".into())
        );
        // A declared source missing from the namespace.
        let no_env = |p: &Path| (p != home.join(".dsh/spark.env")).then(|| p.to_path_buf());
        assert_eq!(
            verify_recipe(&render(&good), home, &[sel], no_env),
            Err("recipe_bind_missing".into())
        );
        // The profiles target absent, an ordinary writable directory, an
        // ordinary writable mount, or a read-only overlay.
        let no_profiles = |p: &Path| (p != overlay_target(home)).then(|| p.to_path_buf());
        assert_eq!(
            verify_recipe(&render(&good), home, &[sel], no_profiles),
            Err("recipe_overlay_missing".into())
        );
        assert_eq!(
            verify_recipe(&render(&rows), home, &[sel], present),
            Err("recipe_overlay_not_private".into())
        );
        for (o, t) in [("rw", "ext4"), ("ro", "overlay")] {
            let mut other = rows.clone();
            other.push((profiles.clone(), o, t));
            assert_eq!(
                verify_recipe(&render(&other), home, &[sel], present),
                Err("recipe_overlay_not_private".into())
            );
        }
    }

    #[test]
    fn the_eval_recipe_declares_every_paid_bind() {
        let bundle = include_str!("../../.brokkr/recipes/eval/bundle.json");
        assert_eq!(declared(bundle), Ok(()));
        let mut value: serde_json::Value = serde_json::from_str(bundle).unwrap();
        let binds = value["seats"]["trial"]["hands"]["binds"].clone();
        // Each bind missing, or with another mode, refuses.
        for index in 0..RECIPE_BINDS.len() {
            let mut fewer = binds.clone();
            fewer.as_array_mut().unwrap().remove(index);
            let mut doc = value.clone();
            doc["seats"]["trial"]["hands"]["binds"] = fewer;
            assert_eq!(declared(&doc.to_string()), Err("brokkr_paid_unbound"));
            let mut writable = binds.clone();
            writable[index]["mode"] = "rw".into();
            doc["seats"]["trial"]["hands"]["binds"] = writable;
            assert_eq!(declared(&doc.to_string()), Err("brokkr_paid_unbound"));
        }
        // A masked bind declares nothing.
        let mut masked = binds.clone();
        masked[0]["mask"] = serde_json::json!(["*"]);
        let mut doc = value.clone();
        doc["seats"]["trial"]["hands"]["binds"] = masked;
        assert_eq!(declared(&doc.to_string()), Err("brokkr_paid_unbound"));
        value["seats"]["trial"]["hands"]["binds"] = serde_json::Value::Null;
        assert_eq!(declared(&value.to_string()), Err("brokkr_paid_unbound"));
        assert_eq!(declared("{"), Err("brokkr_realm_invalid"));
    }

    #[test]
    fn mountinfo_fields_are_unescaped_and_short_lines_skipped() {
        let info = format!("{}\nshort line\n", line("/opt/my\\040cli", "ro"));
        assert_eq!(
            mounts(&info),
            vec![Mount {
                point: PathBuf::from("/opt/my cli"),
                read_only: true,
                fstype: "ext4".into(),
            }]
        );
        assert_eq!(unescape("a\\134b\\9xy\\"), PathBuf::from("a\\b\\9xy\\"));
        assert_eq!(unescape("\\777"), PathBuf::from("\\777"));
        assert_eq!(owner(&[], Path::new("/x")), None);
    }

    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "bind ownership is read from /proc (Linux)"
    )]
    #[test]
    fn a_source_absent_from_the_live_namespace_is_refused() {
        let absent = Path::new("/nonexistent-swarmail-evals-bind-source");
        assert!(verify_live(absent, &[absent]).is_err());
    }
}
