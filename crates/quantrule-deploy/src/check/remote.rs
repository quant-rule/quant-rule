//! Opt-in remote and overlay inspectors for a v3 check.
//!
//! The default check is local plus one HTTPS ingress probe. `--remote docker`
//! and `--overlay` are explicit opt-ins. Their real implementation is the
//! PLAT-001/PLAT-006 seam; until it lands they return one clear unsupported
//! row rather than pretending to inspect a stack they cannot reach.

use super::report::{CheckReport, CheckRow};

/// One opt-in inspector.
pub trait Inspector {
    /// Inspect the deployment and return rows, or one stable unsupported code.
    ///
    /// # Errors
    ///
    /// Returns a human-readable failure that names the missing seam.
    fn inspect(&self) -> Result<Vec<CheckRow>, String>;
}

/// The Docker service and placement inspector, pending PLAT-001.
#[derive(Debug, Default, Clone, Copy)]
pub struct DockerInspector;

impl Inspector for DockerInspector {
    fn inspect(&self) -> Result<Vec<CheckRow>, String> {
        Err(
            "deploy.remote.unsupported: the Docker inspector needs the PLAT-001 remote seam; \
             the local check and the HTTPS ingress probe are complete"
                .to_owned(),
        )
    }
}

/// The encrypted-overlay inspector, pending PLAT-001.
#[derive(Debug, Default, Clone, Copy)]
pub struct OverlayInspector;

impl Inspector for OverlayInspector {
    fn inspect(&self) -> Result<Vec<CheckRow>, String> {
        Err(
            "deploy.overlay.unsupported: the overlay inspector needs the PLAT-001 remote seam; \
             bootstrap.sh refuses a network that is not attachable and encrypted"
                .to_owned(),
        )
    }
}

/// Append one inspector's rows, or its unsupported failure.
pub fn append(
    report: &mut CheckReport,
    group: &'static str,
    subject: &'static str,
    inspector: &dyn Inspector,
) {
    match inspector.inspect() {
        Ok(rows) => {
            for row in rows {
                report.push(row);
            }
        }
        Err(error) => report.push(CheckRow::failed(group, subject, "unsupported", error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl Inspector for Fake {
        fn inspect(&self) -> Result<Vec<CheckRow>, String> {
            Ok(vec![CheckRow::ok("remote", "services", "ok", "3 replicas")])
        }
    }

    #[test]
    fn an_inspector_is_injectable() {
        let mut report = CheckReport::new("swarm-ha", "HA profile (unverified)", "img");
        append(&mut report, "remote", "docker", &Fake);
        assert_eq!(report.rows()[0].state, "ok");
        assert!(!report.failed());
    }

    #[test]
    fn the_default_docker_inspector_is_a_clear_failure() {
        let error = DockerInspector.inspect().expect_err("unsupported");
        assert!(error.contains("deploy.remote.unsupported"), "{error}");
    }
}
