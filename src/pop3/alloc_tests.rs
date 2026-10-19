//! RETR's one copy and allocation-free sizing, counted (issue #143, the
//! guarantee of issue #86; decision 0020 rule 2).
//!
//! A counting global allocator, installed only in the library's test build,
//! forwards every call to `System` and — only while a test has armed it on
//! its own thread — tallies requests, requested bytes, live bytes and their
//! high-water mark in thread-local, nonallocating cells. Parallel tests
//! never see each other's allocations: the tally belongs to the executing
//! thread, and the scope's Drop guard disarms it on every exit, panic
//! included. Fixtures are built before the scope opens and checked after it
//! closes, so setup is never counted.
//!
//! Its cold paths — a second body-sized buffer, a per-mail allocation in
//! sizing, an allocation that never reached the counter — run only while
//! the bug is present, so the proofs cannot live inline.

use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use tokio::io::AsyncReadExt;

/// What one armed scope asked of the allocator on its own thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tally {
    /// `alloc`, `alloc_zeroed` and `realloc` calls: each asks for a buffer.
    requests: usize,
    /// Bytes asked for over all of them (a `realloc` counts its new size).
    requested: usize,
    /// The largest single request.
    largest: usize,
    /// Bytes allocated minus bytes freed since arming (a `realloc` counts
    /// its delta: the logical buffer is replaced, not duplicated).
    live: isize,
    /// The high-water mark of `live`.
    peak: isize,
}

const ZERO: Tally = Tally {
    requests: 0,
    requested: 0,
    largest: 0,
    live: 0,
    peak: 0,
};

thread_local! {
    // Const-initialized `Copy` cells: no lazy registration, no destructor,
    // so reading them from inside the allocator never allocates.
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static TALLY: Cell<Tally> = const { Cell::new(ZERO) };
}

/// Count one request on this thread, if it is armed: `freed` bytes leave
/// the live total (a `realloc`'s old size) and `size` bytes join it.
fn on_request(freed: usize, size: usize) {
    let _ = ARMED.try_with(|armed| {
        if armed.get() {
            TALLY.with(|cell| {
                let mut tally = cell.get();
                tally.requests += 1;
                tally.requested += size;
                tally.largest = tally.largest.max(size);
                tally.live += size as isize - freed as isize;
                tally.peak = tally.peak.max(tally.live);
                cell.set(tally);
            });
        }
    });
}

/// Count one free on this thread, if it is armed.
fn on_free(size: usize) {
    let _ = ARMED.try_with(|armed| {
        if armed.get() {
            TALLY.with(|cell| {
                let mut tally = cell.get();
                tally.live -= size as isize;
                cell.set(tally);
            });
        }
    });
}

/// The library test binary's one global allocator: `System`, observed.
struct Counting;

// SAFETY: every call forwards to `System` with its own arguments; the
// bookkeeping beside it touches only const-initialized thread-local cells
// and never allocates, so it cannot recurse into the allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        on_request(0, layout.size());
        // SAFETY: the caller's contract for `alloc`, passed through.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        on_request(0, layout.size());
        // SAFETY: the caller's contract for `alloc_zeroed`, passed through.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        on_free(layout.size());
        // SAFETY: the caller's contract for `dealloc`, passed through.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        on_request(layout.size(), new_size);
        // SAFETY: the caller's contract for `realloc`, passed through.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// The scope's guard: dropping it disarms this thread's counter, so a
/// panicking scope never leaves it counting.
struct Armed;

impl Drop for Armed {
    fn drop(&mut self) {
        ARMED.with(|armed| armed.set(false));
    }
}

/// Run `work` with this thread's counter armed from a zero tally; returns
/// its value and what it asked of the allocator. The value is dropped by
/// the caller, after the scope — its own free is never counted.
fn observe<R>(work: impl FnOnce() -> R) -> (R, Tally) {
    TALLY.with(|cell| cell.set(ZERO));
    ARMED.with(|armed| armed.set(true));
    let guard = Armed;
    let value = work();
    let tally = TALLY.with(Cell::get);
    drop(guard);
    (value, tally)
}

/// A mail that exercises every stuffing rule: CRLF headers, then `reps`
/// pairs of a dot-led line with a bare LF (stuffed and normalized) and a
/// plain CRLF line. Stuffed, each pair grows from 11 to 13 bytes.
fn fixture(reps: usize) -> Vec<u8> {
    let mut raw = b"Subject: one copy\r\n\r\n".to_vec();
    raw.extend_from_slice(&b".dot\nline\r\n".repeat(reps));
    raw
}

