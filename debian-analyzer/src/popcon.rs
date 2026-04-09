//! Popularity contest data from UDD (Ultimate Debian Database).
//!
//! Queries the `popcon` table to find install counts for packages.

use sqlx::PgPool;
use std::collections::HashMap;

/// Cached popcon data from UDD.
pub struct PopconCache {
    pool: PgPool,
    /// Map from package name to install count. `None` means "looked up, not found".
    inst_by_package: HashMap<String, Option<u32>>,
}

#[derive(sqlx::FromRow)]
struct PopconRow {
    insts: Option<i32>,
}

impl PopconCache {
    /// Create a new popcon cache using the given UDD connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            inst_by_package: HashMap::new(),
        }
    }

    /// Look up the install count for a package, fetching if needed.
    ///
    /// Returns `None` if the package is not found in popcon or the query fails.
    pub async fn get_inst_count(&mut self, package: &str) -> Option<u32> {
        if !self.inst_by_package.contains_key(package) {
            self.fetch_inst_count(package).await;
        }
        self.get_cached_inst_count(package)
    }

    /// Look up the install count from cache only, without fetching.
    ///
    /// Returns `None` if the package has not been fetched yet or was not found.
    pub fn get_cached_inst_count(&self, package: &str) -> Option<u32> {
        self.inst_by_package.get(package)?.as_ref().copied()
    }

    /// Returns `true` if this package has been looked up (hit or miss).
    pub fn is_cached(&self, package: &str) -> bool {
        self.inst_by_package.contains_key(package)
    }

    async fn fetch_inst_count(&mut self, package: &str) {
        let row: Option<PopconRow> =
            match sqlx::query_as("SELECT insts FROM popcon WHERE package = $1 LIMIT 1")
                .bind(package)
                .fetch_optional(&self.pool)
                .await
            {
                Ok(row) => row,
                Err(e) => {
                    log::warn!("UDD popcon query for {} failed: {}", package, e);
                    return;
                }
            };

        match row {
            Some(PopconRow { insts: Some(n) }) => {
                self.inst_by_package
                    .insert(package.to_string(), u32::try_from(n).ok());
            }
            _ => {
                self.inst_by_package.insert(package.to_string(), None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache() -> PopconCache {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(crate::udd::DEFAULT_UDD_URL)
            .unwrap();
        PopconCache::new(pool)
    }

    #[tokio::test]
    async fn test_get_inst_count_unknown_package() {
        let mut cache = make_cache();
        // Pre-insert a "not found" entry to avoid hitting the network.
        cache.inst_by_package.insert("nonexistent-xyz".into(), None);

        let count = cache.get_inst_count("nonexistent-xyz").await;
        assert_eq!(count, None);
    }

    #[tokio::test]
    async fn test_get_inst_count_cached() {
        let mut cache = make_cache();
        cache
            .inst_by_package
            .insert("hello".into(), Some(42000));

        let count = cache.get_inst_count("hello").await;
        assert_eq!(count, Some(42000));
    }
}
