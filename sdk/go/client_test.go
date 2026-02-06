package mycelix

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/gorilla/websocket"
)

func TestCyclePhase(t *testing.T) {
	t.Run("DurationDays", func(t *testing.T) {
		tests := []struct {
			phase    CyclePhase
			expected int
		}{
			{PhaseShadow, 2},
			{PhaseComposting, 5},
			{PhaseLiminal, 3},
			{PhaseNegativeCapability, 3},
			{PhaseEros, 4},
			{PhaseCoCreation, 7},
			{PhaseBeauty, 2},
			{PhaseEmergentPersonhood, 1},
			{PhaseKenosis, 1},
		}

		for _, tt := range tests {
			if got := tt.phase.DurationDays(); got != tt.expected {
				t.Errorf("%s.DurationDays() = %d, want %d", tt.phase, got, tt.expected)
			}
		}
	})

	t.Run("TotalCycleIs28Days", func(t *testing.T) {
		total := TotalCycleDays()
		if total != 28 {
			t.Errorf("TotalCycleDays() = %d, want 28", total)
		}
	})

	t.Run("Next", func(t *testing.T) {
		if got := PhaseShadow.Next(); got != PhaseComposting {
			t.Errorf("Shadow.Next() = %s, want Composting", got)
		}
		if got := PhaseKenosis.Next(); got != PhaseShadow {
			t.Errorf("Kenosis.Next() = %s, want Shadow", got)
		}
	})

	t.Run("Prev", func(t *testing.T) {
		if got := PhaseComposting.Prev(); got != PhaseShadow {
			t.Errorf("Composting.Prev() = %s, want Shadow", got)
		}
		if got := PhaseShadow.Prev(); got != PhaseKenosis {
			t.Errorf("Shadow.Prev() = %s, want Kenosis", got)
		}
	})

	t.Run("Valid", func(t *testing.T) {
		if !PhaseShadow.Valid() {
			t.Error("Shadow should be valid")
		}
		if CyclePhase("Invalid").Valid() {
			t.Error("Invalid phase should not be valid")
		}
	})
}

func TestCycleState(t *testing.T) {
	t.Run("UnmarshalJSON", func(t *testing.T) {
		data := []byte(`{
			"cycleNumber": 3,
			"currentPhase": "Liminal",
			"phaseStarted": "2024-01-15T10:00:00Z",
			"cycleStarted": "2024-01-01T00:00:00Z",
			"phaseDay": 1
		}`)

		var state CycleState
		if err := json.Unmarshal(data, &state); err != nil {
			t.Fatalf("UnmarshalJSON failed: %v", err)
		}

		if state.CycleNumber != 3 {
			t.Errorf("CycleNumber = %d, want 3", state.CycleNumber)
		}
		if state.CurrentPhase != PhaseLiminal {
			t.Errorf("CurrentPhase = %s, want Liminal", state.CurrentPhase)
		}
		if state.PhaseDay != 1 {
			t.Errorf("PhaseDay = %d, want 1", state.PhaseDay)
		}
	})
}

func TestPhaseMetrics(t *testing.T) {
	t.Run("UnmarshalJSON_CamelCase", func(t *testing.T) {
		data := []byte(`{
			"activeAgents": 100,
			"spectralK": 0.75,
			"meanMetabolicTrust": 0.65,
			"activeWounds": 5,
			"compostingEntities": 3,
			"liminalEntities": 2,
			"entangledPairs": 10,
			"heldUncertainties": 4
		}`)

		var metrics PhaseMetrics
		if err := json.Unmarshal(data, &metrics); err != nil {
			t.Fatalf("UnmarshalJSON failed: %v", err)
		}

		if metrics.ActiveAgents != 100 {
			t.Errorf("ActiveAgents = %d, want 100", metrics.ActiveAgents)
		}
		if metrics.SpectralK != 0.75 {
			t.Errorf("SpectralK = %f, want 0.75", metrics.SpectralK)
		}
	})

	t.Run("UnmarshalJSON_SnakeCase", func(t *testing.T) {
		data := []byte(`{
			"active_agents": 50,
			"spectral_k": 0.5,
			"mean_metabolic_trust": 0.6,
			"active_wounds": 2,
			"composting_entities": 1,
			"liminal_entities": 0,
			"entangled_pairs": 5,
			"held_uncertainties": 3
		}`)

		var metrics PhaseMetrics
		if err := json.Unmarshal(data, &metrics); err != nil {
			t.Fatalf("UnmarshalJSON failed: %v", err)
		}

		if metrics.ActiveAgents != 50 {
			t.Errorf("ActiveAgents = %d, want 50", metrics.ActiveAgents)
		}
	})
}

