# SMTP and POP3 protocol edges

Epic #120, batch 1: #121, #126, #127, #128, #131. Design synthesis dated
2026-10-18, baseline `2a325c9ab288ad1bad28aaab5205cbbeaa865930`.

## Feature and rationale

Release failed SMTP transports promptly, remove delayed-ACK stalls from
pipelined replies, reject unusable envelope paths and inbox names, preserve
identity across protocol planes, and let POP3 readers finish large replies
while making write progress. These repairs preserve a test sink's central
promise: the mail acknowledged by SMTP is queryable in the intended inbox.

The source evidence is the five issue bodies (including the supplied review
evidence), epic #120, run 3's intake framing, both current panel positions,
and run 2's patch and notes under `.brokkr/state/`. The files currently under
`.brokkr/state/tasks/` describe earlier coverage runs; the applicable framing
is `.brokkr/state/epic120/run3-intake-framing.md`. The source of truth is
`.brokkr/state/epic120-smtp-pop3-run2.patch`; run 1 is superseded. Run 2's
implementation and gate claims remain a draft to verify on the final tree.

Public identity and timeout contracts change. Proposed decision
[0046](../../docs/decisions/0046-smtp-pop3-edges.md) records the amendments,
including accepted 0028's explicit username-trimming rule. It remains
**proposed**: this design does not confer operator acceptance or weaken a
frozen guarantee. [plan.md](plan.md) and [tasks.md](tasks.md) own execution.

## Panel rulings

| Question | Ruling and disposition | Cost of the choice |
|---|---|---|
| Failed SMTP close | Adopt both positions: drop after a failed session; bound best-effort graceful close after success. Reject a second shutdown window after failure and detached closers. | Failed TLS sessions may lack `close_notify`; successful cleanup can consume one further idle window. |
| TCP_NODELAY | Adopt both positions: set on each accepted SMTP/POP3 socket before any reply, dropping only a socket whose setter fails. Reject a batching framework. | One syscall per accepted socket and one connection lost on setter failure. |
| SMTP path grammar | Adopt simplicity's local refusal guard and robustness's byte-preservation rationale. Reject optional quote-aware scanning and source-route stripping. | Quoted `>` remains unsupported; source routes and nonblank padding remain literal stored data. README must say so. |
| Inbox identity | Adopt the shared verbatim rule and cross-plane caller audit. Reject normalization, a new name type, and Store-wide retrofits. | Accidental padding names a different inbox; historical control-bearing names lose by-name access. |
| STARTTLS test placement | Adopt both current positions: retain run 2's declared sibling harness. Its task-completion deadline diagnostic justifies 0020 rule 2. Earlier disagreement over inline placement is resolved. | Harness assertions lose inline coverage policing; production execution remains measured. Moving unrelated inline tests is refused. |
| POP3 progress proof | Retain run 2's server write-entry/completion/count observer and 1 s window / 50 ms per 256 KiB pacing. Rule for simplicity against robustness's assertion that the timings literally observe parked writes: they show delayed completion, not `Poll::Pending`. Require a server span beyond one window, exact bytes and a behavioral old-deadline red together. | This does not identify each pending poll. Narrow observation and cold failure diagnostics remain necessary; client duration alone proves nothing. |
| Handshake bound | Adopt both current positions and retain the resolved code audit: `timeout(cfg.tls_handshake_timeout, acc.accept(raw))` already bounds the handshake. Keep its dedicated bound and regression; do not replace it with the idle bound merely to share a helper. | No new handshake policy; existing zero-timeout behavior remains. |
| Webhook restart | Adopt both current positions' resolved audit: reject adding a persistence migration. `run_on` creates `Webhooks::default`; targets are an in-memory lock and external replacement validates at PUT. There is no persisted target loader. | No migration or startup subsystem. Invalid PUT must preserve the existing target list. |
| POP3 lifetime | Adopt both positions' progress writer; reject the whole-reply deadline, fixed-size `write_all` chunks, minimum rate and total lifetime additions. | A progressing peer can retain its slot and assembled reply indefinitely; no adversarial availability or global memory bound is promised. |

**Dated clarification — 2026-10-18, run 3:** ratify run 2's local repairs and
both narrow harnesses; correct stale run-1 provenance and overclaims in proof
and product wording. Adopt robustness's explicit RST, retention and future
STLS caveats and its armed-chaos requirement. Reject ratification literally
as-is where it perpetuates those errors. The intake's example equating
`USER  pad` and `USER pad` is also corrected: only the former selects ` pad`;
the latter selects the distinct `pad` inbox, as AC7 requires.

The minimum-rate alternative is rejected for absent requirements and added
policy, not because rate policies are inherently untestable. Historical
control-bearing inboxes are not promised eventual cap eviction: public
by-name ingestion will now refuse them. Their cleanup limitations must be
stated without inventing a migration.

## Required behavior

### SMTP lifecycle and sockets

