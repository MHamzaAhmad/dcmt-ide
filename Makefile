.PHONY: litellm docker-build docker-run ssl-setup tavily-proxy
litellm:
	docker run \
		--env-file .env \
		-v $(PWD)/litellm/config.yaml:/app/config.yaml \
		-p 4000:4000 \
		ghcr.io/berriai/litellm:main-latest \
		--config /app/config.yaml --detailed_debug

docker-build:
	export $$(grep -v '^#' .env | xargs) && \
	docker build \
		--build-arg VITE_API_BASE_URL=$$VITE_API_BASE_URL \
		--build-arg VITE_LITELLM_BASE_URL=$$VITE_LITELLM_BASE_URL \
		-t dcmt-editor:latest .


docker-run:
	docker run \
		--env-file .env \
		-p 8080:80 \
		-v $(PWD)/workspace:/app/workspace \
		-v dcmt-logs:/app/logs \
		--name dcmt-editor \
		--rm \
		-d \
		dcmt-editor:latest

ssl-setup:
	@if [ "$$(id -u)" -ne 0 ]; then \
		echo "❌ This target must be run as root. Use: sudo make ssl-setup"; \
		exit 1; \
	fi
	@./docker/generate-ssl.sh

tavily-proxy:
	@echo "🚀 Building and running Tavily proxy with Docker..."
	@echo "🔨 Building Docker image..."
	@docker build -t tavily-proxy:latest lib/tavily/ --progress=plain
	@echo "✅ Build complete. Starting proxy container..."
	@docker run \
		--env-file .env \
		-p 8082:8082 \
		--name tavily-proxy \
		--rm \
		tavily-proxy:latest