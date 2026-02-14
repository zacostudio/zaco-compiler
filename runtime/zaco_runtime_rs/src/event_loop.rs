use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn init_runtime() {
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create Tokio runtime")
    });
}

pub fn shutdown_runtime() {
    // OnceLock does not give ownership, so we cannot call shutdown_timeout/shutdown_background.
    // Instead, block on an empty future to flush any pending spawned tasks, then the
    // runtime will be cleaned up when the process exits.
    if let Some(rt) = RUNTIME.get() {
        rt.block_on(async {
            // Yield to let pending tasks make progress
            tokio::task::yield_now().await;
        });
    }
}

pub fn get_runtime() -> &'static Runtime {
    RUNTIME.get().expect("Runtime not initialized. Call zaco_runtime_init() first.")
}

/// Block on a future (for sync wrappers and top-level await)
pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    get_runtime().block_on(f)
}

/// Spawn an async task
pub fn spawn<F>(f: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    get_runtime().spawn(f)
}
