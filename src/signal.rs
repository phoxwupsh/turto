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

#[cfg(windows)]
mod windows {
    use tokio::signal::windows::{ctrl_break, ctrl_c, ctrl_close, ctrl_logoff, ctrl_shutdown};
    pub async fn wait_shutdown_signal() {
        let mut c_break = ctrl_break().unwrap();
        let mut c_c = ctrl_c().unwrap();
        let mut c_close = ctrl_close().unwrap();
        let mut c_logoff = ctrl_logoff().unwrap();
        let mut c_shutdown = ctrl_shutdown().unwrap();
        tokio::select! {
            _ = c_break.recv() => (),
            _ = c_c.recv() => (),
            _ = c_close.recv() => (),
            _ = c_logoff.recv() => (),
            _ = c_shutdown.recv() => ()
        };
    }
}

#[cfg(windows)]
pub use windows::wait_shutdown_signal;
