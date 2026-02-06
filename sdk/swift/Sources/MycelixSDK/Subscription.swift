import Foundation
import Combine

// MARK: - Subscription

/// A handle to an active event subscription
public final class Subscription: Sendable {
    /// Unique identifier for this subscription
    public let id: String

    private let cancelAction: @Sendable () -> Void

    init(id: String, cancelAction: @escaping @Sendable () -> Void) {
        self.id = id
        self.cancelAction = cancelAction
    }

    /// Cancel the subscription
    public func cancel() {
        cancelAction()
    }
}

// MARK: - Event Publisher

/// A Combine publisher for Living Protocol events
public struct EventPublisher: Publisher {
    public typealias Output = any LivingProtocolEvent
    public typealias Failure = Never

    private let client: LivingProtocolClient
    private let options: SubscriptionOptions

    init(client: LivingProtocolClient, options: SubscriptionOptions) {
        self.client = client
        self.options = options
    }

    public func receive<S>(subscriber: S) where S: Subscriber, Never == S.Failure, Output == S.Input {
        let subscription = EventSubscription(
            subscriber: subscriber,
            client: client,
            options: options
        )
        subscriber.receive(subscription: subscription)
    }
}

/// Combine subscription for events
private final class EventSubscription<S: Subscriber>: Combine.Subscription
    where S.Input == any LivingProtocolEvent, S.Failure == Never {

    private var subscriber: S?
    private var protocolSubscription: Subscription?

    init(subscriber: S, client: LivingProtocolClient, options: SubscriptionOptions) {
        self.subscriber = subscriber

        // Subscribe to client events
        self.protocolSubscription = client.subscribe(options: options) { [weak self] event in
            _ = self?.subscriber?.receive(event)
        }
    }

    func request(_ demand: Subscribers.Demand) {
        // Unlimited demand - we send all events
    }

    func cancel() {
        subscriber = nil
        protocolSubscription?.cancel()
        protocolSubscription = nil
    }
}

// MARK: - Cycle State Publisher

/// A Combine publisher for cycle state updates
public struct CycleStatePublisher: Publisher {
    public typealias Output = CycleState
    public typealias Failure = Never

    private let client: LivingProtocolClient

    init(client: LivingProtocolClient) {
        self.client = client
    }

    public func receive<S>(subscriber: S) where S: Subscriber, Never == S.Failure, CycleState == S.Input {
        let subscription = CycleStateSubscription(subscriber: subscriber, client: client)
        subscriber.receive(subscription: subscription)
    }
}

/// Combine subscription for cycle state
private final class CycleStateSubscription<S: Subscriber>: Combine.Subscription
    where S.Input == CycleState, S.Failure == Never {

    private var subscriber: S?
    private var protocolSubscription: Subscription?

    init(subscriber: S, client: LivingProtocolClient) {
        self.subscriber = subscriber

        // Subscribe to state updates
        self.protocolSubscription = client.subscribe(
            options: SubscriptionOptions(eventTypes: [.stateUpdate, .phaseTransition])
        ) { [weak self] event in
            guard let self = self else { return }

            if let stateEvent = event as? StateUpdateEvent {
                _ = self.subscriber?.receive(stateEvent.state)
            }
        }

        // Fetch initial state
        Task {
            do {
                let state = try await client.getCycleState()
                _ = self.subscriber?.receive(state)
            } catch {
                // Ignore initial fetch errors
            }
        }
    }

    func request(_ demand: Subscribers.Demand) {
        // Unlimited demand
    }

    func cancel() {
        subscriber = nil
        protocolSubscription?.cancel()
        protocolSubscription = nil
    }
}

// MARK: - Phase Publisher

/// A Combine publisher for phase transitions
public struct PhasePublisher: Publisher {
    public typealias Output = CyclePhase
    public typealias Failure = Never

    private let client: LivingProtocolClient
    private let phases: [CyclePhase]?

    init(client: LivingProtocolClient, phases: [CyclePhase]? = nil) {
        self.client = client
        self.phases = phases
    }

    public func receive<S>(subscriber: S) where S: Subscriber, Never == S.Failure, CyclePhase == S.Input {
        let subscription = PhaseSubscription(
            subscriber: subscriber,
            client: client,
            phases: phases
        )
        subscriber.receive(subscription: subscription)
    }
}

/// Combine subscription for phases
private final class PhaseSubscription<S: Subscriber>: Combine.Subscription
    where S.Input == CyclePhase, S.Failure == Never {

    private var subscriber: S?
    private var protocolSubscription: Subscription?

    init(subscriber: S, client: LivingProtocolClient, phases: [CyclePhase]?) {
        self.subscriber = subscriber

        self.protocolSubscription = client.subscribe(
            options: SubscriptionOptions(eventTypes: [.phaseTransition], phases: phases)
        ) { [weak self] event in
            if let transition = event as? PhaseTransitionEvent {
                _ = self?.subscriber?.receive(transition.toPhase)
            }
        }
    }

    func request(_ demand: Subscribers.Demand) {
        // Unlimited demand
    }

    func cancel() {
        subscriber = nil
        protocolSubscription?.cancel()
        protocolSubscription = nil
    }
}

