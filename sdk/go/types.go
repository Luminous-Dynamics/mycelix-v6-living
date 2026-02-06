// Package mycelix provides a Go SDK for the Mycelix Living Protocol.
//
// This SDK enables Go applications to connect to the Living Protocol
// WebSocket server and interact with the metabolism cycle.
package mycelix

import (
	"encoding/json"
	"fmt"
	"time"
)

// CyclePhase represents a phase in the 28-day metabolism cycle.
type CyclePhase string

const (
	// PhaseShadow is the Shadow phase (2 days): Suppression detection, dissent surfaces.
	PhaseShadow CyclePhase = "Shadow"
	// PhaseComposting is the Composting phase (5 days): Failed entities decomposed.
	PhaseComposting CyclePhase = "Composting"
	// PhaseLiminal is the Liminal phase (3 days): Transitioning entities in threshold.
	PhaseLiminal CyclePhase = "Liminal"
	// PhaseNegativeCapability is the Negative Capability phase (3 days): Voting blocked.
	PhaseNegativeCapability CyclePhase = "NegativeCapability"
	// PhaseEros is the Eros phase (4 days): Attractor fields computed.
	PhaseEros CyclePhase = "Eros"
	// PhaseCoCreation is the Co-Creation phase (7 days): Standard consensus.
	PhaseCoCreation CyclePhase = "CoCreation"
	// PhaseBeauty is the Beauty phase (2 days): Proposals scored on aesthetics.
	PhaseBeauty CyclePhase = "Beauty"
	// PhaseEmergentPersonhood is the Emergent Personhood phase (1 day): Network self-measurement.
	PhaseEmergentPersonhood CyclePhase = "EmergentPersonhood"
	// PhaseKenosis is the Kenosis phase (1 day): Voluntary reputation release.
	PhaseKenosis CyclePhase = "Kenosis"
)

// AllPhases returns all cycle phases in order.
func AllPhases() []CyclePhase {
	return []CyclePhase{
		PhaseShadow,
		PhaseComposting,
		PhaseLiminal,
		PhaseNegativeCapability,
		PhaseEros,
		PhaseCoCreation,
		PhaseBeauty,
		PhaseEmergentPersonhood,
		PhaseKenosis,
	}
}

// DurationDays returns the duration of this phase in days.
func (p CyclePhase) DurationDays() int {
	switch p {
	case PhaseShadow:
		return 2
	case PhaseComposting:
		return 5
	case PhaseLiminal:
		return 3
	case PhaseNegativeCapability:
		return 3
	case PhaseEros:
		return 4
	case PhaseCoCreation:
		return 7
	case PhaseBeauty:
		return 2
	case PhaseEmergentPersonhood:
		return 1
	case PhaseKenosis:
		return 1
	default:
		return 0
	}
}

// Next returns the next phase in the cycle (wraps around).
func (p CyclePhase) Next() CyclePhase {
	phases := AllPhases()
	for i, phase := range phases {
		if phase == p {
			return phases[(i+1)%len(phases)]
		}
	}
	return PhaseShadow
}

// Prev returns the previous phase in the cycle (wraps around).
func (p CyclePhase) Prev() CyclePhase {
	phases := AllPhases()
	for i, phase := range phases {
		if phase == p {
			idx := (i - 1 + len(phases)) % len(phases)
			return phases[idx]
		}
	}
	return PhaseKenosis
}

// Valid checks if the phase string is valid.
func (p CyclePhase) Valid() bool {
	for _, phase := range AllPhases() {
		if phase == p {
			return true
		}
	}
	return false
}

// String returns the string representation of the phase.
func (p CyclePhase) String() string {
	return string(p)
}

// TotalCycleDays returns the total cycle length (28 days).
func TotalCycleDays() int {
	total := 0
	for _, phase := range AllPhases() {
		total += phase.DurationDays()
	}
	return total
}

// CycleState represents the current state of a metabolism cycle.
type CycleState struct {
	// CycleNumber is the current cycle number (1-indexed).
	CycleNumber uint64 `json:"cycleNumber"`
	// CurrentPhase is the current phase of the cycle.
	CurrentPhase CyclePhase `json:"currentPhase"`
	// PhaseStarted is when the current phase started.
	PhaseStarted time.Time `json:"phaseStarted"`
	// CycleStarted is when the current cycle started.
	CycleStarted time.Time `json:"cycleStarted"`
	// PhaseDay is the current day within the phase (0-indexed).
	PhaseDay uint32 `json:"phaseDay"`
}

