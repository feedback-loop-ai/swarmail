# Chaos — break SMTP on purpose

[← README](../../README.md) · [Guide index](README.md)

```
$ curl -X PUT localhost:8025/api/v1/chaos -H 'content-type: application/json' -d \
    '{"data": {"probability": 100, "error": "451 4.3.0 tempfail", "delay_ms": 250}}'

$ python3 -c "
import smtplib
s = smtplib.SMTP('localhost', 1025, timeout=10)
s.sendmail('a@x.io', ['b@x.io'], 'Subject: t\r\n\r\nbody')"
…
smtplib.SMTPDataError: (451, b'4.3.0 tempfail')              # the injected line, verbatim

$ curl -X DELETE localhost:8025/api/v1/chaos                  # recovery is a test too
```

Chaos is per-stage — `connect`, `mail_from`, `rcpt`, `data` — each with its
own probability, error line and delay. Set it, run the retry suite, clear
it. MCP: `swarmail_set_chaos` / `swarmail_clear_chaos`. Wire values are
never narrowed: `probability` must be an integer 0..=100 — present, a true
integer, within the ceiling — each stage refusing by name, and a
wrong-typed `delay_ms` is refused while an absent one honestly means "no
delay" (decision 0018). A supplied `error` must be a reply line: a single
line starting `4xx`/`5xx`, at most 510 octets (RFC 5321 §4.5.3.1.2's 512
less the CRLF `write_line` appends) — a `2xx` error is refused, because a
rule that would answer success while the mail is not stored is a lie the
wire cannot tell. A supplied `delay_ms` is an integer 0..=300000, the
stage-timer scale — a chaos stall is a wire condition, not an unbounded
hang. Refused values answer 400 with the rule's own sentence (the same
words on the MCP plane), and a rejected PUT applies nothing.

A fired rule always applies its delay, then always fails its stage — a
delay-only rule (probability set, no `error`) is a slow failure, never a
harmless slow-down (issue #17):

| Stage | Delay-only rule (no `error`) | With `error` |
|---|---|---|
| `connect` | wait, then close with no line at all — neither banner nor error | wait, then the given line replaces the banner |
| `mail_from` | wait, then `451 4.3.0 chaos` | wait, then the given line |
| `rcpt` | wait, then `451 4.3.0 chaos` | wait, then the given line |
| `data` | wait, then `451 4.3.0 chaos` — the mail is **not** stored | wait, then the given line — the mail is **not** stored |

So an omitted `error` means *the stage's default failure*, not *no failure*
— turn chaos off with `probability: 0` or by clearing the config. A
present `"error": null` is refused like any non-string (decision 0044 §3);
an unset line is omitted from the answer, so a fetched config PUTs back. Two
orderings are pinned: a genuine over-limit `552 5.3.4` is answered before
the data chaos rule is consulted (resource failures precede injected
ones), and a data rule's delay applies after the body has been received
and before the reply — the transfer itself is never slowed.
