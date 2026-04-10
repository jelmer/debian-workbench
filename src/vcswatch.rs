//! VCS watch data from UDD (Ultimate Debian Database).
//!
//! Queries the `vcswatch` table to find the latest packaged version
//! for a given VCS repository URL.

use sqlx::PgPool;
use std::collections::HashMap;

/// Cached VCS watch data from UDD.
pub struct VcsWatchCache {
    pool: PgPool,
    /// Map from VCS URL to packaged version. `None` means "looked up, not found".
    version_by_url: HashMap<String, Option<String>>,
}

#[derive(sqlx::FromRow)]
struct VcsWatchRow {
    url: Option<String>,
    version: Option<String>,
}

impl VcsWatchCache {
    /// Create a new VCS watch cache using the given UDD connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            version_by_url: HashMap::new(),
        }
    }

    /// Look up the packaged version for a VCS URL, fetching if needed.
    ///
    /// Returns `None` if the URL is not found in vcswatch or the query fails.
    pub async fn get_version_for_url(&mut self, url: &str) -> Option<&str> {
        if !self.version_by_url.contains_key(url) {
            self.fetch_version_for_url(url).await;
        }
        self.get_cached_version_for_url(url)
    }

    /// Look up the packaged version from cache only, without fetching.
    pub fn get_cached_version_for_url(&self, url: &str) -> Option<&str> {
        self.version_by_url.get(url).and_then(|v| v.as_deref())
    }

    /// Returns `true` if this URL has been looked up (hit or miss).
    pub fn is_cached(&self, url: &str) -> bool {
        self.version_by_url.contains_key(url)
    }

    async fn fetch_version_for_url(&mut self, url: &str) {
        let row: Option<VcsWatchRow> =
            match sqlx::query_as("SELECT url, version::text FROM vcswatch WHERE url = $1 LIMIT 1")
                .bind(url)
                .fetch_optional(&self.pool)
                .await
            {
                Ok(row) => row,
                Err(e) => {
                    log::warn!("UDD vcswatch query for {} failed: {}", url, e);
                    return;
                }
            };

        match row {
            Some(VcsWatchRow {
                url: Some(row_url),
                version,
            }) => {
                self.version_by_url.insert(row_url, version);
            }
            _ => {
                self.version_by_url.insert(url.to_string(), None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache() -> VcsWatchCache {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(crate::udd::DEFAULT_UDD_URL)
            .unwrap();
        VcsWatchCache::new(pool)
    }

    #[tokio::test]
    async fn test_get_version_cached() {
        let mut cache = make_cache();
        cache.version_by_url.insert(
            "https://salsa.debian.org/python-team/packages/dulwich.git".into(),
            Some("1.1.0-1".into()),
        );

        let version = cache
            .get_version_for_url("https://salsa.debian.org/python-team/packages/dulwich.git")
            .await;
        assert_eq!(version, Some("1.1.0-1"));
    }

    #[tokio::test]
    async fn test_get_version_not_found() {
        let mut cache = make_cache();
        cache
            .version_by_url
            .insert("https://example.com/nonexistent.git".into(), None);

        let version = cache
            .get_version_for_url("https://example.com/nonexistent.git")
            .await;
        assert_eq!(version, None);
    }
}