// UnmarshalJSON implements custom JSON unmarshaling for CycleState.
func (s *CycleState) UnmarshalJSON(data []byte) error {
	type Alias CycleState
	aux := &struct {
		PhaseStarted string `json:"phaseStarted"`
		CycleStarted string `json:"cycleStarted"`
		*Alias
	}{
		Alias: (*Alias)(s),
	}

	if err := json.Unmarshal(data, aux); err != nil {
		return err
	}

	var err error
	s.PhaseStarted, err = time.Parse(time.RFC3339, aux.PhaseStarted)
	if err != nil {
		return fmt.Errorf("parsing phaseStarted: %w", err)
	}

	s.CycleStarted, err = time.Parse(time.RFC3339, aux.CycleStarted)
	if err != nil {
		return fmt.Errorf("parsing cycleStarted: %w", err)
	}

	return nil
}

// TimeRemaining returns the time remaining in the current phase.
func (s *CycleState) TimeRemaining() time.Duration {
	end := s.PhaseStarted.Add(time.Duration(s.CurrentPhase.DurationDays()) * 24 * time.Hour)
	remaining := time.Until(end)
	if remaining < 0 {
		return 0
	}
	return remaining
}

// PhaseExpired checks if the current phase has expired.
func (s *CycleState) PhaseExpired() bool {
	return s.TimeRemaining() == 0
}

// PhaseMetrics represents metrics collected at phase transitions.
type PhaseMetrics struct {
	// ActiveAgents is the number of active agents in the network.
	ActiveAgents uint64 `json:"activeAgents"`
	// SpectralK is the spectral K-Vector derivative.
	SpectralK float64 `json:"spectralK"`
	// MeanMetabolicTrust is the average metabolic trust across agents.
	MeanMetabolicTrust float64 `json:"meanMetabolicTrust"`
	// ActiveWounds is the number of wounds currently being healed.
	ActiveWounds uint64 `json:"activeWounds"`
	// CompostingEntities is the number of entities being composted.
	CompostingEntities uint64 `json:"compostingEntities"`
	// LiminalEntities is the number of entities in liminal transition.
	LiminalEntities uint64 `json:"liminalEntities"`
	// EntangledPairs is the number of entangled agent pairs.
	EntangledPairs uint64 `json:"entangledPairs"`
	// HeldUncertainties is the number of claims held in uncertainty.
	HeldUncertainties uint64 `json:"heldUncertainties"`
}

// UnmarshalJSON implements custom JSON unmarshaling for PhaseMetrics.
// Handles both camelCase and snake_case field names.
func (m *PhaseMetrics) UnmarshalJSON(data []byte) error {
	// Try camelCase first
	type CamelCase struct {
		ActiveAgents       uint64  `json:"activeAgents"`
		SpectralK          float64 `json:"spectralK"`
		MeanMetabolicTrust float64 `json:"meanMetabolicTrust"`
		ActiveWounds       uint64  `json:"activeWounds"`
		CompostingEntities uint64  `json:"compostingEntities"`
		LiminalEntities    uint64  `json:"liminalEntities"`
		EntangledPairs     uint64  `json:"entangledPairs"`
		HeldUncertainties  uint64  `json:"heldUncertainties"`
	}

	type SnakeCase struct {
		ActiveAgents       uint64  `json:"active_agents"`
		SpectralK          float64 `json:"spectral_k"`
		MeanMetabolicTrust float64 `json:"mean_metabolic_trust"`
		ActiveWounds       uint64  `json:"active_wounds"`
		CompostingEntities uint64  `json:"composting_entities"`
		LiminalEntities    uint64  `json:"liminal_entities"`
		EntangledPairs     uint64  `json:"entangled_pairs"`
		HeldUncertainties  uint64  `json:"held_uncertainties"`
	}

	var camel CamelCase
	if err := json.Unmarshal(data, &camel); err == nil && camel.ActiveAgents > 0 {
		m.ActiveAgents = camel.ActiveAgents
		m.SpectralK = camel.SpectralK
		m.MeanMetabolicTrust = camel.MeanMetabolicTrust
		m.ActiveWounds = camel.ActiveWounds
		m.CompostingEntities = camel.CompostingEntities
		m.LiminalEntities = camel.LiminalEntities
		m.EntangledPairs = camel.EntangledPairs
		m.HeldUncertainties = camel.HeldUncertainties
		return nil
	}

	var snake SnakeCase
	if err := json.Unmarshal(data, &snake); err != nil {
		return err
	}

	m.ActiveAgents = snake.ActiveAgents
	m.SpectralK = snake.SpectralK
	m.MeanMetabolicTrust = snake.MeanMetabolicTrust
	m.ActiveWounds = snake.ActiveWounds
	m.CompostingEntities = snake.CompostingEntities
	m.LiminalEntities = snake.LiminalEntities
	m.EntangledPairs = snake.EntangledPairs
	m.HeldUncertainties = snake.HeldUncertainties

	return nil
}