/// The counter is real and per-thread: an allocation in the scope is seen
/// at its exact size, and a megabyte another thread allocates *inside* the
/// same scope — ordered by barriers, not timing — is not. Without this
/// control a zero tally could mean an allocator that was never installed.
#[test]
fn the_counter_sees_this_threads_allocations_and_no_other() {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other = {
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            let big = std::hint::black_box(vec![7u8; 1 << 20]);
            barrier.wait();
            big.len()
        })
    };
    let (mine, tally) = observe(|| {
        let mine = std::hint::black_box(Vec::<u8>::with_capacity(1000));
        barrier.wait(); // the other thread allocates now
        barrier.wait(); // and has finished
        mine
    });
    assert_eq!(mine.capacity(), 1000);
    assert_eq!(other.join().unwrap(), 1 << 20);
    assert_eq!(
        tally,
        Tally {
            requests: 1,
            requested: 1000,
            largest: 1000,
            live: 1000,
            peak: 1000,
        },
        "only this thread's one allocation is counted"
    );
    // Disarmed after the scope: nothing more is counted.
    let after = std::hint::black_box(vec![0u8; 64]);
    assert_eq!(TALLY.with(Cell::get), tally, "{after:?}");
}

/// A panicking scope disarms on unwind: the guard's Drop runs, and an
/// allocation after the caught panic is not counted.
#[test]
fn a_panicking_scope_is_disarmed_on_unwind() {
    let caught = std::panic::catch_unwind(|| {
        observe(|| {
            let _held = std::hint::black_box(vec![1u8; 10]);
            panic!("mid-scope");
        })
    });
    assert!(caught.is_err());
    assert!(!ARMED.with(Cell::get), "the unwind disarmed the counter");
    let frozen = TALLY.with(Cell::get);
    let after = std::hint::black_box(vec![0u8; 4096]);
    assert_eq!(TALLY.with(Cell::get), frozen, "{}", after.len());
}

/// RETR's reply construction is the one body-sized buffer (issue #86): the
/// real `retr_reply` makes one status-line allocation and then only grows
/// that same buffer, so at every instant the live bytes are at most the
/// final buffer — `peak == live == capacity` — and the growth requests sum
/// to less than twice it. Exact pins on this fixture: 21 header bytes and
/// 65 536 pairs give a 720 917-byte mail whose reply is 852 011 bytes:
/// `format!` sizes the status line at 26 bytes and the buffer doubles
/// sixteen times to 26 * 2^16, so the requests sum to 26 * (2^17 - 1).
/// A second full-body
/// copy alive beside the reply (a padded body, a `to_vec`, a clone) raises
/// `requests` and lifts `peak` above the capacity; any extra allocation
/// moves `requests`. The wire bytes are checked after the scope closes.
#[test]
fn retr_builds_one_reply_buffer_and_no_second_body() {
    let raw = fixture(65_536);
    let size = payload_len(&raw);
    assert_eq!(raw.len(), 720_917);
    assert_eq!(size, 21 + 65_536 * 12, "dots are not counted");

    let (out, tally) = observe(|| retr_reply(size, &raw));

    let mut want = format!("+OK {size} octets\r\n").into_bytes();
    want.extend_from_slice(b"Subject: one copy\r\n\r\n");
    want.extend_from_slice(&b"..dot\r\nline\r\n".repeat(65_536));
    want.extend_from_slice(b".\r\n");
    assert_eq!(out.len(), 852_011);
    assert!(out == want, "the RETR wire bytes are the stuffed mail");

    let cap = out.capacity();
    assert_eq!(cap, 1_703_936, "doubling from the status line's 26 bytes");
    assert_eq!(
        tally,
        Tally {
            requests: 17,
            requested: 3_407_846,
            largest: cap,
            live: cap as isize,
            peak: cap as isize,
        },
        "one status-line allocation grown in place sixteen times, nothing beside it"
    );
    assert!(tally.requested < 2 * cap, "geometric growth of one buffer");
}

