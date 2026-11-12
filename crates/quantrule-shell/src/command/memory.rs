//! The `memory` command family.
//!
//! The shell exposes the local research memory through the application service.
//! This module owns the command grammar and the deterministic text rendering; the
//! store owns validation, ordering, and the export/backup rules. The record
//! command computes the model through `AppService` and stores its counts with
//! the run, so the agent and desktop surfaces record the same evidence.

use std::io::Write;
use std::path::PathBuf;

use quantrule_app::memory::{
    NewClaim, NewNote, ResearchMemory, backup, export_to_file, import_json, restore,
};
use quantrule_app::service::{AppService, RecordRunInput};

use super::types::{Command, MemoryArgs, MetricArg, ParseError, RunError};

const USAGE: &str = "usage: memory status|workspaces|runs|record|metrics|metric set|metric delete|claims|claim add|claim delete|notes|note add|note update|note delete|export|import|backup|restore ...";

/// Parses the arguments after `memory`.
///
/// # Errors
///
/// Returns [`ParseError::Memory`] for an unknown subcommand, a missing required
/// flag, a duplicate flag, or a value of the wrong type.
pub(crate) fn parse_memory_arguments(arguments: &[String]) -> Result<Command, ParseError> {
    let Some((family, rest)) = arguments.split_first() else {
        return Err(usage("a memory subcommand is required"));
    };
    match family.as_str() {
        "status" => {
            let flags = flags(rest)?;
            only(&flags, &["db"])?;
            Ok(Command::Memory(MemoryArgs::Status {
                db: path(&flags, "db")?,
            }))
        }
        "record" => {
            let flags = flags(rest)?;
            only(
                &flags,
                &["db", "config", "key", "status", "started", "finished"],
            )?;
            Ok(Command::Memory(MemoryArgs::Record {
                db: path(&flags, "db")?,
                config: path(&flags, "config")?,
                run_key: value(&flags, "key")?,
                status: optional_value(&flags, "status")
                    .unwrap_or_else(|| String::from("complete")),
                started_at: integer(&flags, "started")?,
                finished_at: optional_integer(&flags, "finished")?,
            }))
        }
        "workspaces" => {
            let flags = flags(rest)?;
            only(&flags, &["db"])?;
            Ok(Command::Memory(MemoryArgs::Workspaces {
                db: path(&flags, "db")?,
            }))
        }
        "runs" => {
            let flags = flags(rest)?;
            only(&flags, &["db", "workspace"])?;
            Ok(Command::Memory(MemoryArgs::Runs {
                db: path(&flags, "db")?,
                workspace_id: integer(&flags, "workspace")?,
            }))
        }
        "metrics" => {
            let flags = flags(rest)?;
            only(&flags, &["db", "run"])?;
            Ok(Command::Memory(MemoryArgs::Metrics {
                db: path(&flags, "db")?,
                run_id: integer(&flags, "run")?,
            }))
        }
        "metric" => {
            let Some((action, rest)) = rest.split_first() else {
                return Err(usage("memory metric requires set or delete"));
            };
            match action.as_str() {
                "set" => {
                    let flags = flags(rest)?;
                    only(&flags, &["db", "run", "name", "kind", "value", "text"])?;
                    Ok(Command::Memory(MemoryArgs::MetricSet {
                        db: path(&flags, "db")?,
                        run_id: integer(&flags, "run")?,
                        name: value(&flags, "name")?,
                        value: metric_arg(&flags)?,
                    }))
                }
                "delete" => {
                    let flags = flags(rest)?;
                    only(&flags, &["db", "run", "name"])?;
                    Ok(Command::Memory(MemoryArgs::MetricDelete {
                        db: path(&flags, "db")?,
                        run_id: integer(&flags, "run")?,
                        name: value(&flags, "name")?,
                    }))
                }
                other => Err(usage(format!(
                    "unknown memory metric action '{other}'; use set or delete"
                ))),
            }
        }
        "claims" => {
            let flags = flags(rest)?;
            only(&flags, &["db", "run"])?;
            Ok(Command::Memory(MemoryArgs::Claims {
                db: path(&flags, "db")?,
                run_id: integer(&flags, "run")?,
            }))
        }
        "claim" => {
            let Some((action, rest)) = rest.split_first() else {
                return Err(usage("memory claim requires add or delete"));
            };
            match action.as_str() {
                "add" => {
                    let flags = flags(rest)?;
                    only(
                        &flags,
                        &["db", "run", "type", "statement", "status", "created"],
                    )?;
                    Ok(Command::Memory(MemoryArgs::ClaimAdd {
                        db: path(&flags, "db")?,
                        run_id: integer(&flags, "run")?,
                        claim_type: value(&flags, "type")?,
                        statement: value(&flags, "statement")?,
                        status: value(&flags, "status")?,
                        created_at: integer(&flags, "created")?,
                    }))
                }
                "delete" => {
                    let flags = flags(rest)?;
                    only(&flags, &["db", "id"])?;
                    Ok(Command::Memory(MemoryArgs::ClaimDelete {
                        db: path(&flags, "db")?,
                        id: integer(&flags, "id")?,
                    }))
                }
                other => Err(usage(format!(
                    "unknown memory claim action '{other}'; use add or delete"
                ))),
            }
        }
        "notes" => {
            let flags = flags(rest)?;
            only(&flags, &["db", "workspace", "run"])?;
            let workspace_id = optional_integer(&flags, "workspace")?;
            let run_id = optional_integer(&flags, "run")?;
            if workspace_id.is_some() == run_id.is_some() {
                return Err(usage(
                    "memory notes requires exactly one of --workspace or --run",
                ));
            }
            Ok(Command::Memory(MemoryArgs::Notes {
                db: path(&flags, "db")?,
                workspace_id,
                run_id,
            }))
        }
        "note" => {
            let Some((action, rest)) = rest.split_first() else {
                return Err(usage("memory note requires add, update, or delete"));
            };
            match action.as_str() {
                "add" => {
                    let flags = flags(rest)?;
                    only(
                        &flags,
                        &["db", "workspace", "run", "body", "created", "updated"],
                    )?;
                    let created_at = integer(&flags, "created")?;
                    let updated_at = optional_integer(&flags, "updated")?.unwrap_or(created_at);
                    Ok(Command::Memory(MemoryArgs::NoteAdd {
                        db: path(&flags, "db")?,
                        workspace_id: integer(&flags, "workspace")?,
                        run_id: optional_integer(&flags, "run")?,
                        body: value(&flags, "body")?,
                        created_at,
                        updated_at,
                    }))
                }
                "update" => {
                    let flags = flags(rest)?;
                    only(&flags, &["db", "id", "body", "updated"])?;
                    Ok(Command::Memory(MemoryArgs::NoteUpdate {
                        db: path(&flags, "db")?,
                        id: integer(&flags, "id")?,
                        body: value(&flags, "body")?,
                        updated_at: integer(&flags, "updated")?,
                    }))
                }
                "delete" => {
                    let flags = flags(rest)?;
                    only(&flags, &["db", "id"])?;
                    Ok(Command::Memory(MemoryArgs::NoteDelete {
                        db: path(&flags, "db")?,
                        id: integer(&flags, "id")?,
                    }))
                }
                other => Err(usage(format!(
                    "unknown memory note action '{other}'; use add, update, or delete"
                ))),
            }
        }
        "export" => {
            let flags = flags(rest)?;
            only(&flags, &["db", "out"])?;
            Ok(Command::Memory(MemoryArgs::Export {
                db: path(&flags, "db")?,
                out: path(&flags, "out")?,
            }))
        }
        "import" => {
            let flags = flags(rest)?;
            only(&flags, &["db", "from"])?;
            Ok(Command::Memory(MemoryArgs::Import {
                db: path(&flags, "db")?,
                from: path(&flags, "from")?,
            }))
        }
        "backup" => {
            let flags = flags(rest)?;
            only(&flags, &["db", "out"])?;
            Ok(Command::Memory(MemoryArgs::Backup {
                db: path(&flags, "db")?,
                out: path(&flags, "out")?,
            }))
        }
        "restore" => {
            let flags = flags(rest)?;
            only(&flags, &["backup", "target"])?;
            Ok(Command::Memory(MemoryArgs::Restore {
                backup: path(&flags, "backup")?,
                target: path(&flags, "target")?,
            }))
        }
        other => Err(usage(format!("unknown memory subcommand '{other}'"))),
    }
}

