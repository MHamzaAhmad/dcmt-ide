.PHONY: litellm docker-build docker-run
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
		-p 80:80 \
		-p 443:443 \
		-v dcmt-workspace:/app/workspace \
		-v dcmt-logs:/app/logs \
		--name dcmt-editor \
		--rm \
		dcmt-editor:latest