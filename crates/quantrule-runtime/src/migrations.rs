//! Forward-only schema migrations.
//!
//! The migrations are compiled in by the service that owns them, applied in
//! version order, and recorded with a checksum. A changed checksum, an unknown
//! applied version, or a database ahead of the binary is refused rather than
//! guessed at.
//!
//! This module is pure: it decides what to apply. The Postgres adapter runs the
//! SQL and pins the advisory lock to one connection.

/// One compiled-in migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    /// Ordering key; also the recorded version.
    pub version: i64,
    /// Human name, used in the file name.
    pub name: &'static str,
    /// The SQL to run.
    pub sql: &'static str,
}

/// A migration already recorded in the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// The recorded version.
    pub version: i64,
    /// The recorded name.
    pub name: String,
    /// The recorded checksum.
    pub checksum: String,
}

/// Why a migration set cannot be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    /// Two migrations share a version.
    DuplicateVersion(i64),
    /// A recorded checksum does not match the compiled migration.
    ChecksumMismatch {
        /// The version whose SQL changed.
        version: i64,
    },
    /// The database has a migration the binary does not know.
    UnknownApplied(i64),
    /// The database is ahead of the binary.
    DatabaseAhead {
        /// Highest applied version.
        applied: i64,
        /// Highest version the binary knows.
        available: i64,
    },
    /// The database refused a statement.
    Sql(String),
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateVersion(version) => {
                write!(formatter, "migration {version} is defined twice")
            }
            Self::ChecksumMismatch { version } => write!(
                formatter,
                "migration {version} changed after it was applied"
            ),
            Self::UnknownApplied(version) => {
                write!(
                    formatter,
                    "migration {version} is applied but unknown to this binary"
                )
            }
            Self::DatabaseAhead { applied, available } => write!(
                formatter,
                "database is at {applied}, binary knows {available}: downgrade refused"
            ),
            Self::Sql(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for MigrationError {}

/// The FNV-1a checksum of a migration body, in lowercase hexadecimal.
#[must_use]
pub fn checksum(sql: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in sql.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Decide what to apply.
///
/// # Errors
///
/// Returns [`MigrationError`] when a version repeats, a recorded checksum
/// differs, the database knows a migration this binary does not, or the
/// database is ahead.
pub fn plan<'a>(
    applied: &[Applied],
    available: &'a [Migration],
) -> Result<Vec<&'a Migration>, MigrationError> {
    for pair in available.windows(2) {
        if pair[0].version == pair[1].version {
            return Err(MigrationError::DuplicateVersion(pair[0].version));
        }
    }

    let highest_available = available.last().map_or(0, |migration| migration.version);
    let highest_applied = applied.last().map_or(0, |record| record.version);
    if highest_applied > highest_available {
        return Err(MigrationError::DatabaseAhead {
            applied: highest_applied,
            available: highest_available,
        });
    }

    for record in applied {
        let Some(migration) = available.iter().find(|m| m.version == record.version) else {
            return Err(MigrationError::UnknownApplied(record.version));
        };
        if checksum(migration.sql) != record.checksum {
            return Err(MigrationError::ChecksumMismatch {
                version: record.version,
            });
        }
    }

    Ok(available
        .iter()
        .filter(|migration| {
            !applied
                .iter()
                .any(|record| record.version == migration.version)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migration(version: i64, sql: &'static str) -> Migration {
        Migration {
            version,
            name: "probe",
            sql,
        }
    }

    fn applied(migration: &Migration) -> Applied {
        Applied {
            version: migration.version,
            name: migration.name.to_owned(),
            checksum: checksum(migration.sql),
        }
    }

    #[test]
    fn an_empty_database_gets_every_migration() {
        let available = [migration(1, "SELECT 1"), migration(2, "SELECT 2")];
        assert_eq!(plan(&[], &available).expect("plan").len(), 2);
    }

    #[test]
    fn a_fully_migrated_database_gets_none() {
        let available = [migration(1, "SELECT 1"), migration(2, "SELECT 2")];
        let applied: Vec<Applied> = available.iter().map(applied).collect();
        assert!(plan(&applied, &available).expect("plan").is_empty());
    }

    #[test]
    fn a_partial_database_gets_only_the_rest() {
        let available = [migration(1, "SELECT 1"), migration(2, "SELECT 2")];
        let recorded = vec![applied(&available[0])];
        let pending = plan(&recorded, &available).expect("plan");
        assert_eq!(
            pending.iter().map(|m| m.version).collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn a_changed_body_is_refused() {
        let available = [migration(1, "SELECT 1")];
        let mut recorded = vec![applied(&available[0])];
        recorded[0].checksum = "0000000000000000".to_owned();
        assert_eq!(
            plan(&recorded, &available),
            Err(MigrationError::ChecksumMismatch { version: 1 })
        );
    }

    #[test]
    fn a_database_ahead_is_refused() {
        let available = [migration(1, "SELECT 1")];
        let recorded = vec![Applied {
            version: 99,
            name: "future".to_owned(),
            checksum: "0".to_owned(),
        }];
        assert_eq!(
            plan(&recorded, &available),
            Err(MigrationError::DatabaseAhead {
                applied: 99,
                available: 1
            })
        );
    }

    #[test]
    fn an_unknown_applied_version_is_refused() {
        let available = [migration(1, "SELECT 1"), migration(3, "SELECT 3")];
        let recorded = vec![Applied {
            version: 2,
            name: "runs".to_owned(),
            checksum: "x".to_owned(),
        }];
        assert!(matches!(
            plan(&recorded, &available),
            Err(MigrationError::UnknownApplied(2))
        ));
    }

    #[test]
    fn a_duplicate_version_is_refused() {
        let available = [migration(1, "SELECT 1"), migration(1, "SELECT 2")];
        assert_eq!(
            plan(&[], &available),
            Err(MigrationError::DuplicateVersion(1))
        );
    }

    #[test]
    fn the_checksum_is_stable() {
        assert_eq!(checksum("abc"), checksum("abc"));
        assert_ne!(checksum("abc"), checksum("abd"));
    }
}
