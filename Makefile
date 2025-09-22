.PHONY: litellm docker-build docker-run ssl-setup tavily-proxy pull docker-clean docker-restart prod-build volume-create
litellm:
	docker run \
		--env-file .env \
		-v $(PWD)/litellm/config.yaml:/app/config.yaml \
		-p 4000:4000 \
		ghcr.io/berriai/litellm:main-latest \
		--config /app/config.yaml --detailed_debug

pull:
	@git pull origin main

docker-build:
	export $$(grep -v '^#' .env | xargs) && \
	docker build \
		--build-arg VITE_CLERK_PUBLISHABLE_KEY=$$VITE_CLERK_PUBLISHABLE_KEY \
		--build-arg VITE_CLERK_SIGN_IN_URL=$$VITE_CLERK_SIGN_IN_URL \
		-t dcmt-editor:latest .


docker-run:
	@docker volume inspect dcmt-workspace >/dev/null 2>&1 || docker volume create dcmt-workspace >/dev/null
	docker run \
		--env-file .env \
		-p 8080:3000 \
		-v dcmt-workspace:/app/workspace \
		--name dcmt-editor \
		--rm \
		-d \
		dcmt-editor:latest

docker-clean:
	@docker rm -f dcmt-editor

docker-restart: docker-clean pull docker-build docker-run

prod-build:
	export $$(grep -v '^#' .env | xargs) && \
	docker build \
		--build-arg VITE_CLERK_PUBLISHABLE_KEY=$$VITE_CLERK_PUBLISHABLE_KEY \
		--build-arg VITE_CLERK_SIGN_IN_URL=$$VITE_CLERK_SIGN_IN_URL \
		-t hamzaawan88/dcmt:ide-v2-latest .


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

volume-create:
	@docker volume inspect dcmt-workspace >/dev/null 2>&1 || docker volume create dcmt-workspace