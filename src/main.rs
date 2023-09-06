use clap::Parser;
use jsonrpsee::{http_client::HttpClientBuilder, proc_macros::rpc};
use log::*;
use std::time::Duration;
use tokio::time::sleep;
use zbus::{Connection, Proxy};

/// Poll GNOME Mutter idle time and pause/resume a JSON-RPC service.
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Base URL of the XMRig's JSON-RPC server
    base_url: String,

    /// Bearer token for Authorization header
    bearer: String,

    /// Idle threshold in milliseconds
    threshold_ms: u64,

    /// Polling interval in milliseconds
    #[arg(long, default_value_t = 250)]
    interval_ms: u64,
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

    let rpc_url = format!("{}/json_rpc", args.base_url);

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
    let mut time_to_sleep = args.interval_ms;

    loop {
        if let Err::<(), anyhow::Error>(e) = async {
            // By default, keep checking at regular intervals when idle to preserve interactivity.
            time_to_sleep = args.interval_ms;

            let idle_ms = proxy.call("GetIdletime", &()).await?;

            if let Some(t) = args.threshold_ms.checked_sub(idle_ms) {
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

            info!(
                "{} - will check again in {}ms",
                if running { "RUNNING" } else { "IDLE" },
                time_to_sleep
            );

            Ok(())
        }
        .await
        {
            error!("{e:?}");
        }

        sleep(Duration::from_millis(time_to_sleep)).await;
    }
}
