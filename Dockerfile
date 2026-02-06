# =============================================================================
# Mycelix WebSocket RPC Server - Multi-stage Docker Build
# =============================================================================
#
# Build:
#   docker build -t mycelix-ws-server .
#
# Run:
#   docker run -p 8888:8888 mycelix-ws-server
#   docker run -p 8888:8888 -e LOG_LEVEL=debug mycelix-ws-server
#
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build
# -----------------------------------------------------------------------------
FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release -p ws-server

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/mycelix-ws-server /usr/local/bin/

# Create non-root user
RUN useradd -r -s /bin/false mycelix
USER mycelix

# Environment variables
ENV HOST=0.0.0.0
ENV PORT=8888
ENV LOG_LEVEL=info
ENV RUST_LOG=info

# Expose ports
EXPOSE 8888
EXPOSE 8889

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8889/health || exit 1

# Run server
ENTRYPOINT ["mycelix-ws-server"]
CMD ["--host", "0.0.0.0", "--port", "8888"]
