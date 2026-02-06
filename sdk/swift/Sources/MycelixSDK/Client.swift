import Foundation
import Combine

// MARK: - Living Protocol Client

/// Client for connecting to the Living Protocol server
@MainActor
public final class LivingProtocolClient: ObservableObject {

    // MARK: - Published Properties

    /// Current connection state
    @Published public private(set) var connectionState: ConnectionState = .disconnected

    /// Current cycle state (nil if not yet fetched)
    @Published public private(set) var cycleState: CycleState?

    // MARK: - Private Properties

    private let configuration: ClientConfiguration
    private var webSocketTask: URLSessionWebSocketTask?
    private var session: URLSession
    private var reconnectAttempts = 0
    private var reconnectTask: Task<Void, Never>?
    private var heartbeatTask: Task<Void, Never>?
    private var receiveTask: Task<Void, Never>?

    private var requestId = 0
    private var pendingRequests: [Int: CheckedContinuation<Data, Error>] = [:]
    private var subscriptions: [String: (any LivingProtocolEvent) -> Void] = [:]
    private var serverSubscriptionId: String?

    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    // MARK: - Initialization

    /// Create a new Living Protocol client
    /// - Parameter configuration: Client configuration
    public init(configuration: ClientConfiguration) {
        self.configuration = configuration
        self.session = URLSession(configuration: .default)
    }

    /// Convenience initializer with URL string
    /// - Parameter urlString: WebSocket URL string
    public convenience init(url urlString: String) throws {
        guard let url = URL(string: urlString) else {
            throw LivingProtocolError.connectionFailed("Invalid URL")
        }
        self.init(configuration: ClientConfiguration(url: url))
    }

    deinit {
        disconnect()
    }

    // MARK: - Connection Management

    /// Connect to the Living Protocol server
    public func connect() async throws {
        guard connectionState != .connected else { return }

        connectionState = .connecting

        do {
            try await establishConnection()
            connectionState = .connected
            reconnectAttempts = 0
            startHeartbeat()
            await resubscribeAll()

            // Fetch initial state
            do {
                cycleState = try await getCycleState()
            } catch {
                // Non-fatal - state will be updated via subscription
            }
        } catch {
            connectionState = .error
            throw error
        }
    }

    /// Disconnect from the server
    public func disconnect() {
        reconnectTask?.cancel()
        reconnectTask = nil
        heartbeatTask?.cancel()
        heartbeatTask = nil
        receiveTask?.cancel()
        receiveTask = nil

        webSocketTask?.cancel(with: .goingAway, reason: nil)
        webSocketTask = nil

        connectionState = .disconnected
        serverSubscriptionId = nil

        // Cancel pending requests
        for (_, continuation) in pendingRequests {
            continuation.resume(throwing: LivingProtocolError.notConnected)
        }
        pendingRequests.removeAll()
    }

    // MARK: - RPC Methods

    /// Get the current cycle state
    public func getCycleState() async throws -> CycleState {
        try await rpcCall(method: "getCycleState")
    }

    /// Get the current phase
    public func getCurrentPhase() async throws -> CyclePhase {
        try await rpcCall(method: "getCurrentPhase")
    }

    /// Get the current cycle number
    public func getCycleNumber() async throws -> Int {
        try await rpcCall(method: "getCycleNumber")
    }

    /// Get the phase progress (0-1)
    public func getPhaseProgress() async throws -> Double {
        try await rpcCall(method: "getPhaseProgress")
    }

    /// Get time remaining in current phase (ms)
    public func getTimeRemaining() async throws -> TimeInterval {
        try await rpcCall(method: "getTimeRemaining")
    }

    /// Get cycle history
    /// - Parameter limit: Maximum number of cycles to return
    public func getCycleHistory(limit: Int? = nil) async throws -> [CycleState] {
        if let limit = limit {
            return try await rpcCall(method: "getCycleHistory", params: ["limit": limit])
        } else {
            return try await rpcCall(method: "getCycleHistory")
        }
    }

    /// Advance to the next phase (if allowed)
    public func advancePhase() async throws -> CycleState {
        try await rpcCall(method: "advancePhase")
    }

    // MARK: - Subscriptions

    /// Subscribe to events with a callback
    /// - Parameters:
    ///   - options: Subscription filter options
    ///   - callback: Called when matching events occur
    /// - Returns: Subscription handle
    @discardableResult
    public func subscribe(
        options: SubscriptionOptions = SubscriptionOptions(),
        callback: @escaping (any LivingProtocolEvent) -> Void
    ) -> Subscription {
        let id = generateSubscriptionId()

        subscriptions[id] = { [weak self] event in
            guard self?.eventMatchesOptions(event, options: options) == true else { return }
            callback(event)
        }

        // Setup server subscription if needed
        if connectionState == .connected {
            Task {
                await setupServerSubscription()
            }
        }

        return Subscription(id: id) { [weak self] in
            self?.subscriptions.removeValue(forKey: id)
            if self?.subscriptions.isEmpty == true {
                Task {
                    await self?.teardownServerSubscription()
                }
            }
        }
    }

