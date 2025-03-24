use std::future::ready;

use deadpool::managed::{Metrics, RecycleResult};
use tokio::task::JoinHandle;

use crate::BenchmarkConfig;

struct Manager {}

impl ::deadpool::managed::Manager for Manager {
    type Type = ();
    type Error = ();

    fn create(&self) -> impl Future<Output = Result<Self::Type, Self::Error>> + Send {
        ready(Ok(()))
    }

    fn recycle(
        &self,
        _: &mut Self::Type,
        _metrics: &Metrics,
    ) -> impl Future<Output = RecycleResult<Self::Error>> + Send {
        ready(Ok(()))
    }
}

type Pool = ::deadpool::managed::Pool<Manager>;

pub async fn run(cfg: BenchmarkConfig) -> Vec<JoinHandle<()>> {
    let pool = Pool::builder(Manager {})
        .max_size(cfg.pool_size)
        .build()
        .unwrap();
    (0..cfg.workers)
        .map(|_| {
            let pool = pool.clone();
            tokio::spawn(async move {
                for _ in 0..cfg.operations_per_worker() {
                    let _ = pool.get().await;
                }
            })
        })
        .collect()
}