// PhaseTransition represents a phase transition in the metabolism cycle.
type PhaseTransition struct {
	// From is the phase transitioned from.
	From CyclePhase `json:"from"`
	// To is the phase transitioned to.
	To CyclePhase `json:"to"`
	// CycleNumber is the cycle number when this transition occurred.
	CycleNumber uint64 `json:"cycleNumber"`
	// TransitionedAt is when the transition occurred.
	TransitionedAt time.Time `json:"transitionedAt"`
	// Metrics contains metrics collected at the time of transition.
	Metrics *PhaseMetrics `json:"metrics,omitempty"`
}

// UnmarshalJSON implements custom JSON unmarshaling for PhaseTransition.
func (t *PhaseTransition) UnmarshalJSON(data []byte) error {
	type Alias PhaseTransition
	aux := &struct {
		TransitionedAt string `json:"transitionedAt"`
		*Alias
	}{
		Alias: (*Alias)(t),
	}

	if err := json.Unmarshal(data, aux); err != nil {
		return err
	}

	var err error
	t.TransitionedAt, err = time.Parse(time.RFC3339, aux.TransitionedAt)
	if err != nil {
		return fmt.Errorf("parsing transitionedAt: %w", err)
	}

	return nil
}

// Event represents a Living Protocol event from the WebSocket.
type Event struct {
	// Type is the event type (e.g., "PhaseTransitioned", "CycleStarted").
	Type string `json:"type"`
	// Data contains the event-specific data.
	Data json.RawMessage `json:"data"`
	// Timestamp is when the event was received.
	Timestamp time.Time `json:"-"`
}

// PhaseTransitionedEvent is the data for a phase transition event.
type PhaseTransitionedEvent struct {
	Transition PhaseTransition `json:"transition"`
	Timestamp  time.Time       `json:"timestamp"`
}

// CycleStartedEvent is the data for a cycle start event.
type CycleStartedEvent struct {
	CycleNumber uint64    `json:"cycleNumber"`
	StartedAt   time.Time `json:"startedAt"`
}

// AsPhaseTransition attempts to parse the event data as a PhaseTransition.
// Returns nil if the event is not a phase transition.
func (e *Event) AsPhaseTransition() *PhaseTransition {
	if e.Type != "PhaseTransitioned" {
		return nil
	}

	var data PhaseTransitionedEvent
	if err := json.Unmarshal(e.Data, &data); err != nil {
		return nil
	}

	return &data.Transition
}

// AsCycleStarted attempts to parse the event data as a CycleStartedEvent.
// Returns nil if the event is not a cycle start.
func (e *Event) AsCycleStarted() *CycleStartedEvent {
	if e.Type != "CycleStarted" {
		return nil
	}

	var data CycleStartedEvent
	if err := json.Unmarshal(e.Data, &data); err != nil {
		return nil
	}

	return &data
}

// ParseEvent parses a raw JSON message into an Event.
func ParseEvent(data []byte) (*Event, error) {
	// Try to parse as wrapped event (e.g., {"PhaseTransitioned": {...}})
	var raw map[string]json.RawMessage
	if err := json.Unmarshal(data, &raw); err != nil {
		return nil, fmt.Errorf("parsing event: %w", err)
	}

	event := &Event{
		Timestamp: time.Now(),
	}

	// Check for known event types
	for _, eventType := range []string{"PhaseTransitioned", "CycleStarted"} {
		if eventData, ok := raw[eventType]; ok {
			event.Type = eventType
			event.Data = eventData
			return event, nil
		}
	}

	// Check for generic event structure
	if typeVal, ok := raw["type"]; ok {
		var typeName string
		if err := json.Unmarshal(typeVal, &typeName); err == nil {
			event.Type = typeName
			if dataVal, ok := raw["data"]; ok {
				event.Data = dataVal
			}
			return event, nil
		}
	}

	// Unknown event structure
	event.Type = "Unknown"
	event.Data = data

	return event, nil
}

// RpcError represents an error from an RPC call.
type RpcError struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
}

func (e *RpcError) Error() string {
	return fmt.Sprintf("RPC error %d: %s", e.Code, e.Message)
}

// Common RPC error codes
const (
	RPCErrorMethodNotFound = -32601
	RPCErrorInvalidParams  = -32602
	RPCErrorInternal       = -32603
)
