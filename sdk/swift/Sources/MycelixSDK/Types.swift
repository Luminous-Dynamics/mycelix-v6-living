import Foundation

// MARK: - Cycle Phase

/// Represents the phases in the Living Protocol lifecycle
public enum CyclePhase: String, Codable, CaseIterable, Sendable {
    case dormant
    case germination
    case growth
    case fruiting
    case sporulation

    /// Human-readable name for the phase
    public var displayName: String {
        rawValue.capitalized
    }
}

// MARK: - Connection State

/// Represents the WebSocket connection state
public enum ConnectionState: String, Sendable {
    case disconnected
    case connecting
    case connected
    case reconnecting
    case error
}

// MARK: - Cycle State

/// Represents the current state of the protocol cycle
public struct CycleState: Codable, Sendable, Equatable {
    /// Current phase of the cycle
    public let phase: CyclePhase

    /// Current cycle number
    public let cycleNumber: Int

    /// Timestamp when the current phase started
    public let phaseStartTime: TimeInterval

    /// Timestamp when the current phase will end
    public let phaseEndTime: TimeInterval

    /// Duration of the current phase in milliseconds
    public let phaseDuration: TimeInterval

    /// Progress through the current phase (0.0 - 1.0)
    public let progress: Double

    /// Optional metadata associated with the cycle
    public let metadata: [String: AnyCodable]?

    public init(
        phase: CyclePhase,
        cycleNumber: Int,
        phaseStartTime: TimeInterval,
        phaseEndTime: TimeInterval,
        phaseDuration: TimeInterval,
        progress: Double,
        metadata: [String: AnyCodable]? = nil
    ) {
        self.phase = phase
        self.cycleNumber = cycleNumber
        self.phaseStartTime = phaseStartTime
        self.phaseEndTime = phaseEndTime
        self.phaseDuration = phaseDuration
        self.progress = progress
        self.metadata = metadata
    }

    /// Time remaining in the current phase
    public var timeRemaining: TimeInterval {
        max(0, phaseEndTime - Date().timeIntervalSince1970 * 1000)
    }
}

// MARK: - Events

/// Protocol for all Living Protocol events
public protocol LivingProtocolEvent: Sendable {
    var type: EventType { get }
    var timestamp: TimeInterval { get }
}

/// Event types
public enum EventType: String, Codable, Sendable {
    case phaseTransition = "phase_transition"
    case cycleComplete = "cycle_complete"
    case stateUpdate = "state_update"
    case error = "error"
}

/// Event emitted when transitioning between phases
public struct PhaseTransitionEvent: LivingProtocolEvent, Codable, Sendable {
    public let type: EventType = .phaseTransition
    public let fromPhase: CyclePhase
    public let toPhase: CyclePhase
    public let cycleNumber: Int
    public let timestamp: TimeInterval

    public init(fromPhase: CyclePhase, toPhase: CyclePhase, cycleNumber: Int, timestamp: TimeInterval) {
        self.fromPhase = fromPhase
        self.toPhase = toPhase
        self.cycleNumber = cycleNumber
        self.timestamp = timestamp
    }

    private enum CodingKeys: String, CodingKey {
        case type, fromPhase, toPhase, cycleNumber, timestamp
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        fromPhase = try container.decode(CyclePhase.self, forKey: .fromPhase)
        toPhase = try container.decode(CyclePhase.self, forKey: .toPhase)
        cycleNumber = try container.decode(Int.self, forKey: .cycleNumber)
        timestamp = try container.decode(TimeInterval.self, forKey: .timestamp)
    }
}

/// Event emitted when a cycle completes
public struct CycleCompletionEvent: LivingProtocolEvent, Codable, Sendable {
    public let type: EventType = .cycleComplete
    public let cycleNumber: Int
    public let duration: TimeInterval
    public let timestamp: TimeInterval

    public init(cycleNumber: Int, duration: TimeInterval, timestamp: TimeInterval) {
        self.cycleNumber = cycleNumber
        self.duration = duration
        self.timestamp = timestamp
    }

    private enum CodingKeys: String, CodingKey {
        case type, cycleNumber, duration, timestamp
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        cycleNumber = try container.decode(Int.self, forKey: .cycleNumber)
        duration = try container.decode(TimeInterval.self, forKey: .duration)
        timestamp = try container.decode(TimeInterval.self, forKey: .timestamp)
    }
}

/// Event emitted when state updates
public struct StateUpdateEvent: LivingProtocolEvent, Codable, Sendable {
    public let type: EventType = .stateUpdate
    public let state: CycleState
    public let timestamp: TimeInterval

    public init(state: CycleState, timestamp: TimeInterval) {
        self.state = state
        self.timestamp = timestamp
    }

    private enum CodingKeys: String, CodingKey {
        case type, state, timestamp
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        state = try container.decode(CycleState.self, forKey: .state)
        timestamp = try container.decode(TimeInterval.self, forKey: .timestamp)
    }
}

/// Event emitted on error
public struct ErrorEvent: LivingProtocolEvent, Codable, Sendable {
    public let type: EventType = .error
    public let code: String
    public let message: String
    public let timestamp: TimeInterval

