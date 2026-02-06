package mycelix

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"sync"
	"sync/atomic"
	"time"

	"github.com/gorilla/websocket"
)

// ClientConfig holds configuration for the Living Protocol client.
type ClientConfig struct {
	// URL is the WebSocket server URL (e.g., "ws://localhost:8888").
	URL string
	// RESTURL is the optional REST API URL. If empty, derived from URL.
	RESTURL string
	// Reconnect enables automatic reconnection on disconnect.
	Reconnect bool
	// ReconnectDelay is the initial delay between reconnection attempts.
	ReconnectDelay time.Duration
	// MaxReconnectAttempts is the maximum number of reconnection attempts.
	MaxReconnectAttempts int
	// PingInterval is the interval between WebSocket ping messages.
	PingInterval time.Duration
	// RequestTimeout is the timeout for RPC requests.
	RequestTimeout time.Duration
	// Dialer is the WebSocket dialer to use. If nil, uses default.
	Dialer *websocket.Dialer
}

// DefaultConfig returns a ClientConfig with sensible defaults.
func DefaultConfig(url string) ClientConfig {
	return ClientConfig{
		URL:                  url,
		Reconnect:            true,
		ReconnectDelay:       time.Second,
		MaxReconnectAttempts: 10,
		PingInterval:         30 * time.Second,
		RequestTimeout:       10 * time.Second,
	}
}

// LivingProtocolClient is a WebSocket client for the Living Protocol.
type LivingProtocolClient struct {
	config        ClientConfig
	conn          *websocket.Conn
	connMu        sync.RWMutex
	subscriptions *SubscriptionManager
	pendingReqs   map[string]chan *rpcResponse
	pendingMu     sync.Mutex
	requestID     atomic.Uint64
	connected     atomic.Bool
	closed        atomic.Bool
	closeChan     chan struct{}
	wg            sync.WaitGroup
}

// rpcRequest is the structure for RPC requests.
type rpcRequest struct {
	ID     string      `json:"id"`
	Method string      `json:"method"`
	Params interface{} `json:"params,omitempty"`
}

// rpcResponse is the structure for RPC responses.
type rpcResponse struct {
	ID     string          `json:"id"`
	Result json.RawMessage `json:"result,omitempty"`
	Error  *RpcError       `json:"error,omitempty"`
}

// Connect creates a new client and connects to the server.
func Connect(wsURL string) (*LivingProtocolClient, error) {
	return ConnectWithConfig(DefaultConfig(wsURL))
}

// ConnectWithConfig creates a new client with custom configuration.
func ConnectWithConfig(config ClientConfig) (*LivingProtocolClient, error) {
	client := &LivingProtocolClient{
		config:        config,
		subscriptions: NewSubscriptionManager(),
		pendingReqs:   make(map[string]chan *rpcResponse),
		closeChan:     make(chan struct{}),
	}

	if err := client.connect(); err != nil {
		return nil, err
	}

	return client, nil
}

// connect establishes the WebSocket connection.
func (c *LivingProtocolClient) connect() error {
	dialer := c.config.Dialer
	if dialer == nil {
		dialer = websocket.DefaultDialer
	}

	conn, _, err := dialer.Dial(c.config.URL, nil)
	if err != nil {
		return fmt.Errorf("connecting to %s: %w", c.config.URL, err)
	}

	c.connMu.Lock()
	c.conn = conn
	c.connMu.Unlock()
	c.connected.Store(true)

	// Start background goroutines
	c.wg.Add(2)
	go c.receiveLoop()
	go c.pingLoop()

	return nil
}

// Close closes the client connection.
func (c *LivingProtocolClient) Close() error {
	if c.closed.Swap(true) {
		return nil // Already closed
	}

	close(c.closeChan)
	c.connected.Store(false)

	c.connMu.Lock()
	conn := c.conn
	c.conn = nil
	c.connMu.Unlock()

	var err error
	if conn != nil {
		err = conn.Close()
	}

	// Wait for goroutines to finish
	c.wg.Wait()

	// Close all subscriptions
	c.subscriptions.CloseAll()

	// Cancel all pending requests
	c.pendingMu.Lock()
	for _, ch := range c.pendingReqs {
		close(ch)
	}
	c.pendingReqs = make(map[string]chan *rpcResponse)
	c.pendingMu.Unlock()

	return err
}

// Connected returns true if the client is connected.
func (c *LivingProtocolClient) Connected() bool {
	return c.connected.Load()
}

// receiveLoop handles incoming WebSocket messages.
func (c *LivingProtocolClient) receiveLoop() {
	defer c.wg.Done()

	for {
		select {
		case <-c.closeChan:
			return
		default:
		}

		c.connMu.RLock()
		conn := c.conn
		c.connMu.RUnlock()

		if conn == nil {
			return
		}

		_, message, err := conn.ReadMessage()
		if err != nil {
			if c.closed.Load() {
				return
			}

			c.connected.Store(false)

			if c.config.Reconnect && !c.closed.Load() {
				go c.attemptReconnect()
			}
			return
		}

		c.handleMessage(message)
	}
}

