# keydex — Especificación

> Librería de **acceso a configuración** para Python, con núcleo en Rust.
> Tipada, explicable y rápida: pides una clave, recibes el valor correcto, del origen
> correcto, con el tipo correcto, y puedes preguntar **por qué**.

Estado: especificación cerrada · Fecha: 2026-10-27

---

## 0. Resumen

`keydex` resuelve el problema de "¿de dónde sale este valor de configuración?" con una
API tipada y un motor en Rust. Toma lo mejor de Spring `Environment` (property sources,
perfiles, relaxed binding, precedencia explícita) y de OmegaConf (interpolación,
composición, sentinel de valor ausente), pero sin el coste de un árbol de objetos Python
por nodo y sin el `eval` de expresiones.

**Diferenciador principal:** cada valor del snapshot conoce su procedencia completa
(fuente, fichero, línea, cadena de placeholders, quién fue sombreado). Es lo que
convierte la librería en herramienta de operación, no solo de carga.

---

## 1. Decisiones cerradas

| Decisión | Valor |
|---|---|
| Nombre | `keydex` (`keydex-core`, `keydex-py`, `import keydex`) |
| API principal | `load(Modelo)` tipada + `env.get()` como escape hatch |
| Python | 3.11+ · wheel `abi3-py311` · wheel `cp314t` para free-threading |
| Alcance | Solo Python; `keydex-core` se publica en crates.io cuando el MVP sea usable |
| Precedencia | Bandas ordenadas + orden de inserción por fuente |
| Binding | Relaxed, resuelto contra el esquema (ver §6) |
| Interpolación | Solo referencias + resolvers. **Sin expresiones, sin `eval`** |
| Composición | `imports`/`includes` en core; grupos tipo Hydra = plugin futuro |
| Recarga | `refresh()` en v1; `watch()` en v2 |
| Reparto | Rust: parseo, merge, interpolación, coerción, validación, procedencia. Python: tipos, ergonomía, integraciones |
| Rendimiento | Cero FFI por acceso; snapshot inmutable compartido |
| No-objetivos | DI container, expresiones, FFI multi-lenguaje, wasm, hot reload por defecto |

---

## 2. Principios invariantes

Reglas que no se negocian; son baratas al principio y carísimas después.

1. **`keydex-core` no conoce Python.** `cargo tree -p keydex-core` no puede contener
   `pyo3`. Verificado en CI como gate bloqueante.
2. **El core no lee el entorno, ni el reloj, ni el sistema de ficheros.** Todo entra
   inyectado (`EnvSource`, `FileProvider`, `Clock`). Es lo que permite testearlo,
   embeberlo y publicarlo como crate útil.
3. **Errores estructurados, nunca `String`.** Enums con `thiserror`; cada capa los
   formatea (excepción en Python, JSON en CLI).
4. **Cero estado global mutable.** Sin `static` de configuración, sin caches sin lock.
   Un solo esfuerzo cubre a la vez hilos y free-threading.
5. **Un solo FFI por operación de alto nivel.** `build()` y `refresh()` cruzan la
   frontera; los accesos, no.

---

## 3. Modelo conceptual

```
          ┌──────────────── PropertySource (banda + orden) ────────────────┐
          │  YAML · TOML · JSON · INI · .env · env vars · CLI · secretos  │
          └───────────────────────────────┬───────────────────────────────┘
                                          │ bytes + spans
                                          ▼
                                    Parseo (Rust)
                                          ▼
                                  Árbol de valores con spans
                                          ▼
                     Merge determinista (§9)  +  activación de perfiles (§10)
                                          ▼
                        Grafo de interpolación → orden topológico (§8)
                                          ▼
                        Coerción + validación contra Schema IR (§7, §11)
                                          ▼
                     ┌──── Snapshot inmutable + procedencia (§13) ────┐
                     │  claves planas · interning · arena · inmutable │
                     └───────────────────┬───────────────────────────┘
                                         ▼
                          Decode tipado (una vez)  |  Mapping (dict mode)
```

- **`PropertySource`**: unidad enchufable con `tier` (banda) y `order` (posición
  dentro de la banda). La precedencia no es una lista rígida hardcodeada.
- **`Snapshot`**: árbol de claves plano, inmutable, compartible entre hilos, con swap
  atómico en `refresh()`.
- **`Schema IR`**: descripción serializable del modelo que Python genera y Rust
  consume. Es el contrato de compatibilidad entre ambos mundos.

---

## 4. Precedencia

Bandas de mayor a menor prioridad. Dentro de una banda, `order` ascendente; el último
en aplicarse gana.

