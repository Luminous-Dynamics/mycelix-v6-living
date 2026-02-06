# MycelixSDK

Swift SDK for the Living Protocol - a biologically-inspired protocol for decentralized systems.

## Requirements

- iOS 14.0+ / macOS 11.0+ / watchOS 7.0+ / tvOS 14.0+
- Swift 5.7+
- Xcode 14.0+

## Installation

### Swift Package Manager

Add the following to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/mycelix/swift-sdk.git", from: "1.0.0")
]
```

Or in Xcode: File > Add Package Dependencies... and enter the repository URL.

## Quick Start

```swift
import MycelixSDK

// Create client
let client = LivingProtocolClient(
    configuration: ClientConfiguration(
        url: URL(string: "wss://your-server.com/living-protocol")!
    )
)

// Connect
try await client.connect()

// Get current state
let state = try await client.getCycleState()
print("Phase: \(state.phase.displayName)")
print("Cycle: \(state.cycleNumber)")
print("Progress: \(Int(state.progress * 100))%")
```

## SwiftUI Integration

### Basic Usage

```swift
import SwiftUI
import MycelixSDK

struct ContentView: View {
    @StateObject private var client = LivingProtocolClient(
        configuration: ClientConfiguration(
            url: URL(string: "wss://server.com/ws")!
        )
    )

    var body: some View {
        VStack {
            if client.connectionState == .connected {
                if let state = client.cycleState {
                    PhaseView(state: state)
                }
            } else {
                ProgressView("Connecting...")
            }
        }
        .task {
            try? await client.connect()
        }
    }
}

struct PhaseView: View {
    let state: CycleState

    var body: some View {
        VStack(spacing: 20) {
            Text(state.phase.displayName)
                .font(.largeTitle)
                .fontWeight(.bold)

            Text("Cycle #\(state.cycleNumber)")
                .font(.headline)
                .foregroundColor(.secondary)

            ProgressView(value: state.progress)
                .progressViewStyle(.linear)

            Text("\(Int(state.progress * 100))% complete")
                .font(.caption)
        }
        .padding()
    }
}
```

### Using Combine Publishers

```swift
import SwiftUI
import MycelixSDK
import Combine

class LivingProtocolViewModel: ObservableObject {
    @Published var phase: CyclePhase = .dormant
    @Published var cycleNumber: Int = 0
    @Published var progress: Double = 0
    @Published var isConnected = false

    private let client: LivingProtocolClient
    private var cancellables = Set<AnyCancellable>()

    init() {
        client = LivingProtocolClient(
            configuration: ClientConfiguration(
                url: URL(string: "wss://server.com/ws")!
            )
        )

        setupBindings()
    }

    private func setupBindings() {
        // Connection state
        client.$connectionState
            .map { $0 == .connected }
            .assign(to: &$isConnected)

        // Cycle state updates
        client.$cycleState
            .compactMap { $0 }
            .sink { [weak self] state in
                self?.phase = state.phase
                self?.cycleNumber = state.cycleNumber
                self?.progress = state.progress
            }
            .store(in: &cancellables)

        // Phase transitions
        client.onPhaseEnter(.fruiting)
            .sink { event in
                print("Fruiting phase started!")
            }
            .store(in: &cancellables)

        // Cycle completions
        client.onCycleComplete()
            .sink { event in
                print("Cycle \(event.cycleNumber) completed!")
            }
            .store(in: &cancellables)
    }

    func connect() async {
        try? await client.connect()
    }

    func disconnect() {
        client.disconnect()
    }
}

struct DashboardView: View {
    @StateObject private var viewModel = LivingProtocolViewModel()

    var body: some View {
        VStack(spacing: 30) {
            // Connection indicator
            HStack {
                Circle()
                    .fill(viewModel.isConnected ? Color.green : Color.red)
                    .frame(width: 10, height: 10)
                Text(viewModel.isConnected ? "Connected" : "Disconnected")
            }

            // Phase display
            PhaseCard(phase: viewModel.phase)

            // Progress
            VStack {
                ProgressView(value: viewModel.progress)
                Text("Cycle #\(viewModel.cycleNumber)")
                    .font(.caption)
            }
            .padding()
        }
        .task {
            await viewModel.connect()
        }
    }
}

struct PhaseCard: View {
    let phase: CyclePhase

    private var phaseColor: Color {
        switch phase {
        case .dormant: return .gray
        case .germination: return .green
        case .growth: return .mint
        case .fruiting: return .orange
        case .sporulation: return .purple
        }
    }

    var body: some View {
        VStack {
            Image(systemName: phaseIcon)
                .font(.system(size: 60))

            Text(phase.displayName)
                .font(.title)
                .fontWeight(.bold)
        }
        .foregroundColor(phaseColor)
        .padding(40)
        .background(phaseColor.opacity(0.1))
        .cornerRadius(20)
    }

