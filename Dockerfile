# =============================================================================
# Mycelial Node - Multi-stage Dockerfile
# =============================================================================
# Build: docker build -t mycelial-node .
# Run:   docker run -p 8080:8080 -p 9000:9000 mycelial-node
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build the Rust binary
# -----------------------------------------------------------------------------
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release --bin mycelial-node

# -----------------------------------------------------------------------------
# Stage 2: Runtime image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/mycelial-node /app/mycelial-node

# Create data directory for SQLite
RUN mkdir -p /app/data

# Default environment variables
ENV RUST_LOG=info
ENV DATA_DIR=/app/data

# Expose ports
# 8080 - HTTP/WebSocket server for dashboard
# 9000 - P2P TCP port (default bootstrap)
EXPOSE 8080 9000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command: run as bootstrap node
ENTRYPOINT ["/app/mycelial-node"]
CMD ["--bootstrap", "--name", "Bootstrap", "--port", "9000", "--http-port", "8080"]
