# 0002 — La interpolación no admite expresiones

- Estado: aceptada
- Fecha: 2026-10-27

## Contexto

OmegaConf permita expresiones en resolvers; Spring solo permite referencias y
placeholders. Un lenguaje de expresiones implica un parser propio, semántica de tipos,
casos límite y una superficie de ataque.

## Decisión

La interpolación admite únicamente referencias (`${a.b}`, `${.sibling}`, `${..parent}`),
valor por defecto (`${a:-x}`) y resolvers puros registrados (`${env:VAR}`). Sin
aritmética, sin condicionales, sin acceso a métodos, sin `eval`.

## Consecuencias

- El motor es portable a Rust sin un intérprete.
- Un cálculo derivado se hace en el código del usuario, no en la config.
- Si algún día se necesita, va como plugin y nunca en el core.