The connection owner preserves the original session error and drops the
transport immediately after failure, without further peer I/O. Successful
sessions attempt graceful shutdown through the existing SMTP idle bound;
cleanup failure remains best effort. A failed socket with unread input may
send RST rather than FIN; this is intended. JoinSet continues to own session count,
with the existing single reap at admission. New bounded-I/O sites are limited
to successful close and the POP3 progress writer; no timeout surrounds a commit.

Both accept sites set TCP_NODELAY before greetings, cap refusals or session
work. Setter failure logs a connection-local diagnostic, drops that socket
and continues accepting. STARTTLS wraps the same socket and still sends its
220 before starting the already-bounded handshake.

### SMTP paths

`envelope_path` rejects nonempty paths blank after Unicode whitespace
trimming, and paths containing any Rust `char::is_control` character. Only
literal `<>` is the null path, permitted only for MAIL. Both MAIL and RCPT
use the guard. Accepted paths remain **verbatim, padding included**; no
mailbox grammar, quote scanner or source-route transformation is introduced.

Syntax refusals are 501 when the command is in a state where its path is
eligible for parsing. Existing state precedence remains: nested MAIL still
answers 503 and preserves its sender/recipients. Refusals precede chaos and
mutation. A quoted local part containing `>` is unsupported and can receive
501; source routes are stored literally. Preserve existing local and UTF-8
path support, SIZE/parameter checks and RCPT capacity.

### Inbox names

One function, `model::validate_inbox`, refuses empty/blank-after-Unicode-trim
names and all `char::is_control` characters (C0, DEL, C1). Validation never
trims the returned name. Every accepted name is **verbatim, padding included**.
This is not Unicode normalization or a general invisible-character ban;
format characters are not controls, while whitespace-only names still fail.

SMTP PLAIN authcid and LOGIN username decode strictly and call that function
in prompted and initial-response forms. Refusal is 501 5.5.2 with identity
unchanged and no inbox created. Preserve exact PLAIN framing, accept-any
passwords, ignored authzid and one successful AUTH per TLS epoch; existing
state/mechanism refusal precedence remains.

POP3 strips only the actual line delimiter (LF and its preceding CR), uses
strict UTF-8, and passes USER everything after exactly one separating space.
It does not trim that argument. Invalid USER answers `-ERR` and clears the
pending name, so PASS cannot reuse it. This intentionally differs from SMTP
AUTH failure. Other command argument handling stays as before. A non-UTF-8
command, including PASS, ends the session unanswered, before dispatch; it
cannot select an inbox through replacement characters.

Audit REST path admission, MCP inbox arguments, UI routes, compat scopes,
the shared seed builder, SMTP AUTH, POP3 USER and webhook target filters.
Each uses the same predicate and existing error envelope before effects.
Invalid webhook PUT replaces none of the existing target list. Omitted
webhook inbox remains all inboxes; this batch adds no unknown-key/null or
dispatcher policy from #135.

Previously stored control-bearing names are not renamed or deleted. They
can still appear in discovery but cannot be read, awaited or cleared by name
through validated routes. Document global clear and applicable ID operations
as cleanup avenues, without promising automatic pruning of unreachable names.

### POP3 output progress

Every session reply uses one writer that offers the remaining bytes with
`write`, bounds that await, advances on a positive count and starts a fresh
window for the next write. Zero on a nonempty slice is WriteZero; an error or
expiry ends the session. Progress means bytes accepted by the local transport,
not proof the remote application consumed them.

The default window remains 600 seconds. Greeting, ordinary lines, CAPA,
LIST, UIDL, RETR and TOP share this rule. Accept-cap refusals retain their
existing whole-operation bound; command reads retain their current deadline.
SMTP writes are unchanged. No streaming encoder, extra knob or schema change
is needed. Preserve dot stuffing, octet accounting, PASS snapshot numbering,
fetch-from-store semantics, deletion behavior and oldest-only pruning.

No whole-reply deadline or minimum rate is imposed. A successful local write
within each window can retain a POP3 slot and that reply's buffer indefinitely.
A reader making some remote progress can still expire if the kernel does not
complete a local write within a window. Module docs, README and 0046 must
express these same limits. At the defaults, `max_sessions × max_message_size`
is 256 × 50 MiB = 12.5 GiB of raw-message-scale exposure. This is not a
reply-memory or RSS ceiling: encoding expansion, retained mail/snapshots and
allocation/socket overhead add to it. POP3 is plaintext; future STLS would
need to reassess write-versus-flush progress because TLS may buffer locally.

## Acceptance Criteria

1. **AC1 — #121:** A real generated-certificate STARTTLS client stops reading
   behind a small receive buffer while pipelined EHLO replies cause observed
   backpressure. With a shortened idle bound and one session slot, observe
   failed-session completion and admit a fresh 220 while the original peer
   remains open. Old unbounded shutdown must fail this regression. Plaintext
   timeout, ordinary TLS delivery, clean close and commit-before-250 stay green.
