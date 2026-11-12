//! The desktop path scope and the webview filesystem boundary.

use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use quantrule_app::memory::{MemoryError, ResearchMemory};

pub(super) fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[derive(Debug, Default, Clone)]
pub(super) struct Scope {
    workspace: Option<PathBuf>,
    memory: Option<PathBuf>,
}

pub(super) static SCOPE: Mutex<Scope> = Mutex::new(Scope {
    workspace: None,
    memory: None,
});

fn scope() -> Result<Scope, String> {
    SCOPE
        .lock()
        .map_err(|_| String::from("the desktop path scope is unavailable"))
        .map(|scope| scope.clone())
}

pub(super) fn set_workspace(path: PathBuf) -> Result<(), String> {
    SCOPE
        .lock()
        .map_err(|_| String::from("the desktop path scope is unavailable"))?
        .workspace = Some(path);
    Ok(())
}

pub(super) fn set_memory(path: PathBuf) -> Result<(), String> {
    SCOPE
        .lock()
        .map_err(|_| String::from("the desktop path scope is unavailable"))?
        .memory = Some(path);
    Ok(())
}

fn normalize(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(text)?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    Ok(normalized)
}

pub(super) fn validate_workspace_path(path: &str, label: &str) -> Result<PathBuf, String> {
    let root = scope()?
        .workspace
        .ok_or_else(|| String::from("open a workspace first"))?;
    let path = normalize(path)?;
    if !path.starts_with(&root) {
        return Err(format!(
            "{label} {} is outside the open workspace {}",
            path.display(),
            root.display()
        ));
    }
    Ok(path)
}

pub(super) fn validate_output_path(path: &str) -> Result<PathBuf, String> {
    let scope = scope()?;
    let path = normalize(path)?;
    let in_workspace = scope
        .workspace
        .as_ref()
        .is_some_and(|root| path.starts_with(root));
    let in_memory = scope
        .memory
        .as_ref()
        .and_then(|db| db.parent())
        .is_some_and(|parent| path.starts_with(parent));
    if !in_workspace && !in_memory {
        return Err(format!(
            "path {} is outside the workspace and the memory directory",
            path.display()
        ));
    }
    Ok(path)
}

pub(super) fn validate_memory_db(db: &str) -> Result<PathBuf, String> {
    let configured = scope()?
        .memory
        .ok_or_else(|| String::from("open the memory first"))?;
    let db = normalize(db)?;
    if db != configured {
        return Err(format!(
            "memory database {} is not the open memory {}",
            db.display(),
            configured.display()
        ));
    }
    Ok(db)
}

pub(super) fn open_memory(db: &str) -> Result<ResearchMemory, MemoryError> {
    let db = validate_memory_db(db).map_err(MemoryError::InvalidRecord)?;
    ResearchMemory::open(&db)
}
