use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Instant;

const SEQUENCE_BITS: u64 = 13;
const MAX_SEQUENCE: u64 = (1 << SEQUENCE_BITS) - 1;

#[derive(Debug, Clone, Copy)]
struct Config {
    count: usize,
    threads: usize,
    mode: Mode,
    storage: StorageKind,
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    Normal,
    AdversarialFixedTimestamp,
    AdversarialSharedGenerator,
    SharedGeneratorMultiThread,
}

#[derive(Debug, Clone, Copy)]
enum StorageKind {
    HashSet,
    SortVec,
}

fn parse_args() -> Config {
    let args: Vec<String> = std::env::args().collect();

    let mut count = 1_000_000usize;
    let mut threads = 1usize;
    let mut mode = Mode::Normal;
    let mut storage = StorageKind::HashSet;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--count" | "-c" => {
                i += 1;
                count = args[i].parse().expect("invalid count");
            }
            "--threads" | "-t" => {
                i += 1;
                threads = args[i].parse().expect("invalid threads");
            }
            "--adversarial-fixed" => {
                mode = Mode::AdversarialFixedTimestamp;
            }
            "--adversarial-shared" => {
                mode = Mode::AdversarialSharedGenerator;
            }
            "--shared-mt" => {
                mode = Mode::SharedGeneratorMultiThread;
            }
            "--sort-vec" => {
                storage = StorageKind::SortVec;
            }
            "--hash-set" => {
                storage = StorageKind::HashSet;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }

    if count > 50_000_000 && matches!(storage, StorageKind::HashSet) {
        eprintln!("warning: count > 50M with HashSet may exceed memory. switching to SortVec.");
        storage = StorageKind::SortVec;
    }

    Config {
        count,
        threads,
        mode,
        storage,
    }
}

fn print_help() {
    println!("ax-id collision probability test");
    println!();
    println!("usage: cargo run --example collision_test --release -- [options]");
    println!();
    println!("options:");
    println!("  -c, --count <n>        number of IDs to generate (default: 1_000_000)");
    println!("  -t, --threads <n>      thread count (default: 1)");
    println!("  --adversarial-fixed    force fixed timestamp for all IDs");
    println!("  --adversarial-shared   single thread with shared generator (safety test)");
    println!("  --shared-mt            multi-thread with ONE shared generator (throughput test)");
    println!("  --hash-set             use HashSet<u64> for dedup (default for <50M)");
    println!("  --sort-vec             use sorted Vec<u64> for dedup (default for >50M)");
    println!("  -h, --help             show this help");
    println!();
    println!("examples:");
    println!("  cargo run --example collision_test --release -- -c 10000000 -t 8");
    println!("  cargo run --example collision_test --release -- -c 1000000 --adversarial-fixed");
}

struct CollisionReport {
    total_generated: usize,
    total_unique: usize,
    total_collisions: usize,
    collision_percent: f64,
    thread_count: usize,
    elapsed_ms: u128,
    mode: Mode,
    sample_collisions: Vec<u64>,
    collision_frequency: HashMap<u64, usize>,
    theoretical: String,
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (count, ch) in s.chars().rev().enumerate() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

fn run_single_thread_hashset(config: &Config) -> CollisionReport {
    let start = Instant::now();
    let mut set = HashSet::with_capacity(config.count);
    let mut collision_map = HashMap::new();
    let mut collisions = Vec::new();
    let mut total = 0usize;

    let generator = match config.mode {
        Mode::AdversarialFixedTimestamp => ax_id::Generator::with_timestamp(1, 1000).unwrap(),
        _ => ax_id::Generator::new(1).unwrap(),
    };

    for _ in 0..config.count {
        let id = match config.mode {
            Mode::Normal => match generator.generate() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Generation error after {} IDs: {}", total, e);
                    break;
                }
            },
            _ => generator.generate_simple(),
        };
        let raw: u64 = id.into();
        std::hint::black_box(raw);

