# Aviso legal

*English below.*

## Proyecto independiente

Este panel es un proyecto independiente, sin relación con Telefónica,
Movistar, O2 ni Askey, que no lo patrocinan, revisan ni respaldan. Las marcas
y los nombres de productos citados pertenecen a sus dueños y se usan solo para
decir con qué router funciona el panel.

## El router y su uso

- El router suele ser propiedad del operador, que lo presta con la línea. El
  uso del panel es responsabilidad de quien lo usa.
- Desactivar TR-069 (la gestión remota del operador) o cambiar otros ajustes
  puede afectar al servicio (internet, telefonía, televisión) o a lo que diga
  el contrato con el operador. Antes de cambiar algo, mira qué hace y si te
  conviene.
- El software se ofrece «tal cual», sin garantía de ningún tipo (ver
  [`LICENSE`](LICENSE)).

## Qué contiene y qué no

- No incluye código, firmware ni páginas del router. El panel habla con la web
  del router de cada usuario, en su propia red, igual que el navegador. Las
  pruebas usan un extracto mínimo de la lista del cortafuegos
  (`crates/hgu/fixtures/paginas/`): solo las dos variables de datos que lee el
  panel, con valores inventados, sin el HTML ni el JavaScript de la página.
- No publica credenciales de terceros: el panel usa las que cada usuario
  escribe para su router y las guarda cifradas en su propia instalación.
  Tampoco publica valores por defecto que permitan entrar en routers ajenos.
- Las capturas del repositorio (las del README y las respuestas del router
  que usa el simulador para las pruebas) están saneadas: no llevan direcciones,
  nombres, claves ni identificadores reales.
- [`docs/api.md`](docs/api.md) y `crates/catalogo/funciones.toml` describen la
  API de la web del router con texto propio. Los nombres de las funciones y de
  los campos, y sus etiquetas, son los datos funcionales necesarios para
  hablar con el router (interoperabilidad), no una copia de su código.

---

# Legal notice

## Independent project

This panel is an independent project, not affiliated with Telefónica,
Movistar, O2 or Askey, which do not sponsor, review or endorse it. The
trademarks and product names mentioned belong to their owners and are used
only to say which router the panel works with.

## The router and its use

- The router usually belongs to the operator, which lends it with the line.
  Whoever uses the panel is responsible for that use.
- Turning off TR-069 (the operator's remote management) or changing other
  settings may affect the service (internet, telephony, TV) or what your
  contract with the operator says. Before changing something, check what it
  does and whether it suits you.
- The software is provided "as is", without warranty of any kind (see
  [`LICENSE`](LICENSE)).

## What it contains and what it does not

- It contains no code, firmware or pages from the router. The panel talks to
  each user's own router web interface, on their own network, as a browser
  does. The tests use a minimal extract of the firewall list
  (`crates/hgu/fixtures/paginas/`): only the two data variables the panel
  reads, with made-up values, without the page's HTML or JavaScript.
- It publishes no third-party credentials: the panel uses the ones each user
  types for their router and stores them encrypted in their own installation.
  Nor does it publish default values that would let anyone into other people's
  routers.
- The captures in the repository (the README screenshots and the router
  responses that the simulator uses for tests) are sanitised: they carry no
  real addresses, names, keys or identifiers.
- [`docs/api.md`](docs/api.md) and `crates/catalogo/funciones.toml` describe
  the router's web API in original text. The names of the functions and
  fields, and their labels, are the functional data needed to talk to the
  router (interoperability), not a copy of its code.