| Banda | Origen | Notas |
|---|---|---|
| 0 | Overrides de test / programáticos | `env.override()`, fixture de pytest |
| 1 | Argumentos de CLI mapeados | `--db-host=x` → `db.host` |
| 2 | Variables de entorno del SO | prefijo `APP_`, `*_FILE` para secretos |
| 3 | Ficheros del perfil activo | el último perfil gana; el último documento gana |
| 4 | Ficheros base | la última ubicación gana |
| 5 | Defaults del esquema | `???` = obligatorio ausente (sentinel, no `None`) |
| 6 | Defaults de código | declarados en el modelo |

Reglas adicionales:

- Una fuente puede declarar `tier` reservado (0–6) o insertarse **antes/después de otra
  fuente concreta** (`insert_before="env"`). Esto evita el clásico "mi fuente no encaja".
- Un documento importado (`config.import`) se trata como **inmediatamente inferior** al
  documento que lo importa, no como una fuente independiente.
- El orden es **determinista** y no depende del orden de `os.environ` ni del
  filesystem. Mismo input → mismo `dump()` byte a byte.

---

## 5. Orígenes

### 5.1 Formatos locales (v1)

YAML, TOML, JSON, INI, `.properties`, `.env`, y **YAML multi-documento**.

Requisito técnico: los parsers deben conservar **spans** (línea/columna) porque
`explain` los expone. Esto condiciona las dependencias (ver §17.3).

### 5.2 Descubrimiento de ficheros

- `search_paths` explícito; si falta: `./config`, `.`, `$XDG_CONFIG_HOME/keydex`,
  directorio del paquete consumidor, y `KEYDEX_CONFIG_*` si se define.
- Ficheros por perfil: `app.yaml`, `app-prod.yaml`, `app-prod-eu.yaml`.
- Prefijos de URI: `file:`, `optional:` (`optional:file:./x.yaml` no falla si no existe).
- `imports:` dentro de un documento, resueltos en profundidad con detección de ciclos.

### 5.3 Entorno

- Prefijo configurable (`env_prefix="APP_"`), strip y lowercase.
- Sintaxis: ver §6.
- `*_FILE`: si `APP_DB_PASSWORD_FILE` existe, el valor se lee del fichero y se marca
  como secreto. Nunca se mezcla con `APP_DB_PASSWORD` (gana `*_FILE`).

### 5.4 Otros (v1.1+)

CLI args, `random.*` (int, uuid, port) por arranque, y sources remotas por plugin
(Vault, KMS, S3, Consul, etcd, keyring, 1Password, Bitwarden, SOPS).

---

## 6. Binding relajado

Con esquema, el binding **no adivina**: el esquema desambigua.

### 6.1 Normalización de claves de config

`dbHost`, `db-host`, `db_host`, `DB_HOST` son la misma clave. La forma canónica es
`snake_case` con `.` para niveles y `[i]` para índices.

### 6.2 Variables de entorno → claves

| Env var (`APP_` stripped) | Resultado |
|---|---|
| `DB_HOST` | `db.host` (resuelto con el esquema) |
| `DB__HOST` | `db.host` (nivel explícito) |
| `SERVERS__0__HOST` | `servers[0].host` |
| `LABELS__TEAM_A` | `labels.team_a` (mapa dinámico) |

Algoritmo:

1. Strip de prefijo, lowercase.
2. Se construye un trie con las rutas del esquema.
3. Match **exacto** sobre la forma normalizada; si falla, match **greedy por segmentos
   más largos** contra el trie.
4. Si dos rutas del esquema empatan → `AmbiguousBinding` (error, con las dos candidatas).
5. Los mapas dinámicos (`dict[str, T]`, campos indexados) **exigen** `__` como
   separador de nivel, porque el esquema no puede desambiguar.

**Modo diccionario (sin esquema):** `__` es obligatorio como separador de nivel y `_`
es literal. Determinista y sin heurísticas. Esta es la razón por la que el modo tipado
es el camino principal.

### 6.3 Colisiones

Si dos campos del modelo normalizan al mismo nombre → error de esquema en `load()`, no
en runtime.

---

## 7. Tipos y coerción

### 7.1 Tipos soportados

`str`, `int`, `float`, `decimal.Decimal`, `bool`, `Path`, `Enum`, `datetime`/`date`/`time`
(con o sin TZ), `UUID`, `IPv4Address`/`IPv6Address`, `duration`, `size`, `rate`, `URL`,
`Secret[T]`, `Optional[T]`, `Union[T, ...]` (con discriminador), `Literal[...]`,
`list[T]`, `tuple[...]`, `set[T]`, `dict[str, T]`, dataclasses anidadas, `attrs`,
`pydantic` (vía JSON Schema), `msgspec.Struct`, `TypedDict`, `NamedTuple`.

### 7.2 Reglas de coerción