        if !set.insert(raw) {
            let freq = collision_map.entry(raw).or_insert(0usize);
            *freq += 1;
            if collisions.len() < 5 {
                collisions.push(raw);
            }
        }
        total += 1;
    }

    let elapsed = start.elapsed().as_millis();
    let unique = set.len();
    let collision_count = total - unique;

    let theoretical = match config.mode {
        Mode::AdversarialFixedTimestamp => {
            let space = MAX_SEQUENCE + 1;
            let overfill = total as f64 / space as f64;
            if total as u64 <= space {
                format!(
                    "deterministic: 0 collisions (sequential fill, {}% of {} space)",
                    overfill * 100.0,
                    space
                )
            } else {
                format!(
                    "deterministic: {} guaranteed collisions (sequential, {}x overfill of {} space)",
                    total - space as usize,
                    overfill,
                    space
                )
            }
        }
        Mode::AdversarialSharedGenerator => "impossible: atomic CAS monotonic sequence".to_string(),
        Mode::SharedGeneratorMultiThread => "impossible: atomic CAS monotonic sequence".to_string(),
        Mode::Normal => "impossible: monotonic sequence (timestamp + node + seq)".to_string(),
    };

    CollisionReport {
        total_generated: total,
        total_unique: unique,
        total_collisions: collision_count,
        collision_percent: if total > 0 {
            (collision_count as f64 / total as f64) * 100.0
        } else {
            0.0
        },
        thread_count: 1,
        elapsed_ms: elapsed,
        mode: config.mode,
        sample_collisions: collisions,
        collision_frequency: collision_map,
        theoretical,
    }
}

fn run_single_thread_sortvec(config: &Config) -> CollisionReport {
    let start = Instant::now();
    let mut ids = Vec::with_capacity(config.count);

    let generator = match config.mode {
        Mode::AdversarialFixedTimestamp => ax_id::Generator::with_timestamp(1, 1000).unwrap(),
        _ => ax_id::Generator::new(1).unwrap(),
    };

    for _ in 0..config.count {
        let id = match config.mode {
            Mode::Normal => match generator.generate() {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Generation error after {} IDs: {}", ids.len(), e);
                    break;
                }
            },
            _ => generator.generate_simple(),
        };
        ids.push(id.into());
    }

    ids.sort_unstable();

    let mut collision_map = HashMap::new();
    let mut collisions = Vec::new();
    let mut collision_count = 0usize;

    for window in ids.windows(2) {
        if window[0] == window[1] {
            collision_count += 1;
            let freq = collision_map.entry(window[0]).or_insert(0usize);
            *freq += 1;
            if collisions.len() < 5 && !collisions.contains(&window[0]) {
                collisions.push(window[0]);
            }
        }
    }

    let elapsed = start.elapsed().as_millis();
    let total = ids.len();
    let unique = total - collision_count;

    let theoretical = match config.mode {
        Mode::AdversarialFixedTimestamp => {
            let space = MAX_SEQUENCE + 1;
            let overfill = total as f64 / space as f64;
            if total as u64 <= space {
                format!(
                    "deterministic: 0 collisions (sequential fill, {}% of {} space)",
                    overfill * 100.0,
                    space
                )
            } else {
                format!(
                    "deterministic: {} guaranteed collisions (sequential, {}x overfill of {} space)",
                    total - space as usize,
                    overfill,
                    space
                )
            }
        }
        Mode::AdversarialSharedGenerator => "impossible: atomic CAS monotonic sequence".to_string(),
        Mode::SharedGeneratorMultiThread => "impossible: atomic CAS monotonic sequence".to_string(),
        Mode::Normal => "impossible: monotonic sequence (timestamp + node + seq)".to_string(),
    };

    CollisionReport {
        total_generated: total,
        total_unique: unique,
        total_collisions: collision_count,
        collision_percent: if total > 0 {
            (collision_count as f64 / total as f64) * 100.0
        } else {
            0.0
        },
        thread_count: 1,
        elapsed_ms: elapsed,
        mode: config.mode,
        sample_collisions: collisions,
        collision_frequency: collision_map,
        theoretical,
    }
}

