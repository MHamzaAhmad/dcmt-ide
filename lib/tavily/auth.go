package main

import (
	"context"
	"net/http"
	"strings"

	"github.com/clerk/clerk-sdk-go/v2"
	"github.com/clerk/clerk-sdk-go/v2/jwt"
	"github.com/gin-gonic/gin"
	"go.uber.org/zap"
)

// ClerkAuthMiddleware creates a Gin middleware that validates Clerk JWT tokens
func ClerkAuthMiddleware(secretKey string, logger *zap.Logger) gin.HandlerFunc {
	// Configure Clerk client once
	clerk.SetKey(secretKey)

	return func(c *gin.Context) {
		// Extract the token from Authorization header
		authHeader := c.GetHeader("Authorization")
		if authHeader == "" {
			logger.Warn("Missing Authorization header",
				zap.String("path", c.Request.URL.Path),
				zap.String("client_ip", c.ClientIP()),
			)
			c.JSON(http.StatusUnauthorized, ErrorResponse{
				Error:   "unauthorized",
				Message: "Authorization header is required",
				Code:    http.StatusUnauthorized,
			})
			c.Abort()
			return
		}

		// Check for Bearer token format
		if !strings.HasPrefix(authHeader, "Bearer ") {
			logger.Warn("Invalid Authorization header format",
				zap.String("path", c.Request.URL.Path),
				zap.String("client_ip", c.ClientIP()),
			)
			c.JSON(http.StatusUnauthorized, ErrorResponse{
				Error:   "unauthorized",
				Message: "Authorization header must use Bearer format",
				Code:    http.StatusUnauthorized,
			})
			c.Abort()
			return
		}

		// Extract the token
		token := strings.TrimPrefix(authHeader, "Bearer ")
		if token == "" {
			logger.Warn("Empty Bearer token",
				zap.String("path", c.Request.URL.Path),
				zap.String("client_ip", c.ClientIP()),
			)
			c.JSON(http.StatusUnauthorized, ErrorResponse{
				Error:   "unauthorized",
				Message: "Bearer token cannot be empty",
				Code:    http.StatusUnauthorized,
			})
			c.Abort()
			return
		}

		// Verify the JWT token
		claims, err := jwt.Verify(context.Background(), &jwt.VerifyParams{
			Token: token,
		})
		if err != nil {
			logger.Warn("JWT verification failed",
				zap.String("path", c.Request.URL.Path),
				zap.String("client_ip", c.ClientIP()),
				zap.Error(err),
			)
			c.JSON(http.StatusForbidden, ErrorResponse{
				Error:   "forbidden",
				Message: "Invalid or expired token",
				Code:    http.StatusForbidden,
			})
			c.Abort()
			return
		}

		// Store user information in context for potential use in handlers
		c.Set("user_id", claims.Subject)
		c.Set("session_id", claims.SessionID)

		logger.Debug("Authentication successful",
			zap.String("path", c.Request.URL.Path),
			zap.String("user_id", claims.Subject),
			zap.String("session_id", claims.SessionID),
		)

		// Continue to the next handler
		c.Next()
	}
}