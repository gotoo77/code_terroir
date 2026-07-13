# Build stage
FROM rust:1.90.0-slim-bookworm AS builder

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
COPY config ./config

# Build the application
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm-slim

ARG VERSION=dev
ARG VCS_REF=unknown
ARG BUILD_DATE=unknown

LABEL org.opencontainers.image.title="Code Terroir" \
      org.opencontainers.image.description="Plateforme de traçabilité pour producteurs artisanaux" \
      org.opencontainers.image.source="https://github.com/gotoo77/code_terroir" \
      org.opencontainers.image.version="$VERSION" \
      org.opencontainers.image.revision="$VCS_REF" \
      org.opencontainers.image.created="$BUILD_DATE" \
      org.opencontainers.image.licenses="MIT"

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