fn run_multi_thread(config: &Config) -> CollisionReport {
    let start = Instant::now();
    let total_per_thread = config.count / config.threads;
    let remainder = config.count % config.threads;

    let global_counter = Arc::new(AtomicU64::new(0));

    let handles: Vec<_> = (0..config.threads)
        .map(|i| {
            let count = total_per_thread + if i < remainder { 1 } else { 0 };
            let mode = config.mode;
            let counter = Arc::clone(&global_counter);

            thread::spawn(move || {
                let generator = match mode {
                    Mode::AdversarialFixedTimestamp => {
                        ax_id::Generator::with_timestamp(i as u16 + 1, 1000).unwrap()
                    }
                    Mode::AdversarialSharedGenerator => {
                        return Vec::new();
                    }
                    Mode::SharedGeneratorMultiThread => {
                        return Vec::new(); // handled after thread pool
                    }
                    _ => ax_id::Generator::new(i as u16 + 1).unwrap(),
                };

                let mut local = Vec::with_capacity(count);
                for _ in 0..count {
                    let id: u64 = match mode {
                        Mode::Normal => match generator.generate() {
                            Ok(id) => id.into(),
                            Err(e) => {
                                eprintln!("Thread {} error after {} IDs: {}", i, local.len(), e);
                                break;
                            }
                        },
                        _ => generator.generate_simple().into(),
                    };
                    std::hint::black_box(id);
                    local.push(id);
                }
                counter.fetch_add(local.len() as u64, Ordering::Relaxed);
                local
            })
        })
        .collect();

    let mut all_ids: Vec<u64> = Vec::with_capacity(config.count);
    for h in handles {
        let mut local = h.join().unwrap();
        all_ids.append(&mut local);
    }

    if matches!(config.mode, Mode::AdversarialSharedGenerator) {
        let shared = Arc::new(ax_id::Generator::new(1).unwrap());
        let count = config.count;
        let h = thread::spawn(move || {
            let mut local = Vec::with_capacity(count);
            for _ in 0..count {
                let id: u64 = match shared.generate() {
                    Ok(id) => id.into(),
                    Err(e) => {
                        eprintln!("SharedGenerator error after {} IDs: {}", local.len(), e);
                        break;
                    }
                };
                std::hint::black_box(id);
                local.push(id);
            }
            local
        });
        let mut local = h.join().unwrap();
        all_ids.append(&mut local);
    }

    if matches!(config.mode, Mode::SharedGeneratorMultiThread) {
        let shared = Arc::new(ax_id::Generator::new(1).unwrap());
        let total_per_thread = config.count / config.threads;
        let remainder = config.count % config.threads;

        let handles: Vec<_> = (0..config.threads)
            .map(|i| {
                let count = total_per_thread + if i < remainder { 1 } else { 0 };
                let arc_gen = Arc::clone(&shared);
                thread::spawn(move || {
                    let mut local = Vec::with_capacity(count);
                    for _ in 0..count {
                        let id: u64 = match arc_gen.generate() {
                            Ok(id) => id.into(),
                            Err(e) => {
                                eprintln!("Thread {} error after {} IDs: {}", i, local.len(), e);
                                break;
                            }
                        };
                        std::hint::black_box(id);
                        local.push(id);
                    }
                    local
                })
            })
            .collect();

        for h in handles {
            let mut local = h.join().unwrap();
            all_ids.append(&mut local);
        }
    }

    all_ids.sort_unstable();

    let mut collision_map = HashMap::new();
    let mut collisions = Vec::new();
    let mut collision_count = 0usize;

    for window in all_ids.windows(2) {
        if window[0] == window[1] {
            collision_count += 1;
            let freq = collision_map.entry(window[0]).or_insert(0usize);
            *freq += 1;
            if collisions.len() < 5 && !collisions.contains(&window[0]) {
                collisions.push(window[0]);
            }
        }
    }

    let elapsed = start.elapsed().as_millis();
    let total = all_ids.len();
    let unique = total - collision_count;

    let theoretical = match config.mode {
        Mode::AdversarialFixedTimestamp => {
            let space = MAX_SEQUENCE + 1;
            let overfill = total as f64 / space as f64;
            if total as u64 <= space {
                format!(
                    "deterministic: 0 collisions (sequential fill, {}% of {} space)",
                    overfill * 100.0,
                    space
                )
            } else {
                format!(
                    "deterministic: {} guaranteed collisions (sequential, {}x overfill of {} space)",
                    total - space as usize,
                    overfill,
                    space
                )
            }
        }
        Mode::AdversarialSharedGenerator => "impossible: atomic CAS monotonic sequence".to_string(),
        Mode::SharedGeneratorMultiThread => "impossible: atomic CAS monotonic sequence".to_string(),
        Mode::Normal => "impossible: monotonic sequence (timestamp + node + seq)".to_string(),
    };

    CollisionReport {
        total_generated: total,
        total_unique: unique,
        total_collisions: collision_count,
        collision_percent: if total > 0 {
            (collision_count as f64 / total as f64) * 100.0
        } else {
            0.0
        },
        thread_count: config.threads,
        elapsed_ms: elapsed,
        mode: config.mode,
        sample_collisions: collisions,
        collision_frequency: collision_map,
        theoretical,
    }
}

