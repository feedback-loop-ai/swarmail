# Webhook boundary

Design handoff for run 18, issues #75, #76, #97 and #98. This specifies repairs;
unchecked implementation tasks are not evidence that the feature has shipped.

## Feature and rationale

Make the existing best-effort webhook plane honest at its boundaries: reject
unusable header secrets before configuration changes, account for a lagged
shutdown backlog, send only to the configured endpoint without environment
proxies, hide URL credentials in outward representations, share serialized
payload storage, and expose loss without logging every refused notification.

The current implementation accepts controls that reqwest refuses, stops its
tail sweep on `Lagged`, inherits environment proxies in both clients, echoes
and logs URL credentials, copies payloads per attempt, and ends without making
its final ledger visible to the binary's operator. Successful baseline tests
do not cover these reproduced failures.

This is a public-contract amendment to 0029 §5, recorded in proposed
[0041 — Webhook boundary](../../docs/decisions/0041-webhook-boundary.md).
Only the operator accepts that decision. Frozen guarantees 0001–0003 and
ingest-time extraction (0005) remain intact.

## Scope and contracts

1. **Secrets:** at most 4,096 UTF-8 octets, accepted by reqwest's
   `HeaderValue::from_bytes`, with no leading or trailing SP or HTAB. Empty
   values, internal HTAB and valid non-ASCII UTF-8 remain allowed when accepted
   by that parser. One helper returns the actual header value for both
   validation and delivery. Refuse with a named, non-secret-bearing error;
   validate the complete PUT list before replacing any target. Do not trim.
2. **Shutdown accounting:** the final sweep handles every `Lagged(n)` by
   adding n lagged *mails* and continuing. Retained mail contributes one
   aborted *notification* per matching target. Only Empty or Closed ends the
   sweep. Abort and join outstanding deliveries afterwards; a task that has
   already reported delivered/exhausted is never counted again as aborted.
3. **Destination:** both cached default and extra-CA clients ignore environment
   proxies, refuse redirects, and retain current rustls certificate validation.
   URL Basic auth remains supported. This is an application-client boundary,
   not a DNS, routing, SSRF or operating-system isolation guarantee.
4. **Redaction:** GET, PUT echoes, validation errors and all webhook logs
   (including debug and error text) expose neither URL username nor password.
   One display policy strips the entire userinfo; an unparseable display uses
   a fixed placeholder. Internal URLs, normalized budget keys and actual
   Basic-auth requests retain credentials. `secret` remains redacted;
   `ca_pem` remains visible. Query and fragment are not scrubbed: they must
   not be treated as safe secret storage. Echoes are deliberately not a
   credential-preserving configuration backup.
5. **Payload:** once a mail has at least one admitted target, serialize
   `{ "event": "received", "email": <Email> }` once from a borrowed Email
   into immutable shared bytes. All targets and retry request bodies share
   that allocation. Do not build a payload when nothing is admitted or while
   stopping. Retry bytes survive pruning unchanged. JSON field meaning and
   content type stay unchanged; JSON object key ordering is not a contract.
6. **Drops:** every refusal still increments `dropped`. Per normalized sink
   key, the first refusal warns; subsequent refusals warn only when at least
   60 monotonic seconds have passed since its last warning. Successful
   admissions do not reset that window. Retiring a dead, drained budget also
   retires its suppression state. Independent sinks have independent windows;
   normalized duplicate spellings share a window. Exhaustion warnings retain
   their existing last-cause behavior.
7. **Final observation:** on ordinary dispatcher exit, after sweeping and
   joining, emit exactly one info event `webhook dispatcher stopped` with
   numeric `delivered`, `dropped`, `lagged`, `exhausted`, `aborted`, including
   an all-zero ledger. The default `serve` logger emits this on stdout.
   `/metrics` exposes live counts; final shutdown counts are available through
   this log and the embedding caller's retained `Webhooks::snapshot()` after
   stop. Keep shutdown → join serve tasks → drained → join dispatcher. No
   final HTTP scrape is promised, nor a final log after a crash/forced exit.

Delivery stays downstream of commit, never an awaited prerequisite for SMTP
250. Commit precedes both publication and 250; delivery need not wait for the
SMTP client to receive its reply. Keep subscription before listener startup,
32 slots per normalized URL, retry delays/timeouts, client-cache bounds,
best-effort loss and no replay/outbox. Do not change store, persistence,
SMTP/POP3, extraction, shutdown ordering, gate policy or dependency allowlists.

## Panel rulings and costs

Both run-local positions were read, alongside the four issue bodies and the
task directory. That directory contains only historical run-12 framing;
`run18-feature.txt` and issues #75/#76/#97/#98 supply this run's commission.

