use chrono::Utc;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::db::Database;
use crate::switchbot::SwitchBotClient;

/// Start the hourly polling scheduler.
/// Polls every hour at minute 0 and stores readings in the database.
pub async fn start_poller(
    db: Arc<Database>,
    switchbot: SwitchBotClient,
    device_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Run retention cleanup on startup
    match db.cleanup_old_readings() {
        Ok(deleted) if deleted > 0 => {
            tracing::info!("cleaned up {} old readings on startup", deleted);
        }
        Err(e) => {
            tracing::warn!("failed to clean up old readings: {}", e);
        }
        _ => {}
    }

    // Also poll immediately on startup so we don't wait up to an hour
    {
        let db = Arc::clone(&db);
        let switchbot = switchbot.clone();
        let device_id = device_id.clone();
        tokio::spawn(async move {
            poll_once(&db, &switchbot, &device_id).await;
        });
    }

    let scheduler = JobScheduler::new().await?;

    let job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let db = Arc::clone(&db);
        let switchbot = switchbot.clone();
        let device_id = device_id.clone();
        Box::pin(async move {
            poll_once(&db, &switchbot, &device_id).await;
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    // Keep the scheduler alive by leaking it (it runs in the background).
    // This is intentional — the scheduler must live for the lifetime of the process.
    std::mem::forget(scheduler);

    Ok(())
}

async fn poll_once(db: &Database, switchbot: &SwitchBotClient, device_id: &str) {
    tracing::info!("polling SwitchBot device {}", device_id);

    match switchbot.get_device_status(device_id).await {
        Ok(status) => {
            let now = Utc::now();
            match db.insert_reading(device_id, status.temperature, status.humidity, now) {
                Ok(()) => {
                    tracing::info!(
                        "recorded: temp={:.1}°C humidity={}% at {}",
                        status.temperature,
                        status.humidity,
                        now.format("%Y-%m-%d %H:%M UTC")
                    );
                }
                Err(e) => {
                    tracing::error!("failed to store reading: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::warn!("failed to poll device {}: {}", device_id, e);
        }
    }
}
