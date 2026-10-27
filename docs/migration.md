# Coming from another library

The concepts do not map one to one, but most configurations translate mechanically.

## From `dynaconf`

| dynaconf | keydex |
|---|---|
| `settings.toml` + `settings.prod.toml` | `app.yaml` + `app-prod.yaml`, `profiles=["prod"]` |
| `[default]` / `[production]` sections in one file | separate profile files |
| `ENV_FOR_DYNACONF` | `profiles=["prod"]` at the call site |
| `DYNACONF_DB__HOST` | `APP_DB__HOST` (dictionary) or `APP_DB_HOST` (typed) |
| `@int`, `@bool`, `@json` casting in values | the model's type annotations |
| `settings.get("db.host")` | `Settings.get("db.host")` |
| `settings.from_env("APP_")` | `keydex.load_settings(env_prefix="APP_")` |
| `settings.validators.validate()` | validation happens during `load` |
| `@merge` for lists | lists replace; `_merge_: append` is planned |

## From `pydantic-settings`

| pydantic-settings | keydex |
|---|---|
| `class Settings(BaseSettings)` | a dataclass + `keydex.load` |
| `model_config = SettingsConfigDict(env_prefix="APP_")` | `keydex.load(..., env_prefix="APP_")` |
| `Field(alias=...)` | the field name plus relaxed binding (`db_host` → `db.host` in typed mode) |
| nested `BaseModel` | nested dataclass |
| `SecretStr` | `Secret[str]` |
| `.env` file support | not implemented; use a YAML file plus the environment |
| `ValidationError.errors()` | `keydex.ValidationError.errors` with `path`, `file`, `line` |

## From OmegaConf

| OmegaConf | keydex |
|---|---|
| `OmegaConf.load("app.yaml")` | `keydex.load_settings(files=["app.yaml"])` |
| `conf.db.host` | `settings["db.host"]` (dictionary) or a typed model |
| `OmegaConf.merge(base, override)` | precedence tiers: the override is a higher-priority source |
| `${db.host}` interpolation | the same syntax, resolved after the merge |
| `${oc.env:VAR}` | `${env:VAR}` |
| `${oc.select:key,default}` | `${key:-default}` |
| `???` mandatory value | a dataclass field without a default |
| `struct=True` | `on_unknown="error"` plus type validation |
| `to_container()` | `dict(settings.items())` (redacted) |

## From Spring `Environment`

| Spring | keydex |
|---|---|
| `application.properties` + `application-prod.properties` | `app.yaml` + `app-prod.yaml` |
| `spring.profiles.active` | `profiles=["prod"]` |
| `@Value("${db.host}")` | field access on the typed model |
| `@ConfigurationProperties` | the dataclass itself |
| `PropertySource` with an order | `Source` trait with a `Tier` and an `order` |
| relaxed binding | the same, resolved against the schema |
| `Environment.getProperty()` | `Settings.get()` |

## What has no equivalent yet

- `imports:` between files, remote sources and `watch()`: see the
  [status page](status.md).
- `.env` loading: read the file yourself and pass it through `env=`.
