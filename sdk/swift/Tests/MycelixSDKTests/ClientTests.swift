import XCTest
@testable import MycelixSDK

final class ClientTests: XCTestCase {

    // MARK: - Type Tests

    func testCyclePhaseValues() {
        XCTAssertEqual(CyclePhase.dormant.rawValue, "dormant")
        XCTAssertEqual(CyclePhase.germination.rawValue, "germination")
        XCTAssertEqual(CyclePhase.growth.rawValue, "growth")
        XCTAssertEqual(CyclePhase.fruiting.rawValue, "fruiting")
        XCTAssertEqual(CyclePhase.sporulation.rawValue, "sporulation")
    }

    func testCyclePhaseDisplayName() {
        XCTAssertEqual(CyclePhase.dormant.displayName, "Dormant")
        XCTAssertEqual(CyclePhase.germination.displayName, "Germination")
        XCTAssertEqual(CyclePhase.growth.displayName, "Growth")
        XCTAssertEqual(CyclePhase.fruiting.displayName, "Fruiting")
        XCTAssertEqual(CyclePhase.sporulation.displayName, "Sporulation")
    }

    func testCycleStateCreation() {
        let state = CycleState(
            phase: .growth,
            cycleNumber: 5,
            phaseStartTime: 1000,
            phaseEndTime: 2000,
            phaseDuration: 1000,
            progress: 0.5,
            metadata: nil
        )

        XCTAssertEqual(state.phase, .growth)
        XCTAssertEqual(state.cycleNumber, 5)
        XCTAssertEqual(state.phaseStartTime, 1000)
        XCTAssertEqual(state.phaseEndTime, 2000)
        XCTAssertEqual(state.phaseDuration, 1000)
        XCTAssertEqual(state.progress, 0.5)
    }

    func testCycleStateEquality() {
        let state1 = CycleState(
            phase: .growth,
            cycleNumber: 1,
            phaseStartTime: 1000,
            phaseEndTime: 2000,
            phaseDuration: 1000,
            progress: 0.5,
            metadata: nil
        )

        let state2 = CycleState(
            phase: .growth,
            cycleNumber: 1,
            phaseStartTime: 1000,
            phaseEndTime: 2000,
            phaseDuration: 1000,
            progress: 0.5,
            metadata: nil
        )

        XCTAssertEqual(state1, state2)
    }

    // MARK: - Event Tests

    func testPhaseTransitionEvent() {
        let event = PhaseTransitionEvent(
            fromPhase: .dormant,
            toPhase: .germination,
            cycleNumber: 1,
            timestamp: 12345
        )

        XCTAssertEqual(event.type, .phaseTransition)
        XCTAssertEqual(event.fromPhase, .dormant)
        XCTAssertEqual(event.toPhase, .germination)
        XCTAssertEqual(event.cycleNumber, 1)
        XCTAssertEqual(event.timestamp, 12345)
    }

    func testCycleCompletionEvent() {
        let event = CycleCompletionEvent(
            cycleNumber: 5,
            duration: 10000,
            timestamp: 12345
        )

        XCTAssertEqual(event.type, .cycleComplete)
        XCTAssertEqual(event.cycleNumber, 5)
        XCTAssertEqual(event.duration, 10000)
        XCTAssertEqual(event.timestamp, 12345)
    }

    func testStateUpdateEvent() {
        let state = CycleState(
            phase: .fruiting,
            cycleNumber: 3,
            phaseStartTime: 1000,
            phaseEndTime: 2000,
            phaseDuration: 1000,
            progress: 0.75,
            metadata: nil
        )

        let event = StateUpdateEvent(state: state, timestamp: 12345)

        XCTAssertEqual(event.type, .stateUpdate)
        XCTAssertEqual(event.state.phase, .fruiting)
        XCTAssertEqual(event.timestamp, 12345)
    }

    func testErrorEvent() {
        let event = ErrorEvent(
            code: "CONNECTION_ERROR",
            message: "Failed to connect",
            timestamp: 12345
        )

        XCTAssertEqual(event.type, .error)
        XCTAssertEqual(event.code, "CONNECTION_ERROR")
        XCTAssertEqual(event.message, "Failed to connect")
    }

    // MARK: - Configuration Tests

    func testClientConfigurationDefaults() {
        let url = URL(string: "wss://example.com/ws")!
        let config = ClientConfiguration(url: url)

        XCTAssertEqual(config.url, url)
        XCTAssertTrue(config.autoReconnect)
        XCTAssertEqual(config.reconnectInterval, 3.0)
        XCTAssertEqual(config.maxReconnectAttempts, 10)
        XCTAssertEqual(config.heartbeatInterval, 30.0)
        XCTAssertEqual(config.connectionTimeout, 10.0)
    }

    func testClientConfigurationCustom() {
        let url = URL(string: "wss://example.com/ws")!
        let config = ClientConfiguration(
            url: url,
            autoReconnect: false,
            reconnectInterval: 5.0,
            maxReconnectAttempts: 5,
            heartbeatInterval: 60.0,
            connectionTimeout: 20.0
        )

        XCTAssertFalse(config.autoReconnect)
        XCTAssertEqual(config.reconnectInterval, 5.0)
        XCTAssertEqual(config.maxReconnectAttempts, 5)
        XCTAssertEqual(config.heartbeatInterval, 60.0)
        XCTAssertEqual(config.connectionTimeout, 20.0)
    }

