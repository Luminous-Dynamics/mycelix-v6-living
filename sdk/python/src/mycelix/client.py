"""WebSocket client for the Mycelix Living Protocol.

This module provides the main client class for connecting to and
interacting with the Living Protocol WebSocket server.
"""

import asyncio
import json
import uuid
from collections.abc import AsyncIterator
from dataclasses import dataclass, field
from typing import Any

import aiohttp
import websockets
from websockets.asyncio.client import ClientConnection

from mycelix.subscription import EventSubscription, SubscriptionManager
from mycelix.types import CyclePhase, CycleState, LivingProtocolEvent, PhaseMetrics, PhaseTransition


class LivingProtocolError(Exception):
    """Base exception for Living Protocol errors."""

    pass


class ConnectionError(LivingProtocolError):
    """Raised when connection to the server fails."""

    pass


class RpcError(LivingProtocolError):
    """Raised when an RPC call fails."""

    def __init__(self, code: int, message: str):
        self.code = code
        self.message = message
        super().__init__(f"RPC Error {code}: {message}")


@dataclass
class ClientConfig:
    """Configuration for the Living Protocol client.

    Attributes:
        url: WebSocket server URL (e.g., "ws://localhost:8888").
        rest_url: Optional REST API URL (e.g., "http://localhost:8889").
            If not provided, derived from ws URL.
        reconnect: Whether to automatically reconnect on disconnect.
        reconnect_delay: Delay between reconnection attempts in seconds.
        max_reconnect_attempts: Maximum number of reconnection attempts.
        ping_interval: Interval between ping messages in seconds.
        request_timeout: Timeout for RPC requests in seconds.
    """

    url: str
    rest_url: str | None = None
    reconnect: bool = True
    reconnect_delay: float = 1.0
    max_reconnect_attempts: int = 10
    ping_interval: float = 30.0
    request_timeout: float = 10.0


