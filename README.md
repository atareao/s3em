# s3em — S3 Event Manager

[![Rust](https://img.shields.io/badge/rust-2024-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-yellow.svg)](LICENSE)
[![Docker](https://img.shields.io/badge/docker-ready-2496ed?logo=docker&logoColor=white)](Dockerfile)
[![Alpine](https://img.shields.io/badge/base-alpine-0D597F?logo=alpinelinux&logoColor=white)](Dockerfile)
[![CI](https://img.shields.io/badge/build-passing-brightgreen)]()

**s3em** is a lightweight REST API gateway for S3-compatible object storage (MinIO, AWS S3, etc.). It provides idempotent file uploads, real-time event streaming via SSE, POSIX metadata tracking, and JWT-based authentication — all in a single ~5 MB statically-linked binary.

---

## Features

- **Idempotent uploads** — SHA-256 checksum deduplication; re-uploading the same file returns the existing record.
- **POSIX metadata** — Preserves mode, uid, gid, timestamps, and other file attributes alongside S3 objects.
- **Real-time events** — Server-Sent Events (SSE) stream for file create/update/delete operations.
- **Audit history** — Every operation is persisted to SQLite with paginated query support.
- **JWT + API key auth** — Login endpoint issues JWTs; master API key provides header-based auth bypass.
- **Soft deletes** — Deletions set a `deleted_at` timestamp rather than removing records.
- **Single binary** — Multi-stage Docker build produces a minimal Alpine image (~5 MB).

---

## Quick start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- An S3-compatible store (e.g., [MinIO](https://min.io/))

### Run locally

```bash
# Clone and build
git clone <repo-url> && cd s3em
cargo build --release

# Set required environment variables
export S3_ENDPOINT=http://localhost:9000
export S3_ACCESS_KEY=minioadmin
export S3_SECRET_KEY=minioadmin
export S3_BUCKET=s3manager
export S3_REGION=us-east-1
export MASTER_API_KEY=my-secret-key
export JWT_SECRET=my-jwt-secret
export DATABASE_URL=s3manager.db

# Start the server
cargo run --release
```

The API is now available at `http://0.0.0.0:4004`.

### Run with Docker

```bash
docker build -t s3em .
docker run --rm -p 4004:4004 \
  -e S3_ENDPOINT=http://minio:9000 \
  -e S3_ACCESS_KEY=minioadmin \
  -e S3_SECRET_KEY=minioadmin \
  -e S3_BUCKET=s3manager \
  -e S3_REGION=us-east-1 \
  -e MASTER_API_KEY=my-secret-key \
  -e JWT_SECRET=my-jwt-secret \
  -v s3em_data:/data \
  s3em
```

---

## Configuration

All configuration is done via environment variables:

| Variable | Default | Description |
|---|---|---|
| `SERVER_HOST` | `0.0.0.0` | Bind address |
| `SERVER_PORT` | `4004` | Bind port |
| `DATABASE_URL` | `s3manager.db` | SQLite database path |
| `S3_ENDPOINT` | `http://localhost:9000` | S3-compatible endpoint |
| `S3_REGION` | `us-east-1` | S3 region |
| `S3_BUCKET` | `s3manager` | Target bucket |
| `S3_ACCESS_KEY` | `minioadmin` | S3 access key |
| `S3_SECRET_KEY` | `minioadmin` | S3 secret key |
| `S3_PATH_STYLE` | `true` | Path-style addressing (required for MinIO) |
| `JWT_SECRET` | `change-me-in-production` | JWT signing secret |
| `MASTER_API_KEY` | `dev-key-123` | Master API key |
| `RUST_LOG` | `info` | Log level (`debug`, `trace`, etc.) |

---

## API Reference

### Public endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/health` | Health check |
| `POST` | `/api/auth/login` | Exchange master key for JWT |

### Authenticated endpoints

All other endpoints require `Authorization: Bearer <jwt>` or `X-API-Key: <master_key>`.

| Method | Path | Description |
|---|---|---|
| `PUT` | `/api/files/upload` | Upload a file (multipart) |
| `GET` | `/api/files/` | List files with filters |
| `GET` | `/api/files/{id}` | Get file metadata |
| `GET` | `/api/files/{id}/download` | Download file content |
| `DELETE` | `/api/files/{id}` | Soft-delete a file |
| `GET` | `/api/events/` | SSE real-time event stream |
| `GET` | `/api/events/history` | Paginated event history |

---

## Deployment with Podman Quadlets

The [`quadlets/`](quadlets/) directory contains systemd-style Quadlet files for Podman:

```bash
cp quadlets/* ~/.config/containers/systemd/
systemctl --user daemon-reload
systemctl --user start s3em
```

---

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────────┐
│   Client     │────▶│   Axum API   │────▶│   S3 Backend     │
│ (HTTP/SSE)   │     │  (Rust)      │     │ (MinIO / AWS S3) │
└──────────────┘     ├──────────────┤     └──────────────────┘
                     │  SQLite       │
                     │  (metadata    │
                     │   + events)   │
                     └──────────────┘
```

- **axum** — Async HTTP framework with middleware-based auth
- **rusqlite** — SQLite in WAL mode with connection pooling (`r2d2`)
- **aws-sdk-s3** — Official AWS SDK for S3 operations
- **tokio broadcast** — In-process pub/sub for real-time SSE streaming
- **jsonwebtoken** — JWT encode/decode for stateless authentication

---

## Tech stack

| Layer | Technology |
|---|---|
| Language | Rust (edition 2024) |
| Web framework | [axum](https://github.com/tokio-rs/axum) 0.8 |
| Database | SQLite via [rusqlite](https://github.com/rusqlite/rusqlite) + r2d2 pool |
| Object storage | [aws-sdk-s3](https://github.com/awslabs/aws-sdk-rust) |
| Auth | [jsonwebtoken](https://github.com/Keats/jsonwebtoken) |
| Events | SSE via tokio broadcast + async-stream |
| Container | Multi-stage Docker → Alpine 3.21 |

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.