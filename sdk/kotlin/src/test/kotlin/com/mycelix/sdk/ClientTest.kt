package com.mycelix.sdk

import kotlinx.coroutines.test.runTest
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows

class ClientTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    // MARK: - CyclePhase Tests

    @Test
    fun `CyclePhase values are correct`() {
        assertEquals("dormant", json.encodeToString(CyclePhase.DORMANT).trim('"'))
        assertEquals("germination", json.encodeToString(CyclePhase.GERMINATION).trim('"'))
        assertEquals("growth", json.encodeToString(CyclePhase.GROWTH).trim('"'))
        assertEquals("fruiting", json.encodeToString(CyclePhase.FRUITING).trim('"'))
        assertEquals("sporulation", json.encodeToString(CyclePhase.SPORULATION).trim('"'))
    }

    @Test
    fun `CyclePhase displayName returns capitalized name`() {
        assertEquals("Dormant", CyclePhase.DORMANT.displayName)
        assertEquals("Germination", CyclePhase.GERMINATION.displayName)
        assertEquals("Growth", CyclePhase.GROWTH.displayName)
        assertEquals("Fruiting", CyclePhase.FRUITING.displayName)
        assertEquals("Sporulation", CyclePhase.SPORULATION.displayName)
    }

    // MARK: - CycleState Tests

    @Test
    fun `CycleState serialization works correctly`() {
        val state = CycleState(
            phase = CyclePhase.GROWTH,
            cycleNumber = 5,
            phaseStartTime = 1000L,
            phaseEndTime = 2000L,
            phaseDuration = 1000L,
            progress = 0.5,
            metadata = null
        )

        val jsonString = json.encodeToString(state)
        val decoded = json.decodeFromString<CycleState>(jsonString)

        assertEquals(state.phase, decoded.phase)
        assertEquals(state.cycleNumber, decoded.cycleNumber)
        assertEquals(state.phaseStartTime, decoded.phaseStartTime)
        assertEquals(state.phaseEndTime, decoded.phaseEndTime)
        assertEquals(state.phaseDuration, decoded.phaseDuration)
        assertEquals(state.progress, decoded.progress)
    }

    @Test
    fun `CycleState equality works`() {
        val state1 = CycleState(
            phase = CyclePhase.GROWTH,
            cycleNumber = 1,
            phaseStartTime = 1000L,
            phaseEndTime = 2000L,
            phaseDuration = 1000L,
            progress = 0.5
        )

        val state2 = CycleState(
            phase = CyclePhase.GROWTH,
            cycleNumber = 1,
            phaseStartTime = 1000L,
            phaseEndTime = 2000L,
            phaseDuration = 1000L,
            progress = 0.5
        )

        assertEquals(state1, state2)
    }

    // MARK: - Event Tests

    @Test
    fun `PhaseTransitionEvent serialization works`() {
        val event = PhaseTransitionEvent(
            fromPhase = CyclePhase.DORMANT,
            toPhase = CyclePhase.GERMINATION,
            cycleNumber = 1,
            timestamp = 12345L
        )

        val jsonString = json.encodeToString(event)
        val decoded = json.decodeFromString<PhaseTransitionEvent>(jsonString)

        assertEquals(EventType.PHASE_TRANSITION, decoded.type)
        assertEquals(CyclePhase.DORMANT, decoded.fromPhase)
        assertEquals(CyclePhase.GERMINATION, decoded.toPhase)
        assertEquals(1, decoded.cycleNumber)
        assertEquals(12345L, decoded.timestamp)
    }

    @Test
    fun `CycleCompletionEvent serialization works`() {
        val event = CycleCompletionEvent(
            cycleNumber = 5,
            duration = 10000L,
            timestamp = 12345L
        )

        val jsonString = json.encodeToString(event)
        val decoded = json.decodeFromString<CycleCompletionEvent>(jsonString)

        assertEquals(EventType.CYCLE_COMPLETE, decoded.type)
        assertEquals(5, decoded.cycleNumber)
        assertEquals(10000L, decoded.duration)
        assertEquals(12345L, decoded.timestamp)
    }

    @Test
    fun `StateUpdateEvent serialization works`() {
        val state = CycleState(
            phase = CyclePhase.FRUITING,
            cycleNumber = 3,
            phaseStartTime = 1000L,
            phaseEndTime = 2000L,
            phaseDuration = 1000L,
            progress = 0.75
        )

        val event = StateUpdateEvent(
            state = state,
            timestamp = 12345L
        )

        val jsonString = json.encodeToString(event)
        val decoded = json.decodeFromString<StateUpdateEvent>(jsonString)

        assertEquals(EventType.STATE_UPDATE, decoded.type)
        assertEquals(CyclePhase.FRUITING, decoded.state.phase)
        assertEquals(12345L, decoded.timestamp)
    }

    @Test
    fun `ErrorEvent serialization works`() {
        val event = ErrorEvent(
            code = "CONNECTION_ERROR",
            message = "Failed to connect",
            timestamp = 12345L
        )

        val jsonString = json.encodeToString(event)
        val decoded = json.decodeFromString<ErrorEvent>(jsonString)

        assertEquals(EventType.ERROR, decoded.type)
        assertEquals("CONNECTION_ERROR", decoded.code)
        assertEquals("Failed to connect", decoded.message)
    }

    // MARK: - Configuration Tests

    @Test
    fun `ClientConfiguration has correct defaults`() {
        val config = ClientConfiguration(url = "wss://example.com/ws")

        assertEquals("wss://example.com/ws", config.url)
        assertTrue(config.autoReconnect)
        assertEquals(3000L, config.reconnectIntervalMs)
        assertEquals(10, config.maxReconnectAttempts)
        assertEquals(30000L, config.heartbeatIntervalMs)
        assertEquals(10000L, config.connectionTimeoutMs)
    }

    @Test
    fun `ClientConfiguration custom values work`() {
        val config = ClientConfiguration(
            url = "wss://example.com/ws",
            autoReconnect = false,
            reconnectIntervalMs = 5000L,
            maxReconnectAttempts = 5,
            heartbeatIntervalMs = 60000L,
            connectionTimeoutMs = 20000L
        )

        assertFalse(config.autoReconnect)
        assertEquals(5000L, config.reconnectIntervalMs)
        assertEquals(5, config.maxReconnectAttempts)
        assertEquals(60000L, config.heartbeatIntervalMs)
        assertEquals(20000L, config.connectionTimeoutMs)
    }

    // MARK: - SubscriptionOptions Tests

    @Test
    fun `SubscriptionOptions default values are null`() {
        val options = SubscriptionOptions()

        assertNull(options.eventTypes)
        assertNull(options.phases)
        assertNull(options.cycleNumbers)
    }

    @Test
    fun `SubscriptionOptions custom values work`() {
        val options = SubscriptionOptions(
            eventTypes = listOf(EventType.PHASE_TRANSITION, EventType.CYCLE_COMPLETE),
            phases = listOf(CyclePhase.GROWTH, CyclePhase.FRUITING),
            cycleNumbers = listOf(1, 2, 3)
        )

        assertEquals(listOf(EventType.PHASE_TRANSITION, EventType.CYCLE_COMPLETE), options.eventTypes)
        assertEquals(listOf(CyclePhase.GROWTH, CyclePhase.FRUITING), options.phases)
        assertEquals(listOf(1, 2, 3), options.cycleNumbers)
    }

    // MARK: - Exception Tests

    @Test
    fun `LivingProtocolException messages are correct`() {
        assertEquals(
            "Not connected to the Living Protocol server",
            LivingProtocolException.NotConnected().message
        )

        assertEquals(
            "Connection failed: timeout",
            LivingProtocolException.ConnectionFailed("timeout").message
        )

        assertEquals(
            "Connection timed out",
            LivingProtocolException.ConnectionTimeout().message
        )

        assertEquals(
            "Request timeout: getCycleState",
            LivingProtocolException.RequestTimeout("getCycleState").message
        )

        assertEquals(
            "Invalid response from server",
            LivingProtocolException.InvalidResponse().message
        )

        assertEquals(
            "RPC error (100): Test error",
            LivingProtocolException.RpcError(100, "Test error").message
        )
    }

    // MARK: - Client Creation Tests

    @Test
    fun `Client creation with URL string`() {
        val client = LivingProtocolClient("wss://example.com/ws")

        assertEquals(ConnectionState.DISCONNECTED, client.connectionState.value)
        assertNull(client.cycleState.value)

        client.close()
    }

    @Test
    fun `Client creation with configuration`() {
        val config = ClientConfiguration(
            url = "wss://example.com/ws",
            autoReconnect = false
        )
        val client = LivingProtocolClient(config)

        assertEquals(ConnectionState.DISCONNECTED, client.connectionState.value)

        client.close()
    }

    // MARK: - Subscription Tests

    @Test
    fun `Subscription can be created and cancelled`() {
        val client = LivingProtocolClient("wss://example.com/ws")

        var eventReceived = false
        val subscription = client.subscribe { event ->
            eventReceived = true
        }

        assertNotNull(subscription.id)
        assertTrue(subscription.id.startsWith("sub_"))

        subscription.cancel()

        client.close()
    }

    @Test
    fun `Multiple subscriptions can be created`() {
        val client = LivingProtocolClient("wss://example.com/ws")

        val sub1 = client.subscribe { }
        val sub2 = client.subscribeToEvents(listOf(EventType.PHASE_TRANSITION)) { }
        val sub3 = client.subscribeToPhases(listOf(CyclePhase.FRUITING)) { }

        assertNotEquals(sub1.id, sub2.id)
        assertNotEquals(sub2.id, sub3.id)

        sub1.cancel()
        sub2.cancel()
        sub3.cancel()

        client.close()
    }

    // MARK: - JSON Decoding Tests

    @Test
    fun `PhaseTransitionEvent decodes from JSON`() {
        val jsonString = """
            {
                "type": "phase_transition",
                "fromPhase": "dormant",
                "toPhase": "germination",
                "cycleNumber": 1,
                "timestamp": 12345
            }
        """.trimIndent()

        val event = json.decodeFromString<PhaseTransitionEvent>(jsonString)

        assertEquals(EventType.PHASE_TRANSITION, event.type)
        assertEquals(CyclePhase.DORMANT, event.fromPhase)
        assertEquals(CyclePhase.GERMINATION, event.toPhase)
    }

    @Test
    fun `CycleState decodes from JSON`() {
        val jsonString = """
            {
                "phase": "growth",
                "cycleNumber": 5,
                "phaseStartTime": 1000,
                "phaseEndTime": 2000,
                "phaseDuration": 1000,
                "progress": 0.5
            }
        """.trimIndent()

        val state = json.decodeFromString<CycleState>(jsonString)

        assertEquals(CyclePhase.GROWTH, state.phase)
        assertEquals(5, state.cycleNumber)
        assertEquals(0.5, state.progress)
    }

    // MARK: - RPC Types Tests

    @Test
    fun `RpcRequest serialization works`() {
        val request = RpcRequest(
            id = 1,
            method = "getCycleState"
        )

        val jsonString = json.encodeToString(request)

        assertTrue(jsonString.contains("\"jsonrpc\":\"2.0\""))
        assertTrue(jsonString.contains("\"id\":1"))
        assertTrue(jsonString.contains("\"method\":\"getCycleState\""))
    }

    @Test
    fun `RpcResponse deserialization works`() {
        val jsonString = """
            {
                "jsonrpc": "2.0",
                "id": 1,
                "result": {"phase": "growth"}
            }
        """.trimIndent()

        val response = json.decodeFromString<RpcResponse>(jsonString)

        assertEquals("2.0", response.jsonrpc)
        assertEquals(1, response.id)
        assertNotNull(response.result)
        assertNull(response.error)
    }

    @Test
    fun `RpcResponse with error deserialization works`() {
        val jsonString = """
            {
                "jsonrpc": "2.0",
                "id": 1,
                "error": {
                    "code": -32600,
                    "message": "Invalid Request"
                }
            }
        """.trimIndent()

        val response = json.decodeFromString<RpcResponse>(jsonString)

        assertEquals(1, response.id)
        assertNull(response.result)
        assertNotNull(response.error)
        assertEquals(-32600, response.error?.code)
        assertEquals("Invalid Request", response.error?.message)
    }
}
