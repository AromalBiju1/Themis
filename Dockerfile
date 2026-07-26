# ── Stage 1: Build ───────────────────────────────────────────────────────────
FROM rust:1.95-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first so Docker can cache the dep-compile layer
COPY Cargo.toml Cargo.lock ./

# Compile deps with a stub main — this layer is cached unless Cargo.toml changes
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release --locked
RUN rm -f src/main.rs target/release/themisbot target/release/deps/themisbot*

# Copy real source and do the final compile (only your code recompiles)
COPY src ./src
RUN cargo build --release --locked

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Writable directory for SQLite — /app is read-only (binary layer)
RUN mkdir -p /data && chmod 777 /data

WORKDIR /app

COPY --from=builder /app/target/release/themisbot ./themisbot

EXPOSE 8085

CMD ["./themisbot"]
