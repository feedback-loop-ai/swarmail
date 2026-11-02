# Quickstart, configuration and stopping

[← README](../../README.md) · [Guide index](README.md)

One binary, three ports, no config. **Docker:**

```
docker run --rm --stop-timeout 30 -p 1025:1025 -p 1110:1110 -p 8025:8025 ghcr.io/feedback-loop-ai/swarmail:main
```

`--rm` takes the container's anonymous `/data` volume with it: the image
declares `VOLUME /data`, so a run without `--rm` leaves one volume behind
per container even after `docker rm -f`. `--stop-timeout 30` gives the
graceful drain room (see *Stopping* below).

`:main` is the continuous build of the main branch — mutable, so pin
`:sha-<commit>` (or build locally) when you need a reproducible image.
`:v0.1.0` was never published, and `:0.1.0` is the 2026-10-12 first
build. A release tag publishes both `:v<version>` and `:<version>`: the
signed `v0.2.0` tag publishes `:v0.2.0` and `:0.2.0` once its release run
passes (see [Kubernetes](kubernetes.md) for the conditions).

**Persistence in Docker:** set `SWARMAIL_DATA_FILE` and mount a named
volume at `/data`. The image ships `/data` owned by its runtime uid 65532,
so a fresh named volume is writable (`:main` carries it, as does `:0.2.0`
once the `v0.2.0` release run publishes it; the 2026-10-12 `:0.1.0` build
predates it):

```
docker run --rm --stop-timeout 30 -p 1025:1025 -p 1110:1110 -p 8025:8025 \
  -v swarmail-data:/data -e SWARMAIL_DATA_FILE=/data/swarmail.db \
  ghcr.io/feedback-loop-ai/swarmail:main
```

The named volume outlives the container — that is the point — so remove
it yourself when the data is no longer wanted: `docker volume rm
swarmail-data`. A volume an older image created is owned by root;
`chown 65532:65532` its contents once, or start from a fresh volume.

**From source:** `cargo run -- serve` (Rust stable; everything else is
vendored), or `cargo install --locked --path .` from a checkout — the
crate is not published on crates.io.

**Then loop.** This transcript is real — a stock `smtplib` delivering, the
REST surface answering:

```
$ curl -X POST localhost:8025/api/v1/inboxes/quickstart/seed \
    -H 'content-type: application/json' \
    -d '{"from":"noreply@x.io","to":"ada@x.io","subject":"Verify your account",
         "text":"Welcome! Verify at https://x.io/verify?token=abc123 — code 424242 is your fallback."}'
{"id":"01a1261e-…","inbox":"quickstart", …,
 "links":["https://x.io/verify?token=abc123"], "codes":["424242"]}      # extraction ran

$ python3 - <<'EOF'                                    # a real client, really delivered
import smtplib
s = smtplib.SMTP('localhost', 1025, timeout=10)
s.sendmail('kratos@x.io', ['ada@x.io'],
           "From: kratos@x.io\r\nTo: ada@x.io\r\nSubject: Reset your password\r\n\r\n"
           "Reset: https://x.io/reset?token=xyz789 or use 998877.")
s.quit()
EOF

$ curl "localhost:8025/api/v1/inboxes/quickstart/await?to=ada@x.io&count=1&timeout_ms=5000"
{
  "matched": 1,
  "total": 1,
  "emails": [ { "subject": "Reset your password",
                "links": ["https://x.io/reset?token=xyz789"],
                "codes": ["998877"] } ]                 # no regex over HTML — fields
}
```

`await` blocks until the mail arrives (or the timeout answers `matched: 0`)
— push-based, no polling. Fixtures without SMTP: `…/seed` runs the full
parse + extraction pipeline. Per-test isolation needs no config: the AUTH
username names the inbox.

**One SMTP connection serves one inbox.** A session's first successful
AUTH fixes its inbox; a second AUTH on the same connection answers `503`
and changes nothing (decisions 0028 and 0034). Python's `smtplib` treats that `503`
as "already authenticated" and does not raise, so `s.login('a', 'x')`
followed by `s.login('b', 'x')` on one `SMTP` object files *both* tests'
mail under `a`. Open a fresh connection per inbox (a new `smtplib.SMTP`
per test); only a successful STARTTLS resets the identity on an open
connection.

Environment knobs: `SWARMAIL_SMTP_LISTEN` (default `127.0.0.1:1025`),
`SWARMAIL_POP3_LISTEN` (`127.0.0.1:1110`), `SWARMAIL_HTTP_LISTEN`
(`127.0.0.1:8025` — the image and the chart set `0.0.0.0`, see the
[network posture](network-posture.md)), `SWARMAIL_MAX_PER_INBOX` (`100000`, `0` = unlimited),
`SWARMAIL_URL` (the MCP stdio bridge's upstream, default
`http://127.0.0.1:8025`),
`SWARMAIL_TLS_CERT`/`SWARMAIL_TLS_KEY`,
`SWARMAIL_TLS_HANDSHAKE_TIMEOUT_MS` (`10000`), `SWARMAIL_DATA_FILE`,
`SWARMAIL_MAX_SESSIONS` (`256` concurrent sessions per SMTP and POP3
listener, at least `1`; one more is answered `421`; the flag is
`--max-sessions`. Each session holds a file descriptor, and the default is
sized so that three times the cap stays under the usual 1024 soft limit
(decision 0027 §3): above about 340, raise `ulimit -n` first),
`SWARMAIL_ROUTE_DOMAIN` (unset = off; the flag is `--route-domain`, see
[*One sending identity, many agents*](recipient-routing.md); proposed 0112).
Logging: `RUST_LOG` (env-filter syntax; default
`swarmail=info,tower_http=off`). `serve` and `gen-cert` log to stdout —
`gen-cert` also prints its `wrote <cert> and <key>` line there — and
write a refusal (`swarmail: <reason>`, exit 1) to stderr. Only `mcp`
logs to stderr, because its stdout is the JSON-RPC protocol and nothing
else (decision 0029 §4).

**Stopping:** `SIGTERM` (Docker's default stop signal, and the image's
declared `STOPSIGNAL`) or `SIGINT` starts the graceful drain, which may
take up to its **10 s** deadline (decision 0027 §7). A bare `docker stop`
also waits only 10 s before `SIGKILL`, so its grace meets the drain only
at the knife's edge (0027 §8): a kill that lands there turns the drain's
tail into crash recovery — every mail already answered `250` is
committed, anything not yet answered is refused, so the loss is grace,
never correctness. Give it room: run with `--stop-timeout 30` as above,
or stop with `docker stop -t 30`. Kubernetes' default 30 s grace already
clears the drain. A second signal during the drain exits 70 at once.
