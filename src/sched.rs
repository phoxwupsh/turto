use crate::models::config::YtdlpConfig;
use std::{pin::Pin, sync::Arc};
use tokio_cron_scheduler::JobScheduler;
use uuid::Uuid;

/// Build the periodic yt-dlp update job. It upgrades the sidecar's yt-dlp and,
/// if the version actually changed, recycles the sidecar blue/green (see
/// [`crate::ytdl::sidecar::update`]). The scheduler owns the cadence.
pub fn auto_update_ytdlp(
    config: Arc<YtdlpConfig>,
) -> impl FnMut(Uuid, JobScheduler) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync {
    let nightly = config.use_nightly;
    move |_uuid, _job_scheduler| {
        Box::pin(async move {
            match crate::ytdl::sidecar::update(nightly).await {
                Ok(true) => tracing::info!("yt-dlp updated; sidecar recycled"),
                Ok(false) => tracing::debug!("yt-dlp already up to date"),
                Err(err) => tracing::error!(error = ?err, "failed to update yt-dlp"),
            }
        })
    }
}
