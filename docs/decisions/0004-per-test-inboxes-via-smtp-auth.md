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
