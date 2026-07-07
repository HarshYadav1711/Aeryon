# Security Policy

## Supported Versions

Aeryon is in early development. No releases have been published yet. Security fixes will be applied to the `main` branch.

| Version | Supported |
|---------|-----------|
| main    | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, contact the project maintainers directly with:

- A description of the vulnerability
- Steps to reproduce
- Potential impact assessment
- Any suggested remediation

We will acknowledge receipt within 72 hours and provide a timeline for investigation and resolution.

## Security Considerations

As the platform matures, the following areas will require particular attention:

- **Plugin loading:** Untrusted sensor plugins must be sandboxed or validated before execution.
- **Network interfaces:** The server application will expose APIs that require authentication and input validation.
- **Data handling:** Sensor data may contain sensitive environmental information; storage and transmission must be configurable.
- **Dependencies:** Supply-chain security will be monitored through automated vulnerability scanning in CI.

These considerations inform design decisions documented in `docs/adr/`.
