use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

const WARMUP_MS: u64 = 500;
const BENCH_MS: u64 = 2000;
const LATENCY_SAMPLES: usize = 100_000;

#[derive(Debug, Clone, Copy)]
struct Metrics {
    name: &'static str,
    single_thread_mps: f64,
    multi_thread_mps: f64,
    size_bytes: usize,
    serialize_ns: f64,
    p50_ns: f64,
    p99_ns: f64,
    p999_ns: f64,
}

fn fmt_mps(mps: f64) -> String {
    if mps >= 1000.0 {
        format!("{:.1}B", mps / 1000.0)
    } else if mps >= 1.0 {
        format!("{:.1}M", mps)
    } else {
        format!("{:.0}K", mps * 1000.0)
    }
}

fn fmt_ns(ns: f64) -> String {
    if ns < 1000.0 {
        format!("{:.0}ns", ns)
    } else {
        format!("{:.1}µs", ns / 1000.0)
    }
}

fn measure_latency<F: FnMut()>(mut f: F) -> (f64, f64, f64) {
    let mut samples = Vec::with_capacity(LATENCY_SAMPLES);
    for _ in 0..LATENCY_SAMPLES {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed().as_nanos() as f64;
        samples.push(elapsed);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = samples[LATENCY_SAMPLES / 2];
    let p99 = samples[LATENCY_SAMPLES * 99 / 100];
    let p999 = samples[LATENCY_SAMPLES * 999 / 1000];
    (p50, p99, p999)
}

fn bench_single_thread<F: FnMut()>(_name: &str, mut f: F) -> f64 {
    // Warmup
    let warmup_end = Instant::now() + Duration::from_millis(WARMUP_MS);
    while Instant::now() < warmup_end {
        f();
    }

    let bench_end = Instant::now() + Duration::from_millis(BENCH_MS);
    let mut count = 0u64;
    let start = Instant::now();
    while Instant::now() < bench_end {
        for _ in 0..1000 {
            f();
        }
        count += 1000;
    }
    let elapsed = start.elapsed().as_secs_f64();
    count as f64 / elapsed / 1_000_000.0
}

fn bench_multi_thread<F: Fn() + Send + Sync + 'static>(f: F) -> f64 {
    let threads = 8usize;
    let f = Arc::new(f);
    let barrier = Arc::new(Barrier::new(threads + 1));

    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let f = f.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                // Warmup
                let warmup_end = Instant::now() + Duration::from_millis(WARMUP_MS);
                while Instant::now() < warmup_end {
                    f();
                }

                barrier.wait();

                let bench_end = Instant::now() + Duration::from_millis(BENCH_MS);
                let mut count = 0u64;
                while Instant::now() < bench_end {
                    for _ in 0..1000 {
                        f();
                    }
                    count += 1000;
                }

                barrier.wait();
                count
            })
        })
        .collect();

    barrier.wait();
    let start = Instant::now();
    barrier.wait();
    let elapsed = start.elapsed().as_secs_f64();

    let total: u64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    total as f64 / elapsed / 1_000_000.0
}

fn bench_serialize<F: FnMut() -> String>(mut f: F) -> f64 {
    let start = Instant::now();
    for _ in 0..1_000_000 {
        std::hint::black_box(f());
    }
    start.elapsed().as_nanos() as f64 / 1_000_000.0
}

