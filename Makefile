.PHONY: build test clean dev dev-web

# Build release versions
build:
	cargo build --release --workspace

# Run tests
test:
	cargo test --workspace

# Clean build artifacts
clean:
	cargo clean

# Start desktop development server
dev:
	./scripts/dev.sh desktop

# Start web development server
dev-web:
	./scripts/dev.sh web