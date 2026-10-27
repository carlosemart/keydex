# 0007 — El Schema IR viaja como JSON con `serde`

- Estado: aceptada
- Fecha: 2026-10-27

## Contexto

El motor necesita consumir la descripción del modelo que genera Python. Las opciones
eran: pasar objetos Python al binding (metía PyO3 en el camino de la lógica y rompía la
separación), inventar un formato binario propio, o usar JSON.

## Decisión

`serde` + `serde_json` en `keydex-core`, tras la feature `schema`. `Schema::from_json`
parsea y valida la versión del contrato; el binding solo transporta el texto.

## Consecuencias

- Dos dependencias nuevas en el core: `serde` (con `derive`) y `serde_json`.
- El IR queda desacoplado del transporte: mañana puede venir de un fichero, de la red o
  de otro lenguaje sin tocar la lógica.
- El coste de parseo del IR es despreciable frente al de los ficheros de configuración.
- La feature `schema` permite compilar el core sin soporte de esquema si solo se usa en
  modo diccionario.
