# Build stage
FROM rust:1.75-alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache musl-dev

# Create a dummy project to cache dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
COPY Cargo.toml ./
RUN cargo build --release && rm -rf src target/release/deps/api_gateway*

# Copy actual source and build
COPY src ./src
RUN cargo build --release

# Production stage
FROM alpine:3.19

WORKDIR /app

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Copy binary
COPY --from=builder /app/target/release/api-gateway /usr/local/bin/

# Add non-root user
RUN addgroup -S actoris && adduser -S actoris -G actoris
USER actoris

EXPOSE 8080

ENV RUST_LOG=info

CMD ["api-gateway"]