class LivingProtocolClient:
    """Async WebSocket client for the Mycelix Living Protocol.

    This client provides methods for querying protocol state and
    subscribing to real-time events.

    Example:
        >>> async with LivingProtocolClient("ws://localhost:8888") as client:
        ...     state = await client.get_current_state()
        ...     print(f"Phase: {state.current_phase}")
        ...
        ...     async for event in client.subscribe():
        ...         print(f"Event: {event}")
    """

    def __init__(self, url: str | ClientConfig) -> None:
        """Initialize the client.

        Args:
            url: WebSocket URL or ClientConfig object.
        """
        if isinstance(url, str):
            self.config = ClientConfig(url=url)
        else:
            self.config = url

        self._ws: ClientConnection | None = None
        self._subscriptions = SubscriptionManager()
        self._pending_requests: dict[str, asyncio.Future[Any]] = {}
        self._receive_task: asyncio.Task[None] | None = None
        self._ping_task: asyncio.Task[None] | None = None
        self._connected = asyncio.Event()
        self._closed = False
        self._reconnect_count = 0

    async def connect(self) -> None:
        """Connect to the WebSocket server.

        Raises:
            ConnectionError: If connection fails.
        """
        try:
            self._ws = await websockets.connect(
                self.config.url,
                ping_interval=None,  # We handle pings ourselves
            )
            self._connected.set()
            self._reconnect_count = 0

            # Start background tasks
            self._receive_task = asyncio.create_task(self._receive_loop())
            self._ping_task = asyncio.create_task(self._ping_loop())

        except Exception as e:
            raise ConnectionError(f"Failed to connect to {self.config.url}: {e}") from e

    async def disconnect(self) -> None:
        """Disconnect from the server."""
        self._closed = True
        self._connected.clear()

        # Cancel background tasks
        if self._receive_task:
            self._receive_task.cancel()
            try:
                await self._receive_task
            except asyncio.CancelledError:
                pass

        if self._ping_task:
            self._ping_task.cancel()
            try:
                await self._ping_task
            except asyncio.CancelledError:
                pass

        # Close WebSocket
        if self._ws:
            await self._ws.close()
            self._ws = None

        # Close subscriptions
        self._subscriptions.close_all()

        # Cancel pending requests
        for future in self._pending_requests.values():
            future.cancel()
        self._pending_requests.clear()

    async def __aenter__(self) -> "LivingProtocolClient":
        """Context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, *args: Any) -> None:
        """Context manager exit."""
        await self.disconnect()

    # =========================================================================
    # RPC Methods
    # =========================================================================

    async def get_current_state(self) -> CycleState:
        """Get the current cycle state.

        Returns:
            The current state of the metabolism cycle.

        Raises:
            RpcError: If the RPC call fails.
            ConnectionError: If not connected.
        """
        result = await self._rpc_call("getCycleState")
        return CycleState.from_dict(result)

    async def get_current_phase(self) -> CyclePhase:
        """Get the current cycle phase.

        Returns:
            The current phase.

        Raises:
            RpcError: If the RPC call fails.
        """
        result = await self._rpc_call("getCurrentPhase")
        return CyclePhase.from_str(result)

    async def get_cycle_number(self) -> int:
        """Get the current cycle number.

        Returns:
            The current cycle number.
        """
        return await self._rpc_call("getCycleNumber")

    async def get_transition_history(self) -> list[PhaseTransition]:
        """Get the history of phase transitions.

        Returns:
            List of phase transitions for the current cycle.
        """
        result = await self._rpc_call("getTransitionHistory")
        return [PhaseTransition.from_dict(t) for t in result]

    async def get_phase_metrics(self, phase: CyclePhase) -> PhaseMetrics:
        """Get metrics for a specific phase.

        Args:
            phase: The phase to get metrics for.

        Returns:
            The phase metrics.
        """
        result = await self._rpc_call("getPhaseMetrics", {"phase": phase.value})
        return PhaseMetrics.from_dict(result)

    async def is_operation_permitted(self, operation: str) -> bool:
        """Check if an operation is permitted in the current phase.

        Args:
            operation: The operation to check (e.g., "vote", "kenosis").

        Returns:
            True if the operation is permitted.
        """
        return await self._rpc_call("isOperationPermitted", {"operation": operation})

    # =========================================================================
    # REST API Methods
    # =========================================================================

    def _get_rest_url(self) -> str:
        """Get the REST API base URL."""
        if self.config.rest_url:
            return self.config.rest_url.rstrip("/")

        # Derive from WebSocket URL
        ws_url = self.config.url
        if ws_url.startswith("ws://"):
            base = ws_url[5:]  # Remove "ws://"
        elif ws_url.startswith("wss://"):
            base = ws_url[6:]  # Remove "wss://"
        else:
            base = ws_url

        # Parse host and port, increment port by 1 for REST
        if ":" in base:
            host, port_str = base.rsplit(":", 1)
            try:
                port = int(port_str.split("/")[0]) + 1
                return f"http://{host}:{port}"
            except ValueError:
                pass

        return f"http://{base}:8889"

    async def rest_get_state(self) -> CycleState:
        """Get current state via REST API.

        This is an alternative to the WebSocket RPC method.
        """
        url = f"{self._get_rest_url()}/api/v1/state"
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                response.raise_for_status()
                data = await response.json()
                return CycleState.from_dict(data)

    async def rest_get_phase(self) -> CyclePhase:
        """Get current phase via REST API."""
        url = f"{self._get_rest_url()}/api/v1/phase"
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                response.raise_for_status()
                data = await response.json()
                return CyclePhase.from_str(data["phase"])

    async def rest_get_history(self) -> list[PhaseTransition]:
        """Get transition history via REST API."""
        url = f"{self._get_rest_url()}/api/v1/history"
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                response.raise_for_status()
                data = await response.json()
                return [PhaseTransition.from_dict(t) for t in data["transitions"]]

    async def rest_get_metrics(self, phase: CyclePhase) -> PhaseMetrics:
        """Get phase metrics via REST API."""
        url = f"{self._get_rest_url()}/api/v1/metrics/{phase.value}"
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                response.raise_for_status()
                data = await response.json()
                return PhaseMetrics.from_dict(data)

    # =========================================================================
    # Subscription Methods
    # =========================================================================

    def subscribe(
        self,
        event_types: list[str] | None = None,
    ) -> EventSubscription:
        """Subscribe to Living Protocol events.

        Args:
            event_types: Optional list of event types to filter.
                If None, all events are included.

        Returns:
            An EventSubscription that can be iterated over.

        Example:
            >>> async for event in client.subscribe():
            ...     print(f"Event: {event}")
        """
        return self._subscriptions.subscribe(event_types=event_types)

    def subscribe_phase_transitions(self) -> EventSubscription:
        """Subscribe only to phase transition events."""
        return self._subscriptions.subscribe_phase_transitions()

    def subscribe_cycle_starts(self) -> EventSubscription:
        """Subscribe only to cycle start events."""
        return self._subscriptions.subscribe_cycle_starts()

    async def events(self) -> AsyncIterator[LivingProtocolEvent]:
        """Async generator for all events.

        Yields:
            Living Protocol events as they arrive.

        Example:
            >>> async for event in client.events():
            ...     print(f"Event: {event.event_type}")
        """
        sub = self.subscribe()
        try:
            async for event in sub:
                yield event
        finally:
            self._subscriptions.unsubscribe(sub)

    # =========================================================================
    # Internal Methods
    # =========================================================================

    async def _rpc_call(self, method: str, params: dict[str, Any] | None = None) -> Any:
        """Make an RPC call to the server.

        Args:
            method: The RPC method name.
            params: Optional parameters.

        Returns:
            The result from the server.

        Raises:
            RpcError: If the call fails.
            ConnectionError: If not connected.
        """
        if not self._ws or not self._connected.is_set():
            raise ConnectionError("Not connected to server")

        request_id = str(uuid.uuid4())
        request = {
            "id": request_id,
            "method": method,
        }
        if params:
            request["params"] = params

        # Create future for response
        future: asyncio.Future[Any] = asyncio.get_event_loop().create_future()
        self._pending_requests[request_id] = future

        try:
            # Send request
            await self._ws.send(json.dumps(request))

            # Wait for response with timeout
            result = await asyncio.wait_for(future, timeout=self.config.request_timeout)
            return result

        except asyncio.TimeoutError:
            raise RpcError(-32000, f"Request timeout for method: {method}")
        finally:
            self._pending_requests.pop(request_id, None)

    async def _receive_loop(self) -> None:
        """Background task to receive and dispatch messages."""
        while not self._closed and self._ws:
            try:
                message = await self._ws.recv()
                await self._handle_message(message)
            except websockets.ConnectionClosed:
                self._connected.clear()
                if not self._closed and self.config.reconnect:
                    await self._attempt_reconnect()
                break
            except Exception as e:
                if not self._closed:
                    # Log error but continue
                    print(f"Receive error: {e}")
                    continue

    async def _handle_message(self, message: str) -> None:
        """Handle an incoming WebSocket message."""
        try:
            data = json.loads(message)
        except json.JSONDecodeError:
            return

        # Check if this is an RPC response
        if "id" in data:
            request_id = data["id"]
            if request_id in self._pending_requests:
                future = self._pending_requests[request_id]
                if "error" in data and data["error"]:
                    error = data["error"]
                    future.set_exception(RpcError(error.get("code", -1), error.get("message", "")))
                elif "result" in data:
                    future.set_result(data["result"])
                return

        # Check if this is a pong response
        if data.get("type") == "pong":
            return

        # Otherwise, treat as an event
        event = LivingProtocolEvent.from_dict(data)
        await self._subscriptions.publish(event)

    async def _ping_loop(self) -> None:
        """Background task to send periodic pings."""
        while not self._closed and self._ws:
            try:
                await asyncio.sleep(self.config.ping_interval)
                if self._ws and self._connected.is_set():
                    await self._ws.send(json.dumps({"type": "ping"}))
            except Exception:
                pass

    async def _attempt_reconnect(self) -> None:
        """Attempt to reconnect to the server."""
        while (
            not self._closed
            and self._reconnect_count < self.config.max_reconnect_attempts
        ):
            self._reconnect_count += 1
            delay = self.config.reconnect_delay * (2 ** (self._reconnect_count - 1))
            delay = min(delay, 60.0)  # Cap at 60 seconds

            print(f"Reconnecting in {delay}s (attempt {self._reconnect_count})...")
            await asyncio.sleep(delay)

            try:
                await self.connect()
                print("Reconnected successfully")
                return
            except Exception as e:
                print(f"Reconnection failed: {e}")

        print("Max reconnection attempts reached")
