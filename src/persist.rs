//! Optional SQLite persistence: a write-through mirror of the in-memory store.
//!
//! Enabled per server with `--data-file` / `SWARMAIL_DATA_FILE`. Every
//! mutation (`insert`, `delete`, `clear`, `clear_all`) is committed to SQLite
//! **synchronously, inside the mutation** — ingest stays synchronous
//! (decision 0001): an SMTP `250` is only sent after the row is on disk. On
//! startup the full state is restored: inboxes, messages with raw bytes, and
//! the `emails_inserted`/`emails_dropped` lifetime counters.
//!
//! Layout: one `emails` table (one row per mail, `seq` preserving insertion
//! order) and one `meta` table (counters). All writes serialize through a
//! single connection guarded by a `std` mutex — held for one small
//! transaction, never across an `await`. When the mutation is driven from an
//! async handler, the store's awaited wrappers (`insert_async` and friends)
//! run the whole commit on tokio's blocking pool, so the wait on the disk
//! never occupies an async worker (decision 0023) — it parks a task instead.
//!
//! **One server per data file** — a second process opening the same file
//! refuses at startup. Two advisory locks are held for the process's
//! lifetime: the `<data-file>.lock` sidecar beside the path as spelled
//! (decision 0033 §§2–4, decision 0042), and an identity guard,
//! `/tmp/swarmail-db-locks-<owner uid>/<device>-<inode>.lock`, so the file's
//! identity — not its name — is owned: a second server reaching it through
//! a symlink or a hard link, in any directory, names the same guard and is
//! refused too (decision 0047 §1, issue #129). The root is fixed — never
//! `TMPDIR`, `HOME` or the file's own directory — so every spelling meets
//! there. Neither lock is ever on the database or its `-wal`/`-shm`/
//! `-journal`: on Darwin and the BSDs `flock` and SQLite's POSIX record
//! locks share one table and conflict, so a lock of ours on the database
//! refuses our own SQLite. Both are taken before SQLite touches the file and
//! are owned by [`Persist`], so the kernel releases them when the last owner
//! drops or the process dies. Neither is ever unlinked; the owner directory
//! must be private (0700) to the data file's owner and the guard a private,
//! singly linked file, or startup refuses. An existing data file is
//! identified by metadata alone, never opened — closing a descriptor on it
//! would drop this process's SQLite record locks — and a missing one is
//! created exclusively, no wider than SQLite's own 0644 (subject to umask).
//!
//! An ordinary missing data path is created. A dangling symlink is an
//! `OpenError::Io` refusal for the supplied path with `ENOENT`; the symlink
//! remains and its missing target is not created. Relative paths and paths
//! through a symlinked parent are admitted with their spelling preserved for
//! path sidecars and errors. Cooperating servers using those aliases meet at
//! the identity guard while they share the stable guard namespace.
//!
//! The scope is cooperating current servers under the data file's owner,
//! on a local filesystem with working `flock`, sharing one `/tmp`. Outside
//! it, and not refused: an older, sidecar-only binary reaching the file
//! through a different alias (same path still collides on the sidecar); a
//! writer that takes no lock at all; servers in separate private-`/tmp`
//! mount namespaces; a `/tmp` cleaner deleting a held guard (exclude the
//! directory from age-based cleanup); replacing the file or its directory
//! under a live owner; single-file bind mounts (unproven); and remote
//! filesystems, whose lock semantics vary. A server under another user may
//! be refused as an I/O error rather than as in use. Keep one configured
//! pathname across restarts: owning the inode does not move a crashed
//! owner's `-wal` file to another hard link's basename, and a restart never
//! repairs loss an earlier multi-writer run already caused.

use crate::model::Email;
use rusqlite::{Connection, OptionalExtension, params};
use serde::de::DeserializeOwned;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Every column of a mail row, in the order both statements use it.
const COLUMNS: &str = "id, inbox, from_json, to_json, cc_json, recipients_json, subject, \
                       received_at, received_ms, size, text, html, links_json, codes_json, \
                       message_id, in_reply_to, references_json, raw, envelope_from";

/// The whole schema, created idempotently on every open.
const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS emails (
    id TEXT PRIMARY KEY,
    inbox TEXT NOT NULL,
    seq INTEGER NOT NULL,
    from_json TEXT NOT NULL,
    to_json TEXT NOT NULL,
    cc_json TEXT NOT NULL,
    recipients_json TEXT NOT NULL,
    subject TEXT,
    received_at TEXT NOT NULL,
    received_ms INTEGER NOT NULL,
    size INTEGER NOT NULL,
    text TEXT,
    html TEXT,
    links_json TEXT NOT NULL,
    codes_json TEXT NOT NULL,
    message_id TEXT,
    in_reply_to TEXT,
    references_json TEXT NOT NULL DEFAULT '[]',
    raw BLOB NOT NULL,
    envelope_from TEXT
);
-- Retired (issue #87): `emails_inbox` was maintained on every write and read
-- by no query — the store answers per-inbox reads from memory, and restore
-- scans by seq. Dropped on every open so existing files shed it too.
DROP INDEX IF EXISTS emails_inbox;
-- The restore's sort key (issue #54): without it `load()` sorts every raw
-- BLOB through a temp b-tree on every open. An index on (seq) is (seq,
-- rowid) — SQLite's implicit rowid column — so `ORDER BY seq, rowid` is
-- answered straight from the index, and the tie-break is the write order.
CREATE INDEX IF NOT EXISTS emails_seq ON emails(seq);
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";

/// Columns the first release lacked, with their types: a data file written
/// before threading — or before the envelope sender was kept (decision 0044
/// §8: nullable, no default, no backfill, so an old row restores `None`) —
/// is migrated on open instead of abandoned.
const ADDED_COLUMNS: &[(&str, &str)] = &[
    ("message_id", "TEXT"),
    ("in_reply_to", "TEXT"),
    ("references_json", "TEXT NOT NULL DEFAULT '[]'"),
    ("envelope_from", "TEXT"),
];

/// The full restore scan `load` runs — one builder, so the plan test
/// EXPLAINs exactly this projection and never a narrowed stand-in (issue
/// #87). `seq, rowid`: the index answers it, and rows that share a seq (a
/// legacy file written before the seed fix) come back in write order
/// instead of SQLite's arbitrary tie-break (issue #54).
fn restore_query() -> String {
    format!("SELECT {COLUMNS} FROM emails ORDER BY seq, rowid")
}

/// Counters live in `meta` as decimal strings.
const UPSERT_COUNTER: &str = "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)";

struct State {
    conn: Connection,
    /// Monotonic insertion sequence; restores the per-inbox push order of the
    /// in-memory store exactly (a mailbox is an ordered Vec, oldest first).
    next_seq: i64,
}

/// The write-through mirror. Cheap to clone-free share: `&self` is enough.
pub struct Persist {
    state: Mutex<State>,
    /// The held identity guard (decision 0047 §1) — never the data file
    /// itself. Declared after `state` so SQLite finishes closing and
    /// checkpointing before ownership is released.
    _identity: File,
    /// The held `<data-file>.lock` (decision 0033 §2). Declared after
    /// `state` so the connection closes before ownership is released.
    _lock: File,
}

/// Why a data file could not be opened (decision 0042 §2): another
/// server owns it, its sidecar or the file itself cannot be opened or
/// locked, or SQLite refused it.
#[derive(Debug)]
pub enum OpenError {
    /// Another `Persist` — this process or another — holds the data file.
    InUse(PathBuf),
    /// The sidecar or the data file could not be opened or locked.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// SQLite refused to open, migrate or restore the data file.
    Sqlite(rusqlite::Error),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InUse(path) => write!(
                f,
                "the data file is already in use by another server: {}",
                path.display()
            ),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Sqlite(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InUse(_) => None,
            Self::Io { source, .. } => Some(source),
            Self::Sqlite(e) => Some(e),
        }
    }
}

impl From<rusqlite::Error> for OpenError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

/// The sidecar beside `path`: `.lock` appended to the whole OS path, so
/// `mail.db` and `mail.sqlite` never share one (no extension replacement).
fn lock_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".lock");
    PathBuf::from(name)
}

/// Only contention is `InUse`, named by the data file as supplied; any
/// other lock failure (an unsupported filesystem) refuses startup with the
/// locked file and its cause — never a silent unlocked run.
fn lock_error(path: &Path, locked: PathBuf, err: TryLockError) -> OpenError {
    match err {
        TryLockError::WouldBlock => OpenError::InUse(path.to_path_buf()),
        TryLockError::Error(source) => OpenError::Io {
            path: locked,
            source,
        },
    }
}

/// The I/O refusal naming `path`, keeping its cause.
fn io_at(path: &Path) -> impl FnOnce(std::io::Error) -> OpenError + '_ {
    move |source| OpenError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// An unusable or unsafe filesystem entry: an I/O refusal naming it.
fn refuse_unless(ok: bool, path: &Path, what: &str) -> Result<(), OpenError> {
    if ok {
        Ok(())
    } else {
        Err(io_at(path)(std::io::Error::new(
            ErrorKind::InvalidInput,
            what.to_string(),
        )))
    }
}

