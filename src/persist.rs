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
//! transaction, never across an `await`, so it cannot stall the runtime.

use crate::model::Email;
use rusqlite::{Connection, OptionalExtension, params};
use serde::de::DeserializeOwned;
use std::path::Path;
use std::sync::Mutex;

/// Every column of a mail row, in the order both statements use it.
const COLUMNS: &str = "id, inbox, from_json, to_json, cc_json, recipients_json, subject, \
                       received_at, received_ms, size, text, html, links_json, codes_json, \
                       message_id, in_reply_to, references_json, raw";

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
    raw BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS emails_inbox ON emails(inbox);
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";

/// Columns the first release lacked, with their types: a data file written
/// before threading is migrated on open instead of abandoned.
const ADDED_COLUMNS: &[(&str, &str)] = &[
    ("message_id", "TEXT"),
    ("in_reply_to", "TEXT"),
    ("references_json", "TEXT NOT NULL DEFAULT '[]'"),
];

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
}

impl Persist {
    /// Open (creating if needed) the data file and its schema.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        // WAL keeps per-mail commits cheap; FULL makes a commit mean "on
        // disk" — a `250` stays the truth even across a hard kill.
        let _mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        conn.execute_batch("PRAGMA synchronous = FULL;")?;
        conn.execute_batch(SCHEMA)?;
        migrate(&conn)?;
        let next_seq = conn.query_row("SELECT COALESCE(MAX(seq), 0) FROM emails", [], |row| {
            row.get(0)
        })?;
        Ok(Self {
            state: Mutex::new(State { conn, next_seq }),
        })
    }

    /// Every stored email in insertion order, plus the lifetime counters.
    pub fn load(&self) -> rusqlite::Result<(Vec<Email>, u64, u64)> {
        let state = self.state.lock().expect("persist state poisoned");
        let mut stmt = state
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM emails ORDER BY seq"))?;
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
            // 19 columns: COLUMNS + the ordering seq.
            &format!(
                "INSERT INTO emails ({COLUMNS}, seq)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, \
                         ?17, ?18, ?19)"
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
        let path = db_path("corrupt-row");
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
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    /// A unique data-file path per test; parallel tests must not collide.
    fn db_path(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "swarmail-persist-{tag}-{}-{nanos}.db",
            std::process::id()
        ))
    }

    fn full_email(id: &str, inbox: &str) -> Email {
        let mut e = email(id, inbox, "rcpt@x.io");
        e.from = Some(crate::model::EmailAddress {
            name: Some("Sender".into()),
            address: "sender@x.io".into(),
        });
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
        let persist = Persist::open(&db_path("empty")).unwrap();
        assert_eq!(persist.load().unwrap(), (Vec::new(), 0, 0));
    }

    #[test]
    fn record_insert_round_trips_every_field() {
        let persist = Persist::open(&db_path("roundtrip")).unwrap();
        let sent = full_email("rt1", "box");
        persist.record_insert(&sent, &[], 1, 0).unwrap();
        let (mut emails, inserted, dropped) = persist.load().unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails.pop().unwrap(), sent);
        assert_eq!((inserted, dropped), (1, 0));
    }

    #[test]
    fn thread_headers_round_trip_in_both_directions() {
        let persist = Persist::open(&db_path("thread-headers")).unwrap();
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
        let path = db_path("legacy");
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
        // The migrated file keeps working as a write-through mirror.
        persist
            .record_insert(&email("new-1", "box", "a@x.io"), &[], 2, 0)
            .unwrap();
        let (emails, _, _) = persist.load().unwrap();
        assert_eq!(emails.len(), 2);
        // Reopening the already-migrated file is a no-op.
        assert!(Persist::open(&path).unwrap().load().is_ok());
    }

    #[test]
    fn insertion_order_and_counters_survive_a_reopen() {
        let path = db_path("order");
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
        } // dropped: the next open must seed its sequence from MAX(seq)
        let persist = Persist::open(&path).unwrap();
        persist
            .record_insert(&email("d", "box", "a@x.io"), &[], 4, 0)
            .unwrap();
        let (emails, inserted, dropped) = persist.load().unwrap();
        let ids: Vec<&str> = emails.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c", "d"]);
        assert_eq!((inserted, dropped), (4, 0));
    }

    #[test]
    fn record_insert_deletes_the_pruned_rows() {
        let persist = Persist::open(&db_path("prune")).unwrap();
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
        let persist = Persist::open(&db_path("delete")).unwrap();
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
        let persist = Persist::open(&db_path("clear-ids")).unwrap();
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
        let persist = Persist::open(&db_path("clear-empty")).unwrap();
        persist
            .record_insert(&email("kept", "box", "a@x.io"), &[], 1, 0)
            .unwrap();
        persist.record_deletes(&[]).unwrap();
        let (emails, _, _) = persist.load().unwrap();
        assert_eq!(emails.len(), 1);
    }

    #[test]
    fn writes_fail_loudly_when_the_file_cannot_be_written() {
        let persist = Persist::open(&db_path("readonly")).unwrap();
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
        let garbage = db_path("garbage");
        std::fs::write(&garbage, b"definitely not sqlite").unwrap();
        assert!(Persist::open(&garbage).is_err());
    }
}
