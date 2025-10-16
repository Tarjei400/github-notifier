use log::info;
use std::ops::Deref;
use std::sync::Arc;
use tokio::signal;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

pub fn notify_cancellation_token(
    token: &Arc<CancellationToken>,
    mut shutdown_recv: UnboundedReceiver<()>,
) {
    tokio::spawn({
        let token = token.clone();

        async move {
            //TODO: Unlikely it will run on windows node, but for that case its unsafe
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = signal::ctrl_c() => {
                   info!("Received ctrl+C, initiating graceful shutdown.");
                },
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown.");
                },
                _ = shutdown_recv.recv() => {
                    info!("Received shutdown signal, initiating graceful shutdown.");
                }
            }

            token.cancel();
        }
    });
}
