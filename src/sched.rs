use crate::{
    deps::ytdlp::{get_local_ytdlp, set_ytdlp_path, update_to, version::YtdlpVersion},
    models::config::YtdlpConfig,
};
use std::{path::Path, pin::Pin, sync::Arc};
use tokio_cron_scheduler::JobScheduler;
use tracing::instrument;
use uuid::Uuid;

pub fn auto_update_ytdlp(
    ytdlp_dir: impl AsRef<Path>,
    config: Arc<YtdlpConfig>,
) -> impl FnMut(Uuid, JobScheduler) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync {
    let ytdlp_dir = ytdlp_dir.as_ref().to_path_buf();
    let use_nightly = config.use_nightly;
    move |_uuid, _job_scheduler| {
        let ytdlp_dir = ytdlp_dir.clone();
        Box::pin(async move {
            let res = auto_update_ytdlp_inner(&ytdlp_dir, use_nightly).await;
            if let Err(err) = res {
                tracing::error!(error = ?err, "failed to update yt-dlp");
            }
        })
    }
}

#[instrument(name = "auto_update")]
async fn auto_update_ytdlp_inner(ytdlp_dir: &Path, use_nightly: bool) -> anyhow::Result<()> {
    tracing::info!("auto updating yt-dlp");
    let latest_ver = YtdlpVersion::fetch_lastest(use_nightly).await?;
    tracing::info!(version = latest_ver.tag(), "found latest yt-dlp version");
    let local = get_local_ytdlp(ytdlp_dir).await?;
    if let Some((local_ver, _local_path)) = local
        && latest_ver <= local_ver
    {
        return Ok(());
    }
    tracing::info!(version = latest_ver.tag(), "updating to latest");
    let new_path = update_to(&ytdlp_dir, &latest_ver).await?;
    set_ytdlp_path(new_path);
    Ok(())
}
