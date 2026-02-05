# Mycelix v6.0 Architecture

This document provides visual diagrams of the Living Protocol Layer architecture.

## System Overview

```mermaid
graph TB
    subgraph "Client Layer"
        TS[TypeScript SDK]
        CLI[CLI Tool]
        DAPP[dApps]
    end

    subgraph "Protocol Layer"
        CE[Cycle Engine]

        subgraph "Primitive Modules"
            MET[Metabolism]
            CON[Consciousness]
            EPI[Epistemics]
            REL[Relational]
            STR[Structural]
        end
    end

    subgraph "Persistence Layer"
        HC[Holochain Zomes]
        EVM[EVM Contracts]
    end

    subgraph "Integration Layer"
        BR[Bridge Zome]
        V52[v5.2 Property DNA]
    end

    TS --> CE
    CLI --> CE
    DAPP --> TS

    CE --> MET
    CE --> CON
    CE --> EPI
    CE --> REL
    CE --> STR

    MET --> HC
    CON --> HC
    EPI --> HC
    REL --> HC
    STR --> HC

    MET --> EVM
    STR --> EVM

    HC --> BR
    BR --> V52
```

## 28-Day Metabolism Cycle

```mermaid
graph LR
    subgraph "Cycle Phases"
        SH[Shadow<br/>Day 1-3]
        CO[Composting<br/>Day 4-6]
        LI[Liminal<br/>Day 7-9]
        NC[Negative<br/>Capability<br/>Day 10-12]
        ER[Eros<br/>Day 13-15]
        CC[Co-Creation<br/>Day 16-18]
        BE[Beauty<br/>Day 19-21]
        EP[Emergent<br/>Personhood<br/>Day 22-24]
        KE[Kenosis<br/>Day 25-28]
    end

    SH --> CO --> LI --> NC --> ER --> CC --> BE --> EP --> KE
    KE -->|New Cycle| SH
```

## Data Flow

```mermaid
flowchart TD
    subgraph "Input"
        AG[Agent Actions]
        EV[External Events]
        TI[Time/Tick]
    end

    subgraph "Processing"
        PH[Phase Handler]
        PE[Primitive Engine]
        VA[Validation Gates]
    end

    subgraph "Output"
        EVT[Protocol Events]
        ST[State Changes]
        ME[Metrics]
    end

    AG --> PH
    EV --> PH
    TI --> PH

    PH --> PE
    PE --> VA

    VA -->|Gate 1 Pass| ST
    VA -->|Gate 2 Warning| EVT
    VA -->|Gate 3 Advisory| ME

    ST --> EVT
```

## Primitive Dependencies

```mermaid
graph TD
    subgraph "living-core"
        TYPES[Types]
        EVENTS[Events]
        ERRORS[Errors]
        CONFIG[Config]
    end

    subgraph "metabolism"
        COMP[Composting]
        WOUND[Wound Healing]
        TRUST[Metabolic Trust]
        KEN[Kenosis]
    end

    subgraph "consciousness"
        KVEC[Temporal K-Vector]
        FIELD[Field Interference]
        DREAM[Collective Dreaming]
        PHI[Emergent Personhood]
    end

    subgraph "epistemics"
        SHADOW[Shadow Integration]
        NEGCAP[Negative Capability]
        SILENCE[Silence Signaling]
        BEAUTY[Beauty Validity]
    end

    subgraph "relational"
        ENTANGLE[Entangled Pairs]
        EROS[Eros Attractor]
        LIMINAL[Liminality]
        INTER[Inter-Species]
    end

    subgraph "structural"
        RESONANCE[Resonance Addressing]
        FRACTAL[Fractal Governance]
        MORPHO[Morphogenetic Fields]
        CRYSTAL[Time Crystals]
        MYCELIAL[Mycelial Computation]
    end

    TYPES --> COMP & WOUND & TRUST & KEN
    TYPES --> KVEC & FIELD & DREAM & PHI
    TYPES --> SHADOW & NEGCAP & SILENCE & BEAUTY
    TYPES --> ENTANGLE & EROS & LIMINAL & INTER
    TYPES --> RESONANCE & FRACTAL & MORPHO & CRYSTAL & MYCELIAL

    EVENTS --> COMP & WOUND & KEN
    CONFIG --> COMP & WOUND & TRUST & KEN
```

## Wound Healing State Machine

```mermaid
stateDiagram-v2
    [*] --> Hemostasis: createWound()
    Hemostasis --> Inflammation: advancePhase()
    Inflammation --> Proliferation: advancePhase()
    Proliferation --> Remodeling: advancePhase()
    Remodeling --> Healed: advancePhase()
    Healed --> [*]

    Healed --> ScarTissue: formScar()
    ScarTissue --> [*]

    note right of Hemostasis
        Immediate response
        Escrow funds locked
    end note

    note right of Inflammation
        Assessment period
        Witness gathering
    end note

    note right of Proliferation
        Repair activities
        Restitution begins
    end note

    note right of Remodeling
        Strengthening
        Pattern integration
    end note
```

## Liminal Transition States

```mermaid
stateDiagram-v2
    [*] --> PreLiminal: enterLiminalState()
    PreLiminal --> Liminal: beginTransition()
    Liminal --> PostLiminal: completeTransition()
    PostLiminal --> Integrated: integrate()
    Integrated --> [*]

    note right of PreLiminal
        Preparation phase
        Old identity releasing
    end note

    note right of Liminal
        Threshold state
        Neither/nor
        Recategorization blocked
    end note

    note right of PostLiminal
        Emergence phase
        New patterns forming
    end note
```

