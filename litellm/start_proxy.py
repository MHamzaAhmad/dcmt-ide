import uvicorn
from litellm.proxy.proxy_server import app
import yaml
import os

# Manually load config file
config_path = "/app/litellm/config.yaml"
try:
    with open(config_path, "r") as config_file:
        config = yaml.safe_load(config_file)
    print(f"Loaded config from {config_path}: {config.keys()}")
    # Set config in litellm's internal state (mimics ProxyConfig behavior)
    from litellm.proxy import _config
    _config.config = config
except FileNotFoundError:
    print(f"Config file {config_path} not found. Falling back to env vars.")
    config = None


if __name__ == "__main__":
    uvicorn.run(
        app,
        uds="/app/litellm.sock",
        workers=1,
        log_level="info"
    )