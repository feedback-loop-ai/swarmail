# POP3 — the same store, RFC 1939

[← README](../../README.md) · [Guide index](README.md)

`:1110` — USER, PASS, STAT, LIST, UIDL, RETR, DELE, NOOP, RSET, TOP, CAPA,
QUIT. The maildrop **is** a swarmail inbox: `USER proj-42` names the inbox
exactly like SMTP AUTH does, so mail delivered over SMTP reads back over
POP3 — including mail that arrived before the session opened. The maildrop
is a snapshot taken at successful `PASS`, not a lock: numbering stays stable,
and new arrivals appear in the next session. Listings read the snapshot;
`RETR`, `TOP` and `DELE` consult the live store, so mail removed elsewhere
can remain listed but these commands answer `-ERR no such message`
(decision [0028 §3](../decisions/0028-wire-octets-auth-refusals-and-the-maildrop-snapshot.md)).
The name is everything after `USER` and its one space, verbatim (only the
line's CRLF is
removed, so an extra CR is part of the name and refused); a command that is
not UTF-8 ends the session unanswered. UIDLs are the
swarmail ids; octet counts are the payload the client keeps — every line
CRLF-terminated, the transparency dots RETR adds not counted (RFC 1939
§11), the terminating `.` not counted — derived from the stored payload by
the same traversal RETR writes with, never stored a second time (decision
[0037](../decisions/0037-pop3-size-accounting.md), amending
0028 §1). DELE marks in-session and applies
on QUIT — the update stage; a DELE of mail removed elsewhere answers
`-ERR no such message`, like RETR; RSET unmarks; a dropped connection
leaves the maildrop untouched. Plaintext only: no APOP, no STLS — a test-harness
surface behind the same trust boundary as the SMTP listener.