    // MARK: - Subscription Options Tests

    func testSubscriptionOptionsDefault() {
        let options = SubscriptionOptions()

        XCTAssertNil(options.eventTypes)
        XCTAssertNil(options.phases)
        XCTAssertNil(options.cycleNumbers)
    }

    func testSubscriptionOptionsCustom() {
        let options = SubscriptionOptions(
            eventTypes: [.phaseTransition, .cycleComplete],
            phases: [.growth, .fruiting],
            cycleNumbers: [1, 2, 3]
        )

        XCTAssertEqual(options.eventTypes, [.phaseTransition, .cycleComplete])
        XCTAssertEqual(options.phases, [.growth, .fruiting])
        XCTAssertEqual(options.cycleNumbers, [1, 2, 3])
    }

    // MARK: - Error Tests

    func testLivingProtocolErrorDescriptions() {
        XCTAssertEqual(
            LivingProtocolError.notConnected.errorDescription,
            "Not connected to the Living Protocol server"
        )

        XCTAssertEqual(
            LivingProtocolError.connectionFailed("timeout").errorDescription,
            "Connection failed: timeout"
        )

        XCTAssertEqual(
            LivingProtocolError.connectionTimeout.errorDescription,
            "Connection timed out"
        )

        XCTAssertEqual(
            LivingProtocolError.requestTimeout.errorDescription,
            "Request timed out"
        )

        XCTAssertEqual(
            LivingProtocolError.invalidResponse.errorDescription,
            "Invalid response from server"
        )

        XCTAssertEqual(
            LivingProtocolError.rpcError(code: 100, message: "Test error").errorDescription,
            "RPC error (100): Test error"
        )
    }

    // MARK: - Codable Tests

    func testCyclePhaseEncoding() throws {
        let encoder = JSONEncoder()
        let data = try encoder.encode(CyclePhase.growth)
        let string = String(data: data, encoding: .utf8)!

        XCTAssertEqual(string, "\"growth\"")
    }

    func testCyclePhaseDecoding() throws {
        let decoder = JSONDecoder()
        let data = "\"fruiting\"".data(using: .utf8)!
        let phase = try decoder.decode(CyclePhase.self, from: data)

        XCTAssertEqual(phase, .fruiting)
    }

    func testCycleStateEncoding() throws {
        let state = CycleState(
            phase: .dormant,
            cycleNumber: 1,
            phaseStartTime: 1000,
            phaseEndTime: 2000,
            phaseDuration: 1000,
            progress: 0.0,
            metadata: nil
        )

        let encoder = JSONEncoder()
        let data = try encoder.encode(state)
        let json = try JSONSerialization.jsonObject(with: data) as! [String: Any]

        XCTAssertEqual(json["phase"] as? String, "dormant")
        XCTAssertEqual(json["cycleNumber"] as? Int, 1)
    }

    func testPhaseTransitionEventDecoding() throws {
        let json = """
        {
            "type": "phase_transition",
            "fromPhase": "dormant",
            "toPhase": "germination",
            "cycleNumber": 1,
            "timestamp": 12345
        }
        """.data(using: .utf8)!

        let decoder = JSONDecoder()
        let event = try decoder.decode(PhaseTransitionEvent.self, from: json)

        XCTAssertEqual(event.type, .phaseTransition)
        XCTAssertEqual(event.fromPhase, .dormant)
        XCTAssertEqual(event.toPhase, .germination)
        XCTAssertEqual(event.cycleNumber, 1)
        XCTAssertEqual(event.timestamp, 12345)
    }

    // MARK: - AnyCodable Tests

    func testAnyCodableWithPrimitives() throws {
        let boolCodable = AnyCodable(true)
        let intCodable = AnyCodable(42)
        let doubleCodable = AnyCodable(3.14)
        let stringCodable = AnyCodable("test")

        XCTAssertEqual(boolCodable.value as? Bool, true)
        XCTAssertEqual(intCodable.value as? Int, 42)
        XCTAssertEqual(doubleCodable.value as? Double, 3.14)
        XCTAssertEqual(stringCodable.value as? String, "test")
    }

    func testAnyCodableEquality() {
        XCTAssertEqual(AnyCodable(true), AnyCodable(true))
        XCTAssertEqual(AnyCodable(42), AnyCodable(42))
        XCTAssertEqual(AnyCodable("test"), AnyCodable("test"))
        XCTAssertNotEqual(AnyCodable(true), AnyCodable(false))
    }

    // MARK: - Client Creation Tests

    @MainActor
    func testClientCreation() {
        let url = URL(string: "wss://example.com/ws")!
        let config = ClientConfiguration(url: url)
        let client = LivingProtocolClient(configuration: config)

        XCTAssertEqual(client.connectionState, .disconnected)
        XCTAssertNil(client.cycleState)
    }

    @MainActor
    func testClientCreationWithURL() throws {
        let client = try LivingProtocolClient(url: "wss://example.com/ws")

        XCTAssertEqual(client.connectionState, .disconnected)
    }

    @MainActor
    func testClientCreationWithInvalidURL() {
        XCTAssertThrowsError(try LivingProtocolClient(url: "invalid url")) { error in
            XCTAssertTrue(error is LivingProtocolError)
        }
    }
}
