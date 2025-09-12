package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"syscall"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

// Request/Response structures matching the Rust implementations
type SearchRequest struct {
	Query              string  `json:"query" binding:"required"`
	Topic              *string `json:"topic,omitempty"`
	SearchDepth        *string `json:"search_depth,omitempty"`
	MaxResults         *uint32 `json:"max_results,omitempty"`
	IncludeAnswer      *bool   `json:"include_answer,omitempty"`
	IncludeRawContent  *bool   `json:"include_raw_content,omitempty"`
	IncludeImages      *bool   `json:"include_images,omitempty"`
}

type ExtractRequest struct {
	URLs           []string `json:"urls" binding:"required"`
	IncludeImages  *bool    `json:"include_images,omitempty"`
	IncludeFavicon *bool    `json:"include_favicon,omitempty"`
	ExtractDepth   *string  `json:"extract_depth,omitempty"`
	Format         *string  `json:"format,omitempty"`
	Timeout        *float32 `json:"timeout,omitempty"`
}

type SearchResult struct {
	Title      string   `json:"title"`
	URL        string   `json:"url"`
	Content    string   `json:"content"`
	Score      *float64 `json:"score,omitempty"`
	RawContent *string  `json:"raw_content,omitempty"`
}

type SearchResponse struct {
	Query        string                 `json:"query"`
	Answer       *string                `json:"answer,omitempty"`
	Results      []SearchResult         `json:"results"`
	Images       []string               `json:"images"`
	ResponseTime *float64               `json:"response_time,omitempty"`
	RequestID    *string                `json:"request_id,omitempty"`
	Metadata     map[string]interface{} `json:",inline"`
}

type ExtractResult struct {
	URL           string                 `json:"url"`
	RawContent    string                 `json:"raw_content"`
	Images        *[]string              `json:"images,omitempty"`
	Favicon       *string                `json:"favicon,omitempty"`
	Title         *string                `json:"title,omitempty"`
	ContentLength *int                   `json:"content_length,omitempty"`
	Metadata      map[string]interface{} `json:",inline"`
}

type FailedResult struct {
	URL       string  `json:"url"`
	Error     string  `json:"error"`
	ErrorCode *string `json:"error_code,omitempty"`
}

type ExtractResponse struct {
	Results       []ExtractResult        `json:"results"`
	FailedResults []FailedResult         `json:"failed_results"`
	ResponseTime  *float64               `json:"response_time,omitempty"`
	RequestID     *string                `json:"request_id,omitempty"`
	Metadata      map[string]interface{} `json:",inline"`
}

type ErrorResponse struct {
	Error   string `json:"error"`
	Message string `json:"message,omitempty"`
	Code    int    `json:"code,omitempty"`
}

type TavilyProxy struct {
	client    *http.Client
	apiKey    string
	baseURL   string
	logger    *zap.Logger
}

func NewTavilyProxy(apiKey string, logger *zap.Logger) *TavilyProxy {
	// High-performance HTTP client with connection pooling
	transport := &http.Transport{
		// Connection pooling settings for maximum performance
		MaxIdleConns:        100,
		MaxIdleConnsPerHost: 20,
		IdleConnTimeout:     90 * time.Second,
		
		// TCP connection settings for low latency
		DialContext: (&net.Dialer{
			Timeout:   5 * time.Second,
			KeepAlive: 30 * time.Second,
		}).DialContext,
		
		// HTTP/2 and Keep-Alive settings
		ForceAttemptHTTP2:     true,
		DisableKeepAlives:     false,
		DisableCompression:    false,
		MaxConnsPerHost:       20,
		ResponseHeaderTimeout: 10 * time.Second,
		ExpectContinueTimeout: 1 * time.Second,
		
		// TLS settings for HTTPS performance
		TLSHandshakeTimeout: 5 * time.Second,
	}

	client := &http.Client{
		Transport: transport,
		Timeout:   30 * time.Second,
	}

	return &TavilyProxy{
		client:  client,
		apiKey:  apiKey,
		baseURL: "https://api.tavily.com",
		logger:  logger,
	}
}

func (tp *TavilyProxy) makeRequest(ctx context.Context, endpoint string, requestBody interface{}) ([]byte, error) {
	jsonData, err := json.Marshal(requestBody)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", tp.baseURL+endpoint, bytes.NewBuffer(jsonData))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+tp.apiKey)
	req.Header.Set("User-Agent", "dcmt-tavily-proxy/1.0")

	resp, err := tp.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		return nil, tp.handleErrorResponse(resp.StatusCode, body)
	}

	return body, nil
}

func (tp *TavilyProxy) handleErrorResponse(statusCode int, body []byte) error {
	switch statusCode {
	case http.StatusUnauthorized:
		return fmt.Errorf("invalid API key")
	case http.StatusTooManyRequests:
		return fmt.Errorf("rate limit exceeded")
	case http.StatusRequestTimeout, http.StatusGatewayTimeout:
		return fmt.Errorf("request timeout")
	default:
		return fmt.Errorf("API error %d: %s", statusCode, string(body))
	}
}