// handleMessage processes an incoming message.
func (c *LivingProtocolClient) handleMessage(data []byte) {
	// Try to parse as RPC response
	var resp rpcResponse
	if err := json.Unmarshal(data, &resp); err == nil && resp.ID != "" {
		c.pendingMu.Lock()
		if ch, ok := c.pendingReqs[resp.ID]; ok {
			delete(c.pendingReqs, resp.ID)
			c.pendingMu.Unlock()
			ch <- &resp
			return
		}
		c.pendingMu.Unlock()
	}

	// Try to parse as pong
	var pong struct {
		Type string `json:"type"`
	}
	if err := json.Unmarshal(data, &pong); err == nil && pong.Type == "pong" {
		return
	}

	// Parse as event
	event, err := ParseEvent(data)
	if err != nil {
		return
	}

	c.subscriptions.Publish(event)
}

// pingLoop sends periodic ping messages.
func (c *LivingProtocolClient) pingLoop() {
	defer c.wg.Done()

	ticker := time.NewTicker(c.config.PingInterval)
	defer ticker.Stop()

	for {
		select {
		case <-c.closeChan:
			return
		case <-ticker.C:
			if !c.connected.Load() {
				continue
			}

			c.connMu.RLock()
			conn := c.conn
			c.connMu.RUnlock()

			if conn != nil {
				ping := map[string]string{"type": "ping"}
				data, _ := json.Marshal(ping)
				_ = conn.WriteMessage(websocket.TextMessage, data)
			}
		}
	}
}

// attemptReconnect tries to reconnect with exponential backoff.
func (c *LivingProtocolClient) attemptReconnect() {
	delay := c.config.ReconnectDelay

	for attempt := 1; attempt <= c.config.MaxReconnectAttempts; attempt++ {
		if c.closed.Load() {
			return
		}

		time.Sleep(delay)

		if err := c.connect(); err == nil {
			return
		}

		// Exponential backoff with cap
		delay *= 2
		if delay > time.Minute {
			delay = time.Minute
		}
	}
}

// rpcCall makes an RPC call to the server.
func (c *LivingProtocolClient) rpcCall(ctx context.Context, method string, params interface{}) (json.RawMessage, error) {
	if !c.connected.Load() {
		return nil, errors.New("not connected")
	}

	// Generate request ID
	id := strconv.FormatUint(c.requestID.Add(1), 10)

	// Create response channel
	respChan := make(chan *rpcResponse, 1)
	c.pendingMu.Lock()
	c.pendingReqs[id] = respChan
	c.pendingMu.Unlock()

	defer func() {
		c.pendingMu.Lock()
		delete(c.pendingReqs, id)
		c.pendingMu.Unlock()
	}()

	// Build request
	req := rpcRequest{
		ID:     id,
		Method: method,
		Params: params,
	}

	data, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("marshaling request: %w", err)
	}

	// Send request
	c.connMu.RLock()
	conn := c.conn
	c.connMu.RUnlock()

	if conn == nil {
		return nil, errors.New("not connected")
	}

	if err := conn.WriteMessage(websocket.TextMessage, data); err != nil {
		return nil, fmt.Errorf("sending request: %w", err)
	}

	// Wait for response with timeout
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	case resp, ok := <-respChan:
		if !ok {
			return nil, errors.New("connection closed")
		}
		if resp.Error != nil {
			return nil, resp.Error
		}
		return resp.Result, nil
	case <-time.After(c.config.RequestTimeout):
		return nil, fmt.Errorf("request timeout for method: %s", method)
	}
}

// GetCurrentState returns the current cycle state.
func (c *LivingProtocolClient) GetCurrentState(ctx context.Context) (*CycleState, error) {
	result, err := c.rpcCall(ctx, "getCycleState", nil)
	if err != nil {
		return nil, err
	}

	var state CycleState
	if err := json.Unmarshal(result, &state); err != nil {
		return nil, fmt.Errorf("parsing response: %w", err)
	}

	return &state, nil
}

// GetCurrentPhase returns the current cycle phase.
func (c *LivingProtocolClient) GetCurrentPhase(ctx context.Context) (CyclePhase, error) {
	result, err := c.rpcCall(ctx, "getCurrentPhase", nil)
	if err != nil {
		return "", err
	}

	var phase CyclePhase
	if err := json.Unmarshal(result, &phase); err != nil {
		return "", fmt.Errorf("parsing response: %w", err)
	}

	return phase, nil
}

// GetCycleNumber returns the current cycle number.
func (c *LivingProtocolClient) GetCycleNumber(ctx context.Context) (uint64, error) {
	result, err := c.rpcCall(ctx, "getCycleNumber", nil)
	if err != nil {
		return 0, err
	}

	var num uint64
	if err := json.Unmarshal(result, &num); err != nil {
		return 0, fmt.Errorf("parsing response: %w", err)
	}

	return num, nil
}