    private var phaseIcon: String {
        switch phase {
        case .dormant: return "moon.zzz"
        case .germination: return "leaf"
        case .growth: return "arrow.up.circle"
        case .fruiting: return "sparkles"
        case .sporulation: return "wind"
        }
    }
}
```

### Event Subscriptions

```swift
import MycelixSDK
import Combine

class EventHandler {
    private let client: LivingProtocolClient
    private var cancellables = Set<AnyCancellable>()
    private var subscriptions: [Subscription] = []

    init(client: LivingProtocolClient) {
        self.client = client
        setupSubscriptions()
    }

    private func setupSubscriptions() {
        // Using callbacks
        let sub1 = client.subscribe { event in
            print("Event received: \(event.type)")
        }
        subscriptions.append(sub1)

        // Filter by event type
        let sub2 = client.subscribeToEvents([.phaseTransition, .cycleComplete]) { event in
            if let transition = event as? PhaseTransitionEvent {
                print("Phase changed: \(transition.fromPhase) -> \(transition.toPhase)")
            }
        }
        subscriptions.append(sub2)

        // Filter by phase
        let sub3 = client.subscribeToPhases([.fruiting, .sporulation]) { event in
            print("Important phase event!")
        }
        subscriptions.append(sub3)

        // Using Combine publishers
        client.eventPublisher()
            .sink { event in
                print("Combine event: \(event.type)")
            }
            .store(in: &cancellables)

        // Phase publisher
        client.phasePublisher()
            .sink { phase in
                print("New phase: \(phase.displayName)")
            }
            .store(in: &cancellables)

        // Specific phase entry
        client.onPhaseEnter(.growth)
            .sink { event in
                self.handleGrowthStart(event)
            }
            .store(in: &cancellables)

        // Specific phase exit
        client.onPhaseExit(.dormant)
            .sink { event in
                print("Waking up from dormancy!")
            }
            .store(in: &cancellables)
    }

    private func handleGrowthStart(_ event: PhaseTransitionEvent) {
        print("Growth phase started at cycle \(event.cycleNumber)")
    }

    deinit {
        subscriptions.forEach { $0.cancel() }
    }
}
```

### Using async/await with Events

```swift
import MycelixSDK

func watchEvents() async {
    let client = try! LivingProtocolClient(url: "wss://server.com/ws")
    try! await client.connect()

    // AsyncSequence of events
    for await event in client.events() {
        switch event {
        case let transition as PhaseTransitionEvent:
            print("Phase: \(transition.fromPhase) -> \(transition.toPhase)")

        case let completion as CycleCompletionEvent:
            print("Cycle \(completion.cycleNumber) completed")

        case let update as StateUpdateEvent:
            print("Progress: \(Int(update.state.progress * 100))%")

        case let error as ErrorEvent:
            print("Error: \(error.message)")

        default:
            break
        }
    }
}

// Filter events
func watchPhaseTransitions() async {
    let client = try! LivingProtocolClient(url: "wss://server.com/ws")
    try! await client.connect()

    let options = SubscriptionOptions(eventTypes: [.phaseTransition])

    for await event in client.events(options: options) {
        if let transition = event as? PhaseTransitionEvent {
            print("Phase changed to: \(transition.toPhase.displayName)")
        }
    }
}
```

## API Reference

### LivingProtocolClient

```swift
// Initialize
let client = LivingProtocolClient(configuration: config)
let client = try LivingProtocolClient(url: "wss://server.com/ws")

// Connection
try await client.connect()
client.disconnect()

// Properties (Observable)
client.connectionState  // ConnectionState
client.cycleState       // CycleState?

// RPC Methods
let state = try await client.getCycleState()
let phase = try await client.getCurrentPhase()
let cycleNumber = try await client.getCycleNumber()
let progress = try await client.getPhaseProgress()
let timeRemaining = try await client.getTimeRemaining()
let history = try await client.getCycleHistory(limit: 10)
let newState = try await client.advancePhase()

// Subscriptions
let sub = client.subscribe(options: options) { event in ... }
let sub = client.subscribeToEvents([.phaseTransition]) { event in ... }
let sub = client.subscribeToPhases([.fruiting]) { event in ... }
sub.cancel()

// Combine Publishers
client.eventPublisher(options: options)
client.cycleStatePublisher()
client.phasePublisher(phases: [.growth])
client.connectionStatePublisher
client.onPhaseEnter(.fruiting)
client.onPhaseExit(.dormant)
client.onCycleComplete()

// AsyncSequence
for await event in client.events(options: options) { ... }
```

### Types

```swift
// Phases
enum CyclePhase: String, Codable {
    case dormant, germination, growth, fruiting, sporulation
    var displayName: String
}

