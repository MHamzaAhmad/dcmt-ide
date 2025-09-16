import asyncio
from litellm.proxy.proxy_server import app, prerun_checks  # Import to trigger any pre-loads
from litellm.proxy.utils import ProxyConfig  # Utility for loading config
import uvicorn

# Load config explicitly
proxy_config = ProxyConfig()
config_path = "/app/litellm/config.yaml"  # Set your config path here
config = proxy_config.load_config(config_file=config_path)

# Optional: Run any pre-startup checks or initializations if needed
asyncio.run(prerun_checks())

if __name__ == "__main__":
    uvicorn.run(app, uds="/app/litellm.sock", workers=1)