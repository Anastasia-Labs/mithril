//! SQLite module.
//! This module provides a minimal yet useful Entity framework on top of SQLite
//! with ability to perform any SQL query possible and hydrate results in Rust
//! structs.
mod condition;
mod cursor;
mod entity;
mod projection;
mod provider;
mod source_alias;

pub use condition::WhereCondition;
pub use cursor::EntityCursor;
pub use entity::{HydrationError, SqLiteEntity};
pub use projection::{Projection, ProjectionField};
pub use provider::Provider;
pub use source_alias::SourceAlias;

use sqlite::ConnectionWithFullMutex;
use std::sync::Arc;

use crate::StdResult;

/// Do a [vacuum](https://www.sqlite.org/lang_vacuum.html) on the given connection, this will
/// reconstruct the database file, repacking it into a minimal amount of disk space.
pub async fn vacuum_database(connection: Arc<ConnectionWithFullMutex>) -> StdResult<()> {
    connection.execute("vacuum")?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::sqlite::vacuum_database;
    use sqlite::Connection;
    use std::sync::Arc;

    #[tokio::test]
    async fn calling_vacuum_on_an_empty_in_memory_db_should_not_fail() {
        let connection = Arc::new(Connection::open_with_full_mutex(":memory:").unwrap());

        vacuum_database(connection)
            .await
            .expect("Vacuum should not fail");
    }

    #[test]
    fn sqlite_version_should_be_3_42_or_more() {
        let connection = Connection::open(":memory:").unwrap();
        let mut statement = connection.prepare("select sqlite_version()").unwrap();
        let cursor = statement.iter().next().unwrap().unwrap();
        let db_version = cursor.read::<&str, _>(0);
        let version = semver::Version::parse(db_version)
            .expect("Sqlite version should be parsable to semver");
        let requirement = semver::VersionReq::parse(">=3.42.0").unwrap();

        assert!(
            requirement.matches(&version),
            "Sqlite version {} is lower than 3.42.0",
            version
        )
    }
}