/// Open the sidecar `file` and take its exclusive lock: one nonblocking
/// attempt, no truncation, no retry, no unlink. A file this creates is no
/// wider than SQLite's own 0644 (subject to umask); an existing file keeps
/// its mode and its bytes.
fn acquire(path: &Path, file: &Path) -> Result<File, OpenError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o644);
    let held = options.open(file).map_err(io_at(file))?;
    held.try_lock()
        .map_err(|e| lock_error(path, file.to_path_buf(), e))?;
    Ok(held)
}

/// The fixed rendezvous root of the identity guards (decision 0047 §1):
/// never `TMPDIR`, `HOME`, the working directory or the data file's own
/// directory, so every spelling of one file — a hard link in another
/// directory included — meets at the same guard.
#[cfg(unix)]
const IDENTITY_ROOT: &str = "/tmp";

/// The data file's identity, read without opening an existing file
/// (decision 0047 §1): closing any descriptor on the database drops every
/// POSIX record lock this process's SQLite holds on it, so a refused
/// in-process alias must never open and close one. A missing file is
/// created exclusively — no wider than SQLite's 0644, subject to umask —
/// and the created handle is closed before SQLite opens the file.
#[cfg(unix)]
fn identity_of(path: &Path) -> Result<std::fs::Metadata, OpenError> {
    let found = match std::fs::metadata(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => create_identity(path),
        found => found,
    }
    .map_err(io_at(path))?;
    refuse_unless(found.is_file(), path, "the data file is not a regular file")?;
    Ok(found)
}

/// Create the missing data file exclusively and read the created handle's
/// identity; when a competing creator won, read the winner's metadata
/// instead — never opening the file it created.
#[cfg(unix)]
fn create_identity(path: &Path) -> std::io::Result<std::fs::Metadata> {
    match create_new(path, 0o644)? {
        Some(created) => created.metadata(),
        None => std::fs::metadata(path),
    }
}

/// Create `path` exclusively, read/write, no wider than `mode` (subject to
/// umask): the new file, or `None` when an entry already exists there — an
/// exclusive create never follows a symlink and never truncates.
#[cfg(unix)]
fn create_new(path: &Path, mode: u32) -> std::io::Result<Option<File>> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, mode);
    match options.open(path) {
        Ok(created) => Ok(Some(created)),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(None),
        Err(e) => Err(e),
    }
}

/// The identity guard (decision 0047 §1),
/// `<root>/swarmail-db-locks-<owner uid>/<device>-<inode>.lock`, locked
/// exclusively. The database and its `-wal`/`-shm`/`-journal` companions
/// are never locked by us: on Darwin and the BSDs `flock` and SQLite's
/// POSIX record locks share one lock table and conflict, so our own lock
/// would refuse our own SQLite. Every hard link and symlink of a file
/// shares its owner, device and inode, so every spelling names this one
/// guard. The owner directory must be a private (0700) directory of the
/// data file's owner, and the guard a private, singly linked regular file
/// of that owner — checked before it is opened and again on the opened
/// handle, so a planted symlink, hard link or FIFO never redirects it.
///
/// Accepted decision 0047 §1 says:
/// > Validate the opened metadata before locking too.
/// > The verified private, stable parent is the trust boundary, not a promise against hostile same-user replacement.
/// > Namespace replacement during lookup/open is outside the stable-namespace contract.
///
/// This validates the opened metadata; it does not compare pre-open and
/// post-open inode identity. A replacement seam is not justified outside the
/// accepted stable-namespace contract. This documents the defense-in-depth
/// check and claims no new replacement-race proof.
#[cfg(unix)]
fn identity_guard(root: &Path, path: &Path, data: &std::fs::Metadata) -> Result<File, OpenError> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    let owner = data.uid();
    let dir = root.join(format!("swarmail-db-locks-{owner}"));
    match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
        Err(e) if e.kind() != ErrorKind::AlreadyExists => return Err(io_at(&dir)(e)),
        _ => {}
    }
    let meta = std::fs::symlink_metadata(&dir).map_err(io_at(&dir))?;
    refuse_unless(
        private_dir(&meta, owner),
        &dir,
        "the identity-lock directory is not a private (0700) directory of the data file's owner",
    )?;
    let guard = dir.join(format!("{:x}-{:x}.lock", data.dev(), data.ino()));
    let held = match create_new(&guard, 0o600).map_err(io_at(&guard))? {
        Some(created) => created,
        None => {
            check_guard(
                &std::fs::symlink_metadata(&guard).map_err(io_at(&guard))?,
                owner,
                &guard,
            )?;
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&guard)
                .map_err(io_at(&guard))?
        }
    };
    check_guard(&held.metadata().map_err(io_at(&guard))?, owner, &guard)?;
    held.try_lock()
        .map_err(|e| lock_error(path, guard.clone(), e))?;
    Ok(held)
}

/// A real directory of `owner`, owner-only (0700) — not a symlink.
#[cfg(unix)]
fn private_dir(meta: &std::fs::Metadata, owner: u32) -> bool {
    use std::os::unix::fs::MetadataExt;
    meta.is_dir() && meta.uid() == owner && meta.mode() & 0o777 == 0o700
}

/// A guard is a regular, singly linked file of `owner` with no group or
/// other access — not a symlink, a FIFO or a hard link onto anything else.
#[cfg(unix)]
fn check_guard(meta: &std::fs::Metadata, owner: u32, guard: &Path) -> Result<(), OpenError> {
    use std::os::unix::fs::MetadataExt;
    refuse_unless(
        meta.is_file() && meta.nlink() == 1 && meta.uid() == owner && meta.mode() & 0o077 == 0,
        guard,
        "the identity guard is not a private, singly linked file of the data file's owner",
    )
}

/// Take the data file's identity guard under the fixed root.
#[cfg(unix)]
fn acquire_identity(path: &Path) -> Result<File, OpenError> {
    identity_guard(Path::new(IDENTITY_ROOT), path, &identity_of(path)?)
}

/// No identity mechanism is proven here: refuse persistent startup rather
/// than lock SQLite's own file or fall back to pathname-only ownership.
#[cfg(not(unix))]
fn acquire_identity(path: &Path) -> Result<File, OpenError> {
    Err(io_at(path)(std::io::Error::new(
        ErrorKind::Unsupported,
        "data-file identity locking is unsupported on this platform",
    )))
}

