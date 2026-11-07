# Deliverability fixtures for agents

Two raw RFC 5322 messages, published with the crate (the `/tests/**` include)
so an agent or a client author can send them to a Swarmail and read the
deliverability report of what lands. Decision
[0131](../../../docs/decisions/0131-agent-deliverability.md), issue #261.

| File | What it is |
|---|---|
| `newsletter-failing.eml` | An HTML-only newsletter that carries exactly three findings in the `link.`, `html.` and `spam.` families, one per location kind: `link.scheme.http.v1` (a `link` location), `html.img.alt-missing.v1` (a `mime_part` location) and `spam.subject.all_caps.v1` (a `header` location). |
| `newsletter-fixed.eml` | The same newsletter repaired from those findings: an `https` link, `alt="Autumn Lamp"` on the image and a sentence-case subject. It carries no finding in those three families. |

Send either file as DATA to a running Swarmail (any inbox), then read the
report with `swarmail_get_deliverability_report` (MCP) or
`GET /api/v1/messages/{id}/deliverability` (REST), where `{id}` is the id in
the `250 2.0.0 OK: stored as {id}` reply. Judge a repair by the **rule ids** of
the three families, never by the whole report: under the default DNS policy
the authentication and bulk-sender checklist findings stay, and a report
body, its `weight`/`total` members or its `rule_set_id` may change without the
fixtures changing.

`tests/deliverability.rs` proves both files over real SMTP, by rule id.
`evals/` (the `newsletter-findings` task) reuses the failing file's subject and
HTML as its template, and a test there pins that they are equal.

## Provenance

Hand-written for #261. Only reserved names appear (`shop.example`,
`agent.test`); no real person, address or mail is in them. The `Date` and
`Message-ID` are fixed, so a fixture is byte-stable. Lines end in CRLF as on
the wire. The files are regenerated only in a reviewed change that also
re-proves them and the eval template.
