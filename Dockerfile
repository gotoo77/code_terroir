# Build stage
FROM rustlang/rust:nightly-slim AS builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

#RUN apt-get update && apt-get install -y \
#    pkg-config \
#    libssl-dev \
#    libpq-dev \
#    wkhtmltopdf \
#    && rm -rf /var/lib/apt/lists/*

# Copy dependency files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
#    wkhtmltopdf \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false code-terroir

# Copy binary from builder stage
COPY --from=builder /app/target/release/code-terroir /usr/local/bin/

# Create directories
RUN mkdir -p /app/uploads /app/exports && \
    chown -R code-terroir:code-terroir /app

USER code-terroir

EXPOSE 3030

CMD ["code-terroir"]
