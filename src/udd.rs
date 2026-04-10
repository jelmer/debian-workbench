//! Interface to the Debian Ultimate Debian Database (UDD) mirror
use sqlx::postgres::PgPoolOptions;
use sqlx::{Error, PgPool};
use std::sync::Arc;

/// Default URL for the UDD mirror
pub const DEFAULT_UDD_URL: &str =
    "postgresql://udd-mirror:udd-mirror@udd-mirror.debian.net:5432/udd";

/// A shared UDD connection pool that can be cloned cheaply.
pub type SharedPool = Arc<PgPool>;

/// Connect to the UDD mirror
pub async fn connect_udd_mirror() -> Result<PgPool, Error> {
    PgPool::connect(DEFAULT_UDD_URL).await
}

/// Create a shared lazy connection pool to UDD.
///
/// The pool connects on first use rather than immediately, so this
/// function is cheap to call even when UDD access is not needed.
pub fn shared_pool() -> SharedPool {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_lazy(DEFAULT_UDD_URL)
        .expect("invalid UDD connection URL");
    Arc::new(pool)
}
