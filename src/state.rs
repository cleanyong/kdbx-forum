use std::{path::PathBuf, sync::Arc};

use keepass::{Database, DatabaseKey};
use tokio::sync::RwLock;

/// Shared application state, holding the decrypted KeePass database
/// and the information needed to persist changes back to disk.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<RwLock<Database>>,
    pub db_path: PathBuf,
    pub key: DatabaseKey,
}

impl AppState {
    pub fn new(db: Database, db_path: PathBuf, key: DatabaseKey) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            db_path,
            key,
        }
    }
}