| Tipo | Acepta (modo `relaxed`) |
|---|---|
| `bool` | `true/false`, `yes/no`, `on/off`, `1/0` (case-insensitive) |
| `int` | `"42"`, `"0x1F"`, `"1_000"` |
| `Decimal` | str exacto, nunca float |
| `duration` | `250ms`, `5s`, `3m`, `2h`, `1d`, `PT5M` |
| `size` | `10MB`, `1GiB` (binario vs decimal explícito) |
| `rate` | `100/s`, `5MB/s` |
| `Path` | str; `~` **no** se expande salvo `expand_user=True` |
| `Enum` | nombre o valor; case-insensitive en relaxed |
| `datetime` | ISO-8601; naive → error en strict |

Tres modos: `strict` (solo conversiones sin pérdida), `relaxed` (por defecto, tabla
anterior), `lenient` (mejor esfuerzo). **Nunca** `"false"` → `True`, ni float → int con
pérdida, ni truncado silencioso de strings.

### 7.3 Sentinel `MISSING`

`???` significa "obligatorio y ausente". No es `None`. Solo `MISSING` dispara error de
requerido; `Optional[T]` admite `None` explícito desde config.

---

## 8. Interpolación

### 8.1 Sintaxis

| Forma | Significado |
|---|---|
| `${db.host}` | Referencia absoluta |
| `${.sibling}` | Referencia relativa (hermano) |
| `${..parent.key}` | Relativa hacia arriba |
| `${db.host:-localhost}` | Con valor por defecto |
| `${env:HOME}` | Resolver `env` |
| `${env:NOPE:-fallback}` | Resolver con default |
| `\${...}` | Escape literal |
| `${a:-${b}}` | Defaults anidados |

`${...}` hereda el tipo del valor referenciado; `"${...}"` lo fuerza a string.

### 8.2 Semántica

- Resolución en **orden topológico una sola vez**, cacheada en el snapshot → coste cero
  por acceso.
- Ciclos → error con la **traza completa** del ciclo, no un mensaje genérico.
- Referencia a clave inexistente sin default → error en `strict`, `MISSING` en otro caso.
- Se resuelven también claves que no se llegan a leer si el modo es `strict`
  (`validate_all=True`), para detectar config muerta antes de desplegar.

### 8.3 Resolvers

Registrables desde Python, ejecutables en Rust los built-in:

`env`, `base64`, `int`, `float`, `bool`, `list`, `json`, `uuid`.

- Argumentos separados por `,`, escapables con `\,`.
- `file:` existe pero **desactivado por defecto** (lee rutas arbitrarias: footgun).
- Un resolver debe ser **puro**. No hay acceso a red ni a estado global desde el core.
- No existe lenguaje de expresiones: nada de aritmética, condicionales ni acceso a
  métodos. Si algún día se quiere, va como plugin y nunca en el core.

---

## 9. Composición y merge

### 9.1 Reglas por defecto

- Mapas: merge profundo.
- Listas: **reemplazo** (como Spring), no concatenación.
- Escalares: el último gana.

### 9.2 Marcadores de intención

```yaml
servers:
  _merge_: append          # replace | append | prepend | keyed:name
  _items_: [...]
labels:
  _delete_: [obsolete]
```

`keyed:<campo>` fusiona listas de structs por identidad, reportando duplicados.

### 9.3 Includes

`imports:` / `includes:` resueltos antes del merge, con profundidad máxima configurable
y detección de ciclos. Un import hereda el perfil activo del importador salvo que
declare lo contrario.

> **Estado:** no implementado. Depende de que la capa que llama resuelva los ficheros
> (el core no toca el sistema de ficheros), así que vive en el binding y no en el motor.
> Pendiente para F3.1.

---

## 10. Perfiles

- Activación: config (`profiles.active`), entorno (`APP_PROFILES_ACTIVE`), CLI
  (`--profile`). Precedencia: CLI > entorno > fichero.
- `profiles.include`: añade perfiles sin reemplazar los activos.
- `profiles.group`: alias (`prod` → `prod,metrics,tracing`).
- Activación condicional por documento:

```yaml
---
activate:
  on_profile: prod
  on_property: feature.new_db   # solo si esa clave existe
db:
  host: db.prod.internal
```

- Orden de merge: perfiles en el orden declarado; el último gana. Documentado y con
  tests dorados.
- Perfil `test` disponible como fixture de pytest que aísla el entorno.

---

## 11. Schema IR (contrato de compatibilidad)

JSON versionado, generado por Python y consumido por Rust. Es la superficie pública
que se mantiene estable.

```json
{
  "ir": "keydex.schema/v1",
  "root": {
    "type": "struct",
    "required": true,
    "fields": {
      "db": {
        "type": "struct",
        "required": true,
        "fields": {
          "host": { "type": "str", "required": true },
          "port": { "type": "int", "required": false }
        }
      },
      "note": { "type": "optional", "required": false, "inner": { "type": "str" } },
      "tags": { "type": "list", "required": false, "items": { "type": "str" } },
      "mode": { "type": "str", "required": false, "converter": "enum" }
    }
  }
}
```

