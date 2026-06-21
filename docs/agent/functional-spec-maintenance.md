# Functional Specification Maintenance

The functional specification in `docs/functional-spec/index.md` is the project's single source of truth for product behavior.

When implementing a new feature, changing existing functionality, changing user-visible behavior, changing supported states, changing validation rules, changing errors, or changing limitations, compare the code changes against the functional specification before finishing.

If the real behavior changes, update the relevant functional specification files in the same task so the specification and project behavior stay consistent.

Do not describe implementation details, files, classes, functions, libraries, or architecture in the functional specification. Keep it focused on what the system does, scenarios, rules, states, limitations, and errors.

If a requested code change conflicts with the current functional specification, do not silently implement the change and do not silently rewrite the specification. Ask the user how to proceed: update the specification to match the requested behavior, adjust the requested behavior to match the specification, or handle the discrepancy another way.
