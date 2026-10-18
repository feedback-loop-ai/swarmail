# syntax=docker/dockerfile:1
# Swarmail — single static binary, scratch base. Final image ≈ binary size.
# The builder image is digest-pinned (AC52.4): a moving `rust:1-alpine` tag
# would let an unreviewed toolchain build the release artifact. Re-pin
# deliberately (look the digest up, commit the change) when bumping Rust.
FROM rust:1-alpine@sha256:0cce0a5e0e8ba67b455257a3a02a1d99005f382748789d6464460028810f1627 AS build
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
ENV CARGO_NET_RETRY=10
RUN cargo build --release --locked
RUN strip target/release/swarmail
# scratch has no shell to mkdir with: the empty data directory is made
# here and copied in owned by the runtime uid (issue #103).
RUN mkdir /data-empty

FROM scratch
COPY --from=build /app/target/release/swarmail /swarmail
# A fresh named volume mounted at /data inherits this directory's owner,
# so uid 65532 can write `SWARMAIL_DATA_FILE=/data/swarmail.db` without a
# root entrypoint. Persistence stays opt-in: without SWARMAIL_DATA_FILE the
# store is memory-only and the volume is unused. A volume created by an
# older image stays root-owned until the operator chowns it.
COPY --from=build --chown=65532:65532 /data-empty /data
VOLUME /data
# The container contract (decision 0030 §1): the binary's defaults are
# loopback, which a container's `-p` mappings could never reach — the image
# states its binds explicitly so the published ports are real. An operator
# can still override each through its SWARMAIL_*_LISTEN knob.
ENV SWARMAIL_SMTP_LISTEN=0.0.0.0:1025 \
    SWARMAIL_POP3_LISTEN=0.0.0.0:1110 \
    SWARMAIL_HTTP_LISTEN=0.0.0.0:8025
EXPOSE 1025 1110 8025
USER 65532:65532
# `docker stop` sends this signal — the default is SIGKILL, which would
# skip the drain entirely. SIGTERM is the graceful path the watcher takes
# (decision 0027).
STOPSIGNAL SIGTERM
ENTRYPOINT ["/swarmail", "serve"]
