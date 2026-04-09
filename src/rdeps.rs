//! Reverse dependency counts from UDD (Ultimate Debian Database).
//!
//! Queries the `all_packages` table to count how many source packages
//! depend on a given package.

use sqlx::PgPool;
use std::collections::HashMap;

/// Cached reverse dependency counts from UDD.
pub struct RdepsCache {
    pool: PgPool,
    /// Map from package name to reverse dependency count. `None` means "looked up, not found".
    count_by_package: HashMap<String, Option<u32>>,
}

#[derive(sqlx::FromRow)]
struct RdepsRow {
    count: Option<i64>,
}

impl RdepsCache {
    /// Create a new reverse dependencies cache using the given UDD connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            count_by_package: HashMap::new(),
        }
    }

    /// Look up the reverse dependency count for a package, fetching if needed.
    ///
    /// Returns `None` if the package is not found or the query fails.
    pub async fn get_rdeps_count(&mut self, package: &str) -> Option<u32> {
        if !self.count_by_package.contains_key(package) {
            self.fetch_rdeps_count(package).await;
        }
        self.get_cached_rdeps_count(package)
    }

    /// Look up the reverse dependency count from cache only, without fetching.
    ///
    /// Returns `None` if the package has not been fetched yet or was not found.
    pub fn get_cached_rdeps_count(&self, package: &str) -> Option<u32> {
        self.count_by_package.get(package)?.as_ref().copied()
    }

    /// Returns `true` if this package has been looked up (hit or miss).
    pub fn is_cached(&self, package: &str) -> bool {
        self.count_by_package.contains_key(package)
    }

    async fn fetch_rdeps_count(&mut self, package: &str) {
        let row: Option<RdepsRow> = match sqlx::query_as(
            "SELECT COUNT(DISTINCT source) AS count FROM all_packages \
             WHERE depends LIKE '%' || $1 || '%' AND release = 'sid'",
        )
        .bind(package)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(row) => row,
            Err(e) => {
                log::warn!("UDD rdeps query for {} failed: {}", package, e);
                return;
            }
        };

        match row {
            Some(RdepsRow { count: Some(n) }) => {
                self.count_by_package
                    .insert(package.to_string(), u32::try_from(n).ok());
            }
            _ => {
                self.count_by_package.insert(package.to_string(), None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache() -> RdepsCache {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(crate::udd::DEFAULT_UDD_URL)
            .unwrap();
        RdepsCache::new(pool)
    }

    #[tokio::test]
    async fn test_get_rdeps_count_cached() {
        let mut cache = make_cache();
        cache
            .count_by_package
            .insert("libc6".into(), Some(15000));

        let count = cache.get_rdeps_count("libc6").await;
        assert_eq!(count, Some(15000));
    }

    #[tokio::test]
    async fn test_get_rdeps_count_unknown() {
        let mut cache = make_cache();
        cache
            .count_by_package
            .insert("nonexistent-xyz".into(), None);

        let count = cache.get_rdeps_count("nonexistent-xyz").await;
        assert_eq!(count, None);
    }
}
