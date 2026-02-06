package com.mycelix.sdk

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

/**
 * A handle to an active event subscription
 */
class Subscription internal constructor(
    val id: String,
    private val cancelAction: () -> Unit
) {
    private var isCancelled = false

    /**
     * Cancel the subscription
     */
    fun cancel() {
        if (!isCancelled) {
            isCancelled = true
            cancelAction()
        }
    }
}

/**
 * Extension functions for Flow-based subscriptions
 */

/**
 * Flow of all events matching the given options
 */
fun LivingProtocolClient.eventFlow(
    options: SubscriptionOptions = SubscriptionOptions()
): Flow<LivingProtocolEvent> = callbackFlow {
    val subscription = subscribe(options) { event ->
        trySend(event)
    }
    awaitClose { subscription.cancel() }
}

/**
 * Flow of cycle state updates
 */
fun LivingProtocolClient.cycleStateFlow(): Flow<CycleState> = flow {
    // Emit initial state if available
    cycleState.value?.let { emit(it) }

    // Emit updates
    eventFlow(SubscriptionOptions(eventTypes = listOf(EventType.STATE_UPDATE, EventType.PHASE_TRANSITION)))
        .collect { event ->
            when (event) {
                is StateUpdateEvent -> emit(event.state)
                is PhaseTransitionEvent -> {
                    // Fetch updated state after phase transition
                    try {
                        emit(getCycleState())
                    } catch (e: Exception) {
                        // Ignore fetch errors during collection
                    }
                }
                else -> {}
            }
        }
}

/**
 * Flow of phase transitions
 */
fun LivingProtocolClient.phaseFlow(
    phases: List<CyclePhase>? = null
): Flow<CyclePhase> = eventFlow(
    SubscriptionOptions(eventTypes = listOf(EventType.PHASE_TRANSITION), phases = phases)
).filterIsInstance<PhaseTransitionEvent>().map { it.toPhase }

/**
 * Flow that emits when entering a specific phase
 */
fun LivingProtocolClient.onPhaseEnter(phase: CyclePhase): Flow<PhaseTransitionEvent> =
    eventFlow(SubscriptionOptions(eventTypes = listOf(EventType.PHASE_TRANSITION)))
        .filterIsInstance<PhaseTransitionEvent>()
        .filter { it.toPhase == phase }

/**
 * Flow that emits when exiting a specific phase
 */
fun LivingProtocolClient.onPhaseExit(phase: CyclePhase): Flow<PhaseTransitionEvent> =
    eventFlow(SubscriptionOptions(eventTypes = listOf(EventType.PHASE_TRANSITION)))
        .filterIsInstance<PhaseTransitionEvent>()
        .filter { it.fromPhase == phase }

/**
 * Flow that emits when a cycle completes
 */
fun LivingProtocolClient.onCycleComplete(): Flow<CycleCompletionEvent> =
    eventFlow(SubscriptionOptions(eventTypes = listOf(EventType.CYCLE_COMPLETE)))
        .filterIsInstance<CycleCompletionEvent>()

/**
 * Flow of errors
 */
fun LivingProtocolClient.errorFlow(): Flow<ErrorEvent> =
    eventFlow(SubscriptionOptions(eventTypes = listOf(EventType.ERROR)))
        .filterIsInstance<ErrorEvent>()

/**
 * Collect events in a coroutine scope with automatic cancellation
 */
fun LivingProtocolClient.collectEvents(
    scope: CoroutineScope,
    options: SubscriptionOptions = SubscriptionOptions(),
    onEvent: suspend (LivingProtocolEvent) -> Unit
): Job = scope.launch {
    eventFlow(options).collect { event ->
        onEvent(event)
    }
}

/**
 * Collect phase transitions in a coroutine scope
 */
fun LivingProtocolClient.collectPhaseTransitions(
    scope: CoroutineScope,
    phases: List<CyclePhase>? = null,
    onTransition: suspend (PhaseTransitionEvent) -> Unit
): Job = scope.launch {
    eventFlow(SubscriptionOptions(eventTypes = listOf(EventType.PHASE_TRANSITION), phases = phases))
        .filterIsInstance<PhaseTransitionEvent>()
        .collect { event ->
            onTransition(event)
        }
}

/**
 * Collect cycle state updates in a coroutine scope
 */
fun LivingProtocolClient.collectCycleState(
    scope: CoroutineScope,
    onState: suspend (CycleState) -> Unit
): Job = scope.launch {
    cycleStateFlow().collect { state ->
        onState(state)
    }
}

/**
 * SharedFlow-based event stream for hot sharing
 */
class EventStream internal constructor(
    private val client: LivingProtocolClient,
    private val options: SubscriptionOptions
) {
    private val _events = MutableSharedFlow<LivingProtocolEvent>(
        replay = 0,
        extraBufferCapacity = 64,
        onBufferOverflow = BufferOverflow.DROP_OLDEST
    )

    /**
     * SharedFlow of events
     */
    val events: SharedFlow<LivingProtocolEvent> = _events.asSharedFlow()

    private var subscription: Subscription? = null

    /**
     * Start the event stream
     */
    fun start() {
        if (subscription != null) return

        subscription = client.subscribe(options) { event ->
            _events.tryEmit(event)
        }
    }

    /**
     * Stop the event stream
     */
    fun stop() {
        subscription?.cancel()
        subscription = null
    }
}

/**
 * Create a hot event stream
 */
fun LivingProtocolClient.createEventStream(
    options: SubscriptionOptions = SubscriptionOptions()
): EventStream = EventStream(this, options)

/**
 * StateFlow for connection state with initial value
 */
val LivingProtocolClient.connectionStateFlow: StateFlow<ConnectionState>
    get() = connectionState

/**
 * StateFlow for cycle state
 */
val LivingProtocolClient.cycleStateFlow: StateFlow<CycleState?>
    get() = cycleState

/**
 * Helper to create a callback-based Flow
 */
private fun <T> callbackFlow(block: suspend ProducerScope<T>.() -> Unit): Flow<T> =
    kotlinx.coroutines.flow.callbackFlow(block)

/**
 * Helper interface for ProducerScope
 */
private typealias ProducerScope<T> = kotlinx.coroutines.channels.ProducerScope<T>
