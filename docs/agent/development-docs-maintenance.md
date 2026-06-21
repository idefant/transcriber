# Development Documentation Maintenance

Development documentation in `docs/development/` records codebase-facing explanations: why a complex implementation choice was made, why it works that way, and what constraints must be remembered during future work.

When adding or changing functionality that solves a complex codebase problem and the reasoning may need to be preserved, consider whether the rationale belongs in `docs/development/`.

If questions arise about such a codebase problem during the task, ask the user whether to add a new page or a new section in `docs/development/` for that problem.

This rule applies to questions about the project codebase and its behavior. It does not apply to documentation about how the agent interacts with the codebase; agent behavior rules belong in `docs/agent/`.

Before re-analyzing a previously documented complex problem, check the relevant `docs/development/` page so the reasoning does not need to be rediscovered and so future changes do not accidentally break the documented decision.
