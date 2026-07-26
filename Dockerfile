# ── Stage 1: Build ───────────────────────────────────────────────────────────
FROM rust:1.95-slim-bookworm AS builder

# Build deps for the bundled SQLite (needs cc/make) and TLS
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
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
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the compiled binary
COPY --from=builder /app/target/release/themisbot ./themisbot

# Render injects PORT automatically; our health server reads it
EXPOSE 8085

CMD ["./themisbot"]
