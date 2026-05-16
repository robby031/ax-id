use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::{
    sync::{Arc, Barrier},
    thread,
};

const OPS: usize = 1_000_000;

fn pin_thread(core_id: usize) {
    let cores = core_affinity::get_core_ids().unwrap_or_default();

    if let Some(id) = cores.get(core_id % cores.len()) {
        core_affinity::set_for_current(*id);
    }
}

#[inline(never)]
fn consume<T>(value: T) {
    black_box(value);
}

fn bench_single_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_thread");

    group.sample_size(100);
    group.throughput(Throughput::Elements(OPS as u64));

    // AX-ID SIMPLE
    group.bench_function("ax-id/simple", |b| {
        let generator = ax_id::Generator::new(1).unwrap();

        b.iter(|| {
            for _ in 0..OPS {
                consume(generator.generate_simple());
            }
        });
    });

    // AX-ID SAFE
    group.bench_function("ax-id/safe", |b| {
        let generator = ax_id::Generator::new(1).unwrap();

        b.iter(|| {
            for _ in 0..OPS {
                consume(generator.generate().unwrap());
            }
        });
    });

    // UUID V4
    group.bench_function("uuid_v4", |b| {
        b.iter(|| {
            for _ in 0..OPS {
                consume(uuid::Uuid::new_v4());
            }
        });
    });

    // UUID V7
    group.bench_function("uuid_v7", |b| {
        b.iter(|| {
            for _ in 0..OPS {
                consume(uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)));
            }
        });
    });

    // ULID
    group.bench_function("ulid", |b| {
        b.iter(|| {
            for _ in 0..OPS {
                consume(ulid::Ulid::new());
            }
        });
    });

    // SNOWFLAKE
    group.bench_function("snowflake", |b| {
        let mut generator = snowflake::SnowflakeIdGenerator::new(1, 1);

        b.iter(|| {
            for _ in 0..OPS {
                consume(generator.generate());
            }
        });
    });

    group.finish();
}

fn bench_multi_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_thread");

    group.sample_size(30);

    for threads in [1usize, 2, 4, 8] {
        group.throughput(Throughput::Elements(OPS as u64));

        // AX-ID SIMPLE
        group.bench_with_input(
            BenchmarkId::new("ax-id/simple", threads),
            &threads,
            |b, &n| {
                let barrier = Arc::new(Barrier::new(n + 1));

                let handles: Vec<_> = (0..n)
                    .map(|i| {
                        let barrier = barrier.clone();

                        thread::spawn(move || {
                            pin_thread(i);

                            let generator = ax_id::Generator::new(i as u16 + 1).unwrap();

                            loop {
                                barrier.wait();

                                for _ in 0..(OPS / n) {
                                    consume(generator.generate_simple());
                                }

                                barrier.wait();
                            }
                        })
                    })
                    .collect();

                b.iter(|| {
                    barrier.wait();
                    barrier.wait();
                });

                drop(handles);
            },
        );

        // AX-ID SAFE
        group.bench_with_input(
            BenchmarkId::new("ax-id/safe", threads),
            &threads,
            |b, &n| {
                let barrier = Arc::new(Barrier::new(n + 1));

                let handles: Vec<_> = (0..n)
                    .map(|i| {
                        let barrier = barrier.clone();

                        thread::spawn(move || {
                            pin_thread(i);

                            let generator = ax_id::Generator::new(i as u16 + 1).unwrap();

                            loop {
                                barrier.wait();

                                for _ in 0..(OPS / n) {
                                    consume(generator.generate().unwrap());
                                }

                                barrier.wait();
                            }
                        })
                    })
                    .collect();

                b.iter(|| {
                    barrier.wait();
                    barrier.wait();
                });

                drop(handles);
            },
        );

        // UUID V4
        group.bench_with_input(BenchmarkId::new("uuid_v4", threads), &threads, |b, &n| {
            let barrier = Arc::new(Barrier::new(n + 1));

            let handles: Vec<_> = (0..n)
                .map(|i| {
                    let barrier = barrier.clone();

                    thread::spawn(move || {
                        pin_thread(i);

                        loop {
                            barrier.wait();

                            for _ in 0..(OPS / n) {
                                consume(uuid::Uuid::new_v4());
                            }

                            barrier.wait();
                        }
                    })
                })
                .collect();

            b.iter(|| {
                barrier.wait();
                barrier.wait();
            });

            drop(handles);
        });

        // UUID V7
        group.bench_with_input(BenchmarkId::new("uuid_v7", threads), &threads, |b, &n| {
            let barrier = Arc::new(Barrier::new(n + 1));

            let handles: Vec<_> = (0..n)
                .map(|i| {
                    let barrier = barrier.clone();

                    thread::spawn(move || {
                        pin_thread(i);

                        loop {
                            barrier.wait();

                            for _ in 0..(OPS / n) {
                                consume(uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)));
                            }

                            barrier.wait();
                        }
                    })
                })
                .collect();

            b.iter(|| {
                barrier.wait();
                barrier.wait();
            });

            drop(handles);
        });
    }

    group.finish();
}

criterion_group!(benches, bench_single_thread, bench_multi_thread);

criterion_main!(benches);
