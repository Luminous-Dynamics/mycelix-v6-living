# Mycelix SDK for Kotlin/Android

Kotlin SDK for the Living Protocol - a biologically-inspired protocol for decentralized systems.

## Requirements

- Kotlin 1.9+
- JDK 17+
- Android API 21+ (if used on Android)

## Installation

### Gradle (Kotlin DSL)

```kotlin
dependencies {
    implementation("com.mycelix:mycelix-sdk:1.0.0")
}
```

### Gradle (Groovy)

```groovy
dependencies {
    implementation 'com.mycelix:mycelix-sdk:1.0.0'
}
```

## Quick Start

```kotlin
import com.mycelix.sdk.*
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    // Create client
    val client = LivingProtocolClient("wss://your-server.com/living-protocol")

    // Connect
    client.connect()

    // Get current state
    val state = client.getCycleState()
    println("Phase: ${state.phase.displayName}")
    println("Cycle: ${state.cycleNumber}")
    println("Progress: ${(state.progress * 100).toInt()}%")

    // Cleanup
    client.close()
}
```

## Android Integration

### Basic Usage with ViewModel

```kotlin
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.mycelix.sdk.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

class LivingProtocolViewModel : ViewModel() {
    private val client = LivingProtocolClient(
        ClientConfiguration(
            url = "wss://server.com/ws",
            autoReconnect = true
        )
    )

    // Expose connection state
    val connectionState: StateFlow<ConnectionState> = client.connectionState

    // Expose cycle state
    val cycleState: StateFlow<CycleState?> = client.cycleState

    // Phase as Flow
    val currentPhase: Flow<CyclePhase> = cycleState
        .filterNotNull()
        .map { it.phase }
        .distinctUntilChanged()

    // Progress as Flow
    val progress: Flow<Double> = cycleState
        .filterNotNull()
        .map { it.progress }

    init {
        viewModelScope.launch {
            try {
                client.connect()
            } catch (e: Exception) {
                // Handle connection error
            }
        }

        // Subscribe to phase transitions
        client.collectPhaseTransitions(viewModelScope) { event ->
            println("Phase changed: ${event.fromPhase} -> ${event.toPhase}")
        }
    }

    fun disconnect() {
        client.disconnect()
    }

    override fun onCleared() {
        super.onCleared()
        client.close()
    }
}
```

### Jetpack Compose Integration

```kotlin
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.mycelix.sdk.*

@Composable
fun LivingProtocolScreen(
    viewModel: LivingProtocolViewModel = viewModel()
) {
    val connectionState by viewModel.connectionState.collectAsState()
    val cycleState by viewModel.cycleState.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        // Connection indicator
        ConnectionStatus(connectionState)

        Spacer(modifier = Modifier.height(32.dp))

        // Cycle state display
        cycleState?.let { state ->
            CycleStateCard(state)
        } ?: run {
            CircularProgressIndicator()
            Text("Loading...")
        }
    }
}

@Composable
fun ConnectionStatus(state: ConnectionState) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        val (color, text) = when (state) {
            ConnectionState.CONNECTED -> Color.Green to "Connected"
            ConnectionState.CONNECTING -> Color.Yellow to "Connecting..."
            ConnectionState.RECONNECTING -> Color.Yellow to "Reconnecting..."
            ConnectionState.DISCONNECTED -> Color.Gray to "Disconnected"
            ConnectionState.ERROR -> Color.Red to "Error"
        }

        Box(
            modifier = Modifier
                .size(12.dp)
                .background(color, shape = CircleShape)
        )

        Text(text, style = MaterialTheme.typography.bodyMedium)
    }
}

@Composable
fun CycleStateCard(state: CycleState) {
    val phaseColor = when (state.phase) {
        CyclePhase.DORMANT -> Color.Gray
        CyclePhase.GERMINATION -> Color(0xFF4CAF50)
        CyclePhase.GROWTH -> Color(0xFF8BC34A)
        CyclePhase.FRUITING -> Color(0xFFFF9800)
        CyclePhase.SPORULATION -> Color(0xFF9C27B0)
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = phaseColor.copy(alpha = 0.1f)
        )
    ) {
        Column(
            modifier = Modifier.padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = state.phase.displayName,
                style = MaterialTheme.typography.headlineLarge,
                color = phaseColor
            )

            Spacer(modifier = Modifier.height(8.dp))

            Text(
                text = "Cycle #${state.cycleNumber}",
                style = MaterialTheme.typography.titleMedium,
                color = Color.Gray
            )

            Spacer(modifier = Modifier.height(16.dp))

            LinearProgressIndicator(
                progress = state.progress.toFloat(),
                modifier = Modifier.fillMaxWidth(),
                color = phaseColor
            )

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("${(state.progress * 100).toInt()}%")
                Text(formatTimeRemaining(state.timeRemaining))
            }
        }
    }
}

private fun formatTimeRemaining(ms: Long): String {
    val seconds = (ms / 1000).toInt()
    val minutes = seconds / 60
    val remainingSeconds = seconds % 60
    return "${minutes}:${remainingSeconds.toString().padStart(2, '0')}"
}
```