func TestPhaseTransition(t *testing.T) {
	t.Run("UnmarshalJSON", func(t *testing.T) {
		data := []byte(`{
			"from": "Shadow",
			"to": "Composting",
			"cycleNumber": 1,
			"transitionedAt": "2024-01-03T00:00:00Z"
		}`)

		var transition PhaseTransition
		if err := json.Unmarshal(data, &transition); err != nil {
			t.Fatalf("UnmarshalJSON failed: %v", err)
		}

		if transition.From != PhaseShadow {
			t.Errorf("From = %s, want Shadow", transition.From)
		}
		if transition.To != PhaseComposting {
			t.Errorf("To = %s, want Composting", transition.To)
		}
		if transition.CycleNumber != 1 {
			t.Errorf("CycleNumber = %d, want 1", transition.CycleNumber)
		}
	})
}

func TestParseEvent(t *testing.T) {
	t.Run("PhaseTransitioned", func(t *testing.T) {
		data := []byte(`{
			"PhaseTransitioned": {
				"transition": {
					"from": "Shadow",
					"to": "Composting",
					"cycleNumber": 1,
					"transitionedAt": "2024-01-03T00:00:00Z"
				},
				"timestamp": "2024-01-03T00:00:00Z"
			}
		}`)

		event, err := ParseEvent(data)
		if err != nil {
			t.Fatalf("ParseEvent failed: %v", err)
		}

		if event.Type != "PhaseTransitioned" {
			t.Errorf("Type = %s, want PhaseTransitioned", event.Type)
		}

		transition := event.AsPhaseTransition()
		if transition == nil {
			t.Fatal("AsPhaseTransition returned nil")
		}
		if transition.From != PhaseShadow {
			t.Errorf("From = %s, want Shadow", transition.From)
		}
	})

	t.Run("CycleStarted", func(t *testing.T) {
		data := []byte(`{
			"CycleStarted": {
				"cycleNumber": 2,
				"startedAt": "2024-01-29T00:00:00Z"
			}
		}`)

		event, err := ParseEvent(data)
		if err != nil {
			t.Fatalf("ParseEvent failed: %v", err)
		}

		if event.Type != "CycleStarted" {
			t.Errorf("Type = %s, want CycleStarted", event.Type)
		}
	})
}

func TestSubscription(t *testing.T) {
	t.Run("Push_and_Receive", func(t *testing.T) {
		sub := NewSubscription(10)

		event := &Event{
			Type:      "TestEvent",
			Timestamp: time.Now(),
		}

		if !sub.push(event) {
			t.Error("push failed")
		}

		select {
		case received := <-sub.Events():
			if received.Type != "TestEvent" {
				t.Errorf("Type = %s, want TestEvent", received.Type)
			}
		case <-time.After(100 * time.Millisecond):
			t.Error("timeout waiting for event")
		}
	})

	t.Run("FilterByEventType", func(t *testing.T) {
		sub := NewSubscription(10, EventTypeFilter("TypeA"))

		eventA := &Event{Type: "TypeA", Timestamp: time.Now()}
		eventB := &Event{Type: "TypeB", Timestamp: time.Now()}

		if !sub.push(eventA) {
			t.Error("push TypeA failed")
		}
		if sub.push(eventB) {
			t.Error("push TypeB should have been filtered")
		}
	})

	t.Run("Close", func(t *testing.T) {
		sub := NewSubscription(10)
		sub.Close()

		if !sub.Closed() {
			t.Error("subscription should be closed")
		}

		event := &Event{Type: "Test", Timestamp: time.Now()}
		if sub.push(event) {
			t.Error("push to closed subscription should fail")
		}
	})
}

