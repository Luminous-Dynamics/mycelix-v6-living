package com.mycelix.sdk

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.*
import okhttp3.*
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/**
 * Client for connecting to the Living Protocol server
 */
class LivingProtocolClient(
    private val configuration: ClientConfiguration
) {
    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private val okHttpClient = OkHttpClient.Builder()
        .connectTimeout(configuration.connectionTimeoutMs, TimeUnit.MILLISECONDS)
        .readTimeout(0, TimeUnit.MILLISECONDS) // No read timeout for WebSocket
        .writeTimeout(configuration.requestTimeoutMs, TimeUnit.MILLISECONDS)
        .build()

    private var webSocket: WebSocket? = null
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private val _connectionState = MutableStateFlow(ConnectionState.DISCONNECTED)
    val connectionState: StateFlow<ConnectionState> = _connectionState.asStateFlow()

    private val _cycleState = MutableStateFlow<CycleState?>(null)
    val cycleState: StateFlow<CycleState?> = _cycleState.asStateFlow()

    private val requestId = AtomicInteger(0)
    private val pendingRequests = ConcurrentHashMap<Int, CompletableDeferred<JsonElement>>()
    private val subscriptions = ConcurrentHashMap<String, (LivingProtocolEvent) -> Unit>()

    private var reconnectAttempts = 0
    private var reconnectJob: Job? = null
    private var heartbeatJob: Job? = null
    private var serverSubscriptionId: String? = null

    /**
     * Create client with URL string
     */
    constructor(url: String) : this(ClientConfiguration(url))

    /**
     * Connect to the Living Protocol server
     */
    suspend fun connect() {
        if (_connectionState.value == ConnectionState.CONNECTED) {
            return
        }

        _connectionState.value = ConnectionState.CONNECTING

        try {
            establishConnection()
            _connectionState.value = ConnectionState.CONNECTED
            reconnectAttempts = 0
            startHeartbeat()
            resubscribeAll()

            // Fetch initial state
            try {
                _cycleState.value = getCycleState()
            } catch (e: Exception) {
                // Non-fatal - state will be updated via subscription
            }
        } catch (e: Exception) {
            _connectionState.value = ConnectionState.ERROR
            throw e
        }
    }

    /**
     * Disconnect from the server
     */
    fun disconnect() {
        reconnectJob?.cancel()
        reconnectJob = null
        heartbeatJob?.cancel()
        heartbeatJob = null

        webSocket?.close(1000, "Client disconnect")
        webSocket = null

        _connectionState.value = ConnectionState.DISCONNECTED
        serverSubscriptionId = null

        // Cancel pending requests
        pendingRequests.forEach { (_, deferred) ->
            deferred.completeExceptionally(LivingProtocolException.NotConnected())
        }
        pendingRequests.clear()
    }

    /**
     * Close the client and release resources
     */
    fun close() {
        disconnect()
        scope.cancel()
    }

    // MARK: - RPC Methods

    /**
     * Get the current cycle state
     */
    suspend fun getCycleState(): CycleState {
        return rpcCall("getCycleState")
    }

    /**
     * Get the current phase
     */
    suspend fun getCurrentPhase(): CyclePhase {
        return rpcCall("getCurrentPhase")
    }

    /**
     * Get the current cycle number
     */
    suspend fun getCycleNumber(): Int {
        return rpcCall("getCycleNumber")
    }

    /**
     * Get the phase progress (0-1)
     */
    suspend fun getPhaseProgress(): Double {
        return rpcCall("getPhaseProgress")
    }

    /**
     * Get time remaining in current phase (ms)
     */
    suspend fun getTimeRemaining(): Long {
        return rpcCall("getTimeRemaining")
    }

    /**
     * Get cycle history
     * @param limit Maximum number of cycles to return
     */
    suspend fun getCycleHistory(limit: Int? = null): List<CycleState> {
        return if (limit != null) {
            rpcCall("getCycleHistory", mapOf("limit" to JsonPrimitive(limit)))
        } else {
            rpcCall("getCycleHistory")
        }
    }

    /**
     * Advance to the next phase (if allowed)
     */
    suspend fun advancePhase(): CycleState {
        return rpcCall("advancePhase")
    }

    // MARK: - Subscriptions

    /**
     * Subscribe to events with a callback
     * @param options Subscription filter options
     * @param callback Called when matching events occur
     * @return Subscription handle
     */
    fun subscribe(
        options: SubscriptionOptions = SubscriptionOptions(),
        callback: (LivingProtocolEvent) -> Unit
    ): Subscription {
        val id = generateSubscriptionId()

        subscriptions[id] = { event ->
            if (eventMatchesOptions(event, options)) {
                callback(event)
            }
        }

        // Setup server subscription if needed
        if (_connectionState.value == ConnectionState.CONNECTED) {
            scope.launch {
                setupServerSubscription()
            }
        }

        return Subscription(id) {
            subscriptions.remove(id)
            if (subscriptions.isEmpty()) {
                scope.launch {
                    teardownServerSubscription()
                }
            }
        }
    }

    /**
     * Subscribe to specific event types
     */
    fun subscribeToEvents(
        eventTypes: List<EventType>,
        callback: (LivingProtocolEvent) -> Unit
    ): Subscription {
        return subscribe(SubscriptionOptions(eventTypes = eventTypes), callback)
    }

    /**
     * Subscribe to specific phases
     */
    fun subscribeToPhases(
        phases: List<CyclePhase>,
        callback: (LivingProtocolEvent) -> Unit
    ): Subscription {
        return subscribe(SubscriptionOptions(phases = phases), callback)
    }

    // MARK: - Private Methods

    private suspend fun establishConnection() = suspendCancellableCoroutine { continuation ->
        val request = Request.Builder()
            .url(configuration.url)
            .build()

        val listener = object : WebSocketListener() {
            private var isResumed = false

            override fun onOpen(webSocket: WebSocket, response: Response) {
                if (!isResumed) {
                    isResumed = true
                    this@LivingProtocolClient.webSocket = webSocket
                    continuation.resume(Unit)
                }
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                handleMessage(text)
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                webSocket.close(code, reason)
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                handleDisconnect()
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                if (!isResumed) {
                    isResumed = true
                    continuation.resumeWithException(
                        LivingProtocolException.ConnectionFailed(t.message ?: "Unknown error")
                    )
                } else {
                    handleDisconnect()
                }
            }
        }

        webSocket = okHttpClient.newWebSocket(request, listener)

        continuation.invokeOnCancellation {
            webSocket?.cancel()
        }

        // Connection timeout
        scope.launch {
            delay(configuration.connectionTimeoutMs)
            if (!continuation.isCompleted) {
                webSocket?.cancel()
                continuation.resumeWithException(LivingProtocolException.ConnectionTimeout())
            }
        }
    }

    private fun handleMessage(text: String) {
        try {
            val element = json.parseToJsonElement(text)
            val obj = element.jsonObject

            // Check if it's a response (has id)
            if (obj.containsKey("id") && obj["id"] !is JsonNull) {
                val response = json.decodeFromJsonElement<RpcResponse>(element)
                handleRpcResponse(response)
                return
            }

            // Check if it's a notification
            if (obj.containsKey("method")) {
                val notification = json.decodeFromJsonElement<RpcNotification>(element)
                handleNotification(notification)
                return
            }
        } catch (e: Exception) {
            // Ignore parse errors
        }
    }

    private fun handleRpcResponse(response: RpcResponse) {
        val id = response.id ?: return
        val deferred = pendingRequests.remove(id) ?: return

        if (response.error != null) {
            deferred.completeExceptionally(
                LivingProtocolException.RpcError(response.error.code, response.error.message)
            )
        } else if (response.result != null) {
            deferred.complete(response.result)
        } else {
            deferred.completeExceptionally(LivingProtocolException.InvalidResponse())
        }
    }

    private fun handleNotification(notification: RpcNotification) {
        val params = notification.params ?: return

        try {
            val eventType = params.jsonObject["type"]?.jsonPrimitive?.content ?: return

            val event: LivingProtocolEvent = when (eventType) {
                "phase_transition" -> json.decodeFromJsonElement<PhaseTransitionEvent>(params)
                "cycle_complete" -> json.decodeFromJsonElement<CycleCompletionEvent>(params)
                "state_update" -> {
                    val stateEvent = json.decodeFromJsonElement<StateUpdateEvent>(params)
                    _cycleState.value = stateEvent.state
                    stateEvent
                }
                "error" -> json.decodeFromJsonElement<ErrorEvent>(params)
                else -> return
            }

            // Dispatch to subscribers
            subscriptions.values.forEach { callback ->
                try {
                    callback(event)
                } catch (e: Exception) {
                    // Ignore callback errors
                }
            }
        } catch (e: Exception) {
            // Ignore decoding errors
        }
    }

    private fun handleDisconnect() {
        webSocket = null
        serverSubscriptionId = null

        if (configuration.autoReconnect && reconnectAttempts < configuration.maxReconnectAttempts) {
            _connectionState.value = ConnectionState.RECONNECTING
            scheduleReconnect()
        } else {
            _connectionState.value = ConnectionState.DISCONNECTED
        }
    }

    private fun scheduleReconnect() {
        reconnectJob?.cancel()
        reconnectJob = scope.launch {
            val delay = minOf(
                configuration.reconnectIntervalMs * (1L shl reconnectAttempts),
                30000L
            )

            delay(delay)

            reconnectAttempts++

            try {
                connect()
            } catch (e: Exception) {
                // Connect will handle further reconnection
            }
        }
    }

    private fun startHeartbeat() {
        heartbeatJob?.cancel()
        heartbeatJob = scope.launch {
            while (isActive) {
                delay(configuration.heartbeatIntervalMs)

                try {
                    rpcCall<Boolean>("ping")
                } catch (e: Exception) {
                    // Heartbeat failure will trigger disconnect
                }
            }
        }
    }

    private suspend inline fun <reified T> rpcCall(
        method: String,
        params: Map<String, JsonElement>? = null
    ): T {
        if (_connectionState.value != ConnectionState.CONNECTED || webSocket == null) {
            throw LivingProtocolException.NotConnected()
        }

        val id = requestId.incrementAndGet()
        val request = RpcRequest(
            id = id,
            method = method,
            params = params
        )

        val deferred = CompletableDeferred<JsonElement>()
        pendingRequests[id] = deferred

        try {
            val requestJson = json.encodeToString(request)
            webSocket?.send(requestJson) ?: throw LivingProtocolException.NotConnected()

            val result = withTimeout(configuration.requestTimeoutMs) {
                deferred.await()
            }

            return json.decodeFromJsonElement(result)
        } catch (e: TimeoutCancellationException) {
            pendingRequests.remove(id)
            throw LivingProtocolException.RequestTimeout(method)
        } catch (e: LivingProtocolException) {
            throw e
        } catch (e: Exception) {
            throw LivingProtocolException.DecodingError(e.message ?: "Unknown error")
        }
    }

    private suspend fun setupServerSubscription() {
        if (serverSubscriptionId != null) return

        try {
            val result: SubscribeResult = rpcCall(
                "subscribe",
                mapOf(
                    "events" to JsonArray(
                        listOf(
                            JsonPrimitive("phase_transition"),
                            JsonPrimitive("cycle_complete"),
                            JsonPrimitive("state_update"),
                            JsonPrimitive("error")
                        )
                    )
                )
            )
            serverSubscriptionId = result.subscriptionId
        } catch (e: Exception) {
            // Ignore subscription errors
        }
    }

    private suspend fun teardownServerSubscription() {
        val subId = serverSubscriptionId ?: return

        try {
            rpcCall<Boolean>(
                "unsubscribe",
                mapOf("subscriptionId" to JsonPrimitive(subId))
            )
        } catch (e: Exception) {
            // Ignore
        }

        serverSubscriptionId = null
    }

    private suspend fun resubscribeAll() {
        if (subscriptions.isNotEmpty()) {
            serverSubscriptionId = null
            setupServerSubscription()
        }
    }

    private fun eventMatchesOptions(
        event: LivingProtocolEvent,
        options: SubscriptionOptions
    ): Boolean {
        // Check event type filter
        options.eventTypes?.let { types ->
            if (types.isNotEmpty() && event.type !in types) {
                return false
            }
        }

        // Check phase filter
        options.phases?.let { phases ->
            if (phases.isNotEmpty()) {
                when (event) {
                    is PhaseTransitionEvent -> {
                        if (event.toPhase !in phases) return false
                    }
                    is StateUpdateEvent -> {
                        if (event.state.phase !in phases) return false
                    }
                    else -> {}
                }
            }
        }

        // Check cycle number filter
        options.cycleNumbers?.let { numbers ->
            if (numbers.isNotEmpty()) {
                when (event) {
                    is PhaseTransitionEvent -> {
                        if (event.cycleNumber !in numbers) return false
                    }
                    is CycleCompletionEvent -> {
                        if (event.cycleNumber !in numbers) return false
                    }
                    is StateUpdateEvent -> {
                        if (event.state.cycleNumber !in numbers) return false
                    }
                    else -> {}
                }
            }
        }

        return true
    }

    private fun generateSubscriptionId(): String {
        return "sub_${System.currentTimeMillis()}_${(Math.random() * 100000).toInt()}"
    }
}

/**
 * Create a Living Protocol client
 */
fun livingProtocolClient(
    url: String,
    configure: ClientConfiguration.() -> Unit = {}
): LivingProtocolClient {
    val baseConfig = ClientConfiguration(url)
    val config = baseConfig.copy().apply {
        // Apply configuration through a builder pattern if needed
    }
    return LivingProtocolClient(config)
}
