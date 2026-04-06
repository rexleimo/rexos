use super::*;
use crate::defs::ensure_browser_url_allowed;
use crate::ops::fs::validate_relative_path;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

static ENV_LOCK: Mutex<()> = Mutex::new(());
static ASYNC_ENV_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn async_env_lock() -> &'static tokio::sync::Mutex<()> {
    ASYNC_ENV_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

mod browser;
mod compat;
mod fs;
mod mcp;
mod media;
mod process;
mod web;
