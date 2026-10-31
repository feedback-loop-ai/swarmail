---
name: swarmail
description: Read mail a flow sends you (codes, magic links, resets) through the swarmail MCP tools in four steps - clear, act, wait, extract.
---

## Reading mail with swarmail

Use the swarmail MCP tools whenever a flow mails you something you must read:
a sign-up message, a one-time code, a magic link or a reset link. Work in
four steps, always in this order: clear, act, wait, extract.

1. **Clear.** Pick an inbox name that is yours alone, then call
   `swarmail_clear_inbox` with it. Clear before you act, never between act
   and extract: the clear deletes the mail you are waiting for.
2. **Act.** Trigger the flow exactly once, with a recipient in your inbox. To
   seed a message instead of sending one, call `swarmail_seed_email`. A
   second trigger sends a second mail and makes the first code stale.
3. **Wait.** Call `swarmail_wait_for_email` with your inbox and a
   `timeout_ms` of at most 60000. For one code or one link, use
   `swarmail_wait_for_code` or `swarmail_wait_for_link` instead. The wait
   registers before it scans, so wait after act and never poll-then-decide.
4. **Extract.** Take the email id from the wait, then call
   `swarmail_extract_codes` or `swarmail_extract_links` on that id. Codes and
   links are extracted at ingest, so extract reads stored results and a
   cleared or pruned mail is gone from them too.

A `250` from SMTP, or a successful `swarmail_seed_email`, means the mail is
queryable now. Never re-send after it.

### Failure classes

- `wrong_message`: you read a mail meant for someone else. Keep to your own
  inbox and match the recipient and subject before you extract.
- `stale_code`: you used a code from an older mail. Trigger once, and take
  the code from the newest mail after your clear.
- `wrong_link`: you opened a look-alike link. Choose the link by its anchor
  text, host and path, not by its position in the list.
- `wait_timeout`: the wait ended with nothing. Report it, then look at the
  inbox before you re-trigger; a second trigger makes the first code stale.
- `gave_up`: you stopped before the task was done. Wait again, or report why
  no mail can arrive.
- `tool_misuse`: you called a tool with the wrong arguments, or in the wrong
  order. Read the tool's schema and the four steps above.