func (tp *TavilyProxy) handleSearch(c *gin.Context) {
	requestID := uuid.New().String()
	start := time.Now()
	
	tp.logger.Info("Search request received", 
		zap.String("request_id", requestID),
		zap.String("client_ip", c.ClientIP()),
	)

	var req SearchRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		tp.logger.Warn("Invalid search request", 
			zap.String("request_id", requestID),
			zap.Error(err),
		)
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "invalid_request",
			Message: err.Error(),
			Code:    http.StatusBadRequest,
		})
		return
	}

	// Set defaults matching Rust implementation
	if req.SearchDepth == nil {
		depth := "basic"
		req.SearchDepth = &depth
	}
	if req.MaxResults == nil {
		maxRes := uint32(5)
		req.MaxResults = &maxRes
	}
	if req.IncludeAnswer == nil {
		include := false
		req.IncludeAnswer = &include
	}
	if req.IncludeRawContent == nil {
		include := false
		req.IncludeRawContent = &include
	}
	if req.IncludeImages == nil {
		include := false
		req.IncludeImages = &include
	}

	// Limit max_results to 20 as per API constraints
	if *req.MaxResults > 20 {
		maxRes := uint32(20)
		req.MaxResults = &maxRes
	}

	tp.logger.Debug("Performing search", 
		zap.String("request_id", requestID),
		zap.String("query", req.Query),
		zap.Uint32p("max_results", req.MaxResults),
	)

	responseBody, err := tp.makeRequest(c.Request.Context(), "/search", req)
	if err != nil {
		tp.logger.Error("Search request failed", 
			zap.String("request_id", requestID),
			zap.Error(err),
		)
		
		statusCode := http.StatusInternalServerError
		errorType := "internal_error"
		
		if err.Error() == "invalid API key" {
			statusCode = http.StatusUnauthorized
			errorType = "invalid_api_key"
		} else if err.Error() == "rate limit exceeded" {
			statusCode = http.StatusTooManyRequests
			errorType = "rate_limit"
		} else if err.Error() == "request timeout" {
			statusCode = http.StatusRequestTimeout
			errorType = "timeout"
		}
		
		c.JSON(statusCode, ErrorResponse{
			Error:   errorType,
			Message: err.Error(),
			Code:    statusCode,
		})
		return
	}

	// Parse and enhance response with timing
	var searchResp SearchResponse
	if err := json.Unmarshal(responseBody, &searchResp); err != nil {
		tp.logger.Error("Failed to parse search response", 
			zap.String("request_id", requestID),
			zap.Error(err),
		)
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "invalid_response",
			Message: "Failed to parse Tavily response",
			Code:    http.StatusInternalServerError,
		})
		return
	}

	// Add request timing
	duration := time.Since(start)
	responseTime := duration.Seconds()
	searchResp.ResponseTime = &responseTime
	searchResp.RequestID = &requestID

	tp.logger.Info("Search completed successfully", 
		zap.String("request_id", requestID),
		zap.Int("results_count", len(searchResp.Results)),
		zap.Float64("response_time", responseTime),
	)

	c.JSON(http.StatusOK, searchResp)
}

func (tp *TavilyProxy) handleExtract(c *gin.Context) {
	requestID := uuid.New().String()
	start := time.Now()
	
	tp.logger.Info("Extract request received", 
		zap.String("request_id", requestID),
		zap.String("client_ip", c.ClientIP()),
	)

	var req ExtractRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		tp.logger.Warn("Invalid extract request", 
			zap.String("request_id", requestID),
			zap.Error(err),
		)
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "invalid_request",
			Message: err.Error(),
			Code:    http.StatusBadRequest,
		})
		return
	}

	// Validate URLs
	if len(req.URLs) == 0 {
		c.JSON(http.StatusBadRequest, ErrorResponse{
			Error:   "invalid_request",
			Message: "At least one URL is required",
			Code:    http.StatusBadRequest,
		})
		return
	}

	// Set defaults matching Rust implementation
	if req.IncludeImages == nil {
		include := false
		req.IncludeImages = &include
	}
	if req.IncludeFavicon == nil {
		include := false
		req.IncludeFavicon = &include
	}
	if req.ExtractDepth == nil {
		depth := "basic"
		req.ExtractDepth = &depth
	}
	if req.Format == nil {
		format := "markdown"
		req.Format = &format
	}
	if req.Timeout == nil {
		timeout := float32(15.0)
		req.Timeout = &timeout
	}

	// Validate and clamp timeout (1.0-60.0 seconds)
	if *req.Timeout < 1.0 {
		timeout := float32(1.0)
		req.Timeout = &timeout
	} else if *req.Timeout > 60.0 {
		timeout := float32(60.0)
		req.Timeout = &timeout
	}

	tp.logger.Debug("Performing extraction", 
		zap.String("request_id", requestID),
		zap.Int("urls_count", len(req.URLs)),
		zap.Float32p("timeout", req.Timeout),
	)

	responseBody, err := tp.makeRequest(c.Request.Context(), "/extract", req)
	if err != nil {
		tp.logger.Error("Extract request failed", 
			zap.String("request_id", requestID),
			zap.Error(err),
		)
		
		statusCode := http.StatusInternalServerError
		errorType := "internal_error"
		
		if err.Error() == "invalid API key" {
			statusCode = http.StatusUnauthorized
			errorType = "invalid_api_key"
		} else if err.Error() == "rate limit exceeded" {
			statusCode = http.StatusTooManyRequests
			errorType = "rate_limit"
		} else if err.Error() == "request timeout" {
			statusCode = http.StatusRequestTimeout
			errorType = "timeout"
		}
		
		c.JSON(statusCode, ErrorResponse{
			Error:   errorType,
			Message: err.Error(),
			Code:    statusCode,
		})
		return
	}

	// Parse and enhance response with timing
	var extractResp ExtractResponse
	if err := json.Unmarshal(responseBody, &extractResp); err != nil {
		tp.logger.Error("Failed to parse extract response", 
			zap.String("request_id", requestID),
			zap.Error(err),
		)
		c.JSON(http.StatusInternalServerError, ErrorResponse{
			Error:   "invalid_response",
			Message: "Failed to parse Tavily response",
			Code:    http.StatusInternalServerError,
		})
		return
	}

	// Add request timing
	duration := time.Since(start)
	responseTime := duration.Seconds()
	extractResp.ResponseTime = &responseTime
	extractResp.RequestID = &requestID

	tp.logger.Info("Extract completed successfully", 
		zap.String("request_id", requestID),
		zap.Int("success_count", len(extractResp.Results)),
		zap.Int("failure_count", len(extractResp.FailedResults)),
		zap.Float64("response_time", responseTime),
	)

	c.JSON(http.StatusOK, extractResp)
}

