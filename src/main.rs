use clap::Parser;
use jsonrpsee::{http_client::HttpClientBuilder, proc_macros::rpc};
use serde::de::IgnoredAny;
use std::time::Duration;
use tracing::*;
use url::Url;
use user_idle::UserIdle;

#[rpc(client)]
#[async_trait]
pub trait XmrigApi {
    #[method(name = "pause")]
    async fn pause(&self) -> RpcResult<IgnoredAny>;
    #[method(name = "resume")]
    async fn resume(&self) -> RpcResult<IgnoredAny>;
    #[method(name = "stop")]
    async fn stop(&self) -> RpcResult<IgnoredAny>;
}

#[derive(Parser)]
struct Opt {
    /// XMRig API address
    #[clap(short, long)]
    address: Url,
    /// XMRig API token
    #[clap(short, long)]
    token: String,
    /// Poll interval in milliseconds
    #[clap(short, long, default_value_t = 500)]
    poll_interval: u64,
    /// Idle threshold in seconds
    #[clap(short, long, default_value_t = 5)]
    idle_threshold: u64,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let Opt {
        address,
        token,
        poll_interval,
        idle_threshold,
    } = Opt::parse();

    let address = address
        .join("json_rpc")
        .expect("failed to construct jsonrpc url");
    let poll_interval = Duration::from_millis(poll_interval);
    let idle_threshold = Duration::from_secs(idle_threshold);

    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let client = HttpClientBuilder::default()
        .set_middleware(
            tower::ServiceBuilder::new()
                .layer(tower_http::auth::AddAuthorizationLayer::bearer(&token)),
        )
        .build(address)
        .expect("failed to build jsonrpc client");

    let mut running: Option<bool> = None;

    loop {
        if let Err::<_, anyhow::Error>(e) = async {
            let idle_time = UserIdle::get_time()
                .map_err(|e| anyhow::anyhow!("failed to get idle time: {e}"))?
                .duration();

            trace!("Idle: {idle_time:?}");

            let sleep_duration = if let Some(remaining_idle) = idle_threshold.checked_sub(idle_time)
            {
                if running.unwrap_or(true) {
                    debug!("Stopping miner");
                    client.pause().await?;
                    running = Some(false);
                }

                remaining_idle
            } else {
                if !(running.unwrap_or(false)) {
                    debug!("Starting miner");
                    client.resume().await?;
                    running = Some(true);
                }

                poll_interval
            };

            tokio::time::sleep(sleep_duration).await;

            Ok(())
        }
        .await
        {
            warn!("{:?}", e);
        }
    }
}