Tipos soportados en v1: `str`, `int`, `float`, `bool`, `any`, `struct`, `optional`,
`list`. El campo `converter` indica a la capa de Python qué conversión rica aplicar
después (`enum`, `path`, `decimal`, `uuid`): el motor no conoce esos tipos.

Reglas de evolución:

- Dentro de `v1`, solo cambios **aditivos** (campos nuevos opcionales).
- Cualquier ruptura → `keydex.schema/v2` y soporte simultáneo.
- El IR es también la base de: `.pyi` generado, JSON Schema exportado para docs,
  `keydex schema`, y (futuro) validación desde otros lenguajes.

Snapshot exportable (para tests dorados y CLI):

```json
{
  "ir": "keydex.snapshot/v1",
  "values": { "db.host": "db.internal" },
  "provenance": {
    "db.host": {
      "source": "env", "raw": "APP_DB__HOST", "tier": 2,
      "shadowed": [{ "source": "file", "file": "app.yaml", "line": 4 }],
      "interpolation": []
    }
  }
}
```

---

## 12. Validación y errores

- Validación en Rust desde el IR: tipos, `required`, rangos, regex, longitudes,
  cardinalidad, validadores custom registrados por nombre.
- **Errores agregados**, no fail-fast. Formato por error:

```python
{ "path": "db.port", "message": "must be <= 65535", "value": 70000,
  "source": {"file": "app.yaml", "line": 12}, "suggestion": None }
```

- Claves desconocidas: `on_unknown = "error" | "warn" | "ignore"` (por defecto `warn`),
  con sugerencia por distancia de edición (`db.hots` → *did you mean `db.host`?*).
- Jerarquía de excepciones: `KeydexError` → `SchemaError`, `ParseError`,
  `BindingError`, `ValidationError`, `InterpolationError`, `SecretError`, `SourceError`.
  Todas con `path` y `details` estructurados.

---

## 13. Procedencia

`explain(key)` devuelve un objeto, no un string formateado (el formateo es de cada
frontend):

```python
cfg.explain("db.host")
# Explanation(
#   key="db.host", type=str, value="db.internal",
#   winner=Origin(source="env", raw="APP_DB__HOST", tier=2),
#   shadowed=[Origin(source="file", file="app.yaml", line=4)],
#   interpolation=["db.suffix"],
#   redacted=False,
# )
```

Reglas:

- `shadowed` incluye **todos** los orígenes que definieron la clave y perdieron.
- Si el valor viene de un secreto, `value` es `None` y `redacted=True` salvo
  `reveal=True` explícito.
- `dump(format=...)` es determinista y redactado por defecto.
- CLI equivalente: `python -m keydex explain db.host`.

---

## 14. Secretos

- Tipo `Secret[T]`: `repr`, `str`, `format`, `pformat`, logs y trazas → `********`.
  Valor real solo con `.reveal()`.
- Indirección `*_FILE` para Docker/K8s secrets.
- `zeroize` en Rust para los buffers que contienen secretos.
- Garantías de no-fuga verificadas con tests: `dump`, `explain`, `repr`, mensajes de
  error de validación, JSON Schema exportado.
- Fuentes: `keyring` (v1.1), Vault/1Password/Bitwarden/SOPS (plugin).

---

## 15. Ciclo de vida y recarga

- `load()` construye un snapshot inmutable.
- `refresh()`: vuelve a leer las fuentes, valida, y **solo entonces** hace swap atómico.
  Si falla, el snapshot vigente sigue sirviendo y el error se reporta. Nunca se deja la
  aplicación con config parcial.
- `watch()` (v2): `notify` + debounce, callbacks `on_change(keys)`.
- Thread safety: snapshot inmutable compartido; overrides por `contextvar` para
  tests concurrentes y por-request.
- Fork safety: sin file descriptors ni hilos vivos en el hijo; documentado.
- `close()` para liberar el watcher (v2).

---

## 16. API Python

```python
from keydex import load, Settings, Secret, MISSING, override

@dataclass(frozen=True)
class Db:
    host: str
    port: int = 5432
    password: Secret[str] | None = None

@dataclass(frozen=True)
class App:
    db: Db
    debug: bool = False
    profiles: list[str] = field(default_factory=list)

cfg = load(App, profiles=["prod"], env_prefix="APP_")
cfg.db.host                    # acceso nativo, sin FFI
cfg.explain("db.host")         # procedencia
cfg.dump("yaml", redact=True)
cfg.refresh()
```

```python
def load(
    model: type[T] | None = None,
    *,
    sources: Sequence[Source] | None = None,
    profiles: Sequence[str] | None = None,
    env_prefix: str = "",
    search_paths: Sequence[PathLike] | None = None,
    mode: Literal["relaxed", "strict", "lenient"] = "relaxed",
    on_unknown: Literal["error", "warn", "ignore"] = "warn",
    expand_user: bool = False,
    sources_order: Sequence[str] | None = None,
) -> T | Settings: ...

class Settings(Mapping[str, Any]):
    def get(self, key: str, default: Any = None) -> Any: ...
    def explain(self, key: str, *, reveal: bool = False) -> Explanation: ...
    def dump(self, fmt: str = "yaml", *, redact: bool = True) -> str: ...
    def refresh(self) -> None: ...
    def override(self, key: str, value: Any) -> ContextManager[None]: ...
```

