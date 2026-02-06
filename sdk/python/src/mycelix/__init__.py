"""Mycelix Living Protocol Python SDK.

This SDK provides a Python client for interacting with the Mycelix Living Protocol
WebSocket server, enabling real-time access to metabolism cycle state and events.

Example:
    >>> import asyncio
    >>> from mycelix import LivingProtocolClient
    >>>
    >>> async def main():
    ...     async with LivingProtocolClient("ws://localhost:8888") as client:
    ...         state = await client.get_current_state()
    ...         print(f"Current phase: {state.current_phase}")
    ...
    >>> asyncio.run(main())
"""

from mycelix.client import LivingProtocolClient
from mycelix.subscription import EventSubscription, SubscriptionManager
from mycelix.types import (
    CyclePhase,
    CycleState,
    LivingProtocolEvent,
    PhaseMetrics,
    PhaseTransition,
)

__version__ = "0.1.0"
__all__ = [
    "LivingProtocolClient",
    "CyclePhase",
    "CycleState",
    "PhaseTransition",
    "PhaseMetrics",
    "LivingProtocolEvent",
    "EventSubscription",
    "SubscriptionManager",
]
