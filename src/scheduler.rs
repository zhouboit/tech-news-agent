use std::sync::Arc;

use crate::agent::TechNewsAgent;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

pub async fn start_scheduler(
    agent: Arc<TechNewsAgent>,
    interval_minutes: u64,
) -> Result<(), String> {
    let cron_expr = format!("0 */{} * * * *", interval_minutes);
    info!("Scheduler: cron expression = `{}`", cron_expr);

    let mut scheduler = JobScheduler::new()
        .await
        .map_err(|e| format!("create scheduler: {e}"))?;

    scheduler
        .add(
            Job::new_async(cron_expr.as_str(), move |_uuid, _lock| {
                let agent = Arc::clone(&agent);
                Box::pin(async move {
                    agent.run_once().await;
                })
            })
            .map_err(|e| format!("create cron job: {e}"))?,
        )
        .await
        .map_err(|e| format!("add cron job: {e}"))?;

    scheduler.start().await.map_err(|e| format!("start scheduler: {e}"))?;

    info!("Scheduler started, interval: {} minutes", interval_minutes);
    tokio::signal::ctrl_c().await.map_err(|e| format!("ctrl_c: {e}"))?;
    scheduler.shutdown().await.map_err(|e| format!("scheduler shutdown: {e}"))?;

    Ok(())
}
