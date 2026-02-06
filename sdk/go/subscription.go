package mycelix

import (
	"sync"
)

// EventFilter is a function that filters events.
// Return true to include the event, false to exclude it.
type EventFilter func(*Event) bool

// Subscription represents a subscription to Living Protocol events.
type Subscription struct {
	// events is the channel where matching events are sent.
	events chan *Event
	// filters is the list of filters to apply.
	filters []EventFilter
	// closed indicates if the subscription is closed.
	closed bool
	// mu protects the closed field.
	mu sync.RWMutex
}

// NewSubscription creates a new event subscription.
// The bufferSize determines how many events can be buffered before
// blocking or dropping.
func NewSubscription(bufferSize int, filters ...EventFilter) *Subscription {
	if bufferSize < 1 {
		bufferSize = 100
	}
	return &Subscription{
		events:  make(chan *Event, bufferSize),
		filters: filters,
	}
}

// Events returns the channel for receiving events.
// The channel will be closed when the subscription is closed.
func (s *Subscription) Events() <-chan *Event {
	return s.events
}

// Close closes the subscription and its event channel.
func (s *Subscription) Close() {
	s.mu.Lock()
	defer s.mu.Unlock()

	if !s.closed {
		s.closed = true
		close(s.events)
	}
}

// Closed returns true if the subscription is closed.
func (s *Subscription) Closed() bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.closed
}

// matches checks if an event matches all filters.
func (s *Subscription) matches(event *Event) bool {
	for _, filter := range s.filters {
		if !filter(event) {
			return false
		}
	}
	return true
}

// push attempts to push an event to the subscription.
// Returns true if the event was pushed, false if the subscription
// is closed or the event doesn't match filters.
func (s *Subscription) push(event *Event) bool {
	s.mu.RLock()
	if s.closed {
		s.mu.RUnlock()
		return false
	}
	s.mu.RUnlock()

	if !s.matches(event) {
		return false
	}

	select {
	case s.events <- event:
		return true
	default:
		// Channel full, drop oldest and add new
		select {
		case <-s.events:
		default:
		}
		select {
		case s.events <- event:
			return true
		default:
			return false
		}
	}
}

// SubscriptionManager manages multiple event subscriptions.
type SubscriptionManager struct {
	subscriptions []*Subscription
	mu            sync.RWMutex
}

// NewSubscriptionManager creates a new subscription manager.
func NewSubscriptionManager() *SubscriptionManager {
	return &SubscriptionManager{
		subscriptions: make([]*Subscription, 0),
	}
}

// Subscribe creates a new subscription with optional filters.
func (m *SubscriptionManager) Subscribe(bufferSize int, filters ...EventFilter) *Subscription {
	sub := NewSubscription(bufferSize, filters...)

	m.mu.Lock()
	m.subscriptions = append(m.subscriptions, sub)
	m.mu.Unlock()

	return sub
}

// SubscribeAll creates a subscription that receives all events.
func (m *SubscriptionManager) SubscribeAll(bufferSize int) *Subscription {
	return m.Subscribe(bufferSize)
}

// SubscribePhaseTransitions creates a subscription for phase transition events.
func (m *SubscriptionManager) SubscribePhaseTransitions(bufferSize int) *Subscription {
	return m.Subscribe(bufferSize, EventTypeFilter("PhaseTransitioned"))
}

// SubscribeCycleStarts creates a subscription for cycle start events.
func (m *SubscriptionManager) SubscribeCycleStarts(bufferSize int) *Subscription {
	return m.Subscribe(bufferSize, EventTypeFilter("CycleStarted"))
}

// Unsubscribe removes a subscription.
func (m *SubscriptionManager) Unsubscribe(sub *Subscription) {
	sub.Close()

	m.mu.Lock()
	defer m.mu.Unlock()

	for i, s := range m.subscriptions {
		if s == sub {
			m.subscriptions = append(m.subscriptions[:i], m.subscriptions[i+1:]...)
			return
		}
	}
}

// Publish sends an event to all matching subscriptions.
// Returns the number of subscriptions that received the event.
func (m *SubscriptionManager) Publish(event *Event) int {
	m.mu.Lock()
	// Remove closed subscriptions
	active := make([]*Subscription, 0, len(m.subscriptions))
	for _, sub := range m.subscriptions {
		if !sub.Closed() {
			active = append(active, sub)
		}
	}
	m.subscriptions = active
	m.mu.Unlock()

	count := 0
	for _, sub := range active {
		if sub.push(event) {
			count++
		}
	}

	return count
}

// CloseAll closes all subscriptions.
func (m *SubscriptionManager) CloseAll() {
	m.mu.Lock()
	defer m.mu.Unlock()

	for _, sub := range m.subscriptions {
		sub.Close()
	}
	m.subscriptions = m.subscriptions[:0]
}

// Count returns the number of active subscriptions.
func (m *SubscriptionManager) Count() int {
	m.mu.RLock()
	defer m.mu.RUnlock()

	count := 0
	for _, sub := range m.subscriptions {
		if !sub.Closed() {
			count++
		}
	}
	return count
}

// EventTypeFilter creates a filter that matches specific event types.
func EventTypeFilter(types ...string) EventFilter {
	typeSet := make(map[string]struct{}, len(types))
	for _, t := range types {
		typeSet[t] = struct{}{}
	}

	return func(event *Event) bool {
		_, ok := typeSet[event.Type]
		return ok
	}
}

// PhaseFilter creates a filter that matches events related to specific phases.
func PhaseFilter(phases ...CyclePhase) EventFilter {
	phaseSet := make(map[CyclePhase]struct{}, len(phases))
	for _, p := range phases {
		phaseSet[p] = struct{}{}
	}

	return func(event *Event) bool {
		if event.Type != "PhaseTransitioned" {
			return false
		}

		transition := event.AsPhaseTransition()
		if transition == nil {
			return false
		}

		_, fromMatch := phaseSet[transition.From]
		_, toMatch := phaseSet[transition.To]

		return fromMatch || toMatch
	}
}

// CombineFilters creates a filter that requires all given filters to match.
func CombineFilters(filters ...EventFilter) EventFilter {
	return func(event *Event) bool {
		for _, f := range filters {
			if !f(event) {
				return false
			}
		}
		return true
	}
}

// AnyFilter creates a filter that matches if any of the given filters match.
func AnyFilter(filters ...EventFilter) EventFilter {
	return func(event *Event) bool {
		for _, f := range filters {
			if f(event) {
				return true
			}
		}
		return false
	}
}
