# Política de seguridad

`keydex` maneja configuración que puede contener secretos. Los fallos que nos
preocupan especialmente son:

- fuga de un secreto en `repr`, `str`, logs, mensajes de error, `dump()` o `explain()`;
- lectura de ficheros arbitrarios provocada por la configuración;
- ejecución de código a través de la interpolación o de un resolver;
- corrupción de memoria o `panic` provocado por una entrada hostil.

## Cómo reportar

**No abras un issue público.** Usa
[GitHub Security Advisories](https://github.com/carlosemart/keydex/security/advisories/new)
o escribe a `spi101@gmail.com` con el asunto `keydex security`.

Incluye: versión, plataforma, entrada mínima que reproduce el problema y el impacto
que crees que tiene. Si el reporte contiene secretos reales, redáctalos.

## Compromiso

- Acuse de recibo en 72 horas.
- Evaluación y plan en 7 días.
- Crédito en el aviso si lo deseas.

## Versiones soportadas

Ninguna todavía (pre-alpha). A partir de `0.1.0` se mantendrá la última minor.
