use clap::Parser;
use jsonrpsee::{http_client::HttpClientBuilder, proc_macros::rpc};
use log::*;
use std::{num::NonZeroU64, time::Duration};
use tokio::time::sleep;
use zbus::{Connection, Proxy};

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// URL of XMRig's RPC server
    #[arg(long)]
    url: String,

    /// Bearer token for Authorization header
    #[arg(long)]
    bearer: String,

    /// Idle threshold in milliseconds
    #[arg(long)]
    threshold_ms: NonZeroU64,

    /// Polling interval in milliseconds
    #[arg(long, default_value_t = NonZeroU64::new(250).unwrap())]
    interval_ms: NonZeroU64,
}

#[rpc(client)]
pub trait Rpc {
    #[method(name = "pause")]
    async fn pause(&self);

    #[method(name = "resume")]
    async fn resume(&self);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    let rpc_url = format!("{}/json_rpc", args.url);

    // ----- Build jsonrpsee HTTP client with Authorization: Bearer <token> -----
    let rpc_client = HttpClientBuilder::default()
        .set_http_middleware(tower::ServiceBuilder::new().layer(
            tower_http::auth::AddAuthorizationLayer::bearer(&args.bearer),
        ))
        .build(&rpc_url)?;

    // ----- Connect to D-Bus IdleMonitor via zbus -----
    let connection = Connection::session().await?;
    let proxy = Proxy::new(
        &connection,
        "org.gnome.Mutter.IdleMonitor",
        "/org/gnome/Mutter/IdleMonitor/Core",
        "org.gnome.Mutter.IdleMonitor",
    )
    .await?;

    let mut running = false;
    let mut last_printed_state = !running;
    let mut time_to_sleep = args.interval_ms.get();

    loop {
        if let Err::<(), anyhow::Error>(e) = async {
            // By default, keep checking at regular intervals when idle to preserve interactivity.
            time_to_sleep = args.interval_ms.get();

            let idle_ms = proxy.call("GetIdletime", &()).await?;

            if let Some(t) = args.threshold_ms.get().checked_sub(idle_ms) {
                // User active (idle <= threshold): send pause, sleep and check again at threshold.
                time_to_sleep = t;

                RpcClient::pause(&rpc_client).await?;
                running = false;
            } else {
                // If we were paused, send resume.
                if !running {
                    RpcClient::resume(&rpc_client).await?;
                    running = true;
                }
            };

            if running ^ last_printed_state {
                // Print state change only.
                info!(
                    "State changed: {}",
                    if running { "RUNNING" } else { "PAUSED" }
                );
                debug!("Will check again in {time_to_sleep}ms");
                last_printed_state = running;
            } else {
                // Print periodic status.
                debug!(
                    "State: {}, will check again in {}ms",
                    if running { "RUNNING" } else { "PAUSED" },
                    time_to_sleep
                );
            }

            Ok(())
        }
        .await
        {
            error!("{e:?}");
        }

        sleep(Duration::from_millis(time_to_sleep)).await;
    }
}
