"""Unit tests for the Mycelix Living Protocol Python SDK."""

import asyncio
import json
from datetime import datetime, timezone
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from mycelix import (
    CyclePhase,
    CycleState,
    LivingProtocolClient,
    LivingProtocolEvent,
    PhaseMetrics,
    PhaseTransition,
)
from mycelix.client import ClientConfig, RpcError
from mycelix.subscription import EventSubscription, SubscriptionManager


class TestCyclePhase:
    """Tests for CyclePhase enum."""

    def test_duration_days(self):
        """Test phase duration values."""
        assert CyclePhase.SHADOW.duration_days == 2
        assert CyclePhase.COMPOSTING.duration_days == 5
        assert CyclePhase.LIMINAL.duration_days == 3
        assert CyclePhase.NEGATIVE_CAPABILITY.duration_days == 3
        assert CyclePhase.EROS.duration_days == 4
        assert CyclePhase.CO_CREATION.duration_days == 7
        assert CyclePhase.BEAUTY.duration_days == 2
        assert CyclePhase.EMERGENT_PERSONHOOD.duration_days == 1
        assert CyclePhase.KENOSIS.duration_days == 1

    def test_total_cycle_is_28_days(self):
        """Verify the total cycle length is 28 days."""
        total = sum(phase.duration_days for phase in CyclePhase)
        assert total == 28

    def test_next_phase(self):
        """Test phase progression."""
        assert CyclePhase.SHADOW.next() == CyclePhase.COMPOSTING
        assert CyclePhase.KENOSIS.next() == CyclePhase.SHADOW  # Wraps around

    def test_prev_phase(self):
        """Test phase regression."""
        assert CyclePhase.COMPOSTING.prev() == CyclePhase.SHADOW
        assert CyclePhase.SHADOW.prev() == CyclePhase.KENOSIS  # Wraps around

    def test_from_str(self):
        """Test parsing phases from strings."""
        assert CyclePhase.from_str("Shadow") == CyclePhase.SHADOW
        assert CyclePhase.from_str("CoCreation") == CyclePhase.CO_CREATION

    def test_from_str_invalid(self):
        """Test error handling for invalid phase strings."""
        with pytest.raises(ValueError):
            CyclePhase.from_str("InvalidPhase")


class TestCycleState:
    """Tests for CycleState dataclass."""

    def test_from_dict(self):
        """Test creating CycleState from JSON response."""
        data = {
            "cycleNumber": 3,
            "currentPhase": "Liminal",
            "phaseStarted": "2024-01-15T10:00:00Z",
            "cycleStarted": "2024-01-01T00:00:00Z",
            "phaseDay": 1,
        }

        state = CycleState.from_dict(data)

        assert state.cycle_number == 3
        assert state.current_phase == CyclePhase.LIMINAL
        assert state.phase_day == 1
        assert state.phase_started.year == 2024

    def test_from_dict_with_offset_timezone(self):
        """Test parsing timestamps with timezone offsets."""
        data = {
            "cycleNumber": 1,
            "currentPhase": "Shadow",
            "phaseStarted": "2024-01-15T10:00:00+05:00",
            "cycleStarted": "2024-01-01T00:00:00+00:00",
            "phaseDay": 0,
        }

        state = CycleState.from_dict(data)
        assert state.phase_started.tzinfo is not None


class TestPhaseMetrics:
    """Tests for PhaseMetrics dataclass."""

    def test_from_dict_camel_case(self):
        """Test parsing camelCase keys (from server)."""
        data = {
            "activeAgents": 100,
            "spectralK": 0.75,
            "meanMetabolicTrust": 0.65,
            "activeWounds": 5,
            "compostingEntities": 3,
            "liminalEntities": 2,
            "entangledPairs": 10,
            "heldUncertainties": 4,
        }

        metrics = PhaseMetrics.from_dict(data)

        assert metrics.active_agents == 100
        assert metrics.spectral_k == 0.75
        assert metrics.mean_metabolic_trust == 0.65

    def test_from_dict_snake_case(self):
        """Test parsing snake_case keys (alternative format)."""
        data = {
            "active_agents": 50,
            "spectral_k": 0.5,
            "mean_metabolic_trust": 0.6,
            "active_wounds": 2,
            "composting_entities": 1,
            "liminal_entities": 0,
            "entangled_pairs": 5,
            "held_uncertainties": 3,
        }

        metrics = PhaseMetrics.from_dict(data)
        assert metrics.active_agents == 50


