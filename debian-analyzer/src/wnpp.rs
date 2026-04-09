//! Accessing WNPP bugs in the Debian Bug Tracking System.
use sqlx::error::BoxDynError;
use sqlx::{Error, PgPool, Postgres};

/// Bug identifier
pub type BugId = i32;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
/// Type of WNPP bug.
pub enum BugKind {
    /// Request for packaging
    RFP,
    /// Intent to package
    ITP,
}

impl std::str::FromStr for BugKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RFP" => Ok(BugKind::RFP),
            "ITP" => Ok(BugKind::ITP),
            _ => Err(format!("Unknown bug kind: {}", s)),
        }
    }
}

impl std::fmt::Display for BugKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BugKind::RFP => write!(f, "RFP"),
            BugKind::ITP => write!(f, "ITP"),
        }
    }
}

impl sqlx::Type<Postgres> for BugKind {
    fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<Postgres>>::type_info()
    }
}

impl sqlx::Decode<'_, Postgres> for BugKind {
    fn decode(value: <Postgres as sqlx::Database>::ValueRef<'_>) -> Result<Self, BoxDynError> {
        let s = <String as sqlx::Decode<Postgres>>::decode(value)?;
        s.parse().map_err(Into::into)
    }
}

/// Read DebBugs data through UDD.
pub struct DebBugs {
    pool: PgPool,
}

impl DebBugs {
    /// Create a new DebBugs instance.
    pub fn new(pool: PgPool) -> Self {
        DebBugs { pool }
    }

    /// Create a new DebBugs instance with a default connection.
    pub async fn default() -> Result<Self, Error> {
        Ok(DebBugs {
            pool: crate::udd::connect_udd_mirror().await?,
        })
    }

    /// Check that a bug belongs to a particular package.
    ///
    /// # Arguments
    /// * `package` - Package name
    /// * `bugid` - Bug number
    pub async fn check_bug(&self, package: &str, bugid: BugId) -> Result<bool, Error> {
        let actual_package: Option<String> =
            sqlx::query_scalar("select package from bugs where id = $1")
                .bind(bugid)
                .fetch_optional(&self.pool)
                .await?;

        Ok(actual_package.as_deref() == Some(package))
    }

    /// Find archived ITP/RFP bugs for a package.
    pub async fn find_archived_wnpp_bugs(
        &self,
        source_name: &str,
    ) -> Result<Vec<(BugId, BugKind)>, Error> {
        sqlx::query_as::<_, (BugId, BugKind)>(
            "select id, substring(title, 1, 3) from archived_bugs where package = 'wnpp' and (
            title like 'ITP: ' || $1 || ' -- %' OR
            title like 'RFP: ' || $1 || ' -- %')",
        )
        .bind(source_name)
        .fetch_all(&self.pool)
        .await
    }

    /// Find ITP/RFP bugs for a package.
    pub async fn find_wnpp_bugs(&self, source_name: &str) -> Result<Vec<(BugId, BugKind)>, Error> {
        sqlx::query_as::<_, (BugId, BugKind)>(
            "select id, type from wnpp where source = $1 and type in ('ITP', 'RFP')",
        )
        .bind(source_name)
        .fetch_all(&self.pool)
        .await
    }
}

/// Find WNPP bugs for a package, trying multiple names.
pub async fn find_wnpp_bugs_harder(names: &[&str]) -> Result<Vec<(BugId, BugKind)>, Error> {
    for name in names {
        let debbugs = DebBugs::default().await?;
        let mut wnpp_bugs = debbugs.find_wnpp_bugs(name).await?;
        if wnpp_bugs.is_empty() {
            wnpp_bugs = debbugs.find_archived_wnpp_bugs(name).await?;
            if !wnpp_bugs.is_empty() {
                log::warn!("Found archived ITP/RFP bugs for {}: {:?}", name, wnpp_bugs);
            } else {
                log::warn!("No relevant WNPP bugs found for {}", name);
            }
        }
        if !wnpp_bugs.is_empty() {
            log::info!("Found WNPP bugs for {}: {:?}", name, wnpp_bugs);
            return Ok(wnpp_bugs);
        }
    }
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bug_id_is_i32() {
        // Ensure BugId is i32 to match the database INT4 type
        let _: BugId = 123456i32;
        assert_eq!(std::mem::size_of::<BugId>(), 4);
    }

    #[test]
    fn test_bug_kind_parsing() {
        assert_eq!("RFP".parse::<BugKind>().unwrap(), BugKind::RFP);
        assert_eq!("ITP".parse::<BugKind>().unwrap(), BugKind::ITP);
        assert!("INVALID".parse::<BugKind>().is_err());
    }

    #[test]
    fn test_bug_kind_display() {
        assert_eq!(BugKind::RFP.to_string(), "RFP");
        assert_eq!(BugKind::ITP.to_string(), "ITP");
    }

    #[tokio::test]
    async fn test_query_udd_wnpp() {
        // Try to connect to UDD and query WNPP bugs
        // This test verifies that:
        // 1. We can connect to the UDD database
        // 2. The query structure is correct
        // 3. BugId type (i32) matches the database column type
        let debbugs = DebBugs::default().await.expect("Failed to connect to UDD");

        // Query for a package that historically has had WNPP bugs
        // Using a nonexistent package name should return empty results without error
        let result = debbugs.find_wnpp_bugs("nonexistent-package-12345").await;

        // Even if no bugs are found, the query should succeed (empty result is ok)
        assert!(result.is_ok(), "Query failed: {:?}", result);
        let bugs = result.unwrap();
        assert_eq!(bugs.len(), 0, "Should find no bugs for nonexistent package");
    }

    #[tokio::test]
    async fn test_query_udd_archived() {
        let debbugs = DebBugs::default().await.expect("Failed to connect to UDD");

        // Query archived bugs
        let result = debbugs
            .find_archived_wnpp_bugs("nonexistent-package-12345")
            .await;

        // Even if no bugs are found, the query should succeed
        assert!(result.is_ok(), "Archived query failed: {:?}", result);
        let bugs = result.unwrap();
        assert_eq!(
            bugs.len(),
            0,
            "Should find no archived bugs for nonexistent package"
        );
    }
}
