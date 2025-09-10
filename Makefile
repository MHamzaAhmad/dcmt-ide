.PHONY: litellm docker-build docker-run ssl-setup
litellm:
	docker run \
		--env-file .env \
		-v $(PWD)/litellm/config.yaml:/app/config.yaml \
		-p 4000:4000 \
		ghcr.io/berriai/litellm:main-latest \
		--config /app/config.yaml --detailed_debug

docker-build:
	docker build -t dcmt-editor:latest .

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
