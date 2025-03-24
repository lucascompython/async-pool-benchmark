mod async_object_pool;
mod bb8;
mod deadpool;
mod mobc;

const ITERATIONS: usize = 15;
const TIMEOUT: Duration = Duration::from_secs(5);
const OPERATIONS: usize = 1048576;
const CONFIGS: &[BenchmarkConfig] = &[
    BenchmarkConfig {
        pool_size: 2,
        workers: 8,
    },
    BenchmarkConfig {
        pool_size: 4,
        workers: 8,
    },
    BenchmarkConfig {
        pool_size: 8,
        workers: 8,
    },
    BenchmarkConfig {
        pool_size: 4,
        workers: 16,
    },
    BenchmarkConfig {
        pool_size: 8,
        workers: 16,
    },
    BenchmarkConfig {
        pool_size: 16,
        workers: 16,
    },
    BenchmarkConfig {
        pool_size: 8,
        workers: 32,
    },
    BenchmarkConfig {
        pool_size: 16,
        workers: 32,
    },
    BenchmarkConfig {
        pool_size: 32,
        workers: 32,
    },
];

use std::{
    fs::File,
    future::Future,
    time::{Duration, Instant},
};

use abort_on_drop::ChildTask;
use serde::Serialize;
use tokio::task::JoinHandle;

#[derive(Default, Copy, Clone, Serialize)]
pub struct BenchmarkConfig {
    pub pool_size: usize,
    pub workers: usize,
}

impl BenchmarkConfig {
    fn operations_per_worker(&self) -> usize {
        OPERATIONS / self.workers
    }
}

#[derive(Serialize)]
pub struct Benchmark {
    pub config: BenchmarkConfig,
    pub results: Vec<BenchmarkResult>,
}

impl Benchmark {
    pub async fn run<F: Future<Output = Vec<JoinHandle<()>>>>(
        &mut self,
        name: &'static str,
        version: &'static str,
        f: fn(BenchmarkConfig) -> F,
    ) {
        println!();
        println!("{} {}:", name, version,);
        let mut ops_vec = Vec::new();
        for _ in 0..ITERATIONS {
            let start = Instant::now();
            let handles = f(self.config)
                .await
                .into_iter()
                .map(ChildTask::from)
                .collect::<Vec<_>>();
            let success = tokio::time::timeout(TIMEOUT, async {
                for handle in handles {
                    handle.await.unwrap();
                }
            })
            .await
            .is_ok();
            let end = Instant::now();
            let duration = end.duration_since(start);
            let ops = OPERATIONS as f64 / duration.as_secs_f64();
            ops_vec.push(ops);
            if success {
                println!("- {} ms, {:.0} op/s", duration.as_millis(), ops);
            } else {
                println!("- Timeout");
                ops_vec.clear();
            }
        }
        self.results.push(BenchmarkResult {
            name,
            version,
            ops: if ops_vec.is_empty() {
                None
            } else {
                Some(ops_vec)
            },
        });
    }
}

#[derive(Serialize)]
pub struct BenchmarkResult {
    pub name: &'static str,
    pub version: &'static str,
    pub ops: Option<Vec<f64>>,
}

async fn run_benchmarks(cfg: &BenchmarkConfig) -> Benchmark {
    println!("-----------------------------");
    println!("pool_size: {}", cfg.pool_size);
    println!("workers: {}", cfg.workers);
    println!("-----------------------------");
    let mut benchmark = Benchmark {
        config: BenchmarkConfig {
            pool_size: cfg.pool_size,
            workers: cfg.workers,
        },
        results: Vec::new(),
    };
    benchmark.run("deadpool", "0.12.2", deadpool::run).await;
    benchmark.run("bb8", "0.9", bb8::run).await;
    benchmark.run("mobc", "0.8.5", mobc::run).await;
    benchmark
        .run("async-object-pool", "0.1.5", async_object_pool::run)
        .await;
    benchmark
}

#[tokio::main]
async fn main() {
    let mut benchmarks = Vec::new();
    for config in CONFIGS {
        benchmarks.push(run_benchmarks(config).await);
    }
    let file = File::create("results.json").unwrap();
    serde_json::to_writer_pretty(&file, &benchmarks).unwrap();
}