    public init(code: String, message: String, timestamp: TimeInterval) {
        self.code = code
        self.message = message
        self.timestamp = timestamp
    }

    private enum CodingKeys: String, CodingKey {
        case type, code, message, timestamp
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        code = try container.decode(String.self, forKey: .code)
        message = try container.decode(String.self, forKey: .message)
        timestamp = try container.decode(TimeInterval.self, forKey: .timestamp)
    }
}

// MARK: - Subscription Options

/// Options for filtering event subscriptions
public struct SubscriptionOptions: Sendable {
    /// Event types to subscribe to
    public var eventTypes: [EventType]?

    /// Phases to filter for
    public var phases: [CyclePhase]?

    /// Cycle numbers to filter for
    public var cycleNumbers: [Int]?

    public init(
        eventTypes: [EventType]? = nil,
        phases: [CyclePhase]? = nil,
        cycleNumbers: [Int]? = nil
    ) {
        self.eventTypes = eventTypes
        self.phases = phases
        self.cycleNumbers = cycleNumbers
    }
}

// MARK: - Client Configuration

/// Configuration options for the Living Protocol client
public struct ClientConfiguration: Sendable {
    /// WebSocket URL
    public let url: URL

    /// Whether to automatically reconnect on disconnect
    public var autoReconnect: Bool

    /// Interval between reconnection attempts (seconds)
    public var reconnectInterval: TimeInterval

    /// Maximum number of reconnection attempts
    public var maxReconnectAttempts: Int

    /// Heartbeat interval (seconds)
    public var heartbeatInterval: TimeInterval

    /// Connection timeout (seconds)
    public var connectionTimeout: TimeInterval

    public init(
        url: URL,
        autoReconnect: Bool = true,
        reconnectInterval: TimeInterval = 3.0,
        maxReconnectAttempts: Int = 10,
        heartbeatInterval: TimeInterval = 30.0,
        connectionTimeout: TimeInterval = 10.0
    ) {
        self.url = url
        self.autoReconnect = autoReconnect
        self.reconnectInterval = reconnectInterval
        self.maxReconnectAttempts = maxReconnectAttempts
        self.heartbeatInterval = heartbeatInterval
        self.connectionTimeout = connectionTimeout
    }
}

// MARK: - Errors

/// Errors that can occur in the Living Protocol client
public enum LivingProtocolError: Error, LocalizedError, Sendable {
    case notConnected
    case connectionFailed(String)
    case connectionTimeout
    case requestTimeout
    case invalidResponse
    case rpcError(code: Int, message: String)
    case encodingError
    case decodingError(String)

    public var errorDescription: String? {
        switch self {
        case .notConnected:
            return "Not connected to the Living Protocol server"
        case .connectionFailed(let reason):
            return "Connection failed: \(reason)"
        case .connectionTimeout:
            return "Connection timed out"
        case .requestTimeout:
            return "Request timed out"
        case .invalidResponse:
            return "Invalid response from server"
        case .rpcError(let code, let message):
            return "RPC error (\(code)): \(message)"
        case .encodingError:
            return "Failed to encode request"
        case .decodingError(let details):
            return "Failed to decode response: \(details)"
        }
    }
}

// MARK: - Helper Types

/// Type-erased Codable wrapper for metadata
public struct AnyCodable: Codable, Sendable, Equatable {
    public let value: Any

    public init(_ value: Any) {
        self.value = value
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()

        if container.decodeNil() {
            value = NSNull()
        } else if let bool = try? container.decode(Bool.self) {
            value = bool
        } else if let int = try? container.decode(Int.self) {
            value = int
        } else if let double = try? container.decode(Double.self) {
            value = double
        } else if let string = try? container.decode(String.self) {
            value = string
        } else if let array = try? container.decode([AnyCodable].self) {
            value = array.map { $0.value }
        } else if let dictionary = try? container.decode([String: AnyCodable].self) {
            value = dictionary.mapValues { $0.value }
        } else {
            throw DecodingError.dataCorruptedError(
                in: container,
                debugDescription: "Failed to decode AnyCodable"
            )
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()

        switch value {
        case is NSNull:
            try container.encodeNil()
        case let bool as Bool:
            try container.encode(bool)
        case let int as Int:
            try container.encode(int)
        case let double as Double:
            try container.encode(double)
        case let string as String:
            try container.encode(string)
        case let array as [Any]:
            try container.encode(array.map { AnyCodable($0) })
        case let dictionary as [String: Any]:
            try container.encode(dictionary.mapValues { AnyCodable($0) })
        default:
            throw EncodingError.invalidValue(
                value,
                EncodingError.Context(
                    codingPath: encoder.codingPath,
                    debugDescription: "Failed to encode AnyCodable"
                )
            )
        }
    }

    public static func == (lhs: AnyCodable, rhs: AnyCodable) -> Bool {
        switch (lhs.value, rhs.value) {
        case (is NSNull, is NSNull):
            return true
        case (let l as Bool, let r as Bool):
            return l == r
        case (let l as Int, let r as Int):
            return l == r
        case (let l as Double, let r as Double):
            return l == r
        case (let l as String, let r as String):
            return l == r
        default:
            return false
        }
    }
}