fn flags(arguments: &[String]) -> Result<Vec<(String, String)>, ParseError> {
    let mut parsed = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        let key = arguments[index]
            .strip_prefix("--")
            .filter(|key| !key.is_empty())
            .ok_or_else(|| usage(format!("expected a --flag, found '{}'", arguments[index])))?;
        if parsed
            .iter()
            .any(|(name, _): &(String, String)| name == key)
        {
            return Err(usage(format!("--{key} was supplied twice")));
        }
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| usage(format!("--{key} requires a value")))?;
        parsed.push((key.to_owned(), value.clone()));
        index += 2;
    }
    Ok(parsed)
}

fn only(flags: &[(String, String)], allowed: &[&str]) -> Result<(), ParseError> {
    for (name, _) in flags {
        if !allowed.contains(&name.as_str()) {
            return Err(usage(format!("unknown option --{name}")));
        }
    }
    Ok(())
}

fn optional_value(flags: &[(String, String)], name: &str) -> Option<String> {
    flags
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
}

fn value(flags: &[(String, String)], name: &str) -> Result<String, ParseError> {
    optional_value(flags, name).ok_or_else(|| usage(format!("--{name} is required")))
}

fn path(flags: &[(String, String)], name: &str) -> Result<PathBuf, ParseError> {
    value(flags, name).map(PathBuf::from)
}

