"""Type definitions for the Mycelix Living Protocol SDK.

This module contains dataclasses and enums that mirror the Rust types
from the living-core crate, providing type-safe access to protocol data.
"""

from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Any


class CyclePhase(str, Enum):
    """Phases of the 28-day Metabolism Cycle.

    The cycle progresses through these phases in order:
    Shadow (2d) -> Composting (5d) -> Liminal (3d) -> NegativeCapability (3d) ->
    Eros (4d) -> CoCreation (7d) -> Beauty (2d) -> EmergentPersonhood (1d) ->
    Kenosis (1d) -> [back to Shadow]
    """

    SHADOW = "Shadow"
    COMPOSTING = "Composting"
    LIMINAL = "Liminal"
    NEGATIVE_CAPABILITY = "NegativeCapability"
    EROS = "Eros"
    CO_CREATION = "CoCreation"
    BEAUTY = "Beauty"
    EMERGENT_PERSONHOOD = "EmergentPersonhood"
    KENOSIS = "Kenosis"

    @property
    def duration_days(self) -> int:
        """Get the duration of this phase in days."""
        durations = {
            CyclePhase.SHADOW: 2,
            CyclePhase.COMPOSTING: 5,
            CyclePhase.LIMINAL: 3,
            CyclePhase.NEGATIVE_CAPABILITY: 3,
            CyclePhase.EROS: 4,
            CyclePhase.CO_CREATION: 7,
            CyclePhase.BEAUTY: 2,
            CyclePhase.EMERGENT_PERSONHOOD: 1,
            CyclePhase.KENOSIS: 1,
        }
        return durations[self]

    def next(self) -> "CyclePhase":
        """Get the next phase in the cycle (wraps around)."""
        phases = list(CyclePhase)
        idx = phases.index(self)
        return phases[(idx + 1) % len(phases)]

    def prev(self) -> "CyclePhase":
        """Get the previous phase in the cycle (wraps around)."""
        phases = list(CyclePhase)
        idx = phases.index(self)
        return phases[(idx - 1) % len(phases)]

    @classmethod
    def from_str(cls, value: str) -> "CyclePhase":
        """Parse a phase from its string representation."""
        for phase in cls:
            if phase.value == value:
                return phase
        raise ValueError(f"Unknown phase: {value}")


@dataclass
class CycleState:
    """Current state of a metabolism cycle.

    Attributes:
        cycle_number: The current cycle number (1-indexed).
        current_phase: The current phase of the cycle.
        phase_started: When the current phase started.
        cycle_started: When the current cycle started.
        phase_day: The current day within the phase (0-indexed).
    """

    cycle_number: int
    current_phase: CyclePhase
    phase_started: datetime
    cycle_started: datetime
    phase_day: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "CycleState":
        """Create a CycleState from a dictionary (JSON response)."""
        return cls(
            cycle_number=data["cycleNumber"],
            current_phase=CyclePhase.from_str(data["currentPhase"]),
            phase_started=datetime.fromisoformat(data["phaseStarted"].replace("Z", "+00:00")),
            cycle_started=datetime.fromisoformat(data["cycleStarted"].replace("Z", "+00:00")),
            phase_day=data["phaseDay"],
        )


@dataclass
class PhaseMetrics:
    """Metrics collected at phase transitions.

    Attributes:
        active_agents: Number of active agents in the network.
        spectral_k: The spectral K-Vector derivative.
        mean_metabolic_trust: Average metabolic trust across agents.
        active_wounds: Number of wounds currently being healed.
        composting_entities: Number of entities being composted.
        liminal_entities: Number of entities in liminal transition.
        entangled_pairs: Number of entangled agent pairs.
        held_uncertainties: Number of claims held in uncertainty.
    """

    active_agents: int
    spectral_k: float
    mean_metabolic_trust: float
    active_wounds: int
    composting_entities: int
    liminal_entities: int
    entangled_pairs: int
    held_uncertainties: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "PhaseMetrics":
        """Create PhaseMetrics from a dictionary (JSON response)."""
        return cls(
            active_agents=data.get("activeAgents", data.get("active_agents", 0)),
            spectral_k=data.get("spectralK", data.get("spectral_k", 0.0)),
            mean_metabolic_trust=data.get(
                "meanMetabolicTrust", data.get("mean_metabolic_trust", 0.0)
            ),
            active_wounds=data.get("activeWounds", data.get("active_wounds", 0)),
            composting_entities=data.get("compostingEntities", data.get("composting_entities", 0)),
            liminal_entities=data.get("liminalEntities", data.get("liminal_entities", 0)),
            entangled_pairs=data.get("entangledPairs", data.get("entangled_pairs", 0)),
            held_uncertainties=data.get("heldUncertainties", data.get("held_uncertainties", 0)),
        )


@dataclass
class PhaseTransition:
    """Record of a phase transition in the metabolism cycle.

    Attributes:
        from_phase: The phase transitioned from.
        to_phase: The phase transitioned to.
        cycle_number: The cycle number when this transition occurred.
        transitioned_at: When the transition occurred.
        metrics: Metrics collected at the time of transition.
    """

    from_phase: CyclePhase
    to_phase: CyclePhase
    cycle_number: int
    transitioned_at: datetime
    metrics: PhaseMetrics | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "PhaseTransition":
        """Create a PhaseTransition from a dictionary (JSON response)."""
        metrics = None
        if "metrics" in data:
            metrics = PhaseMetrics.from_dict(data["metrics"])

        return cls(
            from_phase=CyclePhase.from_str(data["from"]),
            to_phase=CyclePhase.from_str(data["to"]),
            cycle_number=data["cycleNumber"],
            transitioned_at=datetime.fromisoformat(
                data["transitionedAt"].replace("Z", "+00:00")
            ),
            metrics=metrics,
        )


@dataclass
class LivingProtocolEvent:
    """A Living Protocol event received from the WebSocket.

    This is a generic wrapper for all event types. The actual event
    data is stored in the `data` field as a dictionary.

    Attributes:
        event_type: The type of event (e.g., "PhaseTransitioned", "CycleStarted").
        data: The event data as a dictionary.
        timestamp: When the event was received.
    """

    event_type: str
    data: dict[str, Any]
    timestamp: datetime = field(default_factory=datetime.now)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "LivingProtocolEvent":
        """Create an event from a dictionary (JSON message)."""
        # Handle wrapped events (from the server)
        if "PhaseTransitioned" in data:
            return cls(
                event_type="PhaseTransitioned",
                data=data["PhaseTransitioned"],
            )
        if "CycleStarted" in data:
            return cls(
                event_type="CycleStarted",
                data=data["CycleStarted"],
            )
        # Generic event structure
        if "type" in data:
            return cls(
                event_type=data["type"],
                data=data.get("data", {}),
            )
        # Fallback for unknown structure
        return cls(
            event_type="Unknown",
            data=data,
        )

    def as_phase_transition(self) -> PhaseTransition | None:
        """Convert this event to a PhaseTransition if applicable."""
        if self.event_type == "PhaseTransitioned" and "transition" in self.data:
            return PhaseTransition.from_dict(self.data["transition"])
        return None
