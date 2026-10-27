# Security policy

`keydex` handles configuration that may contain secrets. Failures we care about in
particular:

- a secret leaking in `repr`, `str`, logs, error messages, `dump()` or `explain()`;
- arbitrary file reads triggered by configuration;
- code execution through interpolation or a resolver;
- memory corruption or a `panic` caused by hostile input.

## How to report

**Do not open a public issue.** Use
[GitHub Security Advisories](https://github.com/carlosemart/keydex/security/advisories/new)
or email `spi101@gmail.com` with the subject `keydex security`.

Include: version, platform, a minimal input that reproduces the problem and the impact you
believe it has. If the report contains real secrets, redact them.

## Commitment

- Acknowledgement within 72 hours.
- Assessment and plan within 7 days.
- Credit in the advisory if you want it.

## Supported versions

None yet (pre-release). From `0.1.0` onwards the latest minor is supported.