    /// Subscribe to specific event types
    public func subscribeToEvents(
        _ eventTypes: [EventType],
        callback: @escaping (any LivingProtocolEvent) -> Void
    ) -> Subscription {
        subscribe(options: SubscriptionOptions(eventTypes: eventTypes), callback: callback)
    }

    /// Subscribe to specific phases
    public func subscribeToPhases(
        _ phases: [CyclePhase],
        callback: @escaping (any LivingProtocolEvent) -> Void
    ) -> Subscription {
        subscribe(options: SubscriptionOptions(phases: phases), callback: callback)
    }

    // MARK: - Private Methods

    private func establishConnection() async throws {
        let request = URLRequest(url: configuration.url)
        webSocketTask = session.webSocketTask(with: request)
        webSocketTask?.resume()

        // Wait for connection or timeout
        try await withTimeout(seconds: configuration.connectionTimeout) {
            // Send a ping to verify connection
            try await self.webSocketTask?.sendPing()
        }

        // Start receiving messages
        startReceiving()
    }

    private func startReceiving() {
        receiveTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self = self, let task = self.webSocketTask else { break }

                do {
                    let message = try await task.receive()
                    await self.handleMessage(message)
                } catch {
                    if !Task.isCancelled {
                        await self.handleDisconnect(error: error)
                    }
                    break
                }
            }
        }
    }

    private func handleMessage(_ message: URLSessionWebSocketTask.Message) async {
        switch message {
        case .string(let text):
            guard let data = text.data(using: .utf8) else { return }
            await processMessage(data)
        case .data(let data):
            await processMessage(data)
        @unknown default:
            break
        }
    }

    private func processMessage(_ data: Data) async {
        // Try to decode as RPC response
        if let response = try? decoder.decode(RPCResponse.self, from: data),
           let requestId = response.id {
            handleRPCResponse(response, requestId: requestId)
            return
        }

        // Try to decode as notification
        if let notification = try? decoder.decode(RPCNotification.self, from: data) {
            await handleNotification(notification)
            return
        }
    }

    private func handleRPCResponse(_ response: RPCResponse, requestId: Int) {
        guard let continuation = pendingRequests.removeValue(forKey: requestId) else { return }

        if let error = response.error {
            continuation.resume(throwing: LivingProtocolError.rpcError(
                code: error.code,
                message: error.message
            ))
        } else if let result = response.result {
            do {
                let data = try JSONSerialization.data(withJSONObject: result)
                continuation.resume(returning: data)
            } catch {
                continuation.resume(throwing: LivingProtocolError.encodingError)
            }
        } else {
            continuation.resume(throwing: LivingProtocolError.invalidResponse)
        }
    }

    private func handleNotification(_ notification: RPCNotification) async {
        guard let params = notification.params,
              let eventTypeString = params["type"] as? String,
              let eventType = EventType(rawValue: eventTypeString) else { return }

        do {
            let eventData = try JSONSerialization.data(withJSONObject: params)
            let event: any LivingProtocolEvent

            switch eventType {
            case .phaseTransition:
                event = try decoder.decode(PhaseTransitionEvent.self, from: eventData)
            case .cycleComplete:
                event = try decoder.decode(CycleCompletionEvent.self, from: eventData)
            case .stateUpdate:
                let stateEvent = try decoder.decode(StateUpdateEvent.self, from: eventData)
                cycleState = stateEvent.state
                event = stateEvent
            case .error:
                event = try decoder.decode(ErrorEvent.self, from: eventData)
            }

            // Dispatch to subscribers
            for callback in subscriptions.values {
                callback(event)
            }
        } catch {
            // Decoding error - ignore
        }
    }

    private func handleDisconnect(error: Error) async {
        webSocketTask = nil
        serverSubscriptionId = nil

        if configuration.autoReconnect && reconnectAttempts < configuration.maxReconnectAttempts {
            connectionState = .reconnecting
            scheduleReconnect()
        } else {
            connectionState = .disconnected
        }
    }

    private func scheduleReconnect() {
        reconnectTask = Task { [weak self] in
            guard let self = self else { return }

            let delay = min(
                self.configuration.reconnectInterval * pow(2.0, Double(self.reconnectAttempts)),
                30.0
            )

            try? await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))

            guard !Task.isCancelled else { return }

            self.reconnectAttempts += 1

            do {
                try await self.connect()
            } catch {
                // Connect will handle further reconnection
            }
        }
    }

    private func startHeartbeat() {
        heartbeatTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: UInt64(self?.configuration.heartbeatInterval ?? 30 * 1_000_000_000))

                guard !Task.isCancelled, let self = self else { break }

                do {
                    let _: Bool = try await self.rpcCall(method: "ping")
                } catch {
                    // Heartbeat failure will trigger disconnect via receive loop
                }
            }
        }
    }

    private func rpcCall<T: Decodable>(method: String, params: [String: Any]? = nil) async throws -> T {
        guard connectionState == .connected, let task = webSocketTask else {
            throw LivingProtocolError.notConnected
        }

        requestId += 1
        let currentId = requestId

        var request: [String: Any] = [
            "jsonrpc": "2.0",
            "id": currentId,
            "method": method
        ]

        if let params = params {
            request["params"] = params
        }

        let requestData = try JSONSerialization.data(withJSONObject: request)
        let message = URLSessionWebSocketTask.Message.string(String(data: requestData, encoding: .utf8)!)

        return try await withTimeout(seconds: 30) {
            try await task.send(message)

            let responseData: Data = try await withCheckedThrowingContinuation { continuation in
                self.pendingRequests[currentId] = continuation
            }

            return try self.decoder.decode(T.self, from: responseData)
        }
    }

    private func setupServerSubscription() async {
        guard serverSubscriptionId == nil else { return }

        do {
            struct SubscribeResult: Decodable {
                let subscriptionId: String
            }

            let result: SubscribeResult = try await rpcCall(
                method: "subscribe",
                params: ["events": ["phase_transition", "cycle_complete", "state_update", "error"]]
            )
            serverSubscriptionId = result.subscriptionId
        } catch {
            // Ignore subscription errors
        }
    }

    private func teardownServerSubscription() async {
        guard let subId = serverSubscriptionId else { return }

        do {
            let _: Bool = try await rpcCall(
                method: "unsubscribe",
                params: ["subscriptionId": subId]
            )
        } catch {
            // Ignore
        }

        serverSubscriptionId = nil
    }

    private func resubscribeAll() async {
        if !subscriptions.isEmpty {
            serverSubscriptionId = nil
            await setupServerSubscription()
        }
    }

    private func eventMatchesOptions(_ event: any LivingProtocolEvent, options: SubscriptionOptions) -> Bool {
        // Check event type filter
        if let eventTypes = options.eventTypes, !eventTypes.isEmpty {
            if !eventTypes.contains(event.type) {
                return false
            }
        }

        // Check phase filter
        if let phases = options.phases, !phases.isEmpty {
            if let transition = event as? PhaseTransitionEvent {
                if !phases.contains(transition.toPhase) {
                    return false
                }
            } else if let stateUpdate = event as? StateUpdateEvent {
                if !phases.contains(stateUpdate.state.phase) {
                    return false
                }
            }
        }

        // Check cycle number filter
        if let cycleNumbers = options.cycleNumbers, !cycleNumbers.isEmpty {
            if let transition = event as? PhaseTransitionEvent {
                if !cycleNumbers.contains(transition.cycleNumber) {
                    return false
                }
            } else if let completion = event as? CycleCompletionEvent {
                if !cycleNumbers.contains(completion.cycleNumber) {
                    return false
                }
            } else if let stateUpdate = event as? StateUpdateEvent {
                if !cycleNumbers.contains(stateUpdate.state.cycleNumber) {
                    return false
                }
            }
        }

        return true
    }

    private func generateSubscriptionId() -> String {
        "sub_\(Date().timeIntervalSince1970)_\(UUID().uuidString.prefix(8))"
    }

    private func withTimeout<T>(seconds: TimeInterval, operation: @escaping () async throws -> T) async throws -> T {
        try await withThrowingTaskGroup(of: T.self) { group in
            group.addTask {
                try await operation()
            }

            group.addTask {
                try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
                throw LivingProtocolError.requestTimeout
            }

            let result = try await group.next()!
            group.cancelAll()
            return result
        }
    }
}

// MARK: - RPC Types

private struct RPCResponse: Decodable {
    let jsonrpc: String
    let id: Int?
    let result: Any?
    let error: RPCError?

    private enum CodingKeys: String, CodingKey {
        case jsonrpc, id, result, error
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        jsonrpc = try container.decode(String.self, forKey: .jsonrpc)
        id = try container.decodeIfPresent(Int.self, forKey: .id)
        error = try container.decodeIfPresent(RPCError.self, forKey: .error)

        if let resultContainer = try? container.decode(AnyCodable.self, forKey: .result) {
            result = resultContainer.value
        } else {
            result = nil
        }
    }
}

private struct RPCError: Decodable {
    let code: Int
    let message: String
}

private struct RPCNotification: Decodable {
    let jsonrpc: String
    let method: String
    let params: [String: Any]?

    private enum CodingKeys: String, CodingKey {
        case jsonrpc, method, params
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        jsonrpc = try container.decode(String.self, forKey: .jsonrpc)
        method = try container.decode(String.self, forKey: .method)

        if let paramsContainer = try? container.decode(AnyCodable.self, forKey: .params),
           let dict = paramsContainer.value as? [String: Any] {
            params = dict
        } else {
            params = nil
        }
    }
}
