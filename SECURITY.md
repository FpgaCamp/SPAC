# Security Policy

## Supported Versions

The project is pre-release. Security fixes target the `main` branch until the
first versioned release exists.

## Reporting a Vulnerability

Open a private security advisory if the hosting platform supports it. If that is
not available, contact the maintainers out of band and avoid posting exploit
details in public issues before triage.

## Security Expectations

- Treat configs, DSL files, traces, and report files as untrusted input.
- Do not execute generated artifacts by default.
- Do not add external tool execution without an adapter, threat model, and ADR.
- Do not add large dependencies without license and maintenance review.