class TestPhaseTransition:
    """Tests for PhaseTransition dataclass."""

    def test_from_dict(self):
        """Test creating PhaseTransition from JSON."""
        data = {
            "from": "Shadow",
            "to": "Composting",
            "cycleNumber": 1,
            "transitionedAt": "2024-01-03T00:00:00Z",
        }

        transition = PhaseTransition.from_dict(data)

        assert transition.from_phase == CyclePhase.SHADOW
        assert transition.to_phase == CyclePhase.COMPOSTING
        assert transition.cycle_number == 1
        assert transition.metrics is None

    def test_from_dict_with_metrics(self):
        """Test parsing transition with metrics."""
        data = {
            "from": "Liminal",
            "to": "NegativeCapability",
            "cycleNumber": 2,
            "transitionedAt": "2024-01-10T00:00:00Z",
            "metrics": {
                "activeAgents": 75,
                "spectralK": 0.8,
                "meanMetabolicTrust": 0.7,
                "activeWounds": 3,
                "compostingEntities": 0,
                "liminalEntities": 5,
                "entangledPairs": 8,
                "heldUncertainties": 2,
            },
        }

        transition = PhaseTransition.from_dict(data)
        assert transition.metrics is not None
        assert transition.metrics.active_agents == 75


class TestLivingProtocolEvent:
    """Tests for LivingProtocolEvent dataclass."""

    def test_from_dict_phase_transitioned(self):
        """Test parsing PhaseTransitioned event."""
        data = {
            "PhaseTransitioned": {
                "transition": {
                    "from": "Shadow",
                    "to": "Composting",
                    "cycleNumber": 1,
                    "transitionedAt": "2024-01-03T00:00:00Z",
                },
                "timestamp": "2024-01-03T00:00:00Z",
            }
        }

        event = LivingProtocolEvent.from_dict(data)
        assert event.event_type == "PhaseTransitioned"
        assert "transition" in event.data

    def test_from_dict_cycle_started(self):
        """Test parsing CycleStarted event."""
        data = {
            "CycleStarted": {
                "cycleNumber": 2,
                "startedAt": "2024-01-29T00:00:00Z",
            }
        }

        event = LivingProtocolEvent.from_dict(data)
        assert event.event_type == "CycleStarted"

    def test_as_phase_transition(self):
        """Test converting event to PhaseTransition."""
        data = {
            "PhaseTransitioned": {
                "transition": {
                    "from": "Eros",
                    "to": "CoCreation",
                    "cycleNumber": 1,
                    "transitionedAt": "2024-01-15T00:00:00Z",
                },
            }
        }

        event = LivingProtocolEvent.from_dict(data)
        transition = event.as_phase_transition()

        assert transition is not None
        assert transition.from_phase == CyclePhase.EROS
        assert transition.to_phase == CyclePhase.CO_CREATION


class TestEventSubscription:
    """Tests for EventSubscription class."""

    @pytest.mark.asyncio
    async def test_matches_all(self):
        """Test subscription with no filters matches everything."""
        sub = EventSubscription()
        event = LivingProtocolEvent(event_type="TestEvent", data={})

        assert sub.matches(event)

    @pytest.mark.asyncio
    async def test_matches_event_type(self):
        """Test filtering by event type."""
        sub = EventSubscription(event_types=["PhaseTransitioned"])

        matching = LivingProtocolEvent(event_type="PhaseTransitioned", data={})
        non_matching = LivingProtocolEvent(event_type="CycleStarted", data={})

        assert sub.matches(matching)
        assert not sub.matches(non_matching)

    @pytest.mark.asyncio
    async def test_matches_custom_filter(self):
        """Test custom filter function."""
        sub = EventSubscription(
            filter_fn=lambda e: e.data.get("important", False)
        )

        matching = LivingProtocolEvent(event_type="Test", data={"important": True})
        non_matching = LivingProtocolEvent(event_type="Test", data={"important": False})

        assert sub.matches(matching)
        assert not sub.matches(non_matching)

    @pytest.mark.asyncio
    async def test_push_and_get(self):
        """Test pushing and getting events."""
        sub = EventSubscription()
        event = LivingProtocolEvent(event_type="Test", data={})

        await sub.push(event)
        received = await sub.get(timeout=1.0)

        assert received is not None
        assert received.event_type == "Test"

    @pytest.mark.asyncio
    async def test_close(self):
        """Test closing subscription."""
        sub = EventSubscription()
        sub.close()

        assert sub.closed
        result = await sub.push(LivingProtocolEvent(event_type="Test", data={}))
        assert not result

    @pytest.mark.asyncio
    async def test_async_iteration(self):
        """Test async iteration over events."""
        sub = EventSubscription()
        events = [
            LivingProtocolEvent(event_type="Event1", data={}),
            LivingProtocolEvent(event_type="Event2", data={}),
        ]

        for event in events:
            await sub.push(event)

        received = []
        async def collect():
            count = 0
            async for event in sub:
                received.append(event)
                count += 1
                if count >= 2:
                    sub.close()

        await asyncio.wait_for(collect(), timeout=1.0)
        assert len(received) == 2