| Question | Ruling and explicit cost |
|---|---|
| Header grammar | Adopt both panels' HTTP-owned parser and robustness's edge-whitespace refusal. Reject simplicity's parser-only acceptance because receiver trimming breaks the verbatim-secret promise. Cost: previously accepted padded secrets now receive 400; internal HTAB/UTF-8 remain supported. |
| Tail sweep | Adopt both panels' loop through every lag and robustness's single lag-effect owner. Reject a one-lag patch and any store reconstruction. Cost: lagged fan-out remains unknowable; embeddings must quiesce producers before declaring drain complete. |
| Client construction | Adopt robustness's one builder for rustls, no redirects and no proxies. Reject simplicity's duplicated policy calls: one small helper prevents future client variants from omitting the boundary. Cost: one extra private function and its coverage obligation. |
| URL display | Adopt simplicity's whole-userinfo removal with the same policy for API and logs; retain Basic auth as both propose. Reject robustness's retained username and split query-display policies. Cost: less identity in diagnostics; query tokens remain visible, explicitly documented. |
| Shared bytes | Adopt the borrowed envelope and Bytes shared by both panels, using simplicity's existing `axum::body::Bytes` re-export. Reject a new direct bytes dependency because this is already the same reqwest-compatible type. Cost: a transport-module re-export coupling. |
| Memory proof | Adopt simplicity's request-boundary sharing proof plus one-versus-four memory observation. Reject robustness's build counter as sufficient: it would still pass if `.body(body.to_string())` returned. Retain a local build count only alongside allocation identity evidence. Cost: a narrowly scoped test observation and a reproducible, platform-sensitive memory experiment. |
| Warning policy | Adopt simplicity's 60-second monotonic window; reject robustness's saturation episodes, which can spam under rapid recover/refuse cycles and do not satisfy the requested time window. Supplied instants permit deterministic tests without wall-clock waits. Cost: one timestamp per budget and no automatic recovery summary. |
| Shutdown visibility | Adopt both panels' final ledger log without reordering stop. Reject keeping HTTP alive or adding a metrics-only listener. Cost: final aborted counts cannot be scraped after drain; operators must retain logs or the embedded ledger. Correct robustness's stderr assumption: the current `serve` logger uses stdout. |

Neither an outbox, global byte budget, serializer pool, generic limiter nor
proxy opt-in is justified by this commission. Sharing removes payload copies;
it does not impose a global memory bound. Four targets still use more tasks,
sockets and transport buffers than one. No claim that a sweep outruns arbitrary
concurrent producers, or that a crash reliably emits a final ledger, is adopted.

## Acceptance Criteria

- **AC1 (#75):** real PUT rejects CR, LF, NUL, U+0001, DEL, padded SP/HTAB,
  and 4,097-octet secrets with a named 400 without disclosing the value. A
  valid first/invalid second target leaves the entire prior list unchanged.
  Empty, internal HTAB, UTF-8 and exact 4,096-octet boundaries are covered;
  accepted values build the delivery header and a real sink receives the
  unpadded accepted bytes. Direct invalid `Webhooks::set` fails safely on
  delivery through the same validator. Existing retry/redaction tests pass.
- **AC2 (#76):** on a current-thread runtime, subscribe before 8,200 unique
  synchronous inserts, never yield before signaling shutdown and drained,
  then join: exactly 8 lagged mails, 8,192 aborted notifications for one
  matching target, and zero delivered/dropped/exhausted or network attempts.
  Also prove non-lagged backlog, filtering/fan-out, empty/closed termination,
  repeated lag continuation and exactly-once terminal accounting of successful,
  exhausted and aborted in-flight tasks. Existing post-stop commit and bounded
  stop tests pass. Synchronization uses events/seams, never sleeps.
- **AC3 (#97):** a real binary with child-only upper/lowercase HTTP_PROXY,
  HTTPS_PROXY and ALL_PROXY set to a recording proxy, and both NO_PROXY
  spellings absent, delivers directly with the expected body/secret and zero
  proxy connections. Exercise both default and extra-CA builder paths, using
  HTTP and generated-fixture HTTPS respectively. Observe the proxy through
  child shutdown. Existing redirect/TLS verification tests remain green.
- **AC4 (#98):** GET/PUT echoes and captured default/debug logs contain no
  username/password, including percent-encoded credentials, non-2xx retries,
  transport failures, budget refusals and malformed directly installed targets.
  Validation errors never include raw credential-bearing input. A real sink
  still receives Basic auth. Fixed placeholder and credential-free URL display
  cases are tested; existing secret redaction persists.
- **AC5 (#98):** one and four stalled targets receiving the same fixed set of
  mails retain one serialized payload allocation per admitted mail. Prove one
  build and the same live backing allocation at every actual request-body
  construction, including retries; compare exact received bytes and retain
  the pruned-mail retry test. Payload-backing bytes equal the sum of distinct
  live serialized mail lengths in both cases, not that sum times fan-out.
  A matched one-versus-four process-memory experiment records RSS and transport
  overhead at explicit readiness barriers; investigate any additional retained
  full-payload-per-target growth before claiming acceptance. No machine-specific
  absolute RSS threshold substitutes for the allocation proof.
- **AC6 (#98):** N refusals in one target's warning window cause exactly N
  dropped increments and one warning. Supplied monotonic instants prove
  suppression just before 60 seconds and a new warning exactly at expiry.
  A successful admission does not reset the window; duplicate keys share it,
  independent keys do not, and retirement releases it.
- **AC7 (#98):** a real binary test waits for a stalled sink to read an
  in-flight delivery, sends SIGTERM, drains stdout through EOF and sees exactly
  one final event with all five fields, `aborted=1`, the other counts zero,
  and clean exit. Inline tests prove final zero/nonzero ledgers after drain
  and channel-close exits. No shutdown ordering change is required.
- **AC8:** README and OpenAPI accurately describe secret validity, direct
  delivery, redaction/echo limitations, windowed warnings, shared payloads and
  final-log versus live-metrics visibility. Each child issue has behavioral
  red-before/green-after evidence against reviewed behavior; a compilation
  error does not qualify. All six repository gates pass with exact coverage
  and no policy weakening before implementation is reported complete.
