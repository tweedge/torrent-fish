# Build stage: compile the Rust binary against libtorrent (Debian).
# A full toolchain + libtorrent-dev only exists here; the runtime image is
# distroless and ships just the binary and the shared libraries it needs.
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
      libtorrent-rasterbar-dev libboost-system-dev libssl-dev \
      pkg-config build-essential cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY shim ./shim
RUN cargo build --release --locked

# Collect the exact shared libraries the binary needs so we can copy them into
# the distroless runtime below.
RUN mkdir -p /out && cp target/release/torrent-fish /out/ && \
    ldd target/release/torrent-fish \
      | grep -oE '/[^ ]+\.so[^ ]*' \
      | xargs -r -I{} cp --parents -L {} /out/

# Runtime stage: distroless cc (ships libstdc++/libgcc), running as nonroot.
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /out/ /

USER nonroot:nonroot
WORKDIR /home/nonroot

ENTRYPOINT ["/torrent-fish"]