impl Persist {
    /// Open (creating if needed) the data file and its schema — after
    /// taking its sidecar lock (decision 0033 §2) and then its identity
    /// guard (decision 0047 §1), so a refused second server, by the same
    /// path or through an alias, never runs SQLite on the database.
    pub fn open(path: &Path) -> Result<Self, OpenError> {
        let lock = acquire(path, &lock_path(path))?;
        let identity = acquire_identity(path)?;
        let conn = Connection::open(path)?;
        // WAL keeps per-mail commits cheap; FULL makes a commit mean "on
        // disk" — a `250` stays the truth even across a hard kill.
        let _mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        conn.execute_batch("PRAGMA synchronous = FULL;")?;
        conn.execute_batch(SCHEMA)?;
        migrate(&conn)?;
        // Seed one *past* the stored maximum (issue #54): `record_insert`
        // consumes the value before incrementing, so seeding `MAX(seq)`
        // itself would reuse the last row's seq on the first insert after
        // every reopen. A failed commit may still consume a sequence — the
        // gap is honest, a duplicate is not.
        let next_seq =
            conn.query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM emails", [], |row| {
                row.get(0)
            })?;
        Ok(Self {
            state: Mutex::new(State { conn, next_seq }),
            _identity: identity,
            _lock: lock,
        })
    }

    /// Every stored email in insertion order, plus the lifetime counters.
    pub fn load(&self) -> rusqlite::Result<(Vec<Email>, u64, u64)> {
        let state = self.state.lock().expect("persist state poisoned");
        let mut stmt = state.conn.prepare(&restore_query())?;
        let emails = stmt
            .query_map([], row_to_email)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let inserted = read_counter(&state.conn, "emails_inserted")?;
        let dropped = read_counter(&state.conn, "emails_dropped")?;
        Ok((emails, inserted, dropped))
    }

    /// Write one accepted email, drop the rows the inbox cap pruned, and
    /// mirror the lifetime counters — one transaction, so a partial insert
    /// can never be the restart copy.
    pub fn record_insert(
        &self,
        email: &Email,
        pruned: &[&str],
        inserted: u64,
        dropped: u64,
    ) -> rusqlite::Result<()> {
        let mut state = self.state.lock().expect("persist state poisoned");
        let seq = state.next_seq;
        state.next_seq += 1;
        let tx = state.conn.transaction()?;
        tx.execute(
            // 20 columns: COLUMNS + the ordering seq.
            &format!(
                "INSERT INTO emails ({COLUMNS}, seq)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, \
                         ?17, ?18, ?19, ?20)"
            ),
            params![
                email.id,
                email.inbox,
                json(&email.from),
                json(&email.to),
                json(&email.cc),
                json(&email.recipients),
                email.subject,
                email.received_at,
                email.received_ms,
                email.size as i64, // message size is capped far below i64::MAX
                email.text,
                email.html,
                json(&email.links),
                json(&email.codes),
                // Optional ids are bound as SQL NULL, not the JSON "null".
                email.message_id,
                email.in_reply_to,
                json(&email.references),
                email.raw,
                // NULL is unknown provenance; '' the known null reverse path.
                email.envelope_from,
                seq,
            ],
        )?;
        for id in pruned {
            tx.execute("DELETE FROM emails WHERE id = ?1", params![id])?;
        }
        write_counter(&tx, "emails_inserted", inserted)?;
        write_counter(&tx, "emails_dropped", dropped)?;
        tx.commit()
    }

    pub fn record_delete(&self, id: &str) -> rusqlite::Result<()> {
        let state = self.state.lock().expect("persist state poisoned");
        state
            .conn
            .execute("DELETE FROM emails WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete exactly these rows, in one transaction — the mirror of a
    /// clear. `clear`/`clear_all` call this with the ids they drained from
    /// memory. Mirroring an inbox-wide `DELETE` instead would erase the row
    /// of any mail inserted between the drain and this delete: that mail was
    /// answered 250 and is queryable in memory, and would vanish on restart
    /// (the clear/insert persistence race, review finding in run
    /// sqlite-persistence-data-file-swa-dc3a9a2a). Deleting exactly the
    /// drained ids cannot touch a post-drain insert, whatever the
    /// interleaving.
    pub fn record_deletes(&self, ids: &[&str]) -> rusqlite::Result<()> {
        let mut state = self.state.lock().expect("persist state poisoned");
        let tx = state.conn.transaction()?;
        for id in ids {
            tx.execute("DELETE FROM emails WHERE id = ?1", params![id])?;
        }
        tx.commit()
    }

    /// Test-only: make every subsequent write fail (read-only connection) so
    /// the error paths are provable without a broken disk.
    #[cfg(test)]
    pub(crate) fn fail_writes(&self) {
        let state = self.state.lock().expect("persist state poisoned");
        state.conn.execute_batch("PRAGMA query_only = 1").unwrap();
    }
}

fn row_to_email(row: &rusqlite::Row<'_>) -> rusqlite::Result<Email> {
    Ok(Email {
        id: row.get(0)?,
        inbox: row.get(1)?,
        from: from_json(row.get(2)?),
        envelope_from: row.get(18)?,
        to: from_json(row.get(3)?),
        cc: from_json(row.get(4)?),
        recipients: from_json(row.get(5)?),
        subject: row.get(6)?,
        received_at: row.get(7)?,
        received_ms: row.get(8)?,
        size: row.get::<_, i64>(9)? as u64,
        text: row.get(10)?,
        html: row.get(11)?,
        links: from_json(row.get(12)?),
        codes: from_json(row.get(13)?),
        message_id: row.get(14)?,
        in_reply_to: row.get(15)?,
        references: from_json(row.get(16)?),
        raw: row.get(17)?,
    })
}

/// Add any `ADDED_COLUMNS` the data file lacks. A fresh file already has them
/// (from `SCHEMA`), so this is a no-op there; an older file keeps every
/// stored mail and gains the columns with their defaults.
fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let mut present: Vec<String> = Vec::new();
    let mut stmt = conn.prepare("PRAGMA table_info(emails)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        present.push(name);
    }
    for (name, kind) in ADDED_COLUMNS {
        if !present.iter().any(|column| column == name) {
            conn.execute_batch(&format!("ALTER TABLE emails ADD COLUMN {name} {kind}"))?;
        }
    }
    Ok(())
}

/// Column ⇄ JSON. These fields are strings and vecs of strings —
/// serialization cannot fail, and a corrupted column degrades to the default
/// instead of failing the whole startup.
fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into())
}

fn from_json<T: DeserializeOwned + Default>(raw: String) -> T {
    serde_json::from_str(&raw).unwrap_or_default()
}

fn write_counter(conn: &Connection, key: &str, value: u64) -> rusqlite::Result<()> {
    conn.execute(UPSERT_COUNTER, params![key, value.to_string()])?;
    Ok(())
}

