#[cfg(unix)]
mod unix {
    use tokio::signal::unix::{signal, SignalKind};

    pub async fn wait_shutdown_signal() {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        tokio::select! {
            _ = sigint.recv() => (),
            _ = sigterm.recv() => ()
        };
    }
}

#[cfg(unix)]
pub use unix::wait_shutdown_signal;
