# 0004 — Per-test inboxes via SMTP AUTH username

**Status**: accepted (original design, v0.1)

## Ruling

Any SMTP client may authenticate with any username/password; the AUTH PLAIN
or LOGIN **username names the inbox** the session's mail lands in.
Unauthenticated sessions deliver to the `default` inbox. No provisioning
step exists: the first mail to `proj-42@anything` creates `proj-42`.

## Why

Parallel test suites and parallel agents need isolation without a control
API round-trip. MailSlurper-style global mailboxes force every consumer to
filter the same stream; Ory Kratos and Nodemailer already put a identity in
the AUTH username, so the inbox is free. A test runner sets
`smtp://proj-42:x@swarmail:1025` and owns that inbox exclusively.

## Consequences

- Auth accepts any credentials — this is a dev tool on a loopback by
  default, not an internet service; the docs say so.
- Inbox listing (`/api/v1/inboxes`) is the discovery surface; clear/await
  operate per-inbox, so parallel runs never contend.
- `clear_all` exists for suite teardown and is the only cross-inbox
  destructive endpoint.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit. The text above stays
as the accepted founding ruling; each marker names a later record with that
record's own verb and scope, and nothing above is rewritten by it.

- *(Amended by [0028](0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md)
  (accepted 2026-10-16): §1 and §3 gain a refusal posture, not a new
  identity rule — the AUTH username still names the inbox, and credentials
  that fail to name one answer `501 5.5.2` and change nothing instead of
  silently succeeding. 0028 §2 is itself later amended by 0034 and 0046.)*