func TestSubscriptionManager(t *testing.T) {
	t.Run("Publish_to_Multiple", func(t *testing.T) {
		manager := NewSubscriptionManager()
		sub1 := manager.SubscribeAll(10)
		sub2 := manager.SubscribeAll(10)

		event := &Event{Type: "Test", Timestamp: time.Now()}
		count := manager.Publish(event)

		if count != 2 {
			t.Errorf("Publish returned %d, want 2", count)
		}

		select {
		case <-sub1.Events():
		case <-time.After(100 * time.Millisecond):
			t.Error("sub1 timeout")
		}

		select {
		case <-sub2.Events():
		case <-time.After(100 * time.Millisecond):
			t.Error("sub2 timeout")
		}
	})

	t.Run("Unsubscribe", func(t *testing.T) {
		manager := NewSubscriptionManager()
		sub := manager.SubscribeAll(10)

		if manager.Count() != 1 {
			t.Errorf("Count = %d, want 1", manager.Count())
		}

		manager.Unsubscribe(sub)

		if manager.Count() != 0 {
			t.Errorf("Count = %d, want 0", manager.Count())
		}
	})

	t.Run("CloseAll", func(t *testing.T) {
		manager := NewSubscriptionManager()
		sub1 := manager.SubscribeAll(10)
		sub2 := manager.SubscribeAll(10)

		manager.CloseAll()

		if !sub1.Closed() {
			t.Error("sub1 should be closed")
		}
		if !sub2.Closed() {
			t.Error("sub2 should be closed")
		}
	})
}

func TestEventFilters(t *testing.T) {
	t.Run("EventTypeFilter", func(t *testing.T) {
		filter := EventTypeFilter("TypeA", "TypeB")

		eventA := &Event{Type: "TypeA"}
		eventB := &Event{Type: "TypeB"}
		eventC := &Event{Type: "TypeC"}

		if !filter(eventA) {
			t.Error("TypeA should match")
		}
		if !filter(eventB) {
			t.Error("TypeB should match")
		}
		if filter(eventC) {
			t.Error("TypeC should not match")
		}
	})

	t.Run("CombineFilters", func(t *testing.T) {
		filter := CombineFilters(
			EventTypeFilter("TypeA"),
			func(e *Event) bool { return e.Timestamp.After(time.Unix(0, 0)) },
		)

		event := &Event{Type: "TypeA", Timestamp: time.Now()}
		if !filter(event) {
			t.Error("combined filter should match")
		}
	})
}

func TestClientConfig(t *testing.T) {
	t.Run("DefaultConfig", func(t *testing.T) {
		config := DefaultConfig("ws://localhost:8888")

		if config.URL != "ws://localhost:8888" {
			t.Errorf("URL = %s, want ws://localhost:8888", config.URL)
		}
		if !config.Reconnect {
			t.Error("Reconnect should be true")
		}
		if config.RequestTimeout != 10*time.Second {
			t.Errorf("RequestTimeout = %v, want 10s", config.RequestTimeout)
		}
	})
}

// MockWebSocketServer creates a test WebSocket server
type MockWebSocketServer struct {
	server   *httptest.Server
	upgrader websocket.Upgrader
	handler  func(*websocket.Conn)
}

func NewMockWebSocketServer(handler func(*websocket.Conn)) *MockWebSocketServer {
	m := &MockWebSocketServer{
		upgrader: websocket.Upgrader{},
		handler:  handler,
	}

	m.server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		conn, err := m.upgrader.Upgrade(w, r, nil)
		if err != nil {
			return
		}
		defer conn.Close()

		if m.handler != nil {
			m.handler(conn)
		}
	}))

	return m
}

func (m *MockWebSocketServer) URL() string {
	return "ws" + strings.TrimPrefix(m.server.URL, "http")
}

func (m *MockWebSocketServer) Close() {
	m.server.Close()
}