fn read_counter(conn: &Connection, key: &str) -> rusqlite::Result<u64> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?;
    Ok(match raw {
        Some(text) => text.parse().unwrap_or(0),
        None => 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::email;
    use serde::ser::Error as _;
    use std::path::PathBuf;

    /// The soft-fail contract: a Serialize that refuses degrades to "null"
    /// instead of failing startup (the only way serde_json::to_string fails
    /// here, since every persisted shape is strings and vecs of strings).
    #[test]
    fn json_degrades_to_null_when_serialization_refuses() {
        struct Refuses;
        impl serde::Serialize for Refuses {
            fn serialize<S: serde::Serializer>(&self, _s: S) -> Result<S::Ok, S::Error> {
                Err(S::Error::custom("refused"))
            }
        }
        assert_eq!(json(&Refuses), "null");
    }

    /// A missing counter row reads as zero, and a present one parses —
    /// the fresh-database path (no counters yet) and the populated one.
    #[test]
    fn read_counter_defaults_to_zero_on_a_missing_row() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)", [])
            .unwrap();
        assert_eq!(read_counter(&conn, "absent").unwrap(), 0);
        conn.execute("INSERT INTO meta VALUES ('k', '7')", [])
            .unwrap();
        assert_eq!(read_counter(&conn, "k").unwrap(), 7);
    }

    /// A row whose column type no longer matches the schema (an integer in
    /// a text column — an old writer's damage) surfaces as a load error
    /// instead of a silently wrong email.
    /// A missing meta table refuses with the query error rather than
    /// reading as zero — a zero would silently misreport the counters.
    #[test]
    fn read_counter_refuses_a_missing_meta_table() {
        let conn = Connection::open_in_memory().unwrap();
        let err = read_counter(&conn, "k").unwrap_err();
        assert!(err.to_string().contains("no such table"), "{err}");
    }

    #[test]
    fn load_surfaces_a_corrupt_row_type() {
        let (_guard, path) = db_path("corrupt-row");
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("c1", "corrupt", "r@x.io"), &[], 1, 0)
            .unwrap();

        // A second connection damages the row the way an old buggy writer
        // would: received_ms stops being an integer.
        let vandal = Connection::open(&path).unwrap();
        vandal
            .execute("UPDATE emails SET received_ms = 'not-a-number'", [])
            .unwrap();

        let err = persist.load().unwrap_err();
        assert!(
            err.to_string().contains("Invalid column type"),
            "the type mismatch is the failure: {err}"
        );
    }

    /// A unique scratch directory per test, owned by a drop guard (issue
    /// #101): the database lives *inside* it, so the `-wal`/`-shm` sidecars
    /// die with the directory by construction and cleanup survives a panic
    /// — the old pid+nanos file under `std::env::temp_dir()` leaked on a
    /// failing test and litters a tmpfs. The guard is returned beside the
    /// path and must be held (bound, even as `_guard`) for the test's whole
    /// life; parallel tests never collide because tempfile's name is.
    fn db_path(tag: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::with_prefix(format!("swarmail-persist-{tag}-")).unwrap();
        let path = dir.path().join("persist.db");
        (dir, path)
    }

    #[cfg(unix)]
    fn assert_alias_refuses_in_both_orders(first: &Path, second: &Path) {
        for (owner_path, rival_path) in [(first, second), (second, first)] {
            let owner = Persist::open(owner_path).expect("the first alias owns the data file");
            let err = Persist::open(rival_path)
                .err()
                .expect("the competing spelling refuses while its owner is held");
            let expected = OpenError::InUse(rival_path.to_path_buf());
            assert_eq!(
                std::mem::discriminant(&err),
                std::mem::discriminant(&expected),
                "the rival is refused specifically as InUse: {err:?}"
            );
            assert_eq!(format!("{err:?}"), format!("{expected:?}"));
            assert_eq!(
                err.to_string(),
                format!(
                    "the data file is already in use by another server: {}",
                    rival_path.display()
                )
            );
            drop(owner);
            drop(Persist::open(rival_path).expect("the rival is reusable after its owner drops"));
        }
    }

    fn full_email(id: &str, inbox: &str) -> Email {
        let mut e = email(id, inbox, "rcpt@x.io");
        e.from = Some(crate::model::EmailAddress {
            name: Some("Sender".into()),
            address: "sender@x.io".into(),
        });
        // Deliberately unlike the header From: the envelope round-trips as
        // its own column.
        e.envelope_from = Some("bounce+1@x.io".into());
        e.cc = vec![crate::model::EmailAddress {
            name: None,
            address: "cc@x.io".into(),
        }];
        e.recipients = vec!["rcpt@x.io".into(), "bcc@x.io".into()];
        e.subject = Some("Quarterly report".into());
        e.received_ms = 1234;
        e.size = 42;
        e.text = Some("body".into());
        e.html = Some("<b>body</b>".into());
        e.links = vec!["https://x.io/a".into()];
        e.codes = vec!["424242".into()];
        e.message_id = Some("m@x.io".into());
        e.in_reply_to = Some("parent@x.io".into());
        e.references = vec!["root@x.io".into(), "parent@x.io".into()];
        // Deliberately not valid UTF-8: raw bytes must round-trip untouched.
        e.raw = vec![0xFF, 0xFE, b'a', b'\r', b'\n'];
        e
    }

    #[test]
    fn load_of_a_fresh_file_is_empty_with_zero_counters() {
        let (_guard, path) = db_path("empty");
        let persist = Persist::open(&path).unwrap();
        assert_eq!(persist.load().unwrap(), (Vec::new(), 0, 0));
    }

    #[test]
    fn record_insert_round_trips_every_field() {
        let (_guard, path) = db_path("roundtrip");
        let persist = Persist::open(&path).unwrap();
        let sent = full_email("rt1", "box");
        persist.record_insert(&sent, &[], 1, 0).unwrap();
        let (mut emails, inserted, dropped) = persist.load().unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails.pop().unwrap(), sent);
        assert_eq!((inserted, dropped), (1, 0));
    }

    #[test]
    fn thread_headers_round_trip_in_both_directions() {
        let (_guard, path) = db_path("thread-headers");
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&full_email("with-ids", "box"), &[], 1, 0)
            .unwrap();
        persist
            .record_insert(&email("without-ids", "box", "a@x.io"), &[], 2, 0)
            .unwrap();
        let (emails, _, _) = persist.load().unwrap();
        let with = emails.iter().find(|e| e.id == "with-ids").unwrap();
        assert_eq!(with.message_id.as_deref(), Some("m@x.io"));
        assert_eq!(with.in_reply_to.as_deref(), Some("parent@x.io"));
        assert_eq!(with.references, vec!["root@x.io", "parent@x.io"]);
        let without = emails.iter().find(|e| e.id == "without-ids").unwrap();
        assert_eq!(without.message_id, None, "absent stays absent");
        assert_eq!(without.in_reply_to, None);
        assert!(without.references.is_empty());
    }

    #[test]
    fn a_pre_thread_data_file_migrates_and_keeps_its_mail() {
        let (_guard, path) = db_path("legacy");
        {
            // The first release's shape: no thread columns at all, one mail
            // already committed.
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE emails (id TEXT PRIMARY KEY, inbox TEXT NOT NULL, \
                 seq INTEGER NOT NULL, from_json TEXT NOT NULL, to_json TEXT NOT NULL, \
                 cc_json TEXT NOT NULL, recipients_json TEXT NOT NULL, subject TEXT, \
                 received_at TEXT NOT NULL, received_ms INTEGER NOT NULL, size INTEGER NOT NULL, \
                 text TEXT, html TEXT, links_json TEXT NOT NULL, codes_json TEXT NOT NULL, \
                 raw BLOB NOT NULL);
                 INSERT INTO emails VALUES ('old-1', 'box', 1, 'null', '[]', '[]', '[]', 'Hi', \
                 't', 1, 1, NULL, NULL, '[]', '[]', x'00');",
            )
            .unwrap();
        }
        let persist = Persist::open(&path).unwrap();
        let (emails, _, _) = persist.load().unwrap();
        assert_eq!(
            emails.len(),
            1,
            "the pre-thread mail survives the migration"
        );
        assert_eq!(emails[0].message_id, None);
        assert!(emails[0].references.is_empty());
        assert_eq!(emails[0].envelope_from, None, "no provenance is invented");
        // The migrated file keeps working as a write-through mirror.
        persist
            .record_insert(&email("new-1", "box", "a@x.io"), &[], 2, 0)
            .unwrap();
        let (emails, _, _) = persist.load().unwrap();
        assert_eq!(emails.len(), 2);
        // Reopening the already-migrated file is a no-op — once its one
        // owner has dropped (decision 0033: one server per data file).
        drop(persist);
        assert!(Persist::open(&path).unwrap().load().is_ok());
    }

    /// Decision 0044 §8: a populated data file of the shape before the
    /// envelope sender was kept — the previous explicit schema, not today's
    /// `SCHEMA`, so the migration really runs — gains the nullable column
    /// without loss. Its old rows restore `None` (unknown, never the header
    /// From), new rows keep the known null reverse path `Some("")` apart from
    /// a real sender, and ids, order, counters and raw bytes survive two
    /// reopens of the migrated file.
    #[test]
    fn a_pre_envelope_data_file_migrates_and_keeps_all_three_sender_states() {
        let (_guard, path) = db_path("pre-envelope");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE emails (id TEXT PRIMARY KEY, inbox TEXT NOT NULL, \
                 seq INTEGER NOT NULL, from_json TEXT NOT NULL, to_json TEXT NOT NULL, \
                 cc_json TEXT NOT NULL, recipients_json TEXT NOT NULL, subject TEXT, \
                 received_at TEXT NOT NULL, received_ms INTEGER NOT NULL, size INTEGER NOT NULL, \
                 text TEXT, html TEXT, links_json TEXT NOT NULL, codes_json TEXT NOT NULL, \
                 message_id TEXT, in_reply_to TEXT, \
                 references_json TEXT NOT NULL DEFAULT '[]', raw BLOB NOT NULL);
                 CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO emails VALUES ('old-1', 'box', 1, \
                 '{\"address\":\"alice@x.io\"}', '[]', '[]', '[]', 'Hi', 't', 1, 1, NULL, NULL, \
                 '[]', '[]', 'm1@x.io', NULL, '[]', x'FF00');
                 INSERT INTO meta VALUES ('emails_inserted', '1');
                 INSERT INTO meta VALUES ('emails_dropped', '0');",
            )
            .unwrap();
        }
        {
            let persist = Persist::open(&path).unwrap();
            let (emails, inserted, dropped) = persist.load().unwrap();
            assert_eq!(emails.len(), 1, "the old mail survives the migration");
            assert_eq!(emails[0].envelope_from, None, "unknown, not header From");
            assert_eq!(emails[0].from.as_ref().unwrap().address, "alice@x.io");
            assert_eq!(emails[0].message_id.as_deref(), Some("m1@x.io"));
            assert_eq!(emails[0].raw, vec![0xFF, 0x00]);
            assert_eq!((inserted, dropped), (1, 0));
            let mut bounce = email("bounce", "box", "a@x.io");
            bounce.envelope_from = Some(String::new());
            let mut sent = email("sent", "box", "a@x.io");
            sent.envelope_from = Some("bounce+77@mailer.io".into());
            persist.record_insert(&bounce, &[], 2, 0).unwrap();
            persist.record_insert(&sent, &[], 3, 0).unwrap();
        }
        for _ in 0..2 {
            let persist = Persist::open(&path).unwrap();
            let (emails, inserted, dropped) = persist.load().unwrap();
            let ids: Vec<&str> = emails.iter().map(|e| e.id.as_str()).collect();
            assert_eq!(ids, vec!["old-1", "bounce", "sent"]);
            let senders: Vec<Option<&str>> =
                emails.iter().map(|e| e.envelope_from.as_deref()).collect();
            assert_eq!(senders, vec![None, Some(""), Some("bounce+77@mailer.io")]);
            assert_eq!(emails[0].raw, vec![0xFF, 0x00], "raw bytes untouched");
            assert_eq!((inserted, dropped), (3, 0));
            drop(persist);
        }
    }

    #[test]
    fn insertion_order_and_counters_survive_a_reopen() {
        let (_guard, path) = db_path("order");
        {
            let persist = Persist::open(&path).unwrap();
            for id in ["a", "b"] {
                persist
                    .record_insert(&email(id, "box", "a@x.io"), &[], 2, 0)
                    .unwrap();
            }
            persist
                .record_insert(&email("c", "other", "a@x.io"), &[], 3, 0)
                .unwrap();
        } // dropped: the next open must seed its sequence one past MAX(seq)
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("d", "box", "a@x.io"), &[], 4, 0)
            .unwrap();
        let (emails, inserted, dropped) = persist.load().unwrap();
        let ids: Vec<&str> = emails.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c", "d"]);
        assert_eq!((inserted, dropped), (4, 0));
    }

    /// The seq column read straight out of the mirror, bypassing `load()`'s
    /// mapping — the observable for issue #54's duplicate-sequence bug.
    fn stored_seqs(path: &Path) -> Vec<i64> {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.prepare("SELECT seq FROM emails ORDER BY rowid")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<i64>>>()
            .unwrap()
    }

    /// Seeding one past `MAX(seq)` (issue #54): `record_insert` consumes the
    /// seeded value, so seeding `MAX(seq)` itself made the first insert after
    /// *every* reopen reuse the last row's seq. Three opens, three inserts —
    /// strictly increasing seq values, in write order.
    #[test]
    fn the_first_insert_after_a_reopen_never_reuses_a_seq() {
        let (_guard, path) = db_path("seq-seed");
        {
            let persist = Persist::open(&path).unwrap();
            persist
                .record_insert(&email("a", "box", "a@x.io"), &[], 1, 0)
                .unwrap();
        }
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("b", "box", "a@x.io"), &[], 2, 0)
            .unwrap();
        drop(persist);
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("c", "box", "a@x.io"), &[], 3, 0)
            .unwrap();
        drop(persist);
        let seqs = stored_seqs(&path);
        let mut sorted = seqs.clone();
        sorted.sort_unstable();
        assert_eq!(seqs, sorted, "write order is seq order: {seqs:?}");
        let distinct = seqs.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(distinct.len(), seqs.len(), "no duplicate seq: {seqs:?}");
    }

    /// A file written before the seed fix can carry duplicate seq values;
    /// the restore must not depend on SQLite's tie-break (issue #54): rows
    /// with equal seq come back in write order — the (seq, rowid) index
    /// order — not whichever order a temp sort happens to emit. The fixture's
    /// legal competing index makes dropping rowid from the actual restore
    /// query reverse the tied rows on the locked SQLite build.
    #[test]
    fn a_legacy_duplicate_seq_restores_in_write_order() {
        let (_guard, path) = db_path("dup-seq");
        {
            // The first release's shape, but with two rows sharing a seq.
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE emails (id TEXT PRIMARY KEY, inbox TEXT NOT NULL, \
                 seq INTEGER NOT NULL, from_json TEXT NOT NULL, to_json TEXT NOT NULL, \
                 cc_json TEXT NOT NULL, recipients_json TEXT NOT NULL, subject TEXT, \
                 received_at TEXT NOT NULL, received_ms INTEGER NOT NULL, size INTEGER NOT NULL, \
                 text TEXT, html TEXT, links_json TEXT NOT NULL, codes_json TEXT NOT NULL, \
                 raw BLOB NOT NULL);
                 CREATE INDEX emails_seq ON emails(seq, received_ms DESC);
                 INSERT INTO emails VALUES ('first', 'box', 1, 'null', '[]', '[]', '[]', 'Hi', \
                 't', 1, 1, NULL, NULL, '[]', '[]', x'00');
                 INSERT INTO emails VALUES ('tied-a', 'box', 2, 'null', '[]', '[]', '[]', 'Hi', \
                 't', 2, 2, NULL, NULL, '[]', '[]', x'00');
                 INSERT INTO emails VALUES ('tied-b', 'box', 2, 'null', '[]', '[]', '[]', 'Hi', \
                 't', 3, 3, NULL, NULL, '[]', '[]', x'00');",
            )
            .unwrap();
        }
        let persist = Persist::open(&path).unwrap();
        let (emails, _, _) = persist.load().unwrap();
        let ids: Vec<&str> = emails.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["first", "tied-a", "tied-b"],
            "equal seq values keep write order: {ids:?}"
        );
    }

    /// The restore scan is answered by the seq index, not a temp b-tree over
    /// every raw BLOB (issue #54): the plan of the exact full-column query
    /// `load` runs (issue #87 — not a narrowed `SELECT id`) names
    /// `emails_seq` and, on every plan row, sorts nothing.
    #[test]
    fn the_restore_scan_reads_the_seq_index() {
        let (_guard, path) = db_path("plan");
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("a", "box", "a@x.io"), &[], 1, 0)
            .unwrap();
        let state = persist.state.lock().unwrap();
        let plan: Vec<String> = state
            .conn
            .prepare(&format!("EXPLAIN QUERY PLAN {}", restore_query()))
            .unwrap()
            .query_map([], |row| row.get(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<String>>>()
            .unwrap();
        assert!(
            plan.iter()
                .any(|row| row.contains("USING INDEX emails_seq")),
            "the seq index serves the restore scan: {plan:?}"
        );
        assert!(
            !plan
                .iter()
                .any(|row| row.to_lowercase().contains("temp b-tree")),
            "no temp sort over the blobs: {plan:?}"
        );
    }

    /// The names of the data file's indexes on `emails`, read straight out
    /// of the schema.
    fn email_indexes(path: &Path) -> Vec<String> {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.prepare(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'emails' \
             AND name NOT LIKE 'sqlite_autoindex%' ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<String>>>()
        .unwrap()
    }

    /// The retired `emails_inbox` index (issue #87) is never created on a
    /// fresh file and is dropped from a legacy one, which keeps `emails_seq`,
    /// its rows and its counters; a reopen is idempotent.
    #[test]
    fn open_retires_the_unused_inbox_index_on_fresh_and_legacy_files() {
        let (_fresh_guard, fresh) = db_path("fresh-index");
        drop(Persist::open(&fresh).unwrap());
        assert_eq!(email_indexes(&fresh), vec!["emails_seq".to_string()]);

        let (_guard, path) = db_path("legacy-index");
        {
            let persist = Persist::open(&path).unwrap();
            persist
                .record_insert(&email("a", "box", "a@x.io"), &[], 1, 0)
                .unwrap();
        }
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch("CREATE INDEX emails_inbox ON emails(inbox);")
            .unwrap();
        assert_eq!(
            email_indexes(&path),
            vec!["emails_inbox".to_string(), "emails_seq".to_string()]
        );
        for _ in 0..2 {
            let persist = Persist::open(&path).unwrap();
            let (emails, inserted, dropped) = persist.load().unwrap();
            assert_eq!(emails.len(), 1);
            assert_eq!((inserted, dropped), (1, 0));
            drop(persist);
            assert_eq!(email_indexes(&path), vec!["emails_seq".to_string()]);
        }
    }

    /// Restore follows the seq order, never the timestamp (decision 0038):
    /// rows stamped deliberately in reverse come back in write order.
    #[test]
    fn restore_follows_seq_not_received_ms() {
        let (_guard, path) = db_path("reversed-stamps");
        let persist = Persist::open(&path).unwrap();
        let mut first = email("first", "box", "a@x.io");
        first.received_ms = 2_000;
        let mut second = email("second", "box", "a@x.io");
        second.received_ms = 1_000;
        persist.record_insert(&first, &[], 1, 0).unwrap();
        persist.record_insert(&second, &[], 2, 0).unwrap();
        let (emails, _, _) = persist.load().unwrap();
        let ids: Vec<&str> = emails.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["first", "second"]);
    }

    #[test]
    fn record_insert_deletes_the_pruned_rows() {
        let (_guard, path) = db_path("prune");
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("old", "box", "a@x.io"), &[], 1, 0)
            .unwrap();
        persist
            .record_insert(&email("kept", "box", "a@x.io"), &[], 2, 0)
            .unwrap();
        persist
            .record_insert(&email("new", "box", "a@x.io"), &["old"], 3, 1)
            .unwrap();
        let (emails, inserted, dropped) = persist.load().unwrap();
        let ids: Vec<&str> = emails.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["kept", "new"]);
        assert_eq!((inserted, dropped), (3, 1));
    }

    #[test]
    fn record_delete_clears_exactly_one_row() {
        let (_guard, path) = db_path("delete");
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("d1", "box", "a@x.io"), &[], 1, 0)
            .unwrap();
        persist
            .record_insert(&email("d2", "box", "a@x.io"), &[], 2, 0)
            .unwrap();
        persist.record_delete("d1").unwrap();
        let (emails, _, _) = persist.load().unwrap();
        assert_eq!(
            emails.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["d2"]
        );
    }

    #[test]
    fn record_deletes_clears_exactly_the_named_ids() {
        let (_guard, path) = db_path("clear-ids");
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("c1", "box", "a@x.io"), &[], 1, 0)
            .unwrap();
        persist
            .record_insert(&email("c2", "other", "a@x.io"), &[], 2, 0)
            .unwrap();
        persist
            .record_insert(&email("c3", "box", "a@x.io"), &[], 3, 0)
            .unwrap();
        persist.record_deletes(&["c1", "c3"]).unwrap();
        let (emails, inserted, _) = persist.load().unwrap();
        assert_eq!(
            emails.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["c2"],
            "only the named ids are gone; a row in another inbox or one not named survives"
        );
        assert_eq!(inserted, 3);
    }

    #[test]
    fn record_deletes_on_an_empty_slice_commits_a_noop() {
        let (_guard, path) = db_path("clear-empty");
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("kept", "box", "a@x.io"), &[], 1, 0)
            .unwrap();
        persist.record_deletes(&[]).unwrap();
        let (emails, _, _) = persist.load().unwrap();
        assert_eq!(emails.len(), 1);
    }

    #[test]
    fn writes_fail_loudly_when_the_file_cannot_be_written() {
        let (_guard, path) = db_path("readonly");
        let persist = Persist::open(&path).unwrap();
        persist.fail_writes();
        assert!(
            persist
                .record_insert(&email("x", "box", "a@x.io"), &[], 1, 0)
                .is_err()
        );
        assert!(persist.record_delete("x").is_err());
        assert!(persist.record_deletes(&["x", "y"]).is_err());
    }

    #[test]
    fn open_fails_on_an_unusable_file() {
        // A path whose directory does not exist.
        assert!(Persist::open(&std::env::temp_dir().join("no-such-dir/swarmail/x.db")).is_err());
        // A path that exists but is not a SQLite database.
        let (_garbage_guard, garbage) = db_path("garbage");
        std::fs::write(&garbage, b"definitely not sqlite").unwrap();
        assert!(Persist::open(&garbage).is_err());
    }

    /// The named refusal (decision 0033 §3): a held data file refuses a
    /// second `Persist` — flock is per open file description, so the
    /// same process proves it with no second process and no sleep. The
    /// lock is advisory: a raw SQLite connection beside the owner still
    /// works (the 0001 proof family's vandal and mirror reads). Dropping
    /// the owner frees the file for an immediate reopen, sidecar intact.
    #[test]
    fn a_second_open_of_a_held_data_file_refuses_by_name() {
        let (_guard, path) = db_path("in-use");
        let first = Persist::open(&path).unwrap();
        let raw = Connection::open(&path).unwrap();
        let rows: i64 = raw
            .query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 0, "a raw connection reads beside the owner");

        let err = Persist::open(&path)
            .err()
            .expect("a held data file refuses a second owner");
        assert_eq!(format!("{err:?}"), format!("InUse({path:?})"));
        assert_eq!(
            err.to_string(),
            format!(
                "the data file is already in use by another server: {}",
                path.display()
            )
        );
        assert!(
            std::error::Error::source(&err).is_none(),
            "contention has no cause"
        );

        drop(first);
        let again = Persist::open(&path).expect("a dropped owner frees it");
        assert!(lock_path(&path).is_file(), "the sidecar is never unlinked");
        drop(again);
        drop(raw);
    }

    /// Lock before database (decision 0042 §1): with the sidecar held and
    /// no database yet, the refused open never creates the database — a
    /// refused server cannot run schema or migration work on the file.
    #[test]
    fn the_lock_is_taken_before_the_database_is_touched() {
        let (_guard, path) = db_path("lock-first");
        let holder = File::create(lock_path(&path)).unwrap();
        holder.try_lock().unwrap();
        let err = Persist::open(&path)
            .err()
            .expect("a held sidecar refuses the open");
        assert_eq!(format!("{err:?}"), format!("InUse({path:?})"));
        assert!(!path.exists(), "the refused open created no database");
        drop(holder);
        drop(Persist::open(&path).expect("the released sidecar admits"));
        assert!(path.is_file());
    }

    /// The sidecar is the literal `.lock` suffix on the whole path, so
    /// `mail.db` and `mail.sqlite` are owned at once (an extension
    /// replacement would collide them on `mail.lock`); an existing
    /// unlocked sidecar is reused without truncation and survives drop.
    #[test]
    fn the_sidecar_is_a_literal_suffix_never_truncated_or_removed() {
        let dir = tempfile::TempDir::with_prefix("swarmail-persist-suffix-").unwrap();
        let db = dir.path().join("mail.db");
        let sqlite = dir.path().join("mail.sqlite");
        let db_lock = dir.path().join("mail.db.lock");
        std::fs::write(&db_lock, b"sentinel").unwrap();
        let a = Persist::open(&db).unwrap();
        let b = Persist::open(&sqlite).expect("distinct sidecars");
        assert!(dir.path().join("mail.sqlite.lock").is_file());
        assert!(!dir.path().join("mail.lock").exists());
        drop(a);
        drop(b);
        assert_eq!(
            std::fs::read(&db_lock).unwrap(),
            b"sentinel",
            "the sidecar is opened without truncation and never unlinked"
        );
    }

    /// An unusable sidecar (a directory where the lock file belongs) is an
    /// I/O refusal naming the sidecar and keeping its cause — never
    /// misreported as contention, and never a silent unlocked open.
    #[test]
    fn an_unusable_sidecar_is_an_io_refusal_not_in_use() {
        let (_guard, path) = db_path("sidecar-dir");
        let sidecar = lock_path(&path);
        std::fs::create_dir(&sidecar).unwrap();
        let err = Persist::open(&path)
            .err()
            .expect("an unusable sidecar refuses the open");
        let debug = format!("{err:?}");
        assert!(
            debug.starts_with(&format!("Io {{ path: {sidecar:?}, source: ")),
            "{debug}"
        );
        let cause = std::error::Error::source(&err)
            .expect("the I/O cause is kept")
            .to_string();
        assert_eq!(err.to_string(), format!("{}: {cause}", sidecar.display()));
        assert!(!path.exists(), "the refused open created no database");
    }

    /// Both lock-error classes, explicitly (decision 0042 §Proof): only
    /// `WouldBlock` is contention; a lock syscall failure keeps the
    /// sidecar path and its cause. A real filesystem without lock support
    /// is not fabricated — this pins the classification alone.
    #[test]
    fn lock_errors_classify_contention_apart_from_failure() {
        let data = Path::new("/srv/mail.db");
        let sidecar = PathBuf::from("/srv/mail.db.lock");
        let in_use = lock_error(data, sidecar.clone(), TryLockError::WouldBlock);
        assert_eq!(format!("{in_use:?}"), r#"InUse("/srv/mail.db")"#);
        let failed = lock_error(
            data,
            sidecar,
            TryLockError::Error(std::io::Error::other("locking unsupported")),
        );
        assert!(
            format!("{failed:?}").starts_with(r#"Io { path: "/srv/mail.db.lock", source: "#),
            "{failed:?}"
        );
        assert_eq!(failed.to_string(), "/srv/mail.db.lock: locking unsupported");
        assert_eq!(
            std::error::Error::source(&failed)
                .expect("the lock failure is the cause")
                .to_string(),
            "locking unsupported"
        );
    }

    /// A malformed database is a SQLite refusal with its cause, and the
    /// failed open releases the sidecar at once — the lock never outlives
    /// a server that did not start.
    #[test]
    fn a_failed_sqlite_open_is_named_and_releases_the_lock() {
        let (_guard, garbage) = db_path("garbage-release");
        std::fs::write(&garbage, b"definitely not sqlite").unwrap();
        let err = Persist::open(&garbage)
            .err()
            .expect("a malformed database refuses the open");
        assert!(format!("{err:?}").starts_with("Sqlite("), "{err:?}");
        assert!(err.to_string().contains("not a database"), "{err}");
        assert_eq!(
            std::error::Error::source(&err)
                .expect("the SQLite cause is kept")
                .to_string(),
            err.to_string()
        );
        let probe = File::open(lock_path(&garbage)).unwrap();
        probe
            .try_lock()
            .expect("the failed open released the sidecar");
    }

    /// A dangling data symlink is an I/O refusal, not a missing ordinary
    /// path: identity lookup follows it, exclusive creation sees its entry,
    /// and the final lookup reports NotFound without creating its target.
    #[cfg(unix)]
    #[test]
    fn a_dangling_data_symlink_refuses_with_enoent_without_creating_target() {
        let (_guard, path) = db_path("dangling-data-link");
        let target_parent = path.parent().unwrap().join("target-parent");
        std::fs::create_dir(&target_parent).unwrap();
        let target = target_parent.join("absent.db");
        std::os::unix::fs::symlink(&target, &path).unwrap();

        let err = Persist::open(&path)
            .err()
            .expect("a dangling data symlink refuses startup");
        let expected = OpenError::Io {
            path: path.clone(),
            source: std::io::Error::from(ErrorKind::NotFound),
        };
        assert_eq!(
            std::mem::discriminant(&err),
            std::mem::discriminant(&expected),
            "the refusal is specifically OpenError::Io: {err:?}"
        );
        assert!(
            format!("{err:?}").starts_with(&format!("Io {{ path: {path:?}, source: ")),
            "the refusal names the supplied data path: {err:?}"
        );
        assert_eq!(
            std::error::Error::source(&err)
                .and_then(|source| source.downcast_ref::<std::io::Error>())
                .map(std::io::Error::kind),
            Some(ErrorKind::NotFound),
            "the source keeps NotFound"
        );
        assert_eq!(
            std::error::Error::source(&err)
                .and_then(|source| source.downcast_ref::<std::io::Error>())
                .and_then(std::io::Error::raw_os_error),
            Some(libc::ENOENT),
            "the source keeps the platform ENOENT"
        );
        assert!(
            std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_link(&path).unwrap(), target);
        assert!(!target.exists());
    }

    /// A relative spelling creates the missing file and keeps the supplied
    /// spelling for a rival refusal; the absolute spelling meets the same
    /// held owner without changing process cwd.
    #[cfg(unix)]
    #[test]
    fn relative_data_path_refuses_its_absolute_alias_in_both_orders() {
        let cwd = std::env::current_dir().unwrap();
        let dir = tempfile::tempdir_in(&cwd).unwrap();
        let absolute = dir.path().join("mail.db");
        let relative = absolute.strip_prefix(&cwd).unwrap().to_path_buf();
        assert!(relative.is_relative());
        drop(Persist::open(&relative).expect("a missing relative data path is admitted"));
        assert!(absolute.is_file());

        assert_alias_refuses_in_both_orders(&relative, &absolute);
    }

    /// A path through a symlinked parent creates the missing file and meets
    /// the real-parent spelling under one owner, in either acquisition order.
    #[cfg(unix)]
    #[test]
    fn symlinked_parent_refuses_its_real_path_in_both_orders() {
        let cwd = std::env::current_dir().unwrap();
        let dir = tempfile::tempdir_in(&cwd).unwrap();
        let real_parent = dir.path().join("real-parent");
        std::fs::create_dir(&real_parent).unwrap();
        let linked_parent = dir.path().join("linked-parent");
        std::os::unix::fs::symlink(&real_parent, &linked_parent).unwrap();
        let real = real_parent.join("mail.db");
        let linked = linked_parent.join("mail.db");

        drop(
            Persist::open(&linked)
                .expect("a missing database through a symlinked parent is admitted"),
        );
        assert!(real.is_file());
        assert_alias_refuses_in_both_orders(&linked, &real);
    }

    /// The refusal follows the file, not its name (decision 0047 §1, issue
    /// #129; red before: both aliases opened beside the owner): a symlink,
    /// a hard link and a hard link in another directory each get their own
    /// sidecar, so only the identity guard can refuse them — in both
    /// acquisition orders, each refusal named by the path the refused open
    /// was given. A first owner is admitted with the links already present.
    #[cfg(unix)]
    #[test]
    fn data_file_aliases_refuse_in_both_orders() {
        let dir = tempfile::TempDir::with_prefix("swarmail-persist-alias-").unwrap();
        let elsewhere = tempfile::TempDir::with_prefix("swarmail-persist-elsewhere-").unwrap();
        let real = dir.path().join("mail.db");
        let soft = dir.path().join("soft.db");
        let hard = dir.path().join("hard.db");
        let far = elsewhere.path().join("far.db");
        drop(Persist::open(&real).unwrap());
        std::os::unix::fs::symlink(&real, &soft).unwrap();
        std::fs::hard_link(&real, &hard).unwrap();
        std::fs::hard_link(&real, &far).unwrap();
        for alias in [&soft, &hard, &far] {
            for (first, second) in [(&real, alias), (alias, &real)] {
                let owner = Persist::open(first).unwrap();
                let err = Persist::open(second)
                    .err()
                    .expect("an alias of a held data file refuses");
                assert_eq!(format!("{err:?}"), format!("InUse({second:?})"));
                assert_eq!(
                    err.to_string(),
                    format!(
                        "the data file is already in use by another server: {}",
                        second.display()
                    )
                );
                assert!(lock_path(second).is_file(), "the sidecar alone admitted it");
                drop(owner);
                drop(Persist::open(second).expect("the dropped owner frees every alias"));
            }
        }
    }

    /// The production identity guard of `path`, taken under the fixed root.
    #[cfg(unix)]
    fn hold_identity(path: &Path) -> Result<File, OpenError> {
        identity_guard(
            Path::new(IDENTITY_ROOT),
            path,
            &std::fs::metadata(path).unwrap(),
        )
    }

    /// The identity guard comes before SQLite (decision 0047 §1): with
    /// only the guard held, the refused open leaves the file empty — no
    /// schema, no `-wal` — and releases its sidecar; the failed SQLite open
    /// of a malformed file releases the identity guard with the sidecar.
    #[cfg(unix)]
    #[test]
    fn identity_guards_release_after_drop_and_failed_open() {
        let (_guard, path) = db_path("identity-first");
        File::create(&path).unwrap();
        let holder = hold_identity(&path).unwrap();
        let err = Persist::open(&path)
            .err()
            .expect("a held identity guard refuses the open");
        assert_eq!(format!("{err:?}"), format!("InUse({path:?})"));
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0, "no schema");
        let mut wal = path.clone().into_os_string();
        wal.push("-wal");
        assert!(!Path::new(&wal).exists(), "no SQLite work ran");
        File::open(lock_path(&path))
            .unwrap()
            .try_lock()
            .expect("the refused open released its sidecar");
        drop(holder);
        drop(Persist::open(&path).expect("the released guard admits"));

        let (_garbage_guard, garbage) = db_path("identity-garbage");
        std::fs::write(&garbage, b"definitely not sqlite").unwrap();
        assert!(Persist::open(&garbage).is_err());
        drop(hold_identity(&garbage).expect("the failed open released the identity guard"));
    }

    /// The macOS self-lock regression (decision 0047 §1, #129 fix-up; red
    /// on `794e919`, whose `flock` on the database itself refused this
    /// probe): while a server owns a live database, no lock of ours sits on
    /// the database or on SQLite's `-wal`/`-shm` — so on Darwin, where
    /// `flock` and SQLite's record locks share one table, our guard cannot
    /// refuse our own SQLite. Linux only: there SQLite's record locks never
    /// meet `flock`, so a refused probe can only be ours; on Darwin SQLite's
    /// legitimate locks would refuse it. WAL mode keeps no `-journal`.
    #[cfg(target_os = "linux")]
    #[test]
    fn server_guards_do_not_lock_sqlite_files() {
        let (_guard, path) = db_path("no-self-lock");
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("x", "box", "a@x.io"), &[], 1, 0)
            .unwrap();
        let companion = |suffix: &str| {
            let mut name = path.clone().into_os_string();
            name.push(suffix);
            PathBuf::from(name)
        };
        for file in [path.clone(), companion("-wal"), companion("-shm")] {
            let probe = File::open(&file).expect("the SQLite file exists");
            let taken = probe.try_lock();
            assert!(taken.is_ok(), "{file:?} is locked by the server: {taken:?}");
        }
        assert!(!companion("-journal").exists(), "WAL mode keeps no journal");
        drop(persist);
    }

    /// A missing data file's creation race (decision 0047 §1): when a
    /// competing creator wins the exclusive create, the winner's identity
    /// is read from metadata — its bytes and mode untouched, no truncation.
    #[cfg(unix)]
    #[test]
    fn identity_creation_race_reads_the_winners_metadata() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let (_guard, path) = db_path("creation-race");
        std::fs::write(&path, b"winner").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let found = create_identity(&path).expect("the winner's metadata");
        assert_eq!(found.ino(), std::fs::metadata(&path).unwrap().ino());
        assert_eq!(std::fs::read(&path).unwrap(), b"winner");
        assert_eq!(found.mode() & 0o777, 0o600);

        let (_fresh_guard, fresh) = db_path("creation-fresh");
        let created = create_identity(&fresh).expect("an exclusive create");
        assert!(created.is_file());
        assert_eq!(created.len(), 0);

        // Any failure but an existing entry is the creator's own error —
        // unreachable through `Persist::open`, whose sidecar fails first.
        let unreachable = fresh.with_extension("absent").join("mail.db");
        assert_eq!(
            create_identity(&unreachable)
                .expect_err("a missing directory refuses")
                .kind(),
            ErrorKind::NotFound
        );
    }

    /// A data path whose lookup fails for any reason but absence (a
    /// symlink loop, whose sidecar still opens) is an I/O refusal naming
    /// the supplied path with its cause — nothing is created through it.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_identity_is_an_io_refusal() {
        let (_guard, path) = db_path("loop");
        std::os::unix::fs::symlink(&path, &path).unwrap();
        let err = Persist::open(&path)
            .err()
            .expect("an unreadable identity refuses the open");
        assert_eq!(
            std::error::Error::source(&err)
                .and_then(|e| e.downcast_ref::<std::io::Error>())
                .and_then(std::io::Error::raw_os_error),
            Some(libc::ELOOP)
        );
        assert!(
            format!("{err:?}").starts_with(&format!("Io {{ path: {path:?}, source: ")),
            "{err:?}"
        );
    }

    /// The rendezvous (decision 0047 §1): the owner directory is created
    /// private (0700), the guard private (0600) and keyed by device and
    /// inode alone — so a symlink and a hard link in another directory
    /// meet the held guard and are refused by their own names, a distinct
    /// file is independent, and a released guard and directory are reused.
    #[cfg(unix)]
    #[test]
    fn identity_guards_meet_by_file_identity_under_a_private_root() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let root = tempfile::TempDir::with_prefix("swarmail-identity-root-").unwrap();
        let (_a, data) = db_path("rendezvous");
        let (_b, other) = db_path("rendezvous-other");
        let (_c, far_dir) = db_path("rendezvous-far");
        std::fs::write(&data, b"").unwrap();
        std::fs::write(&other, b"").unwrap();
        std::fs::create_dir(&far_dir).unwrap();
        let far = far_dir.join("far.db");
        std::fs::hard_link(&data, &far).unwrap();
        let soft = other.with_extension("soft");
        std::os::unix::fs::symlink(&data, &soft).unwrap();
        let meta = |p: &Path| std::fs::metadata(p).unwrap();
        let held = identity_guard(root.path(), &data, &meta(&data)).unwrap();

        let dir = root
            .path()
            .join(format!("swarmail-db-locks-{}", meta(&data).uid()));
        assert_eq!(std::fs::metadata(&dir).unwrap().mode() & 0o777, 0o700);
        let guard = dir.join(format!(
            "{:x}-{:x}.lock",
            meta(&data).dev(),
            meta(&data).ino()
        ));
        assert_eq!(
            std::fs::metadata(&guard).unwrap().permissions().mode() & 0o777,
            0o600
        );

        for alias in [&far, &soft] {
            let err = identity_guard(root.path(), alias, &meta(alias))
                .expect_err("an alias meets the held guard");
            assert_eq!(format!("{err:?}"), format!("InUse({alias:?})"));
        }
        let independent = identity_guard(root.path(), &other, &meta(&other))
            .expect("a distinct file has its own guard");
        drop(held);
        drop(identity_guard(root.path(), &far, &meta(&far)).expect("a released guard is reused"));
        drop(independent);
    }

    /// The owner directory fails closed (decision 0047 §1): a missing root,
    /// a regular file, a symlink to a private directory, a directory open
    /// to others and one of another owner are each an I/O refusal naming
    /// the directory with its cause — never repaired, never replaced.
    #[cfg(unix)]
    #[test]
    fn an_unsafe_identity_directory_is_an_io_refusal() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let (_g, data) = db_path("unsafe-dir");
        std::fs::write(&data, b"").unwrap();
        let meta = std::fs::metadata(&data).unwrap();
        let name = format!("swarmail-db-locks-{}", meta.uid());
        let refused = |root: &Path| {
            let dir = root.join(&name);
            let err = identity_guard(root, &data, &meta).expect_err("an unsafe directory refuses");
            assert!(
                format!("{err:?}").starts_with(&format!("Io {{ path: {dir:?}, source: ")),
                "{err:?}"
            );
            let cause = std::error::Error::source(&err).unwrap().to_string();
            assert_eq!(err.to_string(), format!("{}: {cause}", dir.display()));
        };

        let missing = tempfile::TempDir::with_prefix("swarmail-identity-missing-").unwrap();
        refused(&missing.path().join("absent"));

        let file = tempfile::TempDir::with_prefix("swarmail-identity-file-").unwrap();
        std::fs::write(file.path().join(&name), b"").unwrap();
        refused(file.path());

        let linked = tempfile::TempDir::with_prefix("swarmail-identity-link-").unwrap();
        let real = linked.path().join("real");
        std::fs::create_dir(&real).unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink(&real, linked.path().join(&name)).unwrap();
        refused(linked.path());

        let open = tempfile::TempDir::with_prefix("swarmail-identity-open-").unwrap();
        std::fs::create_dir(open.path().join(&name)).unwrap();
        std::fs::set_permissions(
            open.path().join(&name),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        refused(open.path());
        assert_eq!(
            std::fs::metadata(open.path().join(&name)).unwrap().mode() & 0o777,
            0o755,
            "an unsafe directory is never repaired"
        );

        let private = std::fs::metadata(&real).unwrap();
        assert!(private_dir(&private, private.uid()));
        assert!(
            !private_dir(&private, private.uid() + 1),
            "another owner's directory"
        );
    }

    /// The guard entry fails closed before it is opened (decision 0047
    /// §1): a symlink (planted onto the database itself), a directory, a
    /// hard-linked guard and one open to others are each an I/O refusal
    /// naming the guard; the planted symlink never lets us lock the
    /// database. One of another owner is refused too.
    #[cfg(unix)]
    #[test]
    fn an_unsafe_identity_guard_is_an_io_refusal() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let (_g, data) = db_path("unsafe-guard");
        std::fs::write(&data, b"").unwrap();
        let meta = std::fs::metadata(&data).unwrap();
        let name = format!("swarmail-db-locks-{}", meta.uid());
        let entry = format!("{:x}-{:x}.lock", meta.dev(), meta.ino());
        let planted = |plant: &dyn Fn(&Path)| {
            let root = tempfile::TempDir::with_prefix("swarmail-identity-guard-").unwrap();
            let dir = root.path().join(&name);
            std::fs::create_dir(&dir).unwrap();
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
            let guard = dir.join(&entry);
            plant(&guard);
            let err =
                identity_guard(root.path(), &data, &meta).expect_err("an unsafe guard refuses");
            assert!(
                format!("{err:?}").starts_with(&format!("Io {{ path: {guard:?}, source: ")),
                "{err:?}"
            );
        };
        planted(&|guard| std::os::unix::fs::symlink(&data, guard).unwrap());
        File::open(&data)
            .unwrap()
            .try_lock()
            .expect("the planted symlink never locked the database");
        planted(&|guard| std::fs::create_dir(guard).unwrap());
        planted(&|guard| {
            std::fs::write(guard, b"").unwrap();
            std::fs::set_permissions(guard, std::fs::Permissions::from_mode(0o600)).unwrap();
            std::fs::hard_link(guard, guard.with_extension("twin")).unwrap();
        });
        planted(&|guard| {
            std::fs::write(guard, b"").unwrap();
            std::fs::set_permissions(guard, std::fs::Permissions::from_mode(0o640)).unwrap();
        });

        let guard = data.with_extension("guard");
        std::fs::write(&guard, b"").unwrap();
        std::fs::set_permissions(&guard, std::fs::Permissions::from_mode(0o600)).unwrap();
        let own = std::fs::metadata(&guard).unwrap();
        assert!(check_guard(&own, own.uid(), &guard).is_ok());
        assert!(
            check_guard(&own, own.uid() + 1, &guard).is_err(),
            "another owner's guard"
        );
    }

    /// The data file is never opened to be identified (decision 0047 §1):
    /// an existing file's bytes and mode are untouched; a data path that is
    /// not a regular file is an I/O refusal naming it with its cause, not
    /// a SQLite error.
    #[cfg(unix)]
    #[test]
    fn identity_open_preserves_contents_and_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let (_guard, path) = db_path("identity-mode");
        drop(Persist::open(&path).unwrap());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let before = std::fs::read(&path).unwrap();
        drop(Persist::open(&path).unwrap());
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600,
            "an existing mode is kept"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before, "nothing truncated");

        let (_dir_guard, dir_path) = db_path("identity-dir");
        std::fs::create_dir(&dir_path).unwrap();
        let err = Persist::open(&dir_path)
            .err()
            .expect("a directory is no data file");
        let debug = format!("{err:?}");
        assert!(
            debug.starts_with(&format!("Io {{ path: {dir_path:?}, source: ")),
            "{debug}"
        );
        let cause = std::error::Error::source(&err)
            .expect("the I/O cause is kept")
            .to_string();
        assert_eq!(err.to_string(), format!("{}: {cause}", dir_path.display()));
    }

    /// Our guards lock no SQLite file (decision 0047 §1): a raw connection
    /// beside the owner still takes `BEGIN IMMEDIATE` and commits, the
    /// owner then commits its own write, and the owner's busy bound is the
    /// dependency's. Green on Linux before the macOS repair too — the
    /// Linux red is `server_guards_do_not_lock_sqlite_files`.
    #[test]
    fn identity_lock_preserves_raw_sqlite_and_busy_bound() {
        let (_guard, path) = db_path("identity-raw");
        let persist = Persist::open(&path).unwrap();
        let raw = Connection::open(&path).unwrap();
        raw.execute_batch(
            "BEGIN IMMEDIATE; INSERT INTO meta (key, value) VALUES ('raw', '1'); COMMIT;",
        )
        .expect("a raw writer commits beside the owner");
        persist
            .record_insert(&email("x", "box", "a@x.io"), &[], 1, 0)
            .expect("the owner commits after the raw writer");
        let rows: i64 = raw
            .query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);
        assert_dependency_busy_default(&persist.state.lock().expect("persist state poisoned").conn);
        drop(raw);
    }

    /// The busy bound is a fact of the dependency, not of this crate:
    /// rusqlite sets `sqlite3_busy_timeout(db, 5000)` at open (0.32.1
    /// `src/inner_connection.rs:119`, the line 0012 §Why cites) and nothing
    /// in `src/` overrides it. 0015 §2 makes the ~5 s bound part of the
    /// refusal contract; 0016 §4 pins it with this tripwire. An override
    /// that reaches the server's own connection — or a dependency bump that
    /// moves the default — trips it, and firing it is the decision-document
    /// obligation 0015 §2 already imposes. A fact, never a wall-clock
    /// assertion (0020 rule 5).
    fn assert_dependency_busy_default(conn: &Connection) {
        let timeout: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .expect("busy_timeout is readable on any live connection");
        assert_eq!(
            timeout, 5000,
            "the busy bound moved: PRAGMA busy_timeout is {timeout}, not the dependency \
             default 5000 — an override reached the server's own connection (0015 §2, 0016 §4; \
             rusqlite 0.32.1 inner_connection.rs:119)"
        );
    }

    /// The green leg: the connection `Persist::open` created — the server's
    /// own, the one every refusal contract delay is measured against — keeps
    /// the dependency's default.
    #[test]
    fn the_opened_connection_keeps_the_dependency_busy_default() {
        let (_guard, path) = db_path("busy-default");
        let persist = Persist::open(&path).unwrap();
        let state = persist.state.lock().expect("persist state poisoned");
        assert_dependency_busy_default(&state.conn);
        drop(state);
    }

    /// The can-fire leg: an override on the opened connection trips the
    /// tripwire — the instrument can fire, so its green result means
    /// something. The assertion is the last statement: the panic unwinds
    /// through the drop guard, which is exactly the panic-safe cleanup the
    /// guard exists to provide (issue #101).
    #[test]
    #[should_panic(expected = "the busy bound moved")]
    fn the_busy_default_tripwire_fires_when_the_bound_is_overridden() {
        let (_guard, path) = db_path("tripwire");
        let persist = Persist::open(&path).unwrap();
        {
            let state = persist.state.lock().expect("persist state poisoned");
            state
                .conn
                .execute_batch("PRAGMA busy_timeout = 30000")
                .expect("the override applies on a live connection");
        }
        assert_dependency_busy_default(&persist.state.lock().expect("persist state poisoned").conn);
    }
}
