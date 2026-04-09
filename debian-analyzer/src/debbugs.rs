//! Debian bug tracker data from UDD (Ultimate Debian Database).
//!
//! Queries the `bugs` and `bugs_tags` tables for bug reports filed
//! against source or binary packages. This complements the [`wnpp`]
//! module which focuses specifically on WNPP (packaging request) bugs.

use sqlx::PgPool;
use std::collections::HashMap;

/// Summary of a single Debian bug report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BugSummary {
    /// Numeric Debian bug ID.
    pub id: u32,
    /// Bug title, when available.
    pub title: Option<String>,
    /// Bug severity (e.g. "serious", "normal", "wishlist").
    pub severity: Option<String>,
    /// Whether the bug has been marked as done/resolved.
    pub done: bool,
    /// Tags associated with the bug (e.g. "patch", "confirmed").
    pub tags: Option<String>,
    /// Where the bug has been forwarded to, if anywhere.
    pub forwarded: Option<String>,
    /// Email address of the person who reported the bug.
    pub originator: Option<String>,
}

/// Cached bug data from UDD.
pub struct BugCache {
    pool: PgPool,
    bug_ids_by_key: HashMap<String, Vec<u32>>,
    bug_details_by_id: HashMap<u32, CachedBugDetails>,
}

#[derive(Debug, Clone)]
struct CachedBugDetails {
    title: Option<String>,
    severity: Option<String>,
    done: bool,
    tags: Option<String>,
    forwarded: Option<String>,
    originator: Option<String>,
}

#[derive(sqlx::FromRow)]
struct BugRow {
    id: i32,
    title: Option<String>,
    severity: Option<String>,
    done: Option<String>,
    tags: Option<String>,
    forwarded: Option<String>,
    submitter: Option<String>,
}

