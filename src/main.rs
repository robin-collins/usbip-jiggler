mod hid;
mod usbip;

use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

use hid::mouse::jiggle_task;
use usbip::server::run_server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let (tx, rx) = mpsc::channel(4);

    tokio::spawn(jiggle_task(tx));

    // run_server blocks on the TCP listener; spawn it onto a blocking thread
    // so it doesn't stall the async runtime.
    tokio::task::spawn_blocking(move || run_server(rx)).await.unwrap();
}
