package com.mycelix.sdk

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

/**
 * Represents the phases in the Living Protocol lifecycle
 */
@Serializable
enum class CyclePhase {
    @SerialName("dormant")
    DORMANT,

    @SerialName("germination")
    GERMINATION,

    @SerialName("growth")
    GROWTH,

    @SerialName("fruiting")
    FRUITING,

    @SerialName("sporulation")
    SPORULATION;

    /**
     * Human-readable display name
     */
    val displayName: String
        get() = name.lowercase().replaceFirstChar { it.uppercase() }
}

/**
 * Represents the WebSocket connection state
 */
enum class ConnectionState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
    RECONNECTING,
    ERROR
}

/**
 * Represents the current state of the protocol cycle
 */
@Serializable
data class CycleState(
    val phase: CyclePhase,
    val cycleNumber: Int,
    val phaseStartTime: Long,
    val phaseEndTime: Long,
    val phaseDuration: Long,
    val progress: Double,
    val metadata: Map<String, JsonElement>? = null
) {
    /**
     * Time remaining in the current phase (ms)
     */
    val timeRemaining: Long
        get() = maxOf(0L, phaseEndTime - System.currentTimeMillis())
}

/**
 * Event types in the Living Protocol
 */
@Serializable
enum class EventType {
    @SerialName("phase_transition")
    PHASE_TRANSITION,

    @SerialName("cycle_complete")
    CYCLE_COMPLETE,

    @SerialName("state_update")
    STATE_UPDATE,

    @SerialName("error")
    ERROR
}

/**
 * Base interface for all Living Protocol events
 */
sealed interface LivingProtocolEvent {
    val type: EventType
    val timestamp: Long
}

/**
 * Event emitted when transitioning between phases
 */
@Serializable
data class PhaseTransitionEvent(
    override val type: EventType = EventType.PHASE_TRANSITION,
    val fromPhase: CyclePhase,
    val toPhase: CyclePhase,
    val cycleNumber: Int,
    override val timestamp: Long
) : LivingProtocolEvent

/**
 * Event emitted when a cycle completes
 */
@Serializable
data class CycleCompletionEvent(
    override val type: EventType = EventType.CYCLE_COMPLETE,
    val cycleNumber: Int,
    val duration: Long,
    override val timestamp: Long
) : LivingProtocolEvent

/**
 * Event emitted when state updates
 */
@Serializable
data class StateUpdateEvent(
    override val type: EventType = EventType.STATE_UPDATE,
    val state: CycleState,
    override val timestamp: Long
) : LivingProtocolEvent

/**
 * Event emitted on error
 */
@Serializable
data class ErrorEvent(
    override val type: EventType = EventType.ERROR,
    val code: String,
    val message: String,
    override val timestamp: Long
) : LivingProtocolEvent

/**
 * Options for filtering event subscriptions
 */
data class SubscriptionOptions(
    val eventTypes: List<EventType>? = null,
    val phases: List<CyclePhase>? = null,
    val cycleNumbers: List<Int>? = null
)

/**
 * Configuration options for the Living Protocol client
 */
data class ClientConfiguration(
    val url: String,
    val autoReconnect: Boolean = true,
    val reconnectIntervalMs: Long = 3000L,
    val maxReconnectAttempts: Int = 10,
    val heartbeatIntervalMs: Long = 30000L,
    val connectionTimeoutMs: Long = 10000L,
    val requestTimeoutMs: Long = 30000L
)

/**
 * Errors that can occur in the Living Protocol client
 */
sealed class LivingProtocolException(message: String, cause: Throwable? = null) : Exception(message, cause) {
    class NotConnected : LivingProtocolException("Not connected to the Living Protocol server")
    class ConnectionFailed(reason: String) : LivingProtocolException("Connection failed: $reason")
    class ConnectionTimeout : LivingProtocolException("Connection timed out")
    class RequestTimeout(method: String) : LivingProtocolException("Request timeout: $method")
    class InvalidResponse : LivingProtocolException("Invalid response from server")
    class RpcError(val code: Int, override val message: String) : LivingProtocolException("RPC error ($code): $message")
    class EncodingError(cause: Throwable) : LivingProtocolException("Failed to encode request", cause)
    class DecodingError(details: String) : LivingProtocolException("Failed to decode response: $details")
}

/**
 * Internal RPC request structure
 */
@Serializable
internal data class RpcRequest(
    val jsonrpc: String = "2.0",
    val id: Int,
    val method: String,
    val params: Map<String, JsonElement>? = null
)

/**
 * Internal RPC response structure
 */
@Serializable
internal data class RpcResponse(
    val jsonrpc: String,
    val id: Int? = null,
    val result: JsonElement? = null,
    val error: RpcError? = null
)

/**
 * Internal RPC error structure
 */
@Serializable
internal data class RpcError(
    val code: Int,
    val message: String,
    val data: JsonElement? = null
)

/**
 * Internal RPC notification structure
 */
@Serializable
internal data class RpcNotification(
    val jsonrpc: String,
    val method: String,
    val params: JsonElement? = null
)

/**
 * Internal subscription result
 */
@Serializable
internal data class SubscribeResult(
    val subscriptionId: String
)
