use std::sync::{Mutex, RwLock};
use rusqlite::Connection;
use crate::storage::FlatVectorMatrix;
use crate::license::guard::TrialGuard;

pub struct AppState {
    pub db_conn: Mutex<Connection>,
    pub memory_db: RwLock<FlatVectorMatrix>,
    pub trial_guard: Mutex<TrialGuard>,
}