- **Sin singleton global** por defecto. Helper opcional `keydex.current()` para
  ergonomía, nunca obligatorio.
- Integraciones v1: pytest fixture, `logging`/`structlog` reconfiguration, FastAPI
  (`Depends` + lifespan), Click/Typer. Django settings shim en v2.
- CLI: `python -m keydex validate | explain | dump | schema`.

---

## 17. Arquitectura Rust

### 17.1 Crates

```
keydex/
├── Cargo.toml                # workspace
├── crates/
│   ├── keydex-core           # Rust puro, publicable, cero pyo3
│   └── keydex-py             # PyO3: adaptadores y mapeo de errores
├── python/keydex/            # capa Python pura
├── tests/                    # integración Python
└── benches/                  # criterion + pytest-codspeed
```

### 17.2 Módulos de `keydex-core`

| Módulo | Responsabilidad |
|---|---|
| `source` | `PropertySource` trait, tiers, orden, `EnvSource`, `FileProvider` |
| `parse` | Parsers con spans (YAML, TOML, JSON, INI, properties, .env) |
| `tree` | Árbol plano de claves, interning, arena, spans |
| `merge` | Merge determinista + marcadores |
| `profile` | Activación, includes, grupos |
| `interp` | Grafo, topológico, resolvers, ciclos |
| `schema` | Schema IR (serde), resolución de rutas |
| `coerce` | Conversión tipada y validadores |
| `provenance` | Origen, sombreados, `Explanation` |
| `error` | Enums con `thiserror` |

### 17.3 Dependencias clave

- **YAML con spans:** `saphyr` 0.3 (trae `marked_yaml` dentro). Confirmado en el
  [spike 0001](spikes/0001-parsers-spans.md).
- **TOML con spans:** `toml_edit` 0.25, usando `Document` (no `DocumentMut`, que
  descarta los spans).
- **JSON con posiciones:** `jsonc-parser` 0.34. `jiter` se descartó: es más rápido pero
  no expone posiciones.
- **`LineIndex` por fichero** para convertir offsets en línea/columna; sin él la
  conversión es O(n) por clave y el parseo se vuelve cuadrático.
- `compact_str` / `arcstr` para claves cortas; interner propio.
- `thiserror`, `serde`, `rayon` (multi-fichero), `zeroize`/`secrecy`.
- `pyo3` **solo** en `keydex-py`.

### 17.4 Feature flags del core

`yaml`, `toml`, `json`, `ini`, `dotenv`, `zeroize`, `watch` (v2), `http` (v2).
Default: formatos locales, sin red ni watcher. Un servicio Rust que solo lee TOML no
arrastra medio ecosistema.

### 17.5 Frontera Python

- `build(files, env, schema_ir) -> SnapshotHandle` y `refresh()`: los únicos cruces
  caros. El GIL se libera durante el parseo/merge.
- El decode tipado produce el objeto final en una sola pasada (estilo `msgspec`).
- `gil_used = false` en el módulo PyO3, con estado global cacheado bajo `RwLock`.

---

## 18. Rendimiento

### 18.1 Presupuestos (gates de CI)

| Métrica | Objetivo |
|---|---|
| Cold load, 1k claves YAML + env | < 5 ms |
| Decode a dataclass de 200 campos | < 150 µs |
| Acceso a atributo | ±10% vs dataclass nativa (0 FFI) |
| `refresh()` atómico, 1k claves | < 5 ms |
| `explain()` | < 100 µs |
| Memoria del snapshot | < 3× el texto fuente |
| `import keydex` | < 10 ms |

Comparativa obligatoria en CI contra `omegaconf`, `dynaconf` y `pydantic-settings`,
con `pytest-codspeed` y umbral de regresión.

### 18.2 Técnicas

Interning de claves, trie de rutas, arena, `Cow<str>`, resolución topológica cacheada,
`rayon` para múltiples ficheros, GIL liberado en `build`.

---

## 19. Empaquetado y CI

### 19.1 Wheels

| Artefacto | Cubre |
|---|---|
| `abi3-py311` | 3.11, 3.12, 3.13, 3.14 (con GIL) |
| `cp314t` | 3.14 free-threaded |
| `cp313t` (opcional) | 3.13 free-threaded, etiqueta experimental |

`abi3` y `t` son mutuamente excluyentes: dos perfiles de build con maturin.
Fijar la versión de PyO3 al iniciar y confirmar el estado de soporte de free-threading
en ese momento.

