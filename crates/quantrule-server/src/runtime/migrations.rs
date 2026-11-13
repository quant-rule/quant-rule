//! Forward-only schema migrations.
//!
//! The migrations are compiled in, applied in version order, and recorded with a
//! checksum. A changed checksum, an unknown applied version, or a database ahead
//! of the binary is refused rather than guessed at.
//!
//! This module is pure: it decides what to apply. The adapter runs the SQL.

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

/// The migrations this binary carries, in version order.
#[must_use]
pub fn available() -> Vec<Migration> {
    let mut migrations = vec![
        Migration {
            version: 1,
            name: "catalog",
            sql: include_str!("../../migrations/0001_catalog.sql"),
        },
        Migration {
            version: 2,
            name: "runs",
            sql: include_str!("../../migrations/0002_runs.sql"),
        },
        Migration {
            version: 3,
            name: "simulation",
            sql: include_str!("../../migrations/0003_simulation.sql"),
        },
        Migration {
            version: 4,
            name: "research",
            sql: include_str!("../../migrations/0004_research.sql"),
        },
        Migration {
            version: 5,
            name: "hypothesis-body",
            sql: include_str!("../../migrations/0005_hypothesis_body.sql"),
        },
        Migration {
            version: 6,
            name: "catalog-profile",
            sql: include_str!("../../migrations/0006_catalog_profile.sql"),
        },
        Migration {
            version: 7,
            name: "profiles",
            sql: include_str!("../../migrations/0007_profiles.sql"),
        },
        Migration {
            version: 8,
            name: "hypothesis-profile",
            sql: include_str!("../../migrations/0008_hypothesis_profile.sql"),
        },
        Migration {
            version: 9,
            name: "run-submissions",
            sql: include_str!("../../migrations/0009_run_submissions.sql"),
        },
        Migration {
            version: 10,
            name: "dataset-quality",
            sql: include_str!("../../migrations/0010_dataset_quality.sql"),
        },
    ];
    migrations.sort_by_key(|migration| migration.version);
    migrations
}

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

    fn applied(version: i64, migration: &Migration) -> Applied {
        Applied {
            version,
            name: migration.name.to_owned(),
            checksum: checksum(migration.sql),
        }
    }

    #[test]
    fn an_empty_database_gets_every_migration() {
        let available = available();
        let pending = plan(&[], &available).expect("plan");
        assert_eq!(pending.len(), available.len());
    }

    #[test]
    fn a_fully_migrated_database_gets_none() {
        let available = available();
        let applied: Vec<Applied> = available
            .iter()
            .map(|migration| applied(migration.version, migration))
            .collect();
        assert!(plan(&applied, &available).expect("plan").is_empty());
    }

    #[test]
    fn a_partial_database_gets_only_the_rest() {
        let available = available();
        let applied = vec![applied(1, &available[0])];
        let pending = plan(&applied, &available).expect("plan");
        assert_eq!(
            pending.iter().map(|m| m.version).collect::<Vec<_>>(),
            vec![2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
    }

    #[test]
    fn a_changed_body_is_refused() {
        let available = available();
        let mut applied = vec![applied(1, &available[0])];
        applied[0].checksum = "0000000000000000".to_owned();
        assert_eq!(
            plan(&applied, &available),
            Err(MigrationError::ChecksumMismatch { version: 1 })
        );
    }

    #[test]
    fn a_database_ahead_is_refused() {
        let available = available();
        let mut applied: Vec<Applied> = available
            .iter()
            .map(|migration| applied(migration.version, migration))
            .collect();
        applied.push(Applied {
            version: 99,
            name: "future".to_owned(),
            checksum: "0".to_owned(),
        });
        assert_eq!(
            plan(&applied, &available),
            Err(MigrationError::DatabaseAhead {
                applied: 99,
                available: 10
            })
        );
    }

    #[test]
    fn an_unknown_applied_version_is_refused() {
        let available = available();
        let applied = vec![Applied {
            version: 2,
            name: "runs".to_owned(),
            checksum: "x".to_owned(),
        }];
        assert!(matches!(
            plan(&applied, &available),
            Err(MigrationError::ChecksumMismatch { version: 2 })
        ));
    }

    #[test]
    fn versions_are_unique_and_ordered() {
        let available = available();
        let versions: Vec<i64> = available.iter().map(|m| m.version).collect();
        assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn the_checksum_is_stable() {
        assert_eq!(checksum("abc"), checksum("abc"));
        assert_ne!(checksum("abc"), checksum("abd"));
    }
}
