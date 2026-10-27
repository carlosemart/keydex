# Security policy

`keydex` handles configuration that may contain secrets.

## What is protected

- **`Secret[T]`** in typed models: `repr`, `str` and dataclass printing show
  `Secret('***')`. The value only leaves through `.reveal()`.
- **Diagnostic output**: `Settings.explain()` and `Settings.items()` return `None` for
  keys declared sensitive, and mark the explanation with `redacted: true`. `dump` and
  `python -m keydex dump` use `items()`, so they inherit that.
- **Keys derived from `*_FILE`** are marked sensitive automatically.
- **`*_FILE` is opt-in**: it only resolves when `secret_files=True`, and every resolved
  path must live inside the directory passed as `secret_root`; anything else raises
  `KeydexError`. Without opting in, no file is ever read.

## What is not protected

- `Settings.get(key)` returns the **raw** value. That is the point of the API; do not log
  its result. `settings[key]`, `items()`, `dump()` and `dict(settings)` redact.
- In dictionary mode, a key is only redacted if it is declared sensitive, either through
  the `sensitive=[...]` argument or because it came from `*_FILE`. There is no guessing by
  key name.
- The Rust `Snapshot` derives `Debug`, so `{:?}` on it prints raw values.
- Values already copied into a Python object (a plain `str` field, a log message, an
  exception message) are outside the library's control.

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