### 19.2 Matriz de CI

| Job | Contenido |
|---|---|
| test | 3.11–3.14 sobre wheel abi3 |
| test-ft | 3.14t (verifica `sys._is_gil_enabled()` y la suite completa) |
| fuzz | parsers (cargo-fuzz + atheris) |
| bench | 3.11 y 3.14t, gate de regresión |
| lint | `cargo clippy -D warnings` + `deny(panic, unwrap_used, expect_used, disallowed_methods)` en core; `ruff`, `mypy --strict` |
| gate | `just gate`: falla si `pyo3` aparece en el árbol de dependencias de `keydex-core` |

### 19.3 Publicación

1. `keydex-core` en crates.io cuando el MVP sea usable (0.x, API pequeña, docs.rs con
   `all-features`).
2. Wheels en PyPI.
3. Reservar nombres antes de anunciar (acción manual, es publicación real).

---

## 20. Testing

- **Tests dorados de precedencia**: matriz completa banda × formato × perfil.
- **Tests dorados de binding**: tabla de §6.2 completa, incluido el caso ambiguo.
- **Property-based**: `hypothesis` para merge e interpolación
  (`merge(a, a) == a`, idempotencia del dump, simetría de `explain`).
- **Fuzz**: ningún input provoca `panic!` en Rust; todo error cruza como excepción.
- **Determinismo**: mismo input → mismo `dump()` byte a byte, con `os.environ`
  desordenado a propósito.
- **No fugas de secretos**: suite dedicada por cada superficie de salida.
- **Concurrencia**: lecturas desde N hilos y desde 3.14t.

---

## 21. Roadmap

| Fase | Entregable | Criterio de salida |
|---|---|---|
| **F0** ✅ | Workspace, `AGENTS.md` + `docs/spec.md` + `just gate`, spikes de parsers con spans, esqueleto de errores | Spans correctos y `just gate` en verde |
| **F1** ✅ | `keydex-core` (sources, merge, interp, snapshot) + `keydex-py` dict-mode | Carga YAML+env, `${}`, `explain`, benchmark vs OmegaConf |
| **F2** ✅ | Esquema tipado: dataclasses, coerción, validación agregada | `load(App)` completo con errores con path. `.pyi` generado no hace falta: los dataclasses ya dan tipos a mypy y al IDE |
| **F3** ✅ | Perfiles por convención de ficheros, secretos (`Secret`, `*_FILE`), `refresh()`, CLI | Suite verde; los secretos no se filtran en `repr` |
| **F4** | Publicación: wheels abi3 + 314t, crates.io, docs | Instalable desde PyPI en 3.11–3.14t |
| **F5 (v2)** | `watch()`, fuentes remotas async, plugin de composición tipo Hydra | Solo con demanda real |

Dogfooding desde F1: `cleanyourtv`, `f95downloader`, `pirate-scanner`.

---

## 22. No-objetivos