fn integer(flags: &[(String, String)], name: &str) -> Result<i64, ParseError> {
    value(flags, name)?
        .parse()
        .map_err(|_| usage(format!("--{name} must be an integer")))
}

fn optional_integer(flags: &[(String, String)], name: &str) -> Result<Option<i64>, ParseError> {
    optional_value(flags, name)
        .map(|value| {
            value
                .parse()
                .map_err(|_| usage(format!("--{name} must be an integer")))
        })
        .transpose()
}

fn metric_arg(flags: &[(String, String)]) -> Result<MetricArg, ParseError> {
    let kind = value(flags, "kind")?;
    let real = optional_value(flags, "value");
    let text = optional_value(flags, "text");
    match kind.as_str() {
        "real" => {
            let value = real.ok_or_else(|| usage("--value is required for kind real"))?;
            if text.is_some() {
                return Err(usage("--text is not used for kind real"));
            }
            let number: f64 = value
                .parse()
                .map_err(|_| usage("--value must be a number for kind real"))?;
            if !number.is_finite() {
                return Err(usage("--value must be finite"));
            }
            Ok(MetricArg::Real(number))
        }
        "integer" => {
            if real.is_some() {
                return Err(usage("--value is not used for kind integer; use --text"));
            }
            let value = text.ok_or_else(|| usage("--text is required for kind integer"))?;
            let number: i64 = value
                .parse()
                .map_err(|_| usage("--text must be an integer for kind integer"))?;
            Ok(MetricArg::Integer(number))
        }
        "text" => {
            if real.is_some() {
                return Err(usage("--value is not used for kind text; use --text"));
            }
            let value = text.ok_or_else(|| usage("--text is required for kind text"))?;
            Ok(MetricArg::Text(value))
        }
        "null" => {
            if real.is_some() || text.is_some() {
                return Err(usage("kind null takes no --value or --text"));
            }
            Ok(MetricArg::Null)
        }
        other => Err(usage(format!(
            "kind must be real, integer, text, or null, not '{other}'"
        ))),
    }
}

fn usage(message: impl Into<String>) -> ParseError {
    ParseError::Memory(format!("{}\n{USAGE}", message.into()))
}