func TestClientRPC(t *testing.T) {
	t.Run("GetCurrentState", func(t *testing.T) {
		server := NewMockWebSocketServer(func(conn *websocket.Conn) {
			for {
				_, msg, err := conn.ReadMessage()
				if err != nil {
					return
				}

				var req rpcRequest
				if err := json.Unmarshal(msg, &req); err != nil {
					continue
				}

				if req.Method == "getCycleState" {
					resp := rpcResponse{
						ID: req.ID,
						Result: json.RawMessage(`{
							"cycleNumber": 1,
							"currentPhase": "Shadow",
							"phaseStarted": "2024-01-01T00:00:00Z",
							"cycleStarted": "2024-01-01T00:00:00Z",
							"phaseDay": 0
						}`),
					}
					data, _ := json.Marshal(resp)
					conn.WriteMessage(websocket.TextMessage, data)
				}
			}
		})
		defer server.Close()

		client, err := Connect(server.URL())
		if err != nil {
			t.Fatalf("Connect failed: %v", err)
		}
		defer client.Close()

		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()

		state, err := client.GetCurrentState(ctx)
		if err != nil {
			t.Fatalf("GetCurrentState failed: %v", err)
		}

		if state.CycleNumber != 1 {
			t.Errorf("CycleNumber = %d, want 1", state.CycleNumber)
		}
		if state.CurrentPhase != PhaseShadow {
			t.Errorf("CurrentPhase = %s, want Shadow", state.CurrentPhase)
		}
	})

	t.Run("RPCError", func(t *testing.T) {
		server := NewMockWebSocketServer(func(conn *websocket.Conn) {
			for {
				_, msg, err := conn.ReadMessage()
				if err != nil {
					return
				}

				var req rpcRequest
				if err := json.Unmarshal(msg, &req); err != nil {
					continue
				}

				resp := rpcResponse{
					ID: req.ID,
					Error: &RpcError{
						Code:    -32601,
						Message: "Method not found",
					},
				}
				data, _ := json.Marshal(resp)
				conn.WriteMessage(websocket.TextMessage, data)
			}
		})
		defer server.Close()

		client, err := Connect(server.URL())
		if err != nil {
			t.Fatalf("Connect failed: %v", err)
		}
		defer client.Close()

		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()

		_, err = client.rpcCall(ctx, "unknownMethod", nil)
		if err == nil {
			t.Fatal("expected error")
		}

		rpcErr, ok := err.(*RpcError)
		if !ok {
			t.Fatalf("expected RpcError, got %T", err)
		}
		if rpcErr.Code != -32601 {
			t.Errorf("Code = %d, want -32601", rpcErr.Code)
		}
	})
}

func TestClientSubscriptions(t *testing.T) {
	t.Run("ReceiveEvents", func(t *testing.T) {
		server := NewMockWebSocketServer(func(conn *websocket.Conn) {
			// Send an event
			event := map[string]interface{}{
				"PhaseTransitioned": map[string]interface{}{
					"transition": map[string]interface{}{
						"from":           "Shadow",
						"to":             "Composting",
						"cycleNumber":    1,
						"transitionedAt": "2024-01-03T00:00:00Z",
					},
				},
			}
			data, _ := json.Marshal(event)
			conn.WriteMessage(websocket.TextMessage, data)

			// Keep connection alive
			for {
				if _, _, err := conn.ReadMessage(); err != nil {
					return
				}
			}
		})
		defer server.Close()

		client, err := Connect(server.URL())
		if err != nil {
			t.Fatalf("Connect failed: %v", err)
		}
		defer client.Close()

		sub := client.SubscribePhaseTransitions()

		select {
		case event := <-sub.Events():
			if event.Type != "PhaseTransitioned" {
				t.Errorf("Type = %s, want PhaseTransitioned", event.Type)
			}
			transition := event.AsPhaseTransition()
			if transition == nil {
				t.Fatal("AsPhaseTransition returned nil")
			}
			if transition.To != PhaseComposting {
				t.Errorf("To = %s, want Composting", transition.To)
			}
		case <-time.After(2 * time.Second):
			t.Error("timeout waiting for event")
		}
	})
}

func TestRESTURL(t *testing.T) {
	t.Run("DeriveFromWebSocketURL", func(t *testing.T) {
		client := &LivingProtocolClient{
			config: ClientConfig{
				URL: "ws://localhost:8888",
			},
		}

		url := client.getRESTURL()
		if url != "http://localhost:8889" {
			t.Errorf("getRESTURL() = %s, want http://localhost:8889", url)
		}
	})

	t.Run("ExplicitRESTURL", func(t *testing.T) {
		client := &LivingProtocolClient{
			config: ClientConfig{
				URL:     "ws://localhost:8888",
				RESTURL: "http://api.example.com",
			},
		}

		url := client.getRESTURL()
		if url != "http://api.example.com" {
			t.Errorf("getRESTURL() = %s, want http://api.example.com", url)
		}
	})
}
