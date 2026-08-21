---
name: logging
description: 'Refine this Rust interpreter\'s diagnostic logging for observability and debugging. Use for inserting, improving, or removing logln statements, choosing levels, reducing noise, and adding actionable state context around parser and runtime control flow.'
argument-hint: 'Describe whether to sweep the full codebase or specific modules.'
---

# Rust Logging Refinement

Inject, improve, or remove diagnostic logging in Rust code so debugging is faster, logs are meaningful, and log volume stays manageable. Preserve the existing logger and runtime behavior unless the user explicitly asks for logger design changes.

Primary goal: run add/refine/remove logging passes across all relevant files for the best debugging signal.

## When To Use

- Add logs to new features where runtime visibility is missing.
- Improve noisy or low-signal logs.
- Remove redundant, misleading, or stale logs that do not help debugging.
- Add diagnostics around parsing, branching, retries, loops, and error returns.
- Standardize level usage across modules.
- Review PRs for observability quality.
- Execute full-project logging sweeps for consistency.

Keywords: rust logging, observability, debugging, trace logs, log levels, error context, parser instrumentation, runtime diagnostics.

## Logging Contract

The repository uses a custom function-style logger:

```rust
#[derive(Debug, PartialEq, PartialOrd)]
pub enum LogLevel { Trace = 0, Info = 1, Warning = 2, Error = 3, Fatal = 4 }

pub fn logln(level: LogLevel, message: &str)
```

`logln` currently prints diagnostic messages to stdout. The effective threshold is `Trace` in tests and `Info` in non-test builds. There is no runtime filter, target/module metadata, structured-field API, or `log!` macro. Do not claim that those facilities exist or introduce a logging crate as part of an ordinary logging sweep.

JavaScript `console.log` is application output, not diagnostic logging. Keep its behavior and call sites conceptually separate from `logln`; future changes to the diagnostic sink belong behind the logger implementation.

## Level Rules

- Trace: Fine-grained parser and runtime details, expression lifecycle, loop checkpoints, branch tracing, and value dumps. Use it for high-frequency events.
- Info: High-level parser, compiler, function, or class boundaries; major state transitions; and meaningful successful milestones. Do not use it for routine expression-level entry/exit.
- Warning: Rare recoverable runtime or parser conditions, fallback paths, and handled unsupported cases. State what recovery or fallback occurred.
- Error: A definitive failure at its origin, immediately before returning an error or terminating a recoverable operation. Include the failed operation, location or identifier, and cause.
- Fatal: Immediately before an unrecoverable panic, abort, or assertion failure when the boundary is a genuine fatal diagnostic. Do not add Fatal merely because an internal error is propagated.

Log a failure at the origin where useful context is available. Do not repeat the same error at every propagation layer. A final public or host boundary may log it again only when it adds a distinct user-facing consequence or important context.

## Decision Points

1. Keep `logln` for ordinary changes. Logger routing, runtime configuration, metadata, and backend changes are separate design work.
2. Use `Trace` (not `Info`) inside tight loops and expression-level paths unless the event frequency is intentionally low and the event is a high-level milestone.
3. On error paths, include the failing operation, key identifiers or parser position, and why recovery did or did not happen.
4. Full `JsValue` and `Prototype` debug dumps are acceptable for this interpreter's debugging workflow when they materially help reproduce behavior.
5. Test-time `Trace` output is intentional; do not quiet it or change the test threshold as part of a logging refinement.

## Syntax Rules

For the existing function-style logger (`logln`):

- Dynamic fields: `logln(LogLevel::Info, &format!("Processing identifier={}", id));`
- Static message: `logln(LogLevel::Trace, "Starting parser loop");`

The current API accepts an already-built `&str`; eager `format!` allocation is a known tradeoff and is not a reason to redesign the logger during this skill's normal use.

## Message Naming

- Use `Entering ModuleName ...` and `Exiting ModuleName ...` for meaningful high-level execution or parse boundaries.
- Use `Trace` for expression-level lifecycle messages such as `Entering Expr::Assign`; do not promote routine expression entry/exit to `Info`.
- Keep the module or expression name immediately after `Entering` or `Exiting`, such as `Entering Expr::Assign` and `Exiting Expr::Assign key=name`.
- Keep `Error`, `Warning`, and branch-decision messages explicit instead of forcing them into entry/exit form.
- Use stable names across related boundaries; do not alternate between `parse_X`, `X::parse`, and `Entering X` for the same lifecycle event.

## Procedure

### Full Codebase Sweep (default for broad requests)

1. Enumerate Rust source files and group by subsystem. Treat the full-project sweep as the default for broad requests unless the user narrows the scope.
2. For each file, classify logs into keep, refine, remove, and missing-critical.
3. Apply add/refine/remove edits in one pass per file to avoid partial instrumentation.
4. Prioritize hot execution paths and failure origins first. Check panic/assert boundaries for missing Fatal diagnostics, but avoid duplicate logs for propagated errors.
5. Preserve the separation between diagnostic `logln` output and JavaScript `console.log` output.
6. Run compile/tests and fix logging-related issues before finalizing.

### Per-File Pass

1. Scan target function(s) and map control flow: entry, exits, loops, match arms, retries, and error returns.
2. Add entry `Info` logs only at meaningful high-level boundaries where parameters or mode materially affect behavior; use `Trace` for expression-level entry and exit.
3. Add `Trace` logs at high-frequency decision points and loop checkpoints.
4. Add `Warning` logs for recoverable anomalies and explain the fallback taken.
5. Add `Error` at definitive failure origins and `Fatal` before genuine unrecoverable panic/assert/abort boundaries. Do not duplicate a log solely because an error is being propagated.
6. Remove or downgrade redundant logs that repeat state without adding diagnostic value.
7. Ensure each log answers at least one of: what happened, where, with which state, and why.

## Completion Checks

- Every major failure origin has a nearby `Error` log with actionable context; propagated errors are not redundantly logged.
- Tight loops avoid `Info` log spam.
- Entry/exit `Info` logs exist only for meaningful high-level boundaries; expression-level lifecycle is `Trace`.
- Recoverable `Warning` logs are rare and state the recovery or fallback taken.
- Genuine fatal panic/assert/abort boundaries have a nearby `Fatal` log when that diagnostic is not already supplied at the origin.
- Redundant or stale logs are removed, not just left in place.
- Messages include state that helps reproduce or triage issues.
- Diagnostic logging remains conceptually separate from JavaScript `console.log` output.
- Logging changes preserve existing behavior and compile cleanly.
- A sweep summary reports changed files and add/refine/remove counts.

## Output Format

When applying this skill to user code:

1. Report the changed files and add/refine/remove counts.
2. Briefly explain why representative new or changed logs were placed at their levels.
3. Explicitly list removed or downgraded logs and why they were removed or changed.
4. Include focused snippets only when they clarify a non-obvious change; do not repeat entire updated files by default.