/// The same guarantee through a real POP3 session on one current-thread
/// runtime — the session task and its client both run on the observed
/// thread, so the RETR arm's own construction is counted, not a copy of
/// it. A 4 MiB mail is inserted before the scope opens; at no instant does
/// the session hold more than its reply buffer plus less than half a mail
/// of session state (line buffers, socket registration, the snapshot) —
/// a second body-sized buffer anywhere in the arm breaks that bound. The
/// whole transcript is checked byte for byte after the scope.
#[test]
fn a_real_retr_session_holds_one_copy_of_the_mail() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let raw = fixture(381_300);
    let size = payload_len(&raw);
    let reply = retr_reply(size, &raw);
    let store = Arc::new(Store::new(0));
    let mut email = crate::store::email("one-copy-1", "onecopy", "a@x.io");
    email.raw = raw.clone();
    store.insert(email).unwrap();
    let listener = rt
        .block_on(TcpListener::bind("127.0.0.1:0"))
        .expect("bind a loopback listener");
    let addr = listener.local_addr().unwrap();
    let mut transcript = format!(
        "+OK Swarmail POP3 server ready\r\n\
         +OK name is a valid mailbox\r\n\
         +OK maildrop has 1 messages ({size} octets)\r\n"
    )
    .into_bytes();
    transcript.extend_from_slice(&reply);
    transcript.extend_from_slice(b"+OK Swarmail POP3 server signing off\r\n");
    let mut wire = vec![0u8; transcript.len() + 4096];

    let (filled, tally) = observe(|| {
        rt.block_on(async {
            let client = async {
                let mut stream = TcpStream::connect(addr).await.unwrap();
                stream
                    .write_all(b"USER onecopy\r\nPASS x\r\nRETR 1\r\nQUIT\r\n")
                    .await
                    .unwrap();
                let mut filled = 0;
                loop {
                    let n = stream.read(&mut wire[filled..]).await.unwrap();
                    if n == 0 {
                        break filled;
                    }
                    filled += n;
                }
            };
            let server = async {
                let (stream, _) = listener.accept().await.unwrap();
                handle_conn(stream, store.clone(), Duration::from_secs(60))
                    .await
                    .unwrap();
            };
            tokio::join!(client, server).0
        })
    });

    assert!(
        wire[..filled] == transcript[..],
        "the session's wire bytes are the transcript ({filled} of {} bytes)",
        transcript.len()
    );
    let cap = reply.capacity() as isize;
    let half_mail = raw.len() as isize / 2;
    assert!(
        tally.peak >= cap,
        "the reply was built on the observed thread: peak {} < {cap}",
        tally.peak
    );
    assert!(
        tally.peak < cap + half_mail,
        "a second body-sized buffer was alive beside the reply: peak {} >= {cap} + {half_mail}",
        tally.peak
    );
}

/// Sizing allocates nothing (issue #86): the real `payload_len` over a
/// maildrop of varied mails — empty, dot-led, bare LF, a missing final
/// newline, a bare CR mid-line, non-UTF-8, a large one — asks the
/// allocator for nothing at all, so even one allocation per sized mail
/// moves the tally off zero. The sizes are checked after the scope.
#[test]
fn payload_len_sizes_every_mail_without_allocating() {
    let mails: Vec<Vec<u8>> = vec![
        Vec::new(),
        b".\r\n".to_vec(),
        b"a\nb\n".to_vec(),
        b"no final newline".to_vec(),
        b"a\rb\r\n".to_vec(),
        b"\xff\xfe\n".to_vec(),
        fixture(10),
        fixture(65_536),
    ];
    let (sizes, tally) = observe(|| {
        let mut sizes = [0u64; 8];
        for (slot, mail) in sizes.iter_mut().zip(&mails) {
            *slot = payload_len(mail);
        }
        sizes
    });
    assert_eq!(tally, ZERO, "sizing a maildrop allocates nothing");
    assert_eq!(
        sizes,
        [0, 3, 6, 18, 5, 4, 21 + 10 * 12, 21 + 65_536 * 12],
        "normalized bytes, transparency dots excluded"
    );
}

/// The PASS snapshot — the production caller of `payload_len` — sizes
/// every mail at the cost of its listing alone: the store's filtered
/// listing (no exact length: four slots, then grown to eight), the
/// snapshot's exact-length collect, and one id string per mail — eleven
/// requests for eight mails, none of them body-sized. A per-mail sizing
/// allocation makes it nineteen.
#[test]
fn the_snapshot_sizes_its_maildrop_without_a_per_mail_allocation() {
    let store = Store::new(0);
    let bodies: Vec<Vec<u8>> = (0..8).map(|i| fixture(1000 + i)).collect();
    for (i, body) in bodies.iter().enumerate() {
        let mut email = crate::store::email(&format!("snap-{i}"), "snapbox", "a@x.io");
        email.raw = body.clone();
        store.insert(email).unwrap();
    }
    let (maildrop, tally) = observe(|| snapshot(&store, "snapbox"));
    assert_eq!(
        maildrop
            .iter()
            .map(|m| (m.id.clone(), m.size))
            .collect::<Vec<_>>(),
        (0..8)
            .map(|i| (format!("snap-{i}"), (21 + (1000 + i) * 12) as u64))
            .collect::<Vec<_>>(),
        "oldest first, each sized by its payload"
    );
    assert_eq!(tally.requests, 11, "{tally:?}");
    assert!(
        tally.largest < bodies[0].len(),
        "no request is body-sized: {tally:?}"
    );
}
