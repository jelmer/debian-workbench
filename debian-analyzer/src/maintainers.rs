//! Maintainer identity suggestions from environment and UDD.
//!
//! Provides:
//! 1. The user's identity from `$DEBEMAIL`/`$DEBFULLNAME` (matching `dch` behavior)
//! 2. Common maintainer identities from the UDD `sources` table

use sqlx::PgPool;

/// Cached maintainer identities from UDD.
pub struct MaintainerCache {
    pool: PgPool,
    /// Distinct maintainer identities fetched from UDD, or `None` if not yet fetched.
    maintainers: Option<Vec<String>>,
}

#[derive(sqlx::FromRow)]
struct MaintainerRow {
    maintainer: String,
}

impl MaintainerCache {
    /// Create a new maintainer cache using the given UDD connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            maintainers: None,
        }
    }

    /// Get the list of known maintainer identities, fetching from UDD if needed.
    pub async fn get_maintainers(&mut self) -> &[String] {
        if self.maintainers.is_none() {
            self.fetch_maintainers().await;
        }
        self.maintainers.as_deref().unwrap_or(&[])
    }

    async fn fetch_maintainers(&mut self) {
        let rows: Vec<MaintainerRow> = match sqlx::query_as(
            "SELECT DISTINCT maintainer FROM sources \
             WHERE release = 'sid' \
             ORDER BY maintainer \
             LIMIT 5000",
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                log::warn!("UDD maintainer query failed: {}", e);
                self.maintainers = Some(Vec::new());
                return;
            }
        };

        self.maintainers = Some(rows.into_iter().map(|r| r.maintainer).collect());
    }
}

/// Get the user's identity from Debian environment variables.
///
/// Checks `$DEBFULLNAME` and `$DEBEMAIL` first (matching `dch` behavior),
/// then falls back to `$EMAIL` for the email part.
pub fn get_user_identity() -> Option<String> {
    let name = std::env::var("DEBFULLNAME").ok().filter(|s| !s.is_empty());
    let email = std::env::var("DEBEMAIL")
        .ok()
        .or_else(|| std::env::var("EMAIL").ok())
        .filter(|s| !s.is_empty());

    match (name, email) {
        (Some(n), Some(e)) => Some(format!("{} <{}>", n, e)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache() -> MaintainerCache {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(crate::udd::DEFAULT_UDD_URL)
            .unwrap();
        MaintainerCache::new(pool)
    }

    #[tokio::test]
    async fn test_get_maintainers_from_cache() {
        let mut cache = make_cache();
        cache.maintainers = Some(vec![
            "Alice <alice@example.com>".to_string(),
            "Bob <bob@example.com>".to_string(),
        ]);

        let maintainers = cache.get_maintainers().await;
        assert_eq!(maintainers.len(), 2);
        assert_eq!(maintainers[0], "Alice <alice@example.com>");
    }

    #[tokio::test]
    async fn test_get_maintainers_empty() {
        let mut cache = make_cache();
        cache.maintainers = Some(vec![]);
        let maintainers = cache.get_maintainers().await;
        assert!(maintainers.is_empty());
    }
}