// MARK: - Connection State Publisher

/// A Combine publisher for connection state changes
public struct ConnectionStatePublisher: Publisher {
    public typealias Output = ConnectionState
    public typealias Failure = Never

    private let client: LivingProtocolClient

    init(client: LivingProtocolClient) {
        self.client = client
    }

    public func receive<S>(subscriber: S) where S: Subscriber, Never == S.Failure, ConnectionState == S.Input {
        let subscription = ConnectionStateSubscription(subscriber: subscriber, client: client)
        subscriber.receive(subscription: subscription)
    }
}

/// Combine subscription for connection state
private final class ConnectionStateSubscription<S: Subscriber>: Combine.Subscription
    where S.Input == ConnectionState, S.Failure == Never {

    private var subscriber: S?
    private var cancellable: AnyCancellable?

    init(subscriber: S, client: LivingProtocolClient) {
        self.subscriber = subscriber

        // Subscribe to connection state changes via the client's published property
        self.cancellable = client.connectionStatePublisher
            .sink { [weak self] state in
                _ = self?.subscriber?.receive(state)
            }
    }

    func request(_ demand: Subscribers.Demand) {
        // Unlimited demand
    }

    func cancel() {
        subscriber = nil
        cancellable?.cancel()
        cancellable = nil
    }
}

// MARK: - Client Publisher Extensions

extension LivingProtocolClient {
    /// Publisher for all events matching the given options
    public func eventPublisher(options: SubscriptionOptions = SubscriptionOptions()) -> EventPublisher {
        EventPublisher(client: self, options: options)
    }

    /// Publisher for cycle state updates
    public func cycleStatePublisher() -> CycleStatePublisher {
        CycleStatePublisher(client: self)
    }

    /// Publisher for phase transitions
    public func phasePublisher(phases: [CyclePhase]? = nil) -> PhasePublisher {
        PhasePublisher(client: self, phases: phases)
    }

    /// Publisher for connection state changes
    public var connectionStatePublisher: AnyPublisher<ConnectionState, Never> {
        $connectionState.eraseToAnyPublisher()
    }

    /// Publisher that emits when entering a specific phase
    public func onPhaseEnter(_ phase: CyclePhase) -> AnyPublisher<PhaseTransitionEvent, Never> {
        eventPublisher(options: SubscriptionOptions(eventTypes: [.phaseTransition]))
            .compactMap { $0 as? PhaseTransitionEvent }
            .filter { $0.toPhase == phase }
            .eraseToAnyPublisher()
    }

    /// Publisher that emits when exiting a specific phase
    public func onPhaseExit(_ phase: CyclePhase) -> AnyPublisher<PhaseTransitionEvent, Never> {
        eventPublisher(options: SubscriptionOptions(eventTypes: [.phaseTransition]))
            .compactMap { $0 as? PhaseTransitionEvent }
            .filter { $0.fromPhase == phase }
            .eraseToAnyPublisher()
    }

    /// Publisher that emits when a cycle completes
    public func onCycleComplete() -> AnyPublisher<CycleCompletionEvent, Never> {
        eventPublisher(options: SubscriptionOptions(eventTypes: [.cycleComplete]))
            .compactMap { $0 as? CycleCompletionEvent }
            .eraseToAnyPublisher()
    }
}

// MARK: - AsyncSequence Support

/// AsyncSequence wrapper for events
public struct EventAsyncSequence: AsyncSequence {
    public typealias Element = any LivingProtocolEvent

    private let client: LivingProtocolClient
    private let options: SubscriptionOptions

    init(client: LivingProtocolClient, options: SubscriptionOptions) {
        self.client = client
        self.options = options
    }

    public func makeAsyncIterator() -> AsyncIterator {
        AsyncIterator(client: client, options: options)
    }

    public struct AsyncIterator: AsyncIteratorProtocol {
        private let stream: AsyncStream<any LivingProtocolEvent>
        private var iterator: AsyncStream<any LivingProtocolEvent>.Iterator
        private var subscription: Subscription?

        init(client: LivingProtocolClient, options: SubscriptionOptions) {
            var subscriptionHolder: Subscription?

            self.stream = AsyncStream { continuation in
                subscriptionHolder = client.subscribe(options: options) { event in
                    continuation.yield(event)
                }

                continuation.onTermination = { _ in
                    subscriptionHolder?.cancel()
                }
            }

            self.subscription = subscriptionHolder
            self.iterator = stream.makeAsyncIterator()
        }

        public mutating func next() async -> Element? {
            await iterator.next()
        }
    }
}

extension LivingProtocolClient {
    /// AsyncSequence of events
    public func events(options: SubscriptionOptions = SubscriptionOptions()) -> EventAsyncSequence {
        EventAsyncSequence(client: self, options: options)
    }
}
