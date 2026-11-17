# Ordered implementation tasks — wait-best

Each checkbox includes its proving test. These are implementation tasks;
checked tasks are built in the named commits; T10b (dispatcher host
execution) is the one task genuinely open. Rerun their tests during T11. T10 execution is
explicitly a dispatcher task and remains open.

- [x] T1 — Built at ed17b7f: inspect all nine native_wait rows, capture lock-v5 documents and
  write the selection/evidence matrix (AC1–2). Prove path/size/hash/attribution
  validation and recheck-at-use corruption refusal; assert zero native_wait
  not_inspected cells and cited fallback reasons. Keep official MailSlurper
  blocked. No evidence invented from memory.
- [x] T2 — Built at 46d6e83: Extend Stream, version-specific validation, draft profiles and
  longest-wait-plus-request budget (AC6,10). Prove valid common/best/tight
  matrices and refusals for wrong modes/intervals/counts, missing or duplicate
  targets/profiles, evidence and pairings. Retain historical validation,
  serialization and byte checks. Separately prove lock v4 refuses v5-only
  wait documents on a version-only downgrade and injection into valid v4
  with no cap placeholder; preserve valid historical reviewed documents.
- [x] T3 — Built at fc47a90: implement RFC 6455 transport and module wiring (AC3,5). Prove
  fresh key/ring accept checks, split/coalesced frames, fragmentation,
  interleaved ping/pong/close and client masking; reject bad status/tokens,
  accept/RSV/opcodes/masks/lengths/control frames/UTF-8/negotiation. Test
  all buffer limits, overflow, EOF mid-frame, partial-read cancellation
  and trickle cutoff.
- [x] T4 — Built at fc47a90: implement SSE (AC3,5). Prove status/content type, HTTP transfer
  framing and chunked bodies, split UTF-8/CRLF, comments, multiline data,
  empty dispatch, unterminated EOF, byte limits and trickle cutoff. Use real
  loopback endpoints for protocol tests and parser fixtures for malformed
  branches.
- [x] T5a — Built at 46d6e83 (MailHog matched pong; Mailpit and Inbucket stay cited polls): Resolve MailHog matched-pong readiness and Mailpit/Inbucket
  evidence gaps using T1 captures, preserving cited polling where unresolved.
  Prove target-specific readiness ordering and bounded failure, with no
  sleep, optional stats/history assumption or upgrade-only readiness.
- [x] T5b — Closed by its fallback clause (no SignalR client; smtp4dev stays a cited poll, named in README): Implement the evidenced minimal smtp4dev SignalR JSON exchange
  in targets.rs. Prove split/coalesced bounded record framing, handshake,
  unique probe broadcast, ping/completion/close and malformed envelopes;
  readiness probes never count as SMTP delivery. Missing framework evidence
  keeps the row polled and is named in README.
- [x] T5 — Built at 46d6e83: Implement evidenced registration/readiness and authoritative
  stream observation (AC2,4–5). Use barriers to prove delivery before
  registration, between readiness and scan, and during scan. Prove events
  alone cannot return mail, pagination stays complete, failed scans are not
  empty and readiness failure does not silently select polling.
- [x] T6 — Built at 46d6e83: Implement independent backstop, bounded reconnect and cancellation
  cleanup (AC4–5). Prove lost event yields backstop, reconnect rescans after
  readiness, stalled reconnect preserves backstop, event floods cannot
  starve cutoff, dirty-during-scan survives, coincident cause precedence is
  deterministic and late scans stay censored. Prove owned sockets/tasks
  terminate; no sleeps as synchronization.
- [x] T7 — Built at 45c65a6, late precedence over unrelated fixed in review rework: Add v4 row facts and timing propagation (AC7). Prove duplicate,
  unrelated and mixed answers, expected empty versus missed mail, transport
  error, late match and future row merge. Prove all four causes, negative
  ack_to_return, absent ack and independent send rejection/ambiguity without
  resend. Repeated notifications must not count as duplicate answers.
- [x] T8 — Built at 45c65a6, QUIT-cut acceptance kept in review rework: Extend story A to waits chunks, fresh launches, profile windows
  and cleanup (AC6,8). Prove coordinates/counts, fresh instances, target CPU
  deltas, cap/limit readback, OOM/exit classification, cutoff during send/
  QUIT and cleanup failure exit 2. Extend wrapper only as needed and retain
  its bound/signal tests.
- [x] T9 — Built at 45c65a6: Extend reports/admission (AC7–8,10). Independently recompute
  timing, causes, duplicate/unrelated/missed counts and resource summaries;
  mutate fields, profile, row/window/receipt pairings and remove rows to
  prove named refusal. Prove historical reports/admission unchanged and
  api/testexec/draft collection refusals retained. Clear waits debt only
  once the entire path works.
- [x] T10a — Built at 45c65a6 (MailHog, the one implemented stream): Add a separate Linux-only ignored pinned-binary proof for
  each implemented stream candidate (AC2,9). Required bytes fail when absent;
  executable/runtime identities are checked. Prove cited readiness, real
  SMTP, event-triggered complete authoritative match and cleanup.
  Initial/backstop-only success cannot qualify a stream. Document exact
  qualified names, MAIL_COMPARE_* variables and individual commands in
  docs/evals/README.md; verify test discovery and missing-prerequisite
  refusal. Keep candidates polled pending host success.
- [ ] T10b — OPEN, dispatcher host execution: run each T10a proof alone under
  timeout 600 taskset -c 0-31 with digest-verified pinned bytes, record load
  before/after, preserve test status and cleanup receipts, and attach output
  to the PR before merge. Only successful evidenced rows may then be selected;
  failed candidates stay cited polls. This execution is not a sandbox
  implementation failure and default tests do not discharge it.
- [ ] T11 — Final compatibility and full verification (AC10–11). Run locked
  mail_compare/store_contract_docs tests and bench build; run every house
  gate in docs/house-rules.md, including evals against the candidate binary,
  every recipe compile, coverage plus selftest, publish/seal proofs, licenses
  and native-tls absence proof. Distinguish CI-owned checks. Check no src,
  package, allowlist, active/frozen document or accepted-decision body drift,
  no false freeze. Preserve the six Linux-only cfg_attr ignore guards
  for /proc readers. Report local gates separately from OPEN T10b host
  execution; never claim an unexecuted candidate is qualified. Never collect, publish or push in this task.
