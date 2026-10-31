// ------------------------------------------------------------
// Module: main
// Purpose: Boot one table on this machine and serve it to
//          every browser that knows the passphrase.
// ------------------------------------------------------------

mod app;
mod ruin_scene;
mod forest_scene;
mod asset_import;
mod assets;
mod house;
mod config;
mod dice;
mod map;
mod net;
mod session;
mod storage;
mod systems;
mod vision;

use std::sync::Arc;

use crate::app::App;
use crate::config::Config;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::load();
    let bind = config.bind.clone();
    let passphrase = config.table_passphrase.clone();
    let gm_key = config.gm_key.clone();
    let app = App::boot(config);

    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!("could not listen on {bind}: {err}");
            std::process::exit(1);
        }
    };

    println!();
    println!("  Game Table");
    println!("  listening on http://{bind}");
    println!("  table passphrase: {passphrase}");
    println!("  GM key: {gm_key}");
    println!("  on this computer the lobby can sit you as GM");
    println!("  friends on your tailnet open http://<this-machine>:port");
    println!();

    let shutdown = shutdown_signal(app.clone());
    let service = net::router(app).into_make_service_with_connect_info::<std::net::SocketAddr>();
    if let Err(err) = axum::serve(listener, service).with_graceful_shutdown(shutdown).await {
        tracing::error!("server stopped: {err}");
    }
}

async fn shutdown_signal(app: Arc<App>) {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("saving before exit");
    app.save_now().await;
}
