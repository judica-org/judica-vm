use crate::configuration::Config;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tracing::info;

pub struct Globals {
    pub config: Arc<Config>,
    pub shutdown: AppShutdown,
}

#[derive(Clone)]
pub struct AppShutdown {
    quit: Arc<AtomicBool>,
}

impl std::ops::Deref for AppShutdown {
    type Target = Arc<AtomicBool>;

    fn deref(&self) -> &Self::Target {
        &self.quit
    }
}

impl AppShutdown {
    pub fn new() -> Self {
        Self {
            quit: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn should_quit(&self) -> bool {
        self.quit.load(Ordering::Relaxed)
    }
    pub fn begin_shutdown(&self) {
        info!(event = "SHUTDOWN", "Beginning Node Shutdown",);
        self.quit.store(true, Ordering::Relaxed)
    }
}