impl BugCache {
    /// Create a new bug cache using the given UDD connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            bug_ids_by_key: HashMap::new(),
            bug_details_by_id: HashMap::new(),
        }
    }

    /// Fetch bug IDs and details for a source package from UDD.
    async fn fetch_bugs_for_source_package(&mut self, source_package: &str) {
        let key = format!("src:{}", source_package);
        if self.bug_ids_by_key.contains_key(&key) {
            return;
        }

        let rows: Vec<BugRow> = match sqlx::query_as(
            "SELECT b.id, b.title, b.severity::text, b.done, b.forwarded, b.submitter, \
                    (SELECT string_agg(t.tag, ', ') FROM bugs_tags t WHERE t.id = b.id) AS tags \
             FROM bugs b \
             WHERE b.source = $1 \
             ORDER BY b.id",
        )
        .bind(source_package)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                log::warn!("UDD bug query for source {} failed: {}", source_package, e);
                return;
            }
        };

        let mut ids = Vec::new();
        for row in rows {
            let Some(id) = u32::try_from(row.id).ok() else {
                continue;
            };
            ids.push(id);
            self.bug_details_by_id.insert(
                id,
                CachedBugDetails {
                    title: row.title,
                    severity: row.severity,
                    done: row.done.as_ref().is_some_and(|d| !d.is_empty()),
                    tags: row.tags,
                    forwarded: row.forwarded,
                    originator: row.submitter,
                },
            );
        }

        self.bug_ids_by_key.insert(key, ids);
    }

    /// Fetch bugs filed against a binary package name from UDD.
    async fn fetch_bugs_for_binary_package(&mut self, binary_package: &str) {
        if self.bug_ids_by_key.contains_key(binary_package) {
            return;
        }

        let rows: Vec<BugRow> = match sqlx::query_as(
            "SELECT b.id, b.title, b.severity::text, b.done, b.forwarded, b.submitter, \
                    (SELECT string_agg(t.tag, ', ') FROM bugs_tags t WHERE t.id = b.id) AS tags \
             FROM bugs b \
             WHERE b.package = $1 \
             ORDER BY b.id",
        )
        .bind(binary_package)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                log::warn!(
                    "UDD bug query for binary {} failed: {}",
                    binary_package,
                    e
                );
                return;
            }
        };

        let mut ids = Vec::new();
        for row in rows {
            let Some(id) = u32::try_from(row.id).ok() else {
                continue;
            };
            ids.push(id);
            self.bug_details_by_id
                .entry(id)
                .or_insert(CachedBugDetails {
                    title: row.title,
                    severity: row.severity,
                    done: row.done.as_ref().is_some_and(|d| !d.is_empty()),
                    tags: row.tags,
                    forwarded: row.forwarded,
                    originator: row.submitter,
                });
        }

        self.bug_ids_by_key
            .insert(binary_package.to_string(), ids);
    }

    /// Fetch a single bug by ID from UDD.
    async fn fetch_bug_by_id(&mut self, id: u32) {
        let row: Option<BugRow> = match sqlx::query_as(
            "SELECT b.id, b.title, b.severity::text, b.done, b.forwarded, b.submitter, \
                    (SELECT string_agg(t.tag, ', ') FROM bugs_tags t WHERE t.id = b.id) AS tags \
             FROM bugs b \
             WHERE b.id = $1",
        )
        .bind(id as i32)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(row) => row,
            Err(e) => {
                log::warn!("UDD single bug query for {} failed: {}", id, e);
                return;
            }
        };

        if let Some(row) = row {
            self.bug_details_by_id.insert(
                id,
                CachedBugDetails {
                    title: row.title,
                    severity: row.severity,
                    done: row.done.as_ref().is_some_and(|d| !d.is_empty()),
                    tags: row.tags,
                    forwarded: row.forwarded,
                    originator: row.submitter,
                },
            );
        }
    }

    fn make_summary(&self, id: u32) -> BugSummary {
        match self.bug_details_by_id.get(&id) {
            Some(details) => BugSummary {
                id,
                title: details.title.clone(),
                severity: details.severity.clone(),
                done: details.done,
                tags: details.tags.clone(),
                forwarded: details.forwarded.clone(),
                originator: details.originator.clone(),
            },
            None => BugSummary {
                id,
                title: None,
                severity: None,
                done: false,
                tags: None,
                forwarded: None,
                originator: None,
            },
        }
    }

    /// Return bug summaries for a source package that match a decimal prefix.
    pub async fn get_bug_summaries_with_prefix(
        &mut self,
        package: &str,
        prefix: &str,
    ) -> Vec<BugSummary> {
        self.fetch_bugs_for_source_package(package).await;

        let normalized_prefix = prefix.trim();
        let key = format!("src:{}", package);
        let Some(ids) = self.bug_ids_by_key.get(&key) else {
            return Vec::new();
        };

        ids.iter()
            .filter(|id| id.to_string().starts_with(normalized_prefix))
            .map(|&id| self.make_summary(id))
            .collect()
    }

    /// Return a single bug summary by ID, fetching from UDD if not cached.
    pub async fn get_bug_summary(&mut self, id: u32) -> Option<BugSummary> {
        if !self.bug_details_by_id.contains_key(&id) {
            self.fetch_bug_by_id(id).await;
        }
        if self.bug_details_by_id.contains_key(&id) {
            Some(self.make_summary(id))
        } else {
            None
        }
    }

    /// Count open bugs for a source package from cache only.
    ///
    /// Returns `None` if the source package has not been fetched yet.
    pub fn get_cached_open_bug_count(&self, source_package: &str) -> Option<usize> {
        let key = format!("src:{}", source_package);
        let ids = self.bug_ids_by_key.get(&key)?;
        Some(
            ids.iter()
                .filter(|id| self.bug_details_by_id.get(id).is_some_and(|d| !d.done))
                .count(),
        )
    }

    /// Count open bugs filed against a binary package from cache only.
    ///
    /// Returns `None` if the binary package has not been fetched yet.
    pub fn get_cached_open_binary_bug_count(&self, binary_package: &str) -> Option<usize> {
        let ids = self.bug_ids_by_key.get(binary_package)?;
        Some(
            ids.iter()
                .filter(|id| self.bug_details_by_id.get(id).is_some_and(|d| !d.done))
                .count(),
        )
    }

    /// Pre-fetch bugs for a source package so the data is cached.
    pub async fn prefetch_bugs_for_package(&mut self, package: &str) {
        self.fetch_bugs_for_source_package(package).await;
    }

    /// Pre-fetch bugs filed against a binary package name.
    pub async fn prefetch_bugs_for_binary_package(&mut self, binary_package: &str) {
        self.fetch_bugs_for_binary_package(binary_package).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache() -> BugCache {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(crate::udd::DEFAULT_UDD_URL)
            .unwrap();
        BugCache::new(pool)
    }

    fn insert_test_bugs(cache: &mut BugCache, source_package: &str, bugs: Vec<(u32, Option<&str>)>) {
        let mut ids = Vec::new();
        for (id, title) in bugs {
            ids.push(id);
            cache.bug_details_by_id.insert(
                id,
                CachedBugDetails {
                    title: title.map(ToString::to_string),
                    severity: None,
                    done: false,
                    tags: None,
                    forwarded: None,
                    originator: None,
                },
            );
        }
        ids.sort();
        cache
            .bug_ids_by_key
            .insert(format!("src:{}", source_package), ids);
    }

    #[tokio::test]
    async fn test_get_bug_summaries_with_prefix() {
        let mut cache = make_cache();
        insert_test_bugs(
            &mut cache,
            "foo",
            vec![
                (123456, Some("Fix crash on startup")),
                (123499, None),
                (888888, Some("Unrelated issue")),
            ],
        );

        let summaries = cache.get_bug_summaries_with_prefix("foo", "1234").await;
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].id, 123456);
        assert_eq!(summaries[0].title.as_deref(), Some("Fix crash on startup"));
        assert_eq!(summaries[1].id, 123499);
        assert_eq!(summaries[1].title, None);
    }

    #[tokio::test]
    async fn test_open_bug_count() {
        let mut cache = make_cache();
        insert_test_bugs(
            &mut cache,
            "bar",
            vec![(100, Some("open bug")), (200, Some("another"))],
        );
        // Mark one as done
        cache.bug_details_by_id.get_mut(&200).unwrap().done = true;

        assert_eq!(cache.get_cached_open_bug_count("bar"), Some(1));
    }
}
