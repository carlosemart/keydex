# Interpolation reference

The exact grammar of `${...}`, resolved once at build time.

## Forms

| Form | Meaning |
|---|---|
| `${db.host}` | absolute reference to another key |
| `${db.host:-localhost}` | with a default when the key is absent |
| `${env:VAR}` | `env` resolver |
| `${env:VAR:-fallback}` | resolver with a default |
| `${a:-${b}}` | nested default |
| `\${not.a.reference}` | escaped, stays literal |

A string that is **exactly one placeholder** inherits the type of the referenced value
(`${db.port}` inside an `int` field is an integer). A placeholder inside a larger string is
rendered as text.

## Rules

- Resolution happens **once**, after the merge, in dependency order. Reading is a map lookup.
- A cycle is an error that names the full trail (`a -> b -> a`).
- A reference to a missing key without a default is an error at load time, even if the value
  is never read.
- A value built from a sensitive reference is sensitive too, and is redacted in diagnostics.
- Relative references (`${.sibling}`, `${..parent}`) are **not** implemented.

## Resolvers

`${name:arg,arg}` calls a registered resolver. Built in:

| Resolver | Example | Result |
|---|---|---|
| `env` | `${env:APP_NAME}` | the variable, or the default |

A resolver is a pure function: no network, no filesystem, no global state. Register your own
from Rust with `Builder::resolver`; an unregistered name is a `UnknownResolver` error, not an
empty value. Python resolver registration is not implemented.

## Where it applies

Every string leaf in the merged tree, in every format, including inside sequences and maps.
Interpolation runs after the merge, so `${db.host}` sees the value that survived precedence,
not the one in the file where the line was written.
