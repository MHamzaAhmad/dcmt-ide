import asyncio
import uvicorn
import yaml
import os
from litellm.proxy.proxy_server import app, startup_event

# Manually load config file
config_path = "/app/litellm/config.yaml"
try:
    with open(config_path, "r") as config_file:
        config = yaml.safe_load(config_file)
    print(f"Loaded config from {config_path}: {config.keys()}")
    # Set config in litellm's internal state
    from litellm.proxy import _config
    _config.config = config
except FileNotFoundError:
    print(f"Config file {config_path} not found. Falling back to env vars.")
    config = None

# Optional: Set env vars for fallback (e.g., master key or model list)
os.environ["LITELLM_MASTER_KEY"] = "sk-1234yourmasterkey"  # Replace with your key if needed
# Example: os.environ["LITELLM_MODEL_LIST"] = '[{"model_name": "gpt-3.5-turbo", "litellm_params": {"model": "openai/gpt-3.5-turbo", "api_key": "os.environ/OPENAI_API_KEY"}}]'

# Run startup event to initialize
asyncio.run(startup_event())

if __name__ == "__main__":
    uvicorn.run(
        app,
        uds="/app/litellm.sock",
        workers=1,
        log_level="info"
    )