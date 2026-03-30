---
description: "Use when editing Rust backend code in crates/**/*.rs, including web-app APIs, worker jobs, provider adapters, SQLx queries, logging, and error handling."
name: "Rust Backend Rules"
applyTo: "crates/**/*.rs"
---

# Rust Backend Rules

## Scope

- Applies to Rust source files under crates/.
- Prefer minimal, localized changes that preserve existing public behavior unless requested.

## Error Handling

- Use typed errors for domain/application boundaries (`thiserror`), and `anyhow` for top-level orchestration only.
- Do not swallow errors. Add context before propagating (`.context(...)` when using anyhow).
- Return actionable errors from adapters and worker jobs so operators can diagnose failures quickly.
- In API handlers, map internal errors to stable user-facing error messages; avoid leaking secrets or internal topology.

## Logging

- Use structured `tracing` logs with key-value fields.
- Log lifecycle milestones for long-running jobs (start, retry, success, failure, cleanup).
- Include correlation identifiers when available (`order_id`, `subscription_id`, `invoice_id`, `job_id`, `provider_node`).
- Never log secrets, tokens, password hashes, webhook signatures, or full payment payloads.

## Async and Concurrency

- Keep async functions cancellation-safe and idempotent where possible.
- For provisioning/renewal pipelines, design retry-safe steps and guard against duplicate execution.
- Avoid blocking calls in async contexts; if unavoidable, isolate them behind dedicated boundaries.
- Prefer explicit timeouts/retry policies for external provider or payment calls.

## SQLx and Query Style

- Keep SQL compatible with SQLite unless explicitly asked to target another database.
- Use parameterized queries; never build SQL by string concatenation with untrusted input.
- Co-locate schema changes with migration files in migrations/.
- When changing schema, update Rust query models and compile checks in the same change.
- Preserve numeric precision for money values; avoid floating-point math for billing amounts.

## Worker and Provider Patterns

- Worker code should orchestrate state transitions, not embed provider-specific protocol details.
- Provider adapters should convert external API semantics into stable internal result types.
- Make side effects explicit and log before/after external calls.
- Prefer small, testable functions for each stage: validate -> reserve resources -> execute -> persist -> notify.

## Testing and Validation

- Run these before finalizing backend changes:
  - `just fmt`
  - `just check`
  - `just clippy`
- For risky flow changes, add or update tests covering failure and retry paths.
