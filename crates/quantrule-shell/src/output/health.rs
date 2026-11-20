//! Health report rendering.

use std::io::{self, Write};

use quantrule_api::{HealthReport, ServiceState};

/// Render the aggregate health report.
pub(crate) fn health(writer: &mut dyn Write, report: &HealthReport) -> io::Result<()> {
    writeln!(writer, "api: {}", report.v)?;
    writeln!(writer, "state: {}", service_state(report.state))?;
    writeln!(writer, "queue_depth: {}", report.queue_depth)?;
    writeln!(writer, "workers: {}", report.workers)?;
    writeln!(
        writer,
        "image_digest: {}",
        report
            .image_digest
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    let mut services = report.services.clone();
    services.sort_by(|left, right| left.name.cmp(&right.name));
    for service in services {
        match service.detail {
            Some(detail) => writeln!(
                writer,
                "service {}: {} ({detail})",
                service.name,
                service_state(service.state)
            )?,
            None => writeln!(
                writer,
                "service {}: {}",
                service.name,
                service_state(service.state)
            )?,
        }
    }
    Ok(())
}
fn service_state(state: ServiceState) -> &'static str {
    match state {
        ServiceState::Healthy => "healthy",
        ServiceState::Degraded => "degraded",
        ServiceState::Down => "down",
    }
}