- Contenedor de inyección de dependencias.
- Ciclo de vida de beans estilo Spring.
- Lenguaje de expresiones en la interpolación.
- FFI multi-lenguaje (Go/C#/Java/Kotlin), wasm.
- Hot reload activado por defecto.
- Mutación del esquema en runtime.

---

## 23. Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| Deriva Python↔Rust del Schema IR | IR versionado + tests cruzados con fixtures compartidos |
| Parsers sin spans suficientes | **Resuelto en F0**: los tres formatos dan spans ([spike 0001](spikes/0001-parsers-spans.md)) |
| Binding ambiguo en modo diccionario | Regla estricta `__`, documentada y testeada |
| `abi3` vs free-threading divergen | Dos perfiles de build; CI dedicado a `t` |
| Scope creep hacia framework | Lista de no-objetivos en el README y revisión por fase |
| Sobrecoste de procedencia en memoria | Provenance solo para claves que participan; interning |

---

## 24. Documentación para el agente

El plan vive en `~/.opencode/plan/`, **fuera del repo**: un agente que trabaje en
`keydex` no lo verá. Por eso la documentación de trabajo se muda al repo en F0, en
cuatro capas:

| Capa | Fichero | Contenido | Por qué existe |
|---|---|---|---|
| Reglas | `AGENTS.md` | imperativo y verificable, ≤100 líneas | se carga solo en cada sesión |
| Contexto | `docs/spec.md` | esta especificación | el "por qué" y los contratos |
| Decisiones | `docs/adr/NNNN-*.md` | una ADR por decisión irreversible | evita re-litigar y perder el motivo |
| **Ejecución** | CI + `just gate` | lo que un test puede comprobar | **la prosa se ignora; un build rojo no** |

### 24.1 `AGENTS.md` (raíz del repo)

> Borrador de trabajo. La versión vigente es el `AGENTS.md` de la raíz del repo; esta
> se conserva para no perder el razonamiento.

```markdown
# keydex — instrucciones del proyecto

Especificación: `docs/spec.md`. Decisiones: `docs/adr/`. Léelos antes de tocar
`crates/`. Si el código y la spec discrepan, gana la spec: pregunta antes de cambiarla.

## Qué es keydex
Acceso a configuración: tipado, con procedencia y rápido.

## Qué NO es (no lo añadas aunque parezca útil)
Contenedor DI · motor de expresiones · framework de aplicación · FFI multi-lenguaje ·
hot reload por defecto · validación en Python.

## Reglas duras
1. `keydex-core` NO depende de `pyo3`.           → `just gate`
2. `keydex-core` NO lee entorno, reloj ni FS.    → `just gate`
3. Errores = enums `thiserror`. Nunca `String`.  → clippy `panic`/`unwrap_used`
4. Cero estado global mutable; caches con `RwLock`.
5. Ningún acceso a atributo cruza FFI; solo `build` y `refresh`.
6. Toda clave expuesta en `explain` conserva su span.

## Prohibiciones
- No `serde_yaml` (archivado, sin spans): usa `saphyr`/`marked_yaml`.
- No `unwrap`/`expect`/`panic` fuera de `#[cfg(test)]`.
- No dependencias nuevas sin ADR y `cargo deny`.
- No cambiar `keydex.schema/v1` sin bump, fixture compartido y test cruzado.
- No `eval`, no red, no lectura de ficheros arbitrarios desde un resolver.

## Mapa
crates/keydex-core → motor puro (sin Python)
crates/keydex-py   → PyO3, solo adaptadores
python/keydex      → API, tipos, integraciones
docs/              → spec y ADRs

## Comandos
just check   # fmt + clippy -D warnings + ruff + mypy --strict
just test    # cargo test + pytest (3.11-3.14 y 3.14t)
just gate    # invariantes de arquitectura (reglas 1 y 2)
just bench   # codspeed; falla si hay regresión
just fuzz    # 60s por parser

## Terminado cuando
- [ ] hay un test que falla sin el cambio
- [ ] `just check`, `just gate` y `just test` verdes
- [ ] `just bench` sin regresión
- [ ] `docs/spec.md` actualizado si cambió el comportamiento
- [ ] entrada en `CHANGELOG.md`
```

Además, `crates/keydex-core/AGENTS.md` (se carga al entrar en ese subárbol):

```markdown
Este crate se publica en crates.io: API pública mínima, `#[non_exhaustive]` en enums
públicos, semver 0.x, docs.rs con `all-features`. Cero pyo3 y cero lectura directa
de entorno/reloj/FS: todo entra inyectado. Los errores son enums, nunca strings.
```

### 24.2 Mecánica de OpenCode (verificada)

- Solo se reconoce `AGENTS.md`; `CLAUDE.md` no se usa como fallback.
- Se cargan el global `~/.config/opencode/AGENTS.md` y todos los `AGENTS.md` desde el
  **directorio de trabajo** hacia arriba. Los anidados por debajo se cargan de forma
  perezosa al leer ficheros de esa zona.
- Se **combinan**, no se sustituyen, y OpenCode no resuelve conflictos: no dupliques
  una regla en dos niveles con matices distintos.
- Editar un `AGENTS.md` anidado ya cargado no se detecta hasta abrir sesión nueva.
- `instructions` en `opencode.jsonc` añade ficheros, globs o URLs tras los `AGENTS.md`
  (útil para `docs/adr/*.md`), no para las reglas duras.
- **Operativa:** abrir la sesión con el directorio de trabajo en `~/git/keydex`, no en
  `~/git`. Si arranca un nivel por encima, el `AGENTS.md` del repo no entra hasta que
  el agente explore dentro del repo.

### 24.3 Regla → mecanismo de cumplimiento

| Regla | Mecanismo verificable |
|---|---|
| core sin pyo3 | `just gate`: `cargo tree -p keydex-core` no contiene `pyo3` |
| core sin entorno/reloj/FS | `clippy.toml` con `disallowed-methods` (`std::env::*`, `SystemTime::now`, `std::fs::*`) + `deny(clippy::disallowed_methods)` |
| errores estructurados | `deny(panic, unwrap_used, expect_used)` en core |
| sin estado global | `forbid(static_mut_refs)` + review; caches con `RwLock`/`OnceLock` |
| cero FFI por acceso | bench gate: acceso ±10% vs dataclass nativa |
| spans en `explain` | test dorado con `line != null` |
| dependencias prohibidas | `cargo deny` con `bans` para `serde_yaml` |
| determinismo | test que baraja `os.environ` y compara `dump()` byte a byte |
| no fuga de secretos | suite dedicada por superficie (`repr`, `dump`, `explain`, errores) |

### 24.4 Calidad del propio `AGENTS.md`

- ≤100 líneas. Si crece, el agente lo hojea en vez de leerlo.
- Todo en imperativo y comprobable: nada de "preferiblemente" ni "considera".
- Sin duplicar la spec: solo reglas y punteros a `docs/`.
- Cada regla con su comando de verificación al lado.
- Revisarlo cuando una fase cambia: una regla caducada es peor que ninguna.

---

## 25. Anexo: reserva de nombres

| Nombre | PyPI | crates.io |
|---|---|---|
| `keydex` | reservado: 0.0.1 publicada | no se usa |
| `keydex-core` | n/a | reservado: 0.0.1 publicada |
| `keydex-py` | n/a | no se publica (`publish = false`) |

Reservado el 2026-10-27: `keydex` 0.0.1 en PyPI y `keydex-core` 0.0.1 en crates.io.

---

## 26. Repositorio y publicación

### 26.1 Un solo repo

**`keydex`** (monorepo). Publicar en crates.io y PyPI no requiere repos separados:
son paquetes distintos construidos desde el mismo tag.

La razón técnica no es la comodidad, es que los **tests cruzados** solo tienen sentido
si viven juntos:

- fixtures compartidos del `keydex.schema/v1` consumidos por Rust y por Python;
- gate de arquitectura (`cargo tree` de `keydex-core`) y gate de rendimiento
  (benchmark Rust vs decode Python) en el mismo pipeline;
- un cambio de IR rompe ambas mitades a la vez, y el CI lo ve en un solo run.

Con repos separados eso se convierte en submódulos, duplicación de fixtures o releases
desincronizadas.

### 26.2 Layout del repo

```
keydex/
├── .github/
│   ├── workflows/{ci.yml,release.yml}
│   └── ISSUE_TEMPLATE/
├── crates/
│   ├── keydex-core/          # publish = true en crates.io
│   │   ├── benches/          # criterion
│   │   └── AGENTS.md
│   └── keydex-py/            # publish = false (se distribuye por PyPI)
├── python/
│   ├── keydex/               # API pública, tipos, integraciones
│   ├── tests/
│   └── bench/                # pytest-codspeed
├── docs/
│   ├── spec.md               # esta especificación
│   └── adr/                  # 0001-*.md …
├── examples/                 # uso mínimo, config por fichero y por env
├── AGENTS.md
├── CHANGELOG.md
├── SECURITY.md
├── LICENSE-MIT / LICENSE-APACHE
├── README.md
├── Cargo.toml                # workspace
├── Cargo.lock                # se commitea: distribuimos binarios/wheels
├── pyproject.toml            # maturin: python-source = "python",
│                             #          manifest-path = "crates/keydex-py/Cargo.toml"
├── rust-toolchain.toml
├── clippy.toml               # disallowed-methods (regla 2)
├── deny.toml                 # bans: serde_yaml
├── rustfmt.toml
├── justfile
└── .gitignore / .gitattributes
```

### 26.3 Cuándo dividir en más repos

| Repo futuro | Cuándo crearlo |
|---|---|
| `keydex-hydra` (composición por grupos) | solo si el plugin crece y tiene cadencia propia |
| `keydex-sources-<backend>` | solo si un tercero mantiene el backend |
| `keydex-examples` | nunca por ahora: `examples/` basta |

Regla: **no dividir hasta que exista una cadencia de release distinta o un mantenedor
distinto**. Un repo por concepto es deuda de CI y de versionado.

### 26.4 Tres versiones distintas (no confundirlas)

| Versión | Dónde vive | Cuándo cambia |
|---|---|---|
| Paquete (`0.x.y`) | `Cargo.toml` workspace + `pyproject.toml` | cada release |
| `keydex.schema/v1` | campo `ir` del JSON | solo con ruptura del contrato |
| Artefacto (wheel/`cp314t`) | tags de build | cambios de ABI |

El IR sobrevive a varias versiones del paquete. Es lo que permite que un `dump` viejo
siga siendo legible.

### 26.5 Ficheros que no se pueden saltar

- **`SECURITY.md`**: la librería maneja secretos. Política de divulgación desde el
  primer commit, no después del primer incidente.
- **Licencia dual MIT OR Apache-2.0**: convención del ecosistema Rust y compatible con
  uso corporativo.
- **`CHANGELOG.md`** generado por `release-plz` a partir de conventional commits.
- **`docs/adr/`**: las cinco decisiones irreversibles ya tomadas (nombre, sin
  expresiones, `abi3` + `cp314t`, IR versionado, spans obligatorios).

### 26.6 Flujo de release

1. Conventional commits en `main` (trunk-based, rama protegida).
2. `release-plz` abre PR de release: bump + CHANGELOG.
3. Merge → tag `vX.Y.Z`.
4. CI publica `keydex-core` en crates.io **primero** y después los wheels en PyPI
   (el orden importa: `keydex-py` depende del core por versión).

> La reserva de nombres (`keydex` en PyPI y crates.io) es una publicación real y la
> ejecuta el usuario, no el agente.