/// Executes one `memory` command.
///
/// # Errors
///
/// Returns [`RunError::Memory`] for a store failure, [`RunError::App`] for a
/// run/model failure, and [`RunError::Output`] when the writer fails.
pub(crate) fn run_memory(args: &MemoryArgs, writer: &mut impl Write) -> Result<(), RunError> {
    match args {
        MemoryArgs::Status { db } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            writeln!(
                writer,
                "memory schema={} version={} path={}",
                memory.schema().map_err(RunError::Memory)?,
                memory.version().map_err(RunError::Memory)?,
                db.display()
            )
            .map_err(RunError::Output)?;
        }
        MemoryArgs::Record {
            db,
            config,
            run_key,
            status,
            started_at,
            finished_at,
        } => {
            let service = AppService::open_for_config(config).map_err(RunError::App)?;
            let outcome = service.run_model(config).map_err(RunError::App)?;
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            let recorded = service
                .record_run_with_model(
                    &memory,
                    RecordRunInput {
                        config,
                        run_key,
                        status,
                        started_at: *started_at,
                        finished_at: *finished_at,
                        model: Some(&outcome.summary),
                    },
                )
                .map_err(RunError::App)?;
            writeln!(
                writer,
                "recorded workspace={} dataset={} config={} run={}",
                recorded.workspace_id, recorded.dataset_id, recorded.config_id, recorded.run_id
            )
            .map_err(RunError::Output)?;
        }
        MemoryArgs::Workspaces { db } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            for record in memory.workspaces().map_err(RunError::Memory)? {
                writeln!(
                    writer,
                    "workspace {} {} {}",
                    record.id, record.name, record.root
                )
                .map_err(RunError::Output)?;
            }
        }
        MemoryArgs::Runs { db, workspace_id } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            for record in memory.runs(*workspace_id).map_err(RunError::Memory)? {
                writeln!(
                    writer,
                    "run {} {} {} {}",
                    record.id, record.run_key, record.status, record.data_version
                )
                .map_err(RunError::Output)?;
            }
        }
        MemoryArgs::Metrics { db, run_id } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            for record in memory.metrics(*run_id).map_err(RunError::Memory)? {
                writeln!(
                    writer,
                    "metric {} {} {} {}",
                    record.id,
                    record.name,
                    metric_kind(&record.value),
                    metric_text(&record.value)
                )
                .map_err(RunError::Output)?;
            }
        }
        MemoryArgs::MetricSet {
            db,
            run_id,
            name,
            value,
        } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            let metric_value = metric_value(value);
            let id = memory
                .upsert_metric(*run_id, name, &metric_value)
                .map_err(RunError::Memory)?;
            writeln!(writer, "metric {id}").map_err(RunError::Output)?;
        }
        MemoryArgs::MetricDelete { db, run_id, name } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            let deleted = memory
                .delete_metric(*run_id, name)
                .map_err(RunError::Memory)?;
            writeln!(writer, "deleted {deleted}").map_err(RunError::Output)?;
        }
        MemoryArgs::Claims { db, run_id } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            for record in memory.claims(*run_id).map_err(RunError::Memory)? {
                writeln!(
                    writer,
                    "claim {} {} {} {}",
                    record.id, record.claim_type, record.status, record.statement
                )
                .map_err(RunError::Output)?;
            }
        }
        MemoryArgs::ClaimAdd {
            db,
            run_id,
            claim_type,
            statement,
            status,
            created_at,
        } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            let id = memory
                .insert_claim(NewClaim {
                    run_id: *run_id,
                    claim_type,
                    statement,
                    status,
                    created_at: *created_at,
                })
                .map_err(RunError::Memory)?;
            writeln!(writer, "claim {id}").map_err(RunError::Output)?;
        }
        MemoryArgs::ClaimDelete { db, id } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            let deleted = memory.delete_claim(*id).map_err(RunError::Memory)?;
            writeln!(writer, "deleted {deleted}").map_err(RunError::Output)?;
        }
        MemoryArgs::Notes {
            db,
            workspace_id,
            run_id,
        } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            let records = match (workspace_id, run_id) {
                (Some(workspace), None) => memory.notes(*workspace).map_err(RunError::Memory)?,
                (None, Some(run)) => memory.notes_for_run(*run).map_err(RunError::Memory)?,
                _ => {
                    return Err(RunError::Help(String::from(
                        "memory notes requires exactly one of workspace or run",
                    )));
                }
            };
            for record in records {
                writeln!(
                    writer,
                    "note {} {} {} {} {}",
                    record.id,
                    record
                        .run_id
                        .map_or_else(|| String::from("-"), |id| id.to_string()),
                    record.created_at,
                    record.updated_at,
                    record.body
                )
                .map_err(RunError::Output)?;
            }
        }
        MemoryArgs::NoteAdd {
            db,
            workspace_id,
            run_id,
            body,
            created_at,
            updated_at,
        } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            let id = memory
                .insert_note(NewNote {
                    workspace_id: *workspace_id,
                    run_id: *run_id,
                    body,
                    created_at: *created_at,
                    updated_at: *updated_at,
                })
                .map_err(RunError::Memory)?;
            writeln!(writer, "note {id}").map_err(RunError::Output)?;
        }
        MemoryArgs::NoteUpdate {
            db,
            id,
            body,
            updated_at,
        } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            let updated = memory
                .update_note(*id, body, *updated_at)
                .map_err(RunError::Memory)?;
            writeln!(writer, "updated {updated}").map_err(RunError::Output)?;
        }
        MemoryArgs::NoteDelete { db, id } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            let deleted = memory.delete_note(*id).map_err(RunError::Memory)?;
            writeln!(writer, "deleted {deleted}").map_err(RunError::Output)?;
        }
        MemoryArgs::Export { db, out } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            export_to_file(&memory, out).map_err(RunError::Memory)?;
            writeln!(writer, "exported {}", out.display()).map_err(RunError::Output)?;
        }
        MemoryArgs::Import { db, from } => {
            let document = std::fs::read_to_string(from).map_err(|source| {
                RunError::Memory(quantrule_app::memory::MemoryError::Io {
                    path: from.clone(),
                    source,
                })
            })?;
            let value: serde_json::Value = serde_json::from_str(&document).map_err(|error| {
                RunError::Memory(quantrule_app::memory::MemoryError::Json(error))
            })?;
            let mut memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            import_json(&mut memory, &value).map_err(RunError::Memory)?;
            writeln!(writer, "imported {}", from.display()).map_err(RunError::Output)?;
        }
        MemoryArgs::Backup { db, out } => {
            let memory = ResearchMemory::open(db).map_err(RunError::Memory)?;
            backup(&memory, out).map_err(RunError::Memory)?;
            writeln!(writer, "backed_up {}", out.display()).map_err(RunError::Output)?;
        }
        MemoryArgs::Restore { backup, target } => {
            let memory = restore(backup, target).map_err(RunError::Memory)?;
            drop(memory);
            writeln!(writer, "restored {}", target.display()).map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}

fn metric_kind(value: &quantrule_app::memory::MetricValue) -> &'static str {
    value.kind()
}

fn metric_text(value: &quantrule_app::memory::MetricValue) -> String {
    match value {
        quantrule_app::memory::MetricValue::Real(number) => number.to_string(),
        quantrule_app::memory::MetricValue::Integer(number) => number.to_string(),
        quantrule_app::memory::MetricValue::Text(text) => text.clone(),
        quantrule_app::memory::MetricValue::Null => String::from("null"),
    }
}

fn metric_value(value: &MetricArg) -> quantrule_app::memory::MetricValue {
    match value {
        MetricArg::Real(number) => quantrule_app::memory::MetricValue::Real(*number),
        MetricArg::Integer(number) => quantrule_app::memory::MetricValue::Integer(*number),
        MetricArg::Text(text) => quantrule_app::memory::MetricValue::Text(text.clone()),
        MetricArg::Null => quantrule_app::memory::MetricValue::Null,
    }
}
