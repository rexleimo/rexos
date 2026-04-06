pub(super) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static ASYNC_ENV_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();

pub(super) fn async_env_lock() -> &'static tokio::sync::Mutex<()> {
    ASYNC_ENV_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

pub(super) struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    pub(super) fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
