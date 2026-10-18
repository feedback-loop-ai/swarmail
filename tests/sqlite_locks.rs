//! A refused alias must not release the owner's SQLite locks (proposed 0047
//! §1, #129 macOS fix-up). By POSIX, closing *any* descriptor on a file
//! drops every record lock the process holds on it — so an alias open that
//! opened and closed the database (as `794e919`'s data-file lock did) would
//! silently strip the owning process's SQLite locks. Only a separate process
//! can observe that, so the contender is this test binary re-executed as a
//! raw SQLite child. This binary holds no other test: a child forked beside
//! a sibling test would briefly share that test's flocked descriptors.

use std::path::Path;
use std::process::Command;
use swarmail::persist::Persist;

/// The child's database, passed by environment to the re-executed binary.
const PROBE_ENV: &str = "SWARMAIL_RAW_SQLITE_PROBE";

/// Run the raw contender in a separate process and return its verdicts:
/// `write=` a zero-timeout `BEGIN IMMEDIATE` write, and `journal=` a switch
/// out of WAL, which needs the database file's exclusive lock — refused
/// while the owner's SQLite holds its shared lock on the file.
fn contend(path: &Path) -> (String, String) {
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "raw_sqlite_probe_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(PROBE_ENV, path)
        .output()
        .expect("spawn the raw SQLite child");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    let verdict = |key: &str| {
        stdout
            .lines()
            .find_map(|line| line.split_once(key).map(|(_, verdict)| verdict))
            .unwrap_or_else(|| panic!("no {key} verdict: {stdout}"))
            .to_string()
    };
    (verdict("probe write="), verdict("probe journal="))
}

/// The child: a raw connection with no busy wait, reporting each attempt.
#[test]
#[ignore = "re-executed by refused_aliases_preserve_the_owners_sqlite_locks"]
fn raw_sqlite_probe_child() {
    let Some(path) = std::env::var_os(PROBE_ENV) else {
        return;
    };
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.busy_timeout(std::time::Duration::ZERO).unwrap();
    let outcome = |sql: &str| match conn.execute_batch(sql) {
        Ok(()) => "ok".to_string(),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::DatabaseBusy =>
        {
            "busy".to_string()
        }
        Err(e) => format!("error {e}"),
    };
    let write =
        outcome("BEGIN IMMEDIATE; INSERT INTO meta (key, value) VALUES ('child', '1'); COMMIT;");
    let _ = conn.execute_batch("ROLLBACK");
    let journal = outcome("PRAGMA journal_mode = DELETE");
    println!("probe write={write}");
    println!("probe journal={journal}");
}

/// A live owner and a raw writer of this process in a write transaction;
/// refused opens through a hard link and a symlink must leave every SQLite
/// lock in place: the child is refused the write and the switch out of
/// WAL, then after the commit it writes — and the owner's shared lock still
/// refuses the WAL switch. Red on `794e919`: the refused alias's closed
/// database descriptor dropped this process's lock on the file, so the
/// child's WAL switch succeeded beside a live owner.
#[test]
fn refused_aliases_preserve_the_owners_sqlite_locks() {
    let dir = tempfile::TempDir::with_prefix("swarmail-held-locks-").unwrap();
    let real = dir.path().join("mail.db");
    let hard = dir.path().join("hard.db");
    let soft = dir.path().join("soft.db");
    let owner = Persist::open(&real).unwrap();
    std::fs::hard_link(&real, &hard).unwrap();
    std::os::unix::fs::symlink(&real, &soft).unwrap();

    let writer = rusqlite::Connection::open(&real).unwrap();
    writer
        .execute_batch("BEGIN IMMEDIATE; INSERT INTO meta (key, value) VALUES ('parent', '1');")
        .unwrap();
    for alias in [&hard, &soft] {
        let err = Persist::open(alias).err().expect("the alias is refused");
        assert_eq!(format!("{err:?}"), format!("InUse({alias:?})"));
    }
    assert_eq!(
        contend(&real),
        ("busy".to_string(), "busy".to_string()),
        "the process's SQLite locks survived the refused aliases"
    );
    writer.execute_batch("COMMIT").unwrap();
    drop(writer);
    assert_eq!(
        contend(&real),
        ("ok".to_string(), "busy".to_string()),
        "after the commit a raw writer proceeds; the owner's shared lock stays"
    );
    let (emails, inserted, _) = owner.load().unwrap();
    assert_eq!((emails.len(), inserted), (0, 0), "the owner still reads");
}
