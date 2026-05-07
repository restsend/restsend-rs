FROM rust:1.91-bookworm AS builder

WORKDIR /app
# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

ADD . .
# Build release binary
RUN cargo build --release -p restsend-backend

# Runtime stage
FROM debian:bookworm-slim
LABEL maintainer="shenjindi@fourz.cn"
LABEL org.opencontainers.image.source="https://github.com/restsend/restsend-rs"
LABEL org.opencontainers.image.description="A IM Server"

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set environment variables
ARG DEBIAN_FRONTEND=noninteractive
ENV LANG=C.UTF-8
ENV TZ=UTC

# Copy binary from builder
COPY --from=builder /app/target/release/restsend-backend /usr/local/bin/restsend-backend

# Copy static files
COPY --from=builder /app/static ./static
COPY --from=builder /app/js ./js

ENTRYPOINT ["restsend-backend"]