fn main() {
    println!("ax-id competitor benchmark");
    println!("warming up...\n");

    let mut results: Vec<Metrics> = Vec::new();

    // AX-ID SIMPLE
    {
        let generator = ax_id::Generator::new(1).unwrap();
        let st = bench_single_thread("ax-id/simple", || {
            std::hint::black_box(generator.generate_simple());
        });
        let generator2 = ax_id::Generator::new(1).unwrap();
        let mt = bench_multi_thread(move || {
            std::hint::black_box(generator2.generate_simple());
        });
        let (p50, p99, p999) = measure_latency(|| {
            std::hint::black_box(generator.generate_simple());
        });
        let ser = bench_serialize(|| generator.generate_simple().to_string());
        results.push(Metrics {
            name: "ax-id/simple",
            single_thread_mps: st,
            multi_thread_mps: mt,
            size_bytes: std::mem::size_of::<ax_id::Id>(),
            serialize_ns: ser,
            p50_ns: p50,
            p99_ns: p99,
            p999_ns: p999,
        });
    }

    // AX-ID SAFE
    {
        let generator = ax_id::Generator::new(1).unwrap();
        let st = bench_single_thread("ax-id/safe", || {
            std::hint::black_box(generator.generate().unwrap());
        });
        let generator2 = ax_id::Generator::new(1).unwrap();
        let mt = bench_multi_thread(move || {
            std::hint::black_box(generator2.generate().unwrap());
        });
        let (p50, p99, p999) = measure_latency(|| {
            std::hint::black_box(generator.generate().unwrap());
        });
        let ser = bench_serialize(|| generator.generate().unwrap().to_string());
        results.push(Metrics {
            name: "ax-id/safe",
            single_thread_mps: st,
            multi_thread_mps: mt,
            size_bytes: std::mem::size_of::<ax_id::Id>(),
            serialize_ns: ser,
            p50_ns: p50,
            p99_ns: p99,
            p999_ns: p999,
        });
    }

    // UUID v4
    {
        let st = bench_single_thread("uuid/v4", || {
            std::hint::black_box(uuid::Uuid::new_v4());
        });
        let mt = bench_multi_thread(|| {
            std::hint::black_box(uuid::Uuid::new_v4());
        });
        let (p50, p99, p999) = measure_latency(|| {
            std::hint::black_box(uuid::Uuid::new_v4());
        });
        let ser = bench_serialize(|| uuid::Uuid::new_v4().to_string());
        results.push(Metrics {
            name: "uuid/v4",
            single_thread_mps: st,
            multi_thread_mps: mt,
            size_bytes: std::mem::size_of::<uuid::Uuid>(),
            serialize_ns: ser,
            p50_ns: p50,
            p99_ns: p99,
            p999_ns: p999,
        });
    }

    // UUID v7
    {
        let ctx = uuid::NoContext;
        let st = bench_single_thread("uuid/v7", || {
            std::hint::black_box(uuid::Uuid::new_v7(uuid::Timestamp::now(ctx)));
        });
        let mt = bench_multi_thread(move || {
            std::hint::black_box(uuid::Uuid::new_v7(uuid::Timestamp::now(ctx)));
        });
        let (p50, p99, p999) = measure_latency(|| {
            std::hint::black_box(uuid::Uuid::new_v7(uuid::Timestamp::now(ctx)));
        });
        let ser = bench_serialize(|| uuid::Uuid::new_v7(uuid::Timestamp::now(ctx)).to_string());
        results.push(Metrics {
            name: "uuid/v7",
            single_thread_mps: st,
            multi_thread_mps: mt,
            size_bytes: std::mem::size_of::<uuid::Uuid>(),
            serialize_ns: ser,
            p50_ns: p50,
            p99_ns: p99,
            p999_ns: p999,
        });
    }

    // ULID
    {
        let st = bench_single_thread("ulid", || {
            std::hint::black_box(ulid::Ulid::new());
        });
        let mt = bench_multi_thread(|| {
            std::hint::black_box(ulid::Ulid::new());
        });
        let (p50, p99, p999) = measure_latency(|| {
            std::hint::black_box(ulid::Ulid::new());
        });
        let ser = bench_serialize(|| ulid::Ulid::new().to_string());
        results.push(Metrics {
            name: "ulid",
            single_thread_mps: st,
            multi_thread_mps: mt,
            size_bytes: std::mem::size_of::<ulid::Ulid>(),
            serialize_ns: ser,
            p50_ns: p50,
            p99_ns: p99,
            p999_ns: p999,
        });
    }

    // Snowflake
    {
        use snowflake::SnowflakeIdBucket;
        let mut bucket = SnowflakeIdBucket::new(1, 1);
        let st = bench_single_thread("snowflake", || {
            std::hint::black_box(bucket.get_id());
        });
        let bucket2 = std::sync::Mutex::new(SnowflakeIdBucket::new(1, 1));
        let mt = bench_multi_thread(move || {
            std::hint::black_box(bucket2.lock().unwrap().get_id());
        });
        let mut bucket3 = SnowflakeIdBucket::new(1, 1);
        let (p50, p99, p999) = measure_latency(|| {
            std::hint::black_box(bucket3.get_id());
        });
        let mut bucket4 = SnowflakeIdBucket::new(1, 1);
        let ser = bench_serialize(|| bucket4.get_id().to_string());
        results.push(Metrics {
            name: "snowflake",
            single_thread_mps: st,
            multi_thread_mps: mt,
            size_bytes: std::mem::size_of::<i64>(),
            serialize_ns: ser,
            p50_ns: p50,
            p99_ns: p99,
            p999_ns: p999,
        });
    }

    // Sonyflake
    {
        let sf = sonyflake::Sonyflake::new().unwrap();
        let st = bench_single_thread("sonyflake", || {
            std::hint::black_box(sf.next_id().unwrap());
        });
        let sf2 = std::sync::Mutex::new(sonyflake::Sonyflake::new().unwrap());
        let mt = bench_multi_thread(move || {
            std::hint::black_box(sf2.lock().unwrap().next_id().unwrap());
        });
        let sf3 = sonyflake::Sonyflake::new().unwrap();
        let (p50, p99, p999) = measure_latency(|| {
            std::hint::black_box(sf3.next_id().unwrap());
        });
        let sf4 = sonyflake::Sonyflake::new().unwrap();
        let ser = bench_serialize(|| sf4.next_id().unwrap().to_string());
        results.push(Metrics {
            name: "sonyflake",
            single_thread_mps: st,
            multi_thread_mps: mt,
            size_bytes: std::mem::size_of::<u64>(),
            serialize_ns: ser,
            p50_ns: p50,
            p99_ns: p99,
            p999_ns: p999,
        });
    }

    // KSUID
    {
        let st = bench_single_thread("ksuid", || {
            std::hint::black_box(ksuid::Ksuid::generate());
        });
        let mt = bench_multi_thread(|| {
            std::hint::black_box(ksuid::Ksuid::generate());
        });
        let (p50, p99, p999) = measure_latency(|| {
            std::hint::black_box(ksuid::Ksuid::generate());
        });
        let ser = bench_serialize(|| ksuid::Ksuid::generate().to_base62());
        results.push(Metrics {
            name: "ksuid",
            single_thread_mps: st,
            multi_thread_mps: mt,
            size_bytes: std::mem::size_of::<ksuid::Ksuid>(),
            serialize_ns: ser,
            p50_ns: p50,
            p99_ns: p99,
            p999_ns: p999,
        });
    }

    // FastID
    {
        let worker = fastid::FastIdWorker::new(1);
        let st = bench_single_thread("fastid", || {
            std::hint::black_box(worker.next_id());
        });
        let worker2 = std::sync::Mutex::new(fastid::FastIdWorker::new(1));
        let mt = bench_multi_thread(move || {
            std::hint::black_box(worker2.lock().unwrap().next_id());
        });
        let worker3 = fastid::FastIdWorker::new(1);
        let (p50, p99, p999) = measure_latency(|| {
            std::hint::black_box(worker3.next_id());
        });
        let worker4 = fastid::FastIdWorker::new(1);
        let ser = bench_serialize(|| worker4.next_id().to_string());
        results.push(Metrics {
            name: "fastid",
            single_thread_mps: st,
            multi_thread_mps: mt,
            size_bytes: std::mem::size_of::<u64>(),
            serialize_ns: ser,
            p50_ns: p50,
            p99_ns: p99,
            p999_ns: p999,
        });
    }

    println!("{:=<110}", "");
    println!(
        "  {:<16} {:>12} {:>12} {:>10} {:>12} {:>10} {:>10} {:>10}",
        "Generator", "1-Thread", "8-Threads", "Size", "Serialize", "P50", "P99", "P99.9"
    );
    println!("{:=<110}", "");
    for m in &results {
        println!(
            "  {:<16} {:>12} {:>12} {:>10} {:>12} {:>10} {:>10} {:>10}",
            m.name,
            fmt_mps(m.single_thread_mps),
            fmt_mps(m.multi_thread_mps),
            format!("{}B", m.size_bytes),
            fmt_ns(m.serialize_ns),
            fmt_ns(m.p50_ns),
            fmt_ns(m.p99_ns),
            fmt_ns(m.p999_ns),
        );
    }
    println!("{:=<110}", "");

    println!("\n  Notes:");
    println!("    - 1-Thread / 8-Threads: generation throughput (higher is better)");
    println!("    - Size: memory footprint per ID instance");
    println!("    - Serialize: time to convert ID to canonical string representation");
    println!("    - P50/P99/P99.9: generation latency percentiles (lower is better)");
    println!("    - Multi-thread safe variants use independent generators per thread");

    println!("\n  External service benchmarks (require setup):");
    println!(
        "    - Postgres insert:     cargo run --example bench_postgres --features bench-postgres"
    );
    println!(
        "    - RocksDB write amp:   cargo run --example bench_rocksdb --features bench-rocksdb"
    );
    println!("    - Kafka serialization:   cargo run --example bench_kafka --features bench-kafka");
}