// GetTransitionHistory returns the phase transition history.
func (c *LivingProtocolClient) GetTransitionHistory(ctx context.Context) ([]PhaseTransition, error) {
	result, err := c.rpcCall(ctx, "getTransitionHistory", nil)
	if err != nil {
		return nil, err
	}

	var transitions []PhaseTransition
	if err := json.Unmarshal(result, &transitions); err != nil {
		return nil, fmt.Errorf("parsing response: %w", err)
	}

	return transitions, nil
}

// GetPhaseMetrics returns metrics for a specific phase.
func (c *LivingProtocolClient) GetPhaseMetrics(ctx context.Context, phase CyclePhase) (*PhaseMetrics, error) {
	params := map[string]string{"phase": string(phase)}
	result, err := c.rpcCall(ctx, "getPhaseMetrics", params)
	if err != nil {
		return nil, err
	}

	var metrics PhaseMetrics
	if err := json.Unmarshal(result, &metrics); err != nil {
		return nil, fmt.Errorf("parsing response: %w", err)
	}

	return &metrics, nil
}

// IsOperationPermitted checks if an operation is permitted in the current phase.
func (c *LivingProtocolClient) IsOperationPermitted(ctx context.Context, operation string) (bool, error) {
	params := map[string]string{"operation": operation}
	result, err := c.rpcCall(ctx, "isOperationPermitted", params)
	if err != nil {
		return false, err
	}

	var permitted bool
	if err := json.Unmarshal(result, &permitted); err != nil {
		return false, fmt.Errorf("parsing response: %w", err)
	}

	return permitted, nil
}

// Subscribe creates a subscription to all events.
func (c *LivingProtocolClient) Subscribe() *Subscription {
	return c.subscriptions.SubscribeAll(100)
}

// SubscribeFiltered creates a subscription with filters.
func (c *LivingProtocolClient) SubscribeFiltered(filters ...EventFilter) *Subscription {
	return c.subscriptions.Subscribe(100, filters...)
}

// SubscribePhaseTransitions creates a subscription for phase transitions.
func (c *LivingProtocolClient) SubscribePhaseTransitions() *Subscription {
	return c.subscriptions.SubscribePhaseTransitions(100)
}

// SubscribeCycleStarts creates a subscription for cycle starts.
func (c *LivingProtocolClient) SubscribeCycleStarts() *Subscription {
	return c.subscriptions.SubscribeCycleStarts(100)
}

// REST API methods

// getRESTURL returns the REST API base URL.
func (c *LivingProtocolClient) getRESTURL() string {
	if c.config.RESTURL != "" {
		return c.config.RESTURL
	}

	// Derive from WebSocket URL
	wsURL := c.config.URL
	u, err := url.Parse(wsURL)
	if err != nil {
		return "http://localhost:8889"
	}

	// Change scheme and increment port
	scheme := "http"
	if u.Scheme == "wss" {
		scheme = "https"
	}

	host := u.Hostname()
	port := u.Port()
	if port != "" {
		if p, err := strconv.Atoi(port); err == nil {
			port = strconv.Itoa(p + 1)
		}
	} else {
		port = "8889"
	}

	return fmt.Sprintf("%s://%s:%s", scheme, host, port)
}

// RESTGetState gets the current state via REST API.
func (c *LivingProtocolClient) RESTGetState(ctx context.Context) (*CycleState, error) {
	url := c.getRESTURL() + "/api/v1/state"
	return restGet[CycleState](ctx, url)
}

// RESTGetPhase gets the current phase via REST API.
func (c *LivingProtocolClient) RESTGetPhase(ctx context.Context) (CyclePhase, error) {
	url := c.getRESTURL() + "/api/v1/phase"

	var resp struct {
		Phase CyclePhase `json:"phase"`
	}
	if err := restGetInto(ctx, url, &resp); err != nil {
		return "", err
	}

	return resp.Phase, nil
}

// RESTGetHistory gets the transition history via REST API.
func (c *LivingProtocolClient) RESTGetHistory(ctx context.Context) ([]PhaseTransition, error) {
	url := c.getRESTURL() + "/api/v1/history"

	var resp struct {
		Transitions []PhaseTransition `json:"transitions"`
	}
	if err := restGetInto(ctx, url, &resp); err != nil {
		return nil, err
	}

	return resp.Transitions, nil
}

// RESTGetMetrics gets phase metrics via REST API.
func (c *LivingProtocolClient) RESTGetMetrics(ctx context.Context, phase CyclePhase) (*PhaseMetrics, error) {
	url := fmt.Sprintf("%s/api/v1/metrics/%s", c.getRESTURL(), phase)
	return restGet[PhaseMetrics](ctx, url)
}

// restGet makes a GET request and returns the parsed response.
func restGet[T any](ctx context.Context, url string) (*T, error) {
	var result T
	if err := restGetInto(ctx, url, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// restGetInto makes a GET request and unmarshals into the provided value.
func restGetInto(ctx context.Context, url string, v interface{}) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return fmt.Errorf("creating request: %w", err)
	}

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("making request: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(body))
	}

	if err := json.NewDecoder(resp.Body).Decode(v); err != nil {
		return fmt.Errorf("decoding response: %w", err)
	}

	return nil
}
