# Build stage
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/jira-mock-server ./crates/jira-mock-server

# Create dummy lib files for other workspace members to satisfy dependencies
RUN mkdir -p crates/jira-db-core/src && echo "fn main() {}" > crates/jira-db-core/src/lib.rs
RUN mkdir -p crates/jira-db-cli/src && echo "fn main() {}" > crates/jira-db-cli/src/main.rs
RUN mkdir -p crates/jira-db-mcp/src && echo "fn main() {}" > crates/jira-db-mcp/src/main.rs
RUN mkdir -p crates/jira-db-tauri/src-tauri/src && echo "fn main() {}" > crates/jira-db-tauri/src-tauri/src/main.rs

# Copy minimal Cargo.toml for other crates
COPY crates/jira-db-core/Cargo.toml crates/jira-db-core/
COPY crates/jira-db-cli/Cargo.toml crates/jira-db-cli/
COPY crates/jira-db-mcp/Cargo.toml crates/jira-db-mcp/
COPY crates/jira-db-tauri/src-tauri/Cargo.toml crates/jira-db-tauri/src-tauri/

# Build mock server
RUN cargo build --release -p jira-mock-server

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/jira-mock-server /app/jira-mock-server

# Create data directory
RUN mkdir -p /app/data

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV DATA_DIR=/app/data

# Run the server
ENTRYPOINT ["/app/jira-mock-server"]
CMD ["--port", "8080", "--data-dir", "/app/data"]
