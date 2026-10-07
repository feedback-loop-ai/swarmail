# syntax=docker/dockerfile:1
# Swarmail — single static binary, scratch base. Final image ≈ binary size.
FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
ENV CARGO_NET_RETRY=10
RUN cargo build --release --locked
RUN strip target/release/swarmail

FROM scratch
COPY --from=build /app/target/release/swarmail /swarmail
EXPOSE 1025 8025
USER 65532:65532
ENTRYPOINT ["/swarmail", "serve"]
