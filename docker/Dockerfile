# Single Dockerfile for the pub/sub demo binaries. Pick which one to
# build with --build-arg BIN_NAME=pub-rustdds (or sub-rustdds).
#
# Tauri explorer is NOT containerized — it needs GTK/WebKit and an X
# server. See INSTALL-linux.md at the repo root.

# syntax=docker/dockerfile:1.6
ARG RUST_VERSION=1
ARG BIN_NAME=pub-rustdds

FROM rust:${RUST_VERSION}-bookworm AS builder
ARG BIN_NAME
WORKDIR /src
COPY . .
# Build only the requested binary. The Tauri/WebKit workspace member is
# skipped because it isn't a transitive dependency of pub-rustdds /
# sub-rustdds.
RUN cargo build --release -p ${BIN_NAME} \
 && cp /src/target/release/${BIN_NAME} /tmp/app

# Distroless: no shell, just libc + libgcc. ~25 MB final image.
FROM gcr.io/distroless/cc-debian12
COPY --from=builder /tmp/app /app
ENTRYPOINT ["/app"]