// State
struct CycleState: Codable {
    let phase: CyclePhase
    let cycleNumber: Int
    let phaseStartTime: TimeInterval
    let phaseEndTime: TimeInterval
    let phaseDuration: TimeInterval
    let progress: Double
    let metadata: [String: AnyCodable]?
    var timeRemaining: TimeInterval
}

// Events
struct PhaseTransitionEvent: LivingProtocolEvent
struct CycleCompletionEvent: LivingProtocolEvent
struct StateUpdateEvent: LivingProtocolEvent
struct ErrorEvent: LivingProtocolEvent

// Connection
enum ConnectionState {
    case disconnected, connecting, connected, reconnecting, error
}

// Configuration
struct ClientConfiguration {
    let url: URL
    var autoReconnect: Bool = true
    var reconnectInterval: TimeInterval = 3.0
    var maxReconnectAttempts: Int = 10
    var heartbeatInterval: TimeInterval = 30.0
    var connectionTimeout: TimeInterval = 10.0
}
```

## Complete SwiftUI Example

```swift
import SwiftUI
import MycelixSDK
import Combine

@main
struct MycelixApp: App {
    var body: some Scene {
        WindowGroup {
            LivingProtocolView()
        }
    }
}

struct LivingProtocolView: View {
    @StateObject private var client = LivingProtocolClient(
        configuration: ClientConfiguration(
            url: URL(string: "wss://mycelix.example.com/ws")!
        )
    )

    @State private var events: [String] = []

    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                // Status
                statusSection

                // Current state
                if let state = client.cycleState {
                    stateSection(state)
                }

                // Event log
                eventLogSection
            }
            .padding()
            .navigationTitle("Living Protocol")
            .task {
                await connect()
            }
        }
    }

    private var statusSection: some View {
        HStack {
            Circle()
                .fill(statusColor)
                .frame(width: 12, height: 12)

            Text(statusText)
                .font(.subheadline)

            Spacer()

            Button(client.connectionState == .connected ? "Disconnect" : "Connect") {
                Task {
                    if client.connectionState == .connected {
                        client.disconnect()
                    } else {
                        await connect()
                    }
                }
            }
            .buttonStyle(.bordered)
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(10)
        .shadow(radius: 2)
    }

    private func stateSection(_ state: CycleState) -> some View {
        VStack(spacing: 15) {
            Text(state.phase.displayName)
                .font(.system(size: 36, weight: .bold))
                .foregroundColor(phaseColor(state.phase))

            Text("Cycle #\(state.cycleNumber)")
                .font(.headline)
                .foregroundColor(.secondary)

            ProgressView(value: state.progress)
                .progressViewStyle(.linear)
                .tint(phaseColor(state.phase))

            HStack {
                Text("\(Int(state.progress * 100))%")
                Spacer()
                Text(formatTime(state.timeRemaining))
            }
            .font(.caption)
            .foregroundColor(.secondary)
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(10)
        .shadow(radius: 2)
    }

    private var eventLogSection: some View {
        VStack(alignment: .leading) {
            Text("Events")
                .font(.headline)

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 8) {
                    ForEach(events, id: \.self) { event in
                        Text(event)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .frame(maxHeight: 200)
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(10)
        .shadow(radius: 2)
    }

    private var statusColor: Color {
        switch client.connectionState {
        case .connected: return .green
        case .connecting, .reconnecting: return .yellow
        case .disconnected: return .gray
        case .error: return .red
        }
    }

    private var statusText: String {
        switch client.connectionState {
        case .connected: return "Connected"
        case .connecting: return "Connecting..."
        case .reconnecting: return "Reconnecting..."
        case .disconnected: return "Disconnected"
        case .error: return "Error"
        }
    }

    private func phaseColor(_ phase: CyclePhase) -> Color {
        switch phase {
        case .dormant: return .gray
        case .germination: return .green
        case .growth: return .mint
        case .fruiting: return .orange
        case .sporulation: return .purple
        }
    }

    private func formatTime(_ ms: TimeInterval) -> String {
        let seconds = Int(ms / 1000)
        let minutes = seconds / 60
        let remainingSeconds = seconds % 60
        return String(format: "%d:%02d", minutes, remainingSeconds)
    }

    private func connect() async {
        do {
            try await client.connect()

            // Subscribe to events
            client.subscribe { event in
                let timestamp = Date().formatted(date: .omitted, time: .standard)
                events.insert("[\(timestamp)] \(event.type.rawValue)", at: 0)
                if events.count > 50 {
                    events.removeLast()
                }
            }
        } catch {
            print("Connection failed: \(error)")
        }
    }
}
```

## License

MIT
