# One Program, Two First-Class Surfaces

- Date: 2026-11-11 (UTC)
- Status: Accepted
- Claim type: ADR (Architecture Decision)
- Deciders: Human
- Related: [`app-service-memory-audit.md`](../../audits/2026-11-11/app-service-memory-audit.md), [`native-tauri-window-audit.md`](../../audits/2026-11-11/native-tauri-window-audit.md), [`audit-app-service-native-remediation.md`](../../execution-plans/2026-11-11/audit-app-service-native-remediation.md)

## Context

v0.9.0 ships a Tauri desktop UI and an interactive command shell. Both are local
and single-user. The application service (`quantrule-app`) owns workspace,
dataset, configuration, model-run, export, and research-memory use cases. The
shell still owns the orchestration for the data inspection, event, report,
strategy, execution, portfolio, backtest, qualification, and analytics stages.
The desktop UI cannot reach those stages, and the shell cannot reach the
research memory.

The 2026-11-11 audits found the resulting gap: the GUI and the shell are not
feature-equivalent, and adding the missing GUI stages directly to the Tauri
commands would duplicate the shell. The human decided that the desktop UI is a
first-class surface, equal to the shell, and that the two must remain one
program.

## Decision

1. The desktop UI and the command shell are two first-class surfaces of the same
   program. The human can run both. The shell is primarily for agents; the
   desktop UI is the human workstation.
2. Both surfaces call one shared use-case layer. A capability is implemented
   once and exposed by both adapters; neither adapter duplicates domain logic,
   path resolution, configuration parsing, or orchestration.
3. The shared application service is the home of the use cases. The shell keeps
   command parsing, terminal I/O, explicit source fetch, and rendering. The
   desktop keeps windowing, IPC, and rendering. The engines stay pure.
4. Feature parity is the target. Until a capability is exposed from both
   adapters, it is a known parity gap, not a completed capability. The parity
   work is tracked by the audit remediation plan.
5. The shell command set remains the stable agent interface. The desktop UI may
   present the same use cases with a different interaction, but it must not
   grow a second, divergent implementation.
6. Explicit source fetch is a network boundary. If the desktop UI is to expose
   it, that is a separate product/security decision; until then, source
   acquisition remains shell-only with a recorded exception. The local-first
   promise must not be weakened silently.

## Alternatives considered

| Option | Consequence | Verdict |
|---|---|---|
| Keep the shell as the only surface | Rejects the human decision and the released desktop UI | Rejected |
| Keep the GUI as a partial viewer | The GUI is not first-class; the two surfaces drift | Rejected |
| Have the GUI spawn the shell binary | The GUI depends on the CLI; the "same program" and "does not spawn the CLI" boundaries fail | Rejected |
| Duplicate the pipeline orchestration in Tauri commands | Two implementations of the same rules; guaranteed drift | Rejected |
| Make the shell binary a library and call its command engine from the GUI | One source, but it inverts the application-service boundary and keeps the CLI parser in the GUI | Rejected as the target; a temporary bridge is not the architecture |
| Extract the use cases behind the application service and make both adapters thin | One source, explicit boundary, parity possible; largest change | Chosen |

## Consequences

- The shell's stage orchestration moves behind the shared application service
  (or a shared use-case layer the service owns). The shell becomes an adapter
  for parsing, terminal I/O, fetch, and rendering; the desktop becomes an
  adapter for windowing, IPC, and rendering.
- The research memory's metric, claim, and note CRUD must be reachable from both
  surfaces, not only from the service tests and the GUI read views.
- The application service gains explicit use-case methods and tests; the
  architecture checker records the new edges. No engine gains an I/O or a
  presentation dependency.
- The GUI and shell need a parity test. A capability is not claimed as shipped
  until both adapters expose it.
- The desktop UI is a first-class distribution surface. Its native window needs
  a native IPC E2E, path scoping, and the same release gate as the shell.
- The `AGENTS.md` shell-only discovery rule is superseded: a capability is
  reached from a first-class surface, and parity is required before it is
  called complete.
- Source fetch remains shell-only until a separate network-boundary decision.
- The work is large; it is delivered in phases under the remediation plan, and
  the plan is not `Verified` while a parity finding is open.

## References

- [Application service and research memory audit](../../audits/2026-11-11/app-service-memory-audit.md)
- [Native Tauri window audit](../../audits/2026-11-11/native-tauri-window-audit.md)
- [Audit remediation plan](../../execution-plans/2026-11-11/audit-app-service-native-remediation.md)
- [Architecture baseline](../2026-10-31/adopt-architecture-baseline.md)
