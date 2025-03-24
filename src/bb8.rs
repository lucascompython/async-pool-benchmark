use std::future::ready;
use tokio::task::JoinHandle;

use crate::BenchmarkConfig;

struct Manager {}

impl ::bb8::ManageConnection for Manager {
    type Connection = ();
    type Error = ();

    fn connect(&self) -> impl Future<Output = Result<Self::Connection, Self::Error>> + Send {
        ready(Ok(()))
    }

    fn is_valid(
        &self,
        _conn: &mut Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

type Pool = bb8::Pool<Manager>;

pub async fn run(cfg: BenchmarkConfig) -> Vec<JoinHandle<()>> {
    let pool = Pool::builder()
        .max_size(cfg.pool_size as u32)
        .build(Manager {})
        .await
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
