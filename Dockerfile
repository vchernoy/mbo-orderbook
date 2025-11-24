# ============================
# 1) Builder image
# ============================
FROM rust:1.91-bookworm AS builder

# Create app dir
WORKDIR /app

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
# If you later add more files (e.g. src/lib.rs), you can copy just Cargo.* first for better caching
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release >/dev/null 2>&1 || true

# Now copy real sources
COPY src ./src

# Build all your binaries in release mode
RUN cargo build --release \
    --bin mbo-replay \
    --bin mbo-streamer \
    --bin mbo-streamer-raw \
    --bin mbo-consumer

# ============================
# 2) Runtime image
# ============================
FROM debian:bookworm-slim AS runtime

# Install minimal runtime deps (if needed)
# For pure Rust binaries, usually nothing is needed; but CA certs are nice for TLS
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m appuser
USER appuser

WORKDIR /app

# Copy only the binaries from the builder
COPY --from=builder /app/target/release/mbo-replay        /usr/local/bin/mbo-replay
COPY --from=builder /app/target/release/mbo-streamer      /usr/local/bin/mbo-streamer
COPY --from=builder /app/target/release/mbo-streamer-raw  /usr/local/bin/mbo-streamer-raw
COPY --from=builder /app/target/release/mbo-consumer      /usr/local/bin/mbo-consumer

# Default entrypoint (you can override per-container)
ENTRYPOINT ["mbo-replay"]
CMD ["--help"]