## K-Vector Dimensions

```mermaid
pie title "8D Consciousness Vector"
    "Presence" : 12.5
    "Coherence" : 12.5
    "Receptivity" : 12.5
    "Integration" : 12.5
    "Generativity" : 12.5
    "Surrender" : 12.5
    "Discernment" : 12.5
    "Emergence" : 12.5
```

## Cross-DNA Integration

```mermaid
sequenceDiagram
    participant LP as Living Protocol DNA
    participant BR as Bridge Zome
    participant PR as Property DNA (v5.2)

    LP->>BR: fetch_matl_score(agent)
    BR->>PR: call_remote(get_matl_score)
    PR-->>BR: MatlScoreResponse
    BR-->>LP: MetabolicTrust input

    PR->>BR: slash_event(agent, amount)
    BR->>BR: intercept_slash()
    BR->>LP: create_wound()
    LP-->>BR: WoundRecord
    BR-->>PR: slash_intercepted

    LP->>BR: attach_beauty_score(proposal)
    BR->>PR: call_remote(attach_metadata)
    PR-->>BR: success
```

## Gate System Flow

```mermaid
flowchart TD
    INPUT[Operation Request]

    G1{Gate 1<br/>Hard Invariants}
    G2{Gate 2<br/>Soft Constraints}
    G3{Gate 3<br/>Network Health}

    BLOCK[BLOCKED<br/>Operation Rejected]
    WARN[WARNING<br/>Proceed with Caution]
    ADVISE[ADVISORY<br/>Monitor Recommended]
    SUCCESS[SUCCESS<br/>Operation Completed]

    INPUT --> G1
    G1 -->|Fail| BLOCK
    G1 -->|Pass| G2
    G2 -->|Fail| WARN
    G2 -->|Pass| G3
    G3 -->|Fail| ADVISE
    G3 -->|Pass| SUCCESS

    WARN --> SUCCESS
    ADVISE --> SUCCESS
```

## Holochain Zome Architecture

```mermaid
graph TB
    subgraph "DNA: mycelix-living-protocol"
        subgraph "Integrity Zomes"
            MI[metabolism_integrity]
            CI[consciousness_integrity]
            EI[epistemics_integrity]
            RI[relational_integrity]
            SI[structural_integrity]
            BI[bridge_integrity]
        end

        subgraph "Coordinator Zomes"
            MC[living_metabolism]
            CC[living_consciousness]
            EC[living_epistemics]
            RC[living_relational]
            SC[living_structural]
            BC[bridge]
        end

        MC --> MI
        CC --> CI
        EC --> EI
        RC --> RI
        SC --> SI
        BC --> BI
    end

    subgraph "Shared"
        SH[mycelix_shared]
    end

    MI --> SH
    CI --> SH
    EI --> SH
    RI --> SH
    SI --> SH
    BI --> SH
```

## Solidity Contract Interactions

```mermaid
sequenceDiagram
    participant Agent
    participant WE as WoundEscrow
    participant KB as KenosisBurn
    participant FD as FractalDAO

    Agent->>WE: createWound(severity, amount)
    WE-->>Agent: woundId

    loop Healing Phases
        Agent->>WE: advancePhase(woundId)
        WE-->>Agent: newPhase
    end

    Agent->>WE: releaseEscrow(woundId)
    WE-->>Agent: funds returned

    Agent->>KB: commitKenosis(percentage)
    Note over KB: Max 20% per cycle
    KB-->>Agent: commitmentId

    Agent->>KB: executeKenosis(commitmentId)
    KB-->>Agent: reputation burned

    Agent->>FD: createPattern(scale, params)
    FD-->>Agent: patternId

    Agent->>FD: submitProposal(patternId, content)
    FD-->>Agent: proposalId

    Agent->>FD: vote(proposalId, support)
    FD-->>Agent: voteRecorded
```

## Event Bus Architecture

```mermaid
flowchart LR
    subgraph "Producers"
        PE1[Composting Engine]
        PE2[Wound Healing]
        PE3[Kenosis Engine]
        PE4[Cycle Engine]
    end

    EB[Event Bus<br/>InMemoryEventBus]

    subgraph "Consumers"
        C1[Metrics Collector]
        C2[Bridge Zome]
        C3[Notification Service]
        C4[Audit Logger]
    end

    PE1 -->|CompostingStarted| EB
    PE2 -->|WoundCreated| EB
    PE3 -->|KenosisCommitted| EB
    PE4 -->|PhaseTransition| EB

    EB --> C1
    EB --> C2
    EB --> C3
    EB --> C4
```

## Deployment Architecture

```mermaid
graph TB
    subgraph "Client"
        WEB[Web Browser]
        NODE[Node.js App]
    end

    subgraph "Holochain Network"
        HC1[Conductor 1]
        HC2[Conductor 2]
        HC3[Conductor N]
        DHT[(DHT)]
    end

    subgraph "EVM Network"
        ETH[Ethereum/L2]
        SC1[WoundEscrow]
        SC2[KenosisBurn]
        SC3[FractalDAO]
    end

    WEB --> HC1
    NODE --> HC1

    HC1 <--> DHT
    HC2 <--> DHT
    HC3 <--> DHT

    HC1 <--> HC2
    HC2 <--> HC3

    HC1 --> ETH
    ETH --> SC1
    ETH --> SC2
    ETH --> SC3
```

---

These diagrams can be rendered using any Mermaid-compatible viewer, including:
- GitHub markdown
- GitLab markdown
- VS Code with Mermaid extension
- [Mermaid Live Editor](https://mermaid.live)
