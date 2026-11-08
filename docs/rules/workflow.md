# Development workflow

Follow these rules when [AGENTS.md](../../AGENTS.md) directs you here.

## AI Coding Agent Workflow

Before implementing a change:

1. Inspect the relevant existing code.
2. Understand the current architecture and established conventions.
3. Identify the responsible feature and architectural layer.
4. Reuse existing components and abstractions where appropriate.
5. For significant changes, briefly describe the implementation plan.
6. Implement the smallest complete solution.
7. Add or update meaningful tests.
8. Update the affected project documentation as part of the implementation.
9. Run relevant quality checks, including documentation formatting and applicable documented
   commands.
10. Review code and documentation for consistency, unnecessary complexity, and architectural
    violations.

Additional rules:

- Do not rewrite working architecture without a clear reason.
- Do not introduce unnecessary design patterns.
- Do not add abstractions purely for future possibilities.
- Do not create new dependencies without evaluating existing solutions.
- Avoid unrelated refactoring.
- Prefer targeted, maintainable changes.
- Preserve compatibility with existing project conventions.
- When an architectural rule must be broken, explain the trade-off before proceeding.
- Never invent library APIs or claim unsupported functionality exists.
- Prefer official documentation and examples matching installed dependency versions.