func (tp *TavilyProxy) handleHealth(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status":    "healthy",
		"service":   "tavily-proxy",
		"version":   "1.0.0",
		"timestamp": time.Now().UTC().Format(time.RFC3339),
	})
}

func setupLogger() *zap.Logger {
	config := zap.NewProductionConfig()
	config.Level = zap.NewAtomicLevelAt(zap.InfoLevel)
	config.OutputPaths = []string{"stdout"}
	config.ErrorOutputPaths = []string{"stderr"}
	config.EncoderConfig.TimeKey = "timestamp"
	config.EncoderConfig.EncodeTime = zapcore.ISO8601TimeEncoder

	logger, _ := config.Build()
	return logger
}

func main() {
	// Setup structured logging
	logger := setupLogger()
	defer logger.Sync()

	// Get configuration from environment
	apiKey := os.Getenv("TAVILY_API_KEY")
	if apiKey == "" {
		logger.Fatal("TAVILY_API_KEY environment variable is required")
	}

	port := os.Getenv("TAVILY_PROXY_PORT")
	if port == "" {
		port = "8082"
	}

	// Validate port
	if _, err := strconv.Atoi(port); err != nil {
		logger.Fatal("Invalid TAVILY_PROXY_PORT", zap.String("port", port), zap.Error(err))
	}

	// Initialize Tavily proxy
	proxy := NewTavilyProxy(apiKey, logger)

	// Setup Gin in production mode
	gin.SetMode(gin.ReleaseMode)
	router := gin.New()

	// Add middleware for logging and recovery
	router.Use(gin.LoggerWithConfig(gin.LoggerConfig{
		SkipPaths: []string{"/health"},
	}))
	router.Use(gin.Recovery())

	// Add CORS middleware for web client support
	router.Use(func(c *gin.Context) {
		c.Header("Access-Control-Allow-Origin", "*")
		c.Header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		c.Header("Access-Control-Allow-Headers", "Content-Type, Authorization")
		
		if c.Request.Method == "OPTIONS" {
			c.AbortWithStatus(http.StatusNoContent)
			return
		}
		
		c.Next()
	})

	// API routes
	router.POST("/search", proxy.handleSearch)
	router.POST("/extract", proxy.handleExtract)
	router.GET("/health", proxy.handleHealth)

	// Setup HTTP server with optimized settings
	server := &http.Server{
		Addr:         ":" + port,
		Handler:      router,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 30 * time.Second,
		IdleTimeout:  60 * time.Second,
		// Optimize for high concurrent connections
		MaxHeaderBytes: 1 << 20, // 1MB
	}

	// Graceful shutdown handling
	go func() {
		logger.Info("Starting Tavily proxy server", 
			zap.String("port", port),
			zap.String("base_url", proxy.baseURL),
		)
		
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Fatal("Failed to start server", zap.Error(err))
		}
	}()

	// Wait for interrupt signal to gracefully shutdown
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit
	
	logger.Info("Shutting down Tavily proxy server...")

	// Give outstanding requests 30 seconds to complete
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	
	if err := server.Shutdown(ctx); err != nil {
		logger.Error("Server forced to shutdown", zap.Error(err))
	} else {
		logger.Info("Server shutdown complete")
	}
}