2. **AC2 — #121:** A local close-policy test proves failed sessions perform no
   shutdown I/O and preserve their error, while successful shutdown completes,
   fails or expires within one cleanup window. Keep the existing real stalled
   handshake regression and 220-before-handshake assertion green.
3. **AC3 — #126:** Read back `nodelay()` from actual accepted SMTP and POP3
   sockets before any reply. Exercise setter failure, verify that socket closes
   and that the next peer is served. Removing the setter makes the option
   assertion fail. Each cfg(test) seam has an installing test.
4. **AC4 — #126:** Record loopback pipelined group measurements for
   MAIL+RCPT+RCPT+DATA, body+NOOP+NOOP and POP3 NOOP+STAT+NOOP+RSET. For each,
   record sample count, median, p99 and maximum, with median under 5 ms and
   a baseline Nagle comparison. Timing evidence belongs in the benchmark,
   not a scheduler-sensitive cargo-test latency assertion.
5. **AC5 — #127:** MAIL and RCPT blank/control matrices refuse 501 without
   effects; include C0, DEL, C1 and Unicode whitespace. A refused first MAIL
   opens no transaction; refused RCPT retains existing sender/recipients;
   nested MAIL keeps its 503 precedence. Literal null MAIL still delivers,
   null RCPT refuses, and later stored envelopes match accepted bytes exactly.
   Run eligible MAIL/RCPT refusal cases with the corresponding chaos rule armed
   at 100%: syntax still answers 501, not the injected reply, and preserves
   state. Pin quoted-`>` refusal and literal source routes in README.
6. **AC6 — #128:** LOGIN and PLAIN invalid names refuse 501 5.5.2 without
   identity/store effects, including a PLAIN-shaped LOGIN username containing
   password material. Valid UTF-8, prompted and initial-response forms route
   correctly. A validator table proves blank/C0/DEL/C1 refusals and verbatim
   accepted names; do not echo rejected credential contents in diagnostics.
7. **AC7 — #128:** Seed and deliver into the same padded name over REST and
   SMTP, then retrieve the exact IDs using the padded POP3 USER argument;
   the unpadded name is distinct. Invalid USER clears a prior pending name.
   Pre-create a U+FFFD inbox and prove invalid UTF-8 USER cannot select it;
   non-UTF-8 commands end unanswered without dispatch.
8. **AC8 — #128:** An explicit caller-to-test inventory covers all listed
   admission planes. Each representative control-bearing input refuses before
   effects, including no watcher registration and no webhook replacement.
   Keep valid padded-name routing/filters and existing output escaping tested.
   Record the memory-only webhook lifecycle and historical-name limitations.
9. **AC9 — #131:** Over real TCP, a large RETR remains unfinished at the
   server beyond one reply window. Record real write entry/completion times
   and positive counts, including at least two writes delayed by at least
   50 ms in the initial fixture, and receive exact bytes plus terminator.
   These durations are evidence of delayed completion, not pending-poll
   observations. Reinstating one `write_all` deadline must truncate this same
   workload; a missing observer or compilation error cannot supply the red.
   Client elapsed time alone cannot satisfy the proof. Begin with a 1 s
   window and 50 ms paced drains per 256 KiB; pacing shapes load, never
   substitutes for server observations or orders test events. An independent
   whole-test deadline bounds broken reads, with task cleanup on failure.
10. **AC10 — #131:** Real stalled-reader tests still prove expiry, session
    completion and fresh admission for large replies, including LIST/UIDL.
    Local writer tests exercise partial writes, zero, errors and expiry; all
    session reply sites use the owner. Docs accurately distinguish progress
    writes, command reads, cap refusal and indefinite retention.
11. **AC11 — proof and preservation:** Each changed behavior has a named
    behavioral red on `2a325c9` (or the precisely restored faulty behavior)
    and green on the final implementation. Compile failures are not reds;
    unchanged limitations are preservation pins. Keep 0001–0005, the 5 s busy
    bound, awaits-registry cap, exact extraction counts, single-writer lock,
    Origin guard, bridge, webhook plane, persisted envelope sender, chart
    startupProbe/knobs and publish guard intact. Follow 0020 placement and
    keep exact nonzero 100/100/100 coverage with no exclusions or dependencies.
12. **AC12 — delivery:** Implement on `epic120-smtp-pop3`, off freshly fetched
    main at `2a325c9` or later, resuming the patch as described in the plan.
    Re-run all final-tree gates and retain actual receipts. Signed commits
    cite all five issues and their decisive tests. Never push, create a PR,
    merge or close issues; leave the branch checked out and tree clean.
    A sandbox-refused brokkr compile is disclosed and delegated to verify,
    not treated as a green gate; cargo refusal or unavailable signing blocks
    implementation delivery under the commission's rules.

## Scope limits

Only this batch's protocol and admission repairs are designed here. No
transport framework, normalization/migration, storage schema, global budget,
new configuration, dependency allowance, chart or delivery-policy repair is
authorized by this spec. This design seat produces only the three spec files
and proposed 0046; implementation owns runtime changes and product docs.
