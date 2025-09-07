.PHONY: litellm
litellm:
	docker run \
		--env-file .env \
		-v $(PWD)/litellm/config.yaml:/app/config.yaml \
		-p 4000:4000 \
		ghcr.io/berriai/litellm:main-latest \
		--config /app/config.yaml --detailed_debug