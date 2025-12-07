# Build stage - use Debian for easier OpenSSL compilation
FROM rust:latest AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy and build
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

# Production stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/api-gateway /app/api-gateway

# Add non-root user
RUN useradd -r -s /bin/false actoris
USER actoris

# Railway uses PORT env var
EXPOSE 8080

ENV RUST_LOG=info
ENV HOST=0.0.0.0

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${PORT:-8080}/health || exit 1

CMD ["/app/api-gateway"]