class TestSubscriptionManager:
    """Tests for SubscriptionManager class."""

    @pytest.mark.asyncio
    async def test_subscribe_and_publish(self):
        """Test subscribing and receiving events."""
        manager = SubscriptionManager()
        sub = manager.subscribe()

        event = LivingProtocolEvent(event_type="Test", data={"value": 42})
        count = await manager.publish(event)

        assert count == 1
        received = await sub.get(timeout=1.0)
        assert received is not None
        assert received.data["value"] == 42

    @pytest.mark.asyncio
    async def test_multiple_subscriptions(self):
        """Test publishing to multiple subscribers."""
        manager = SubscriptionManager()
        sub1 = manager.subscribe()
        sub2 = manager.subscribe()

        event = LivingProtocolEvent(event_type="Test", data={})
        count = await manager.publish(event)

        assert count == 2
        assert (await sub1.get(timeout=1.0)) is not None
        assert (await sub2.get(timeout=1.0)) is not None

    @pytest.mark.asyncio
    async def test_filtered_subscriptions(self):
        """Test that filters work correctly."""
        manager = SubscriptionManager()
        sub1 = manager.subscribe(event_types=["TypeA"])
        sub2 = manager.subscribe(event_types=["TypeB"])

        event_a = LivingProtocolEvent(event_type="TypeA", data={})
        event_b = LivingProtocolEvent(event_type="TypeB", data={})

        await manager.publish(event_a)
        await manager.publish(event_b)

        received1 = await sub1.get(timeout=0.1)
        received2 = await sub2.get(timeout=0.1)

        assert received1 is not None
        assert received1.event_type == "TypeA"
        assert received2 is not None
        assert received2.event_type == "TypeB"

    @pytest.mark.asyncio
    async def test_unsubscribe(self):
        """Test unsubscribing."""
        manager = SubscriptionManager()
        sub = manager.subscribe()

        assert manager.subscription_count == 1
        manager.unsubscribe(sub)
        assert manager.subscription_count == 0

    def test_close_all(self):
        """Test closing all subscriptions."""
        manager = SubscriptionManager()
        sub1 = manager.subscribe()
        sub2 = manager.subscribe()

        manager.close_all()

        assert sub1.closed
        assert sub2.closed
        assert manager.subscription_count == 0


class TestClientConfig:
    """Tests for ClientConfig."""

    def test_defaults(self):
        """Test default configuration values."""
        config = ClientConfig(url="ws://localhost:8888")

        assert config.reconnect is True
        assert config.reconnect_delay == 1.0
        assert config.request_timeout == 10.0

    def test_custom_values(self):
        """Test custom configuration."""
        config = ClientConfig(
            url="ws://example.com:9000",
            rest_url="http://example.com:9001",
            reconnect=False,
            request_timeout=30.0,
        )

        assert config.url == "ws://example.com:9000"
        assert config.rest_url == "http://example.com:9001"
        assert config.reconnect is False


class TestLivingProtocolClient:
    """Tests for LivingProtocolClient."""

    def test_init_with_string(self):
        """Test initializing with a URL string."""
        client = LivingProtocolClient("ws://localhost:8888")
        assert client.config.url == "ws://localhost:8888"

    def test_init_with_config(self):
        """Test initializing with a ClientConfig."""
        config = ClientConfig(url="ws://example.com:9000", reconnect=False)
        client = LivingProtocolClient(config)
        assert client.config.reconnect is False

    def test_rest_url_derivation(self):
        """Test REST URL derivation from WebSocket URL."""
        client = LivingProtocolClient("ws://localhost:8888")
        rest_url = client._get_rest_url()
        assert rest_url == "http://localhost:8889"

    def test_rest_url_explicit(self):
        """Test explicit REST URL."""
        config = ClientConfig(
            url="ws://localhost:8888",
            rest_url="http://api.example.com",
        )
        client = LivingProtocolClient(config)
        rest_url = client._get_rest_url()
        assert rest_url == "http://api.example.com"

    @pytest.mark.asyncio
    async def test_subscribe_returns_subscription(self):
        """Test that subscribe returns an EventSubscription."""
        client = LivingProtocolClient("ws://localhost:8888")
        sub = client.subscribe()
        assert isinstance(sub, EventSubscription)

    @pytest.mark.asyncio
    async def test_subscribe_with_filter(self):
        """Test subscribing with event type filter."""
        client = LivingProtocolClient("ws://localhost:8888")
        sub = client.subscribe(event_types=["PhaseTransitioned"])
        assert sub.event_types == ["PhaseTransitioned"]


# Integration tests (require running server)
# These are marked to skip unless explicitly enabled


@pytest.mark.skip(reason="Requires running server")
class TestClientIntegration:
    """Integration tests requiring a running server."""

    @pytest.mark.asyncio
    async def test_connect_and_get_state(self):
        """Test connecting and getting cycle state."""
        async with LivingProtocolClient("ws://localhost:8888") as client:
            state = await client.get_current_state()
            assert state.cycle_number >= 1
            assert isinstance(state.current_phase, CyclePhase)

    @pytest.mark.asyncio
    async def test_subscribe_to_events(self):
        """Test subscribing to events."""
        async with LivingProtocolClient("ws://localhost:8888") as client:
            sub = client.subscribe_phase_transitions()

            # Wait briefly for any events
            event = await sub.get(timeout=5.0)
            # Event may or may not arrive depending on timing
