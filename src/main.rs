use clap::{Parser, ValueEnum};
use std::time::Instant;

#[derive(Parser)]
#[command(name = "ax-id")]
#[command(about = "Ultra-fast unique ID generator CLI")]
#[command(version)]
struct Cli {
    // Number of IDs to generate
    #[arg(short, long, default_value_t = 1)]
    count: usize,

    // Node ID (0-1023, auto-detected if omitted)
    #[arg(short, long)]
    node: Option<u16>,

    // Generation mode
    #[arg(short, long, value_enum, default_value_t = Mode::Distributed)]
    mode: Mode,

    // Output format
    #[arg(short, long, value_enum, default_value_t = Format::Hex)]
    format: Format,

    // Bulk mode: IDs separated by space (no newlines)
    #[arg(short, long)]
    bulk: bool,

    // Benchmark mode: generate IDs and report throughput
    #[arg(long)]
    benchmark: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Mode {
    Simple,
    Monotonic,
    Distributed,
    Timestamp,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Format {
    Raw,
    Hex,
    Json,
    Inspect,
}

fn main() {
    let cli = Cli::parse();

    let generator = match cli.node {
        Some(node_id) => ax_id::Generator::new(node_id).expect("invalid node_id"),
        None => ax_id::Generator::new_auto(),
    };

    if cli.benchmark {
        run_benchmark(&generator, cli.count, cli.mode);
        return;
    }

    if cli.count == 0 {
        return;
    }

    let ids: Vec<u64> = match cli.mode {
        Mode::Simple => (0..cli.count)
            .map(|_| generator.generate_simple().0)
            .collect(),
        Mode::Monotonic | Mode::Distributed => (0..cli.count)
            .map(|_| generator.generate().unwrap().0)
            .collect(),
        Mode::Timestamp => {
            let g = ax_id::Generator::with_timestamp(generator.node_id(), 1000)
                .expect("invalid timestamp mode");
            (0..cli.count).map(|_| g.generate_simple().0).collect()
        }
    };

    match cli.format {
        Format::Raw => {
            if cli.bulk {
                let s: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
                println!("{}", s.join(" "));
            } else {
                for id in ids {
                    println!("{}", id);
                }
            }
        }
        Format::Hex => {
            if cli.bulk {
                let s: Vec<String> = ids.iter().map(|id| format!("{:016x}", id)).collect();
                println!("{}", s.join(" "));
            } else {
                for id in ids {
                    println!("{:016x}", id);
                }
            }
        }
        Format::Json => {
            println!(
                "[{}]",
                ids.iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        Format::Inspect => {
            let id = ax_id::Id::from(ids[0]);
            println!("timestamp_ms: {}", id.timestamp_ms());
            println!("node_id: {}", id.node_id());
            println!("sequence: {}", id.raw_sequence());
            println!("raw: {}", ids[0]);
            println!("hex: {:016x}", ids[0]);
        }
    }
}

fn run_benchmark(generator: &ax_id::Generator, count: usize, mode: Mode) {
    let start = Instant::now();

    match mode {
        Mode::Simple => {
            for _ in 0..count {
                std::hint::black_box(generator.generate_simple());
            }
        }
        Mode::Monotonic | Mode::Distributed => {
            for _ in 0..count {
                std::hint::black_box(generator.generate().unwrap());
            }
        }
        Mode::Timestamp => {
            let g = ax_id::Generator::with_timestamp(generator.node_id(), 1000).unwrap();
            for _ in 0..count {
                std::hint::black_box(g.generate_simple());
            }
        }
    }

    let elapsed = start.elapsed();
    let ns_per_id = elapsed.as_nanos() as f64 / count as f64;
    let ids_per_sec = count as f64 / elapsed.as_secs_f64();

    eprintln!("Benchmark: {} IDs (mode: {:?})", count, mode);
    eprintln!("  elapsed: {:?}", elapsed);
    eprintln!("  throughput: {:.2} IDs/sec", ids_per_sec);
    eprintln!("  latency: {:.2} ns/ID", ns_per_id);
}
