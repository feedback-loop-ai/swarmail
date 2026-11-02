# Roadmap

[← README](../../README.md) · [Guide index](README.md)

- [x] STARTTLS + self-signed cert generation
- [x] HTTPS webhook targets (reqwest + rustls)
- [x] SQLite persistence (`--data-file` / `SWARMAIL_DATA_FILE`)
- [x] POP3 server; MailHog/Mailpit API compat shims
- [x] Exact 100/100/100 coverage contract (lines, branches, functions) in the gate and CI
- [x] MCP: HTTP + stdio, 18 tools
- [x] Kubernetes: the Helm chart (OCI-published, decision 0008 *accepted*)
- [ ] crates.io publish — the package is publish-ready (see [*Publishing*](publishing.md));
  the operator runs `cargo publish --locked` with their token
