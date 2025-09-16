import asyncio
import uvicorn
from litellm.proxy.proxy_server import app, startup  # Import app and startup (if named 'startup' in your version)
from litellm.proxy.proxy_server import ProxyConfig  # Correct import for ProxyConfig

# Load config explicitly
config_path = "/app/litellm/config.yaml"  # Your config path
proxy_config = ProxyConfig()
loaded_config = proxy_config.load_config(config_file=config_path)

# Optional: Print confirmation (for debugging)
print(f"Loaded config from {config_path}: {loaded_config.keys()}")

# Optional: Run startup manually if needed (uvicorn will trigger it automatically, but this ensures init)
asyncio.run(startup())  # Assuming the startup function is named 'startup'; check logs if error and remove if not needed

if __name__ == "__main__":
    uvicorn.run(
        app,
        uds="/app/litellm.sock",
        workers=1,
        log_level="info"
    )