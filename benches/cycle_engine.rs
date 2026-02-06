//! Benchmarks for the cycle engine and primitive operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;

use cycle_engine::scheduler::CycleEngineBuilder;
use living_core::{
    EpistemicClassification, EpistemicTier, EventBus, InMemoryEventBus, KenosisConfig,
    MaterialityTier, NormativeTier, WoundHealingConfig, WoundSeverity,
};

/// Benchmark engine creation.
fn bench_engine_creation(c: &mut Criterion) {
    c.bench_function("engine_creation", |b| {
        b.iter(|| {
            let engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();
            black_box(engine)
        })
    });
}

/// Benchmark engine start.
fn bench_engine_start(c: &mut Criterion) {
    c.bench_function("engine_start", |b| {
        b.iter_batched(
            || {
                CycleEngineBuilder::new()
                    .with_simulated_time(86400.0)
                    .build()
            },
            |mut engine| black_box(engine.start().unwrap()),
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Benchmark single tick operation.
fn bench_tick(c: &mut Criterion) {
    let mut engine = CycleEngineBuilder::new()
        .with_simulated_time(86400.0)
        .build();
    engine.start().unwrap();

    c.bench_function("tick", |b| b.iter(|| black_box(engine.tick().unwrap())));
}

/// Benchmark phase transition.
fn bench_phase_transition(c: &mut Criterion) {
    c.bench_function("phase_transition", |b| {
        b.iter_batched(
            || {
                let mut engine = CycleEngineBuilder::new()
                    .with_simulated_time(86400.0)
                    .build();
                engine.start().unwrap();
                engine
            },
            |mut engine| black_box(engine.force_transition().unwrap()),
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Benchmark full cycle (9 transitions).
fn bench_full_cycle(c: &mut Criterion) {
    c.bench_function("full_cycle", |b| {
        b.iter_batched(
            || {
                let mut engine = CycleEngineBuilder::new()
                    .with_simulated_time(86400.0)
                    .build();
                engine.start().unwrap();
                engine
            },
            |mut engine| {
                for _ in 0..9 {
                    engine.force_transition().unwrap();
                }
                black_box(engine.cycle_number())
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Benchmark multiple cycles.
fn bench_multiple_cycles(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_cycles");

    for cycles in [1, 5, 10, 25].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(cycles), cycles, |b, &cycles| {
            b.iter_batched(
                || {
                    let mut engine = CycleEngineBuilder::new()
                        .with_simulated_time(86400.0)
                        .build();
                    engine.start().unwrap();
                    engine
                },
                |mut engine| {
                    for _ in 0..(cycles * 9) {
                        engine.force_transition().unwrap();
                    }
                    black_box(engine.cycle_number())
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

/// Benchmark wound healing operations.
fn bench_wound_healing(c: &mut Criterion) {
    use metabolism::wound_healing::WoundHealingEngine;

    let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
    let config = WoundHealingConfig::default();

    c.bench_function("wound_create", |b| {
        b.iter_batched(
            || WoundHealingEngine::new(config.clone(), event_bus.clone()),
            |mut engine: WoundHealingEngine| {
                black_box(
                    engine
                        .create_wound(
                            "did:agent:bench".to_string(),
                            WoundSeverity::Moderate,
                            "benchmark test".to_string(),
                        )
                        .unwrap(),
                )
            },
            criterion::BatchSize::SmallInput,
        )
    });

    c.bench_function("wound_advance_phase", |b| {
        b.iter_batched(
            || {
                let mut engine = WoundHealingEngine::new(config.clone(), event_bus.clone());
                let record = engine
                    .create_wound(
                        "did:agent:bench".to_string(),
                        WoundSeverity::Moderate,
                        "benchmark test".to_string(),
                    )
                    .unwrap();
                (engine, record.id)
            },
            |(mut engine, id): (WoundHealingEngine, String)| {
                black_box(engine.advance_phase(&id).unwrap())
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Benchmark composting operations.
fn bench_composting(c: &mut Criterion) {
    use living_core::{CompostableEntity, CompostingConfig};
    use metabolism::composting::{CompostingEngine, CompostingReason};

    let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
    let config = CompostingConfig::default();

    c.bench_function("composting_start", |b| {
        b.iter_batched(
            || CompostingEngine::new(config.clone(), event_bus.clone()),
            |mut engine| {
                black_box(
                    engine
                        .start_composting(
                            CompostableEntity::FailedProposal,
                            "proposal-bench".to_string(),
                            CompostingReason::ProposalFailed {
                                vote_count: 5,
                                required: 10,
                            },
                        )
                        .unwrap(),
                )
            },
            criterion::BatchSize::SmallInput,
        )
    });

    c.bench_function("composting_extract_nutrient", |b| {
        b.iter_batched(
            || {
                let mut engine = CompostingEngine::new(config.clone(), event_bus.clone());
                let record = engine
                    .start_composting(
                        CompostableEntity::FailedProposal,
                        "proposal-bench".to_string(),
                        CompostingReason::ProposalFailed {
                            vote_count: 5,
                            required: 10,
                        },
                    )
                    .unwrap();
                (engine, record.id)
            },
            |(mut engine, record_id)| {
                let classification = EpistemicClassification {
                    e: EpistemicTier::Testimonial,
                    n: NormativeTier::NetworkConsensus,
                    m: MaterialityTier::Persistent,
                };
                black_box(
                    engine
                        .extract_nutrient(
                            &record_id,
                            "learned-something".to_string(),
                            classification,
                        )
                        .unwrap(),
                )
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Benchmark kenosis operations.
fn bench_kenosis(c: &mut Criterion) {
    use metabolism::kenosis::KenosisEngine;

    let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
    let config = KenosisConfig::default();

    c.bench_function("kenosis_commit", |b| {
        b.iter_batched(
            || {
                let mut engine = KenosisEngine::new(config.clone(), event_bus.clone());
                engine.set_current_cycle(1);
                engine.register_agent("did:agent:bench", 1000.0);
                engine
            },
            |mut engine| black_box(engine.commit_kenosis("did:agent:bench", 0.10).unwrap()),
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Benchmark entanglement operations.
fn bench_entanglement(c: &mut Criterion) {
    use chrono::Utc;
    use living_core::EntanglementConfig;
    use relational::entangled_pairs::EntanglementEngine;

    let mut config = EntanglementConfig::default();
    config.min_co_creation_events = 1;

    c.bench_function("entanglement_record_cocreation", |b| {
        b.iter_batched(
            || EntanglementEngine::new(config.clone()),
            |mut engine| {
                black_box(engine.record_co_creation(
                    &"did:agent:alice".to_string(),
                    &"did:agent:bob".to_string(),
                    "collaborated on benchmark",
                    0.9,
                ))
            },
            criterion::BatchSize::SmallInput,
        )
    });

    c.bench_function("entanglement_form", |b| {
        b.iter_batched(
            || {
                let mut engine = EntanglementEngine::new(config.clone());
                engine.record_co_creation(
                    &"did:agent:alice".to_string(),
                    &"did:agent:bob".to_string(),
                    "collaborated",
                    0.9,
                );
                engine
            },
            |mut engine| {
                black_box(engine.form_entanglement(
                    &"did:agent:alice".to_string(),
                    &"did:agent:bob".to_string(),
                ))
            },
            criterion::BatchSize::SmallInput,
        )
    });

    c.bench_function("entanglement_decay_all", |b| {
        b.iter_batched(
            || {
                let mut engine = EntanglementEngine::new(config.clone());
                for i in 0..100 {
                    let alice = format!("did:agent:alice-{}", i);
                    let bob = format!("did:agent:bob-{}", i);
                    engine.record_co_creation(&alice, &bob, "collaborated", 0.9);
                    let _ = engine.form_entanglement(&alice, &bob);
                }
                engine
            },
            |mut engine| black_box(engine.decay_all(Utc::now())),
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Benchmark beauty scoring.
fn bench_beauty_scoring(c: &mut Criterion) {
    use epistemics::beauty_validity::BeautyValidityEngine;

    c.bench_function("beauty_score_proposal", |b| {
        b.iter_batched(
            || BeautyValidityEngine::new(),
            |mut engine| {
                black_box(engine.score_proposal(
                    "proposal-bench",
                    "A beautifully crafted proposal with clear intent and elegant design.",
                    "did:scorer:bench",
                    &["pattern-1".to_string(), "pattern-2".to_string()],
                    &["requirement-1".to_string()],
                ))
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Benchmark negative capability operations.
fn bench_negative_capability(c: &mut Criterion) {
    use epistemics::negative_capability::NegativeCapabilityEngine;

    c.bench_function("negative_capability_hold", |b| {
        b.iter_batched(
            || NegativeCapabilityEngine::new(),
            |mut engine| {
                black_box(engine.hold_in_uncertainty(
                    "claim-bench",
                    "needs more research",
                    7,
                    "did:holder:bench",
                ))
            },
            criterion::BatchSize::SmallInput,
        )
    });

    c.bench_function("negative_capability_auto_release", |b| {
        b.iter_batched(
            || {
                let mut engine = NegativeCapabilityEngine::new();
                for i in 0..100 {
                    engine.hold_in_uncertainty(
                        &format!("claim-{}", i),
                        "needs research",
                        0,
                        "did:holder:bench",
                    );
                }
                engine
            },
            |mut engine| {
                // max_hold_days = 0 means immediate expiry
                black_box(engine.auto_release_expired(0))
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Benchmark shadow integration.
fn bench_shadow_integration(c: &mut Criterion) {
    use epistemics::shadow_integration::ShadowIntegrationEngine;

    c.bench_function("shadow_record_suppression", |b| {
        b.iter_batched(
            || ShadowIntegrationEngine::new(),
            |mut engine| {
                black_box(engine.record_suppression(
                    "content-bench",
                    "low quality",
                    0.8,
                    0.2,
                    false,
                ))
            },
            criterion::BatchSize::SmallInput,
        )
    });

    c.bench_function("shadow_run_phase", |b| {
        b.iter_batched(
            || {
                let mut engine = ShadowIntegrationEngine::new();
                for i in 0..50 {
                    engine.record_suppression(
                        &format!("content-{}", i),
                        "suppressed",
                        0.8,
                        0.2,
                        false,
                    );
                }
                engine
            },
            |mut engine| {
                use living_core::ShadowConfig;
                black_box(engine.run_shadow_phase(0.3, &ShadowConfig::default()))
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    bench_engine_creation,
    bench_engine_start,
    bench_tick,
    bench_phase_transition,
    bench_full_cycle,
    bench_multiple_cycles,
    bench_wound_healing,
    bench_composting,
    bench_kenosis,
    bench_entanglement,
    bench_beauty_scoring,
    bench_negative_capability,
    bench_shadow_integration,
);

criterion_main!(benches);