fn print_report(report: &CollisionReport) {
    println!("{}", "=".repeat(70));
    println!("  ax-id collision probability test report");
    println!("{}", "=".repeat(70));
    println!();
    println!("  mode               : {:?}", report.mode);
    println!("  threads            : {}", report.thread_count);
    println!(
        "  total generated    : {}",
        format_number(report.total_generated)
    );
    println!(
        "  total unique       : {}",
        format_number(report.total_unique)
    );
    println!(
        "  total collisions   : {}",
        format_number(report.total_collisions)
    );
    println!("  collision rate     : {:.6}%", report.collision_percent);
    println!("  collision model    : {}", report.theoretical);
    println!("  elapsed time       : {} ms", report.elapsed_ms);
    println!(
        "  throughput         : {:.2} IDs/sec",
        (report.total_generated as f64) / (report.elapsed_ms as f64 / 1000.0)
    );
    println!();

    if !report.sample_collisions.is_empty() {
        println!("  sample collisions:");
        for (i, id) in report.sample_collisions.iter().enumerate() {
            let freq = report.collision_frequency.get(id).unwrap_or(&0);
            println!(
                "    #{}: id={:020} (collided {} times)",
                i + 1,
                id,
                freq + 1
            );
        }
        println!();
    }

    if report.total_collisions > 0 && report.collision_frequency.len() > 5 {
        println!(
            "  distinct collided IDs: {}",
            report.collision_frequency.len()
        );
    }

    println!("{}", "=".repeat(70));
}

fn main() {
    let config = parse_args();

    println!();
    println!("ax-id collision test");
    println!("  count    : {}", format_number(config.count));
    println!("  threads  : {}", config.threads);
    println!("  mode     : {:?}", config.mode);
    println!("  storage  : {:?}", config.storage);
    println!();

    let report = if config.threads == 1 {
        match config.storage {
            StorageKind::HashSet => run_single_thread_hashset(&config),
            StorageKind::SortVec => run_single_thread_sortvec(&config),
        }
    } else {
        run_multi_thread(&config)
    };

    print_report(&report);

    if matches!(config.mode, Mode::Normal | Mode::SharedGeneratorMultiThread)
        && report.total_collisions > 0
    {
        eprintln!("error: collision detected! this is a bug.");
        std::process::exit(1);
    }
}
