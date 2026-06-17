# Build with Podman:
#   podman build -t s3em .
#   podman run --rm -p 4004:4004 \
#     -e S3_ENDPOINT=http://minio:9000 \
#     -v s3manager_data:/data \
#     s3em
# ── Stage 1: Builder ───────────────────────────────────────────────────────────
FROM docker.io/library/rust:alpine3.21 AS builder

RUN apk add --no-cache --update \
    build-base \
    autoconf \
    musl-dev \
    pkgconfig \
    openssl \
    openssl-dev \
    openssl-libs-static

WORKDIR /app

# Cache dependencies (avoid recompiling every time)
RUN cargo init --bin --name s3em .

COPY Cargo.toml Cargo.lock ./
RUN cargo build --release && \
    rm src/*.rs

# Real compilation
COPY ./src/ ./src

RUN touch src/main.rs && \
    cargo build --release && \
    strip target/release/s3em

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM alpine:3.21

RUN apk add --update --no-cache \
    ca-certificates \
    openssl \
    && \
    adduser -S -u 1000 -D app

COPY --from=builder /app/target/release/s3em /usr/local/bin/

EXPOSE 4004
ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=4004 \
    DATABASE_URL=/data/s3manager.db
VOLUME /data

USER app
CMD ["s3em"]