## Flow-Based Event Handling

### Event Flows

```kotlin
import com.mycelix.sdk.*
import kotlinx.coroutines.flow.*

// All events
client.eventFlow()
    .collect { event ->
        println("Event: ${event.type}")
    }

// Filter by event type
client.eventFlow(SubscriptionOptions(eventTypes = listOf(EventType.PHASE_TRANSITION)))
    .filterIsInstance<PhaseTransitionEvent>()
    .collect { event ->
        println("Phase: ${event.fromPhase} -> ${event.toPhase}")
    }

// Phase transitions only
client.phaseFlow()
    .collect { phase ->
        println("Current phase: ${phase.displayName}")
    }

// Specific phase entry
client.onPhaseEnter(CyclePhase.FRUITING)
    .collect { event ->
        println("Fruiting phase started!")
    }

// Specific phase exit
client.onPhaseExit(CyclePhase.DORMANT)
    .collect { event ->
        println("Waking up from dormancy!")
    }

// Cycle completions
client.onCycleComplete()
    .collect { event ->
        println("Cycle ${event.cycleNumber} completed in ${event.duration}ms")
    }

// Cycle state updates
client.cycleStateFlow()
    .collect { state ->
        println("Progress: ${(state.progress * 100).toInt()}%")
    }

// Errors
client.errorFlow()
    .collect { error ->
        println("Error: ${error.message}")
    }
```

### Collecting in Coroutine Scope

```kotlin
import com.mycelix.sdk.*
import kotlinx.coroutines.*

class MyService {
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private val client = LivingProtocolClient("wss://server.com/ws")

    fun start() {
        scope.launch {
            client.connect()
        }

        // Collect events with automatic cancellation
        client.collectEvents(scope) { event ->
            handleEvent(event)
        }

        // Collect phase transitions
        client.collectPhaseTransitions(scope, phases = listOf(CyclePhase.FRUITING)) { event ->
            handleFruiting(event)
        }

        // Collect state updates
        client.collectCycleState(scope) { state ->
            updateUI(state)
        }
    }

    fun stop() {
        scope.cancel()
        client.close()
    }

    private suspend fun handleEvent(event: LivingProtocolEvent) { ... }
    private suspend fun handleFruiting(event: PhaseTransitionEvent) { ... }
    private suspend fun updateUI(state: CycleState) { ... }
}
```

### SharedFlow for Hot Sharing

```kotlin
import com.mycelix.sdk.*
import kotlinx.coroutines.flow.*

// Create a hot event stream
val eventStream = client.createEventStream()
eventStream.start()

// Multiple collectors can share the same stream
scope.launch {
    eventStream.events
        .filterIsInstance<PhaseTransitionEvent>()
        .collect { println("Collector 1: $it") }
}

scope.launch {
    eventStream.events
        .filterIsInstance<CycleCompletionEvent>()
        .collect { println("Collector 2: $it") }
}

// Stop when done
eventStream.stop()
```

## Callback-Based Subscriptions

```kotlin
import com.mycelix.sdk.*

// Subscribe to all events
val subscription = client.subscribe { event ->
    when (event) {
        is PhaseTransitionEvent -> {
            println("Phase: ${event.fromPhase} -> ${event.toPhase}")
        }
        is CycleCompletionEvent -> {
            println("Cycle ${event.cycleNumber} complete")
        }
        is StateUpdateEvent -> {
            println("Progress: ${(event.state.progress * 100).toInt()}%")
        }
        is ErrorEvent -> {
            println("Error: ${event.message}")
        }
    }
}

// Subscribe with filters
val filteredSub = client.subscribe(
    SubscriptionOptions(
        eventTypes = listOf(EventType.PHASE_TRANSITION),
        phases = listOf(CyclePhase.FRUITING, CyclePhase.SPORULATION)
    )
) { event ->
    println("Important phase event!")
}

// Subscribe to specific event types
val eventSub = client.subscribeToEvents(
    listOf(EventType.PHASE_TRANSITION, EventType.CYCLE_COMPLETE)
) { event ->
    println("Event: ${event.type}")
}

// Subscribe to specific phases
val phaseSub = client.subscribeToPhases(
    listOf(CyclePhase.GROWTH)
) { event ->
    println("Growth phase event")
}

// Cancel subscriptions
subscription.cancel()
filteredSub.cancel()
eventSub.cancel()
phaseSub.cancel()
```

## API Reference

### LivingProtocolClient

```kotlin
// Create
val client = LivingProtocolClient(url)
val client = LivingProtocolClient(configuration)

// Connection
suspend fun connect()
fun disconnect()
fun close()

// Properties
val connectionState: StateFlow<ConnectionState>
val cycleState: StateFlow<CycleState?>

// RPC Methods
suspend fun getCycleState(): CycleState
suspend fun getCurrentPhase(): CyclePhase
suspend fun getCycleNumber(): Int
suspend fun getPhaseProgress(): Double
suspend fun getTimeRemaining(): Long
suspend fun getCycleHistory(limit: Int? = null): List<CycleState>
suspend fun advancePhase(): CycleState

// Subscriptions
fun subscribe(options: SubscriptionOptions = SubscriptionOptions(), callback: (LivingProtocolEvent) -> Unit): Subscription
fun subscribeToEvents(eventTypes: List<EventType>, callback: (LivingProtocolEvent) -> Unit): Subscription
fun subscribeToPhases(phases: List<CyclePhase>, callback: (LivingProtocolEvent) -> Unit): Subscription
```

### Flow Extensions

```kotlin
// Event flows
fun eventFlow(options: SubscriptionOptions = SubscriptionOptions()): Flow<LivingProtocolEvent>
fun cycleStateFlow(): Flow<CycleState>
fun phaseFlow(phases: List<CyclePhase>? = null): Flow<CyclePhase>
fun onPhaseEnter(phase: CyclePhase): Flow<PhaseTransitionEvent>
fun onPhaseExit(phase: CyclePhase): Flow<PhaseTransitionEvent>
fun onCycleComplete(): Flow<CycleCompletionEvent>
fun errorFlow(): Flow<ErrorEvent>

// Scope collectors
fun collectEvents(scope: CoroutineScope, options: SubscriptionOptions = SubscriptionOptions(), onEvent: suspend (LivingProtocolEvent) -> Unit): Job
fun collectPhaseTransitions(scope: CoroutineScope, phases: List<CyclePhase>? = null, onTransition: suspend (PhaseTransitionEvent) -> Unit): Job
fun collectCycleState(scope: CoroutineScope, onState: suspend (CycleState) -> Unit): Job

// Hot stream
fun createEventStream(options: SubscriptionOptions = SubscriptionOptions()): EventStream
```

### Types

```kotlin
// Phases
enum class CyclePhase {
    DORMANT, GERMINATION, GROWTH, FRUITING, SPORULATION
    val displayName: String
}

// State
data class CycleState(
    val phase: CyclePhase,
    val cycleNumber: Int,
    val phaseStartTime: Long,
    val phaseEndTime: Long,
    val phaseDuration: Long,
    val progress: Double,
    val metadata: Map<String, JsonElement>? = null
) {
    val timeRemaining: Long
}

// Events
sealed interface LivingProtocolEvent {
    val type: EventType
    val timestamp: Long
}

data class PhaseTransitionEvent(...)
data class CycleCompletionEvent(...)
data class StateUpdateEvent(...)
data class ErrorEvent(...)

// Connection
enum class ConnectionState {
    DISCONNECTED, CONNECTING, CONNECTED, RECONNECTING, ERROR
}

// Configuration
data class ClientConfiguration(
    val url: String,
    val autoReconnect: Boolean = true,
    val reconnectIntervalMs: Long = 3000L,
    val maxReconnectAttempts: Int = 10,
    val heartbeatIntervalMs: Long = 30000L,
    val connectionTimeoutMs: Long = 10000L,
    val requestTimeoutMs: Long = 30000L
)
```

## Complete Android Example

```kotlin
// App.kt
class MycelixApp : Application() {
    lateinit var livingProtocolClient: LivingProtocolClient
        private set

    override fun onCreate() {
        super.onCreate()

        livingProtocolClient = LivingProtocolClient(
            ClientConfiguration(
                url = "wss://mycelix.example.com/ws",
                autoReconnect = true,
                reconnectIntervalMs = 5000L
            )
        )
    }

    override fun onTerminate() {
        super.onTerminate()
        livingProtocolClient.close()
    }
}

// MainActivity.kt
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            MycelixTheme {
                Surface {
                    LivingProtocolApp()
                }
            }
        }
    }
}

// LivingProtocolApp.kt
@Composable
fun LivingProtocolApp() {
    val viewModel: LivingProtocolViewModel = viewModel()
    val connectionState by viewModel.connectionState.collectAsState()
    val cycleState by viewModel.cycleState.collectAsState()
    val events by viewModel.recentEvents.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Living Protocol") },
                actions = {
                    ConnectionIndicator(connectionState)
                }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            item {
                cycleState?.let { state ->
                    CycleStateCard(state)
                }
            }

            item {
                Text(
                    "Recent Events",
                    style = MaterialTheme.typography.titleMedium
                )
            }

            items(events) { event ->
                EventCard(event)
            }
        }
    }
}

// ViewModel with event history
class LivingProtocolViewModel(application: Application) : AndroidViewModel(application) {
    private val client = (application as MycelixApp).livingProtocolClient

    val connectionState = client.connectionState
    val cycleState = client.cycleState

    private val _recentEvents = MutableStateFlow<List<LivingProtocolEvent>>(emptyList())
    val recentEvents = _recentEvents.asStateFlow()

    init {
        viewModelScope.launch {
            client.connect()
        }

        client.collectEvents(viewModelScope) { event ->
            _recentEvents.update { events ->
                (listOf(event) + events).take(50)
            }
        }
    }
}
```

## License

MIT
