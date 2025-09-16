#!/usr/bin/env python3
"""
LiteLLM UDS startup script
This script explicitly loads the LiteLLM config before starting the UDS server
"""

import asyncio
import os
import sys
from pathlib import Path

def main():
    # Set working directory to app root
    os.chdir('/app')

    # Explicitly load config
    print("Loading LiteLLM config from /app/litellm/config.yaml")
    from litellm.proxy.proxy_server import app, prerun_checks
    from litellm.proxy.utils import ProxyConfig

    proxy_config = ProxyConfig()
    config_path = "/app/litellm/config.yaml"

    if not os.path.exists(config_path):
        print(f"ERROR: Config file not found at {config_path}")
        sys.exit(1)

    try:
        config = proxy_config.load_config(config_file=config_path)
        print("LiteLLM config loaded successfully")
    except Exception as e:
        print(f"ERROR: Failed to load config: {e}")
        sys.exit(1)

    # Run pre-startup checks
    print("Running pre-startup checks...")
    try:
        asyncio.run(prerun_checks())
        print("Pre-startup checks completed")
    except Exception as e:
        print(f"WARNING: Pre-startup checks failed: {e}")

    # Set socket permissions after uvicorn creates it
    socket_path = "/app/litellm.sock"

    print(f"Starting LiteLLM proxy on UDS: {socket_path}")

    # Import uvicorn here to avoid any path issues
    import uvicorn

    # Run the server
    uvicorn.run(
        app,
        uds=socket_path,
        workers=1,
        log_level="info",
        access_log=False
    )

if __name__ == "__main__":
    main()