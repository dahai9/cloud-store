---
description: "Use when planning a new feature implementation in this workspace. Produces a consistent, execution-ready plan with migrations, APIs, tests, and regression checklist."
name: "Add Feature Workflow"
argument-hint: "Describe feature goal, scope, and constraints"
agent: "agent"
---

You are implementing a new feature for this Rust + Dioxus workspace.

Feature request:
{{input}}

Follow this standard workflow and output all sections.

## 1. Requirement Split

- Summarize business goal in 1-2 lines.
- List in-scope items.
- List explicit out-of-scope items.
- List assumptions you are making.

## 2. Change Design

- Identify affected crates and responsibilities (`web-app`, `shared-domain`, `worker`, `provider-adapter`).
- Describe data-flow and state transitions.
- Describe API contract changes (request/response, error shape).

## 3. Database and Migration Plan

- State whether schema changes are needed.
- If yes, list new/changed tables, keys, indexes, and constraints.
- Ensure SQL is SQLite-compatible.
- Include rollback/backward-compatibility considerations.

## 4. Implementation Plan

- Provide ordered implementation steps.
- Keep each step atomic and verifiable.
- Highlight idempotency, retry safety, and concurrency controls where relevant.

## 5. Test Plan

- Unit tests to add/update.
- Integration tests to add/update.
- Failure-mode tests (timeouts, retries, duplicate events, invalid input).

## 6. Regression Checklist

- Build/lint commands to run (`just fmt`, `just check`, `just clippy`).
- Critical user paths to verify manually.
- Observability checks (required logs/metrics/traces).

## 7. Delivery Output

Return results in this format:

1. Summary
2. Files to modify
3. Ordered steps
4. Test checklist
5. Risks and mitigations

Keep the plan concise but implementation-ready. Do not include unrelated refactors.
