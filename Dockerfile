# ==========================================
# Stage 1: The Builder
# ==========================================
FROM rust:1-slim AS builder

WORKDIR /app

# Copy all your source code into the builder
COPY . .

# Build ONLY the cache_server binary in release mode
RUN cargo build --release --bin cache_server

# ==========================================
# Stage 2: The Runtime
# ==========================================
FROM debian:bookworm-slim

WORKDIR /app

# Install minimal OS libraries needed to run Rust programs
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libc6 \
    && rm -rf /var/lib/apt/lists/*

# Copy ONLY the compiled binary from the builder stage
COPY --from=builder /app/target/release/cache_server /app/cache_server

# Tell Docker this container will use port 3000
EXPOSE 3000

# The command to run when the container starts
CMD ["./cache_server"]