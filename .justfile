default: build

# Build the project
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Run linter
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
fmt:
    cargo fmt --check

# Format code
fmt-fix:
    cargo fmt

# Run the server locally (requires MinIO accessible)
run:
    cargo run

# Run with Docker Compose (starts MinIO + s3em)
up:
    docker compose up -d

# Stop Docker Compose services
down:
    docker compose down

# View logs
logs:
    docker compose logs -f s3em

# Rebuild and restart Docker Compose
rebuild: down
    docker compose build --no-cache
    docker compose up -d

# Build Docker image
docker-build:
    docker build -t s3em .

# Clean build artifacts
clean:
    cargo clean

# Run all checks (fmt + lint + test + build)
check: fmt lint test build