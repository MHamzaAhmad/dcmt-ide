"""
Custom authentication for LiteLLM using Clerk Python SDK
"""
import os
from fastapi import Request, HTTPException
from litellm.proxy._types import UserAPIKeyAuth
from clerk_backend_api import Clerk


async def user_api_key_auth(request: Request, api_key: str) -> UserAPIKeyAuth:
    """
    Custom authentication function for LiteLLM using Clerk session tokens

    Args:
        request: FastAPI request object
        api_key: API key from Authorization header (Bearer token)

    Returns:
        UserAPIKeyAuth: Authentication object for LiteLLM

    Raises:
        Exception: If authentication fails
    """
    try:
        # Get Clerk secret key from environment
        clerk_secret_key = os.getenv('CLERK_SECRET_KEY')
        if not clerk_secret_key:
            raise Exception("CLERK_SECRET_KEY not configured")

        # Initialize Clerk SDK
        clerk = Clerk(bearer_auth=clerk_secret_key)

        # Authenticate the request using Clerk
        request_state = clerk.authenticate_request(request)

        # Check if user is signed in
        if not request_state.is_signed_in:
            raise Exception("Invalid or expired session token")

        # Return authentication object for LiteLLM
        return UserAPIKeyAuth(api_key=api_key)

    except Exception as e:
        # Log the error for debugging
        print(f"Authentication failed: {str(e)}")
        raise Exception(f"Authentication failed: {str(e)}")