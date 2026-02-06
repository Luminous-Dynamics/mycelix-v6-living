"""Event subscription management for the Mycelix Living Protocol SDK.

This module provides utilities for subscribing to and filtering
Living Protocol events from the WebSocket connection.
"""

import asyncio
from collections.abc import AsyncIterator, Callable
from dataclasses import dataclass, field
from typing import Any

from mycelix.types import CyclePhase, LivingProtocolEvent


@dataclass
class EventSubscription:
    """A subscription to Living Protocol events.

    Subscriptions can filter events by type and apply custom filters.
    They maintain an internal queue of matching events.

    Attributes:
        event_types: Optional list of event types to subscribe to.
            If None, all events are included.
        filter_fn: Optional custom filter function.
        queue: Internal async queue for buffering events.
        max_queue_size: Maximum number of events to buffer.
    """

    event_types: list[str] | None = None
    filter_fn: Callable[[LivingProtocolEvent], bool] | None = None
    queue: asyncio.Queue[LivingProtocolEvent] = field(
        default_factory=lambda: asyncio.Queue(maxsize=1000)
    )
    max_queue_size: int = 1000
    _closed: bool = field(default=False, repr=False)

    def matches(self, event: LivingProtocolEvent) -> bool:
        """Check if an event matches this subscription's filters."""
        # Check event type filter
        if self.event_types is not None:
            if event.event_type not in self.event_types:
                return False

        # Check custom filter
        if self.filter_fn is not None:
            if not self.filter_fn(event):
                return False

        return True

    async def push(self, event: LivingProtocolEvent) -> bool:
        """Push an event to the subscription queue.

        Returns:
            True if the event was queued, False if the queue is full or closed.
        """
        if self._closed:
            return False

        if not self.matches(event):
            return False

        try:
            self.queue.put_nowait(event)
            return True
        except asyncio.QueueFull:
            # Drop oldest event and add new one
            try:
                self.queue.get_nowait()
                self.queue.put_nowait(event)
                return True
            except asyncio.QueueEmpty:
                return False

    async def get(self, timeout: float | None = None) -> LivingProtocolEvent | None:
        """Get the next event from the subscription.

        Args:
            timeout: Optional timeout in seconds. If None, waits forever.

        Returns:
            The next event, or None if timeout expired or subscription closed.
        """
        if self._closed:
            return None

        try:
            if timeout is not None:
                return await asyncio.wait_for(self.queue.get(), timeout=timeout)
            return await self.queue.get()
        except asyncio.TimeoutError:
            return None
        except asyncio.CancelledError:
            return None

    def close(self) -> None:
        """Close this subscription, preventing further events."""
        self._closed = True

    @property
    def closed(self) -> bool:
        """Check if this subscription is closed."""
        return self._closed

    async def __aiter__(self) -> AsyncIterator[LivingProtocolEvent]:
        """Iterate over events asynchronously."""
        while not self._closed:
            event = await self.get()
            if event is None:
                break
            yield event


class SubscriptionManager:
    """Manages multiple event subscriptions.

    The subscription manager distributes events to all active
    subscriptions and handles subscription lifecycle.

    Example:
        >>> manager = SubscriptionManager()
        >>> sub = manager.subscribe(event_types=["PhaseTransitioned"])
        >>> async for event in sub:
        ...     print(f"Phase changed: {event}")
    """

    def __init__(self) -> None:
        self._subscriptions: list[EventSubscription] = []
        self._lock = asyncio.Lock()

    def subscribe(
        self,
        event_types: list[str] | None = None,
        filter_fn: Callable[[LivingProtocolEvent], bool] | None = None,
        max_queue_size: int = 1000,
    ) -> EventSubscription:
        """Create a new event subscription.

        Args:
            event_types: Optional list of event types to subscribe to.
            filter_fn: Optional custom filter function.
            max_queue_size: Maximum events to buffer.

        Returns:
            A new EventSubscription that will receive matching events.
        """
        sub = EventSubscription(
            event_types=event_types,
            filter_fn=filter_fn,
            max_queue_size=max_queue_size,
        )
        self._subscriptions.append(sub)
        return sub

    def subscribe_phase_transitions(self) -> EventSubscription:
        """Convenience method to subscribe to phase transition events."""
        return self.subscribe(event_types=["PhaseTransitioned"])

    def subscribe_cycle_starts(self) -> EventSubscription:
        """Convenience method to subscribe to cycle start events."""
        return self.subscribe(event_types=["CycleStarted"])

    def subscribe_phase(self, phase: CyclePhase) -> EventSubscription:
        """Subscribe to events that occur during a specific phase.

        Note: This filters events based on the phase field in the event data,
        which is only available for certain event types.
        """

        def phase_filter(event: LivingProtocolEvent) -> bool:
            # Check if event has phase information
            if "transition" in event.data:
                trans = event.data["transition"]
                return trans.get("to") == phase.value or trans.get("from") == phase.value
            return False

        return self.subscribe(filter_fn=phase_filter)

    async def publish(self, event: LivingProtocolEvent) -> int:
        """Publish an event to all matching subscriptions.

        Args:
            event: The event to publish.

        Returns:
            The number of subscriptions that received the event.
        """
        count = 0
        async with self._lock:
            # Remove closed subscriptions
            self._subscriptions = [s for s in self._subscriptions if not s.closed]

            # Publish to all matching subscriptions
            for sub in self._subscriptions:
                if await sub.push(event):
                    count += 1

        return count

    def unsubscribe(self, subscription: EventSubscription) -> bool:
        """Remove a subscription.

        Args:
            subscription: The subscription to remove.

        Returns:
            True if the subscription was found and removed.
        """
        subscription.close()
        try:
            self._subscriptions.remove(subscription)
            return True
        except ValueError:
            return False

    def close_all(self) -> None:
        """Close all subscriptions."""
        for sub in self._subscriptions:
            sub.close()
        self._subscriptions.clear()

    @property
    def subscription_count(self) -> int:
        """Get the number of active subscriptions."""
        return len([s for s in self._subscriptions if not s.closed])


def create_phase_filter(phases: list[CyclePhase]) -> Callable[[LivingProtocolEvent], bool]:
    """Create a filter function for specific phases.

    Args:
        phases: List of phases to include.

    Returns:
        A filter function suitable for EventSubscription.
    """
    phase_values = {p.value for p in phases}

    def filter_fn(event: LivingProtocolEvent) -> bool:
        if event.event_type == "PhaseTransitioned":
            if "transition" in event.data:
                trans = event.data["transition"]
                return trans.get("to") in phase_values
        return False

    return filter_fn


def create_event_type_filter(types: list[str]) -> Callable[[LivingProtocolEvent], bool]:
    """Create a filter function for specific event types.

    Args:
        types: List of event type strings to include.

    Returns:
        A filter function suitable for EventSubscription.
    """
    type_set = set(types)

    def filter_fn(event: LivingProtocolEvent) -> bool:
        return event.event_type in type_set

    return filter_fn
