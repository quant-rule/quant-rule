//! Deterministic loopback port allocation.
//!
//! The supervisor prefers a fixed port per service and walks a small forward
//! window when the preferred port is taken. It holds every candidate listener
//! until the whole plan is chosen, so two services in the same plan can never
//! be assigned the same port. The listeners are dropped immediately before the
//! children start; the plan is not a reservation beyond that moment, which is
//! why a busy port is retried rather than assumed free.
//!
//! Loopback only. The desktop profile is never reachable from another machine.

use std::net::TcpListener;

/// The five ports the gateway and the four HTTP services bind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortPlan {
    /// The public gateway port.
    pub gateway: u16,
    /// `qr-data`.
    pub data: u16,
    /// `qr-model`.
    pub model: u16,
    /// `qr-evidence`.
    pub evidence: u16,
    /// `qr-simulation`.
    pub simulation: u16,
}

impl PortPlan {
    /// The port for one service, or `None` for the worker.
    #[must_use]
    pub fn port_for(self, name: &str) -> Option<u16> {
        match name {
            "qr-gateway" => Some(self.gateway),
            "qr-data" => Some(self.data),
            "qr-model" => Some(self.model),
            "qr-evidence" => Some(self.evidence),
            "qr-simulation" => Some(self.simulation),
            _ => None,
        }
    }
}

/// Why no port plan could be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortError {
    /// The preferred port whose window was exhausted.
    pub preferred: u16,
    /// The width of the window that was tried.
    pub width: u16,
}

impl std::fmt::Display for PortError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "no free loopback port in {}..{}",
            self.preferred,
            u32::from(self.preferred) + u32::from(self.width)
        )
    }
}

impl std::error::Error for PortError {}

/// The number of candidates tried from each preferred port.
pub const FALLBACK_WIDTH: u16 = 10;

/// Build a plan, preferring `gateway` and `internal_base`.
///
/// # Errors
///
/// Returns [`PortError`] when a window is fully occupied. The caller retries
/// for a short window because children of a previous supervisor may still be
/// exiting.
pub fn allocate(gateway: u16, internal_base: u16) -> Result<PortPlan, PortError> {
    let mut held = Vec::new();
    let gateway = reserve(gateway, &mut held)?;
    let data = reserve(internal_base, &mut held)?;
    let model = reserve(internal_base.saturating_add(1), &mut held)?;
    let evidence = reserve(internal_base.saturating_add(2), &mut held)?;
    let simulation = reserve(internal_base.saturating_add(3), &mut held)?;
    Ok(PortPlan {
        gateway,
        data,
        model,
        evidence,
        simulation,
    })
}

fn reserve(preferred: u16, held: &mut Vec<TcpListener>) -> Result<u16, PortError> {
    for offset in 0..FALLBACK_WIDTH {
        let Some(port) = preferred.checked_add(offset) else {
            break;
        };
        if port == 0 {
            continue;
        }
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
            held.push(listener);
            return Ok(port);
        }
    }
    Err(PortError {
        preferred,
        width: FALLBACK_WIDTH,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::Mutex;

    /// A block of twenty ports that no other test in this process will use.
    ///
    /// The tests run in parallel; a shared scan for a free window would hand
    /// two tests the same ports and make the results depend on timing.
    fn free_block() -> u16 {
        static NEXT: Mutex<u16> = Mutex::new(0);
        let mut next = NEXT.lock().unwrap_or_else(|error| error.into_inner());
        if *next == 0 {
            *next = 20_000 + u16::try_from(std::process::id() % 64).unwrap_or(0) * 20;
        }
        loop {
            let base = *next;
            if base > 40_000 {
                panic!("no free port block");
            }
            let listeners: Vec<_> = (0..20)
                .filter_map(|offset| TcpListener::bind(("127.0.0.1", base + offset)).ok())
                .collect();
            let complete = listeners.len() == 20;
            drop(listeners);
            *next = base + 20;
            if complete {
                return base;
            }
        }
    }

    #[test]
    fn the_preferred_ports_are_used_when_free() {
        let base = free_block();
        let plan = allocate(base, base + 10).expect("plan");
        assert_eq!(plan.gateway, base);
        assert_eq!(plan.data, base + 10);
        assert_eq!(plan.model, base + 11);
        assert_eq!(plan.evidence, base + 12);
        assert_eq!(plan.simulation, base + 13);
    }

    #[test]
    fn a_taken_gateway_falls_forward() {
        let base = free_block();
        let listener = TcpListener::bind(("127.0.0.1", base)).expect("hold");
        let plan = allocate(base, base + 10).expect("plan");
        assert_eq!(plan.gateway, base + 1);
        drop(listener);
    }

    #[test]
    fn internal_ports_never_collide() {
        let base = free_block();
        let listener = TcpListener::bind(("127.0.0.1", base + 10)).expect("hold");
        let plan = allocate(base, base + 10).expect("plan");
        let ports = [
            plan.gateway,
            plan.data,
            plan.model,
            plan.evidence,
            plan.simulation,
        ];
        let mut unique = ports.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ports.len());
        assert_ne!(plan.data, base + 10);
        drop(listener);
    }

    #[test]
    fn an_exhausted_window_is_an_error() {
        let base = free_block();
        let listeners: Vec<_> = (0..FALLBACK_WIDTH)
            .filter_map(|offset| TcpListener::bind(("127.0.0.1", base + offset)).ok())
            .collect();
        assert_eq!(listeners.len(), usize::from(FALLBACK_WIDTH));
        assert!(allocate(base, base).is_err());
        drop(listeners);
    }

    #[test]
    fn a_service_name_maps_to_its_port() {
        let plan = PortPlan {
            gateway: 1,
            data: 2,
            model: 3,
            evidence: 4,
            simulation: 5,
        };
        assert_eq!(plan.port_for("qr-gateway"), Some(1));
        assert_eq!(plan.port_for("qr-backtest"), None);
        assert_eq!(plan.port_for("qr-simulation"), Some(5));
    }
}
