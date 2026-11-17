# Ordered implementation tasks — wait-best

Each checkbox includes its proving test. These are implementation tasks;
none is checked off merely for having specified it.

- [ ] T1 — Inspect all nine native_wait rows, capture lock-v5 documents and
  write the selection/evidence matrix (AC1–2). Prove path/size/hash/attribution
  validation and recheck-at-use corruption refusal; assert zero native_wait
  not_inspected cells and cited fallback reasons. Keep official MailSlurper
  blocked. No evidence invented from memory.
- [ ] T2 — Extend Stream, version-specific validation, draft profiles and
  longest-wait-plus-request budget (AC6,10). Prove valid common/best/tight
  matrices and refusals for wrong modes/intervals/counts, missing or duplicate
  targets/profiles, evidence and pairings. Retain historical validation,
  serialization and byte checks.
- [ ] T3 — Implement RFC 6455 transport and module wiring (AC3,5). Prove
  fresh key/ring accept checks, split/coalesced frames, fragmentation,
  interleaved ping/pong/close and client masking; reject bad status/tokens,
  accept/RSV/opcodes/masks/lengths/control frames/UTF-8/negotiation. Test
  all buffer limits, overflow, EOF mid-frame, partial-read cancellation
  and trickle cutoff.
- [ ] T4 — Implement SSE (AC3,5). Prove status/content type, HTTP transfer
  framing and chunked bodies, split UTF-8/CRLF, comments, multiline data,
  empty dispatch, unterminated EOF, byte limits and trickle cutoff. Use real
  loopback endpoints for protocol tests and parser fixtures for malformed
  branches.
- [ ] T5 — Implement evidenced registration/readiness and authoritative
  stream observation (AC2,4–5). Use barriers to prove delivery before
  registration, between readiness and scan, and during scan. Prove events
  alone cannot return mail, pagination stays complete, failed scans are not
  empty and readiness failure does not silently select polling.
- [ ] T6 — Implement independent backstop, bounded reconnect and cancellation
  cleanup (AC4–5). Prove lost event yields backstop, reconnect rescans after
  readiness, stalled reconnect preserves backstop, event floods cannot
  starve cutoff, dirty-during-scan survives, coincident cause precedence is
  deterministic and late scans stay censored. Prove owned sockets/tasks
  terminate; no sleeps as synchronization.
- [ ] T7 — Add v4 row facts and timing propagation (AC7). Prove duplicate,
  unrelated and mixed answers, expected empty versus missed mail, transport
  error, late match and future row merge. Prove all four causes, negative
  ack_to_return, absent ack and independent send rejection/ambiguity without
  resend. Repeated notifications must not count as duplicate answers.
- [ ] T8 — Extend story A to waits chunks, fresh launches, profile windows
  and cleanup (AC6,8). Prove coordinates/counts, fresh instances, target CPU
  deltas, cap/limit readback, OOM/exit classification, cutoff during send/
  QUIT and cleanup failure exit 2. Extend wrapper only as needed and retain
  its bound/signal tests.
- [ ] T9 — Extend reports/admission (AC7–8,10). Independently recompute
  timing, causes, duplicate/unrelated/missed counts and resource summaries;
  mutate fields, profile, row/window/receipt pairings and remove rows to
  prove named refusal. Prove historical reports/admission unchanged and
  api/testexec/draft collection refusals retained. Clear waits debt only
  once the entire path works.
- [ ] T10 — Add and execute a separate ignored pinned-binary proof per
  selected stream (AC2,9). Verify executable/runtime digests, establish cited
  readiness, send real SMTP, require an event-triggered complete match and
  prove cleanup. Run each alone under timeout 600/taskset 0-31, recording
  load before/after; missing bytes fail. Document exact test names, variables,
  commands and actual results in the eval README. Initial/backstop-only
  success cannot qualify a stream; source variants need separate proofs.
- [ ] T11 — Final compatibility and full verification (AC10–11). Run locked
  mail_compare/store_contract_docs tests and bench build; run every house
  gate in docs/house-rules.md, including evals against the candidate binary,
  every recipe compile, coverage plus selftest, publish/seal proofs, licenses
  and native-tls absence proof. Distinguish CI-owned checks. Check no src,
  package, allowlist, active/frozen document or accepted-decision body drift,
  no false freeze and no outstanding selected-stream proof before claiming
  implementation completion. Never collect, publish or push in this task.
