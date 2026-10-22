import { expect, type Locator, type Page, test } from "@playwright/test";
import { cambiarUpnp, esperarFinal, plazoDeTest } from "./cambios";
import {
  asentar,
  entrar,
  quieta,
  recogerErrores,
  sinDesbordamiento,
  violacionesGraves,
} from "./util";

// Puertos y Red local en lista con columnas (Tarea 5 del rediseño), en
// español contra router-sim, a 360 y 1440 px y en los dos temas:
// - ordenar por la columna «Puerto externo» (de menor a mayor y al revés);
// - el filtro de texto aparece con 9 reglas y no con 8 (las que faltan, en
//   la respuesta de la lectura: nada se escribe en el router);
// - pulsar una fila estrecha la lista a unos 360 px y el detalle entra a
//   su lado, con la dirección `/ajustes/puertos/<id>`; «✕» devuelve el ancho;
// - «Borrar» (desde la barra con dos marcadas y desde el detalle) pide
//   confirmación con el botón de peligro; aquí se cancela;
// - a 360 px las filas van apiladas y el detalle ocupa la pantalla;
// - axe sin violaciones serias, con la lista y con una fila abierta.
// Las capturas (lista entera, una fila abierta y dos marcadas) van a
// capturas/rediseno-5.

const CAPTURAS = "capturas/rediseno-5";

const t = {
  externo: "Puerto externo",
  filtrar: "Filtrar la lista",
  borrar: "Borrar",
  cancelar: "Cancelar",
  revisar: "Revisa el cambio",
  marcadas: "2 marcadas",
  reglas: "Puertos abiertos",
  fijas: "IP fijas",
  dispositivos: "Dispositivos conectados",
};

/** Puertos de las reglas que se añaden a la lectura, en este orden. */
const PUERTOS = [9000, 22, 443, 27015, 3074, 80, 5000, 1194];

/** Valor legible (`{ Texto: … }`) de un campo de una instancia. */
type Campo = { id: string; valor: Record<string, unknown> };
type Instancia = { id: string; campos: Campo[] };

/** Cuántas reglas se añaden a la lectura de Puertos (ver `conReglas`). */
const extra = { n: 0 };

/**
 * Añade `n` reglas a la lectura de Puertos (copias de la de las fixtures,
 * con otro nombre y otro puerto) y la vuelve a leer. Solo cambia lo que ve
 * la página: el router no se toca.
 */
async function conReglas(page: Page, n: number) {
  const primera = extra.n === 0;
  extra.n = n;
  if (primera) await anadirReglas(page);
  // La primera lectura es del servidor: «Volver a leer» la pide otra vez.
  await page.getByRole("button", { name: "Volver a leer" }).click();
  await expect(filas(page)).toHaveCount(1 + n);
}

async function anadirReglas(page: Page) {
  await page.route("**/api/leer_tema", async (ruta) => {
    const r = await ruta.fetch();
    const pagina = await r.json();
    if (pagina.tema === "puertos") {
      const lista = pagina.elementos.find((e: { id: string }) => e.id === "reglas");
      const instancias: Instancia[] = lista.ajustes[0].instancias;
      const base = instancias[0];
      for (let k = 0; k < extra.n; k++) {
        const puerto = String(PUERTOS[k]);
        const campos = base.campos.map((c) => {
          if (c.id === "nombre") return { ...c, valor: { Texto: `Regla ${k + 1}` } };
          if (c.id.startsWith("externo_") || c.id.startsWith("interno_")) {
            return { ...c, valor: { Texto: puerto } };
          }
          return c;
        });
        instancias.push({ id: `e2e-${k + 1}`, campos });
      }
    }
    await ruta.fulfill({ response: r, json: pagina });
  });
}

/** Las filas de la lista de reglas. */
function filas(page: Page): Locator {
  return lista(page, t.reglas).locator("tbody tr.lista-fila");
}

/** La sección de la lista cuyo título es `titulo`. */
function lista(page: Page, titulo: string): Locator {
  return page.locator("section.lista-seccion").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/** Ancho de la tabla de la lista (sin la animación a medias). */
async function anchoDeLaTabla(page: Page): Promise<number> {
  await quieta(page);
  const caja = await lista(page, t.reglas).locator(".lista-tabla").boundingBox();
  return caja?.width ?? 0;
}

async function captura(page: Page, nombre: string, completa: boolean) {
  await quieta(page);
  await page.screenshot({
    path: `${CAPTURAS}/${nombre}.png`,
    fullPage: completa,
    animations: "disabled",
  });
}

test.describe.configure({ mode: "parallel" });

for (const ancho of [360, 1440]) {
  for (const tema of ["oscuro", "claro"] as const) {
    const sufijo = `${ancho}-${tema}`;
    const movil = ancho < 900;
    test.describe(`listas a ${ancho} px en tema ${tema}`, () => {
      test.use({
        locale: "es-ES",
        viewport: { width: ancho, height: 900 },
        colorScheme: tema === "oscuro" ? "dark" : "light",
      });

      test("puertos: ordenar, filtrar, abrir una fila y borrar con confirmación", async ({
        page,
      }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/puertos");
        await asentar(page);
        // Con 8 reglas, sin filtro; con 9, con él.
        await expect(filas(page)).toHaveCount(1);
        extra.n = 0;
        await conReglas(page, 7);
        await expect(page.getByLabel(t.filtrar)).toHaveCount(0);
        await conReglas(page, 8);
        await expect(page.getByLabel(t.filtrar)).toBeVisible();
        await sinDesbordamiento(page, `puertos ${sufijo}`);
        await captura(page, `puertos-lista-${sufijo}`, !movil);
        expect(await violacionesGraves(page), "lista").toEqual([]);

        // Ordenar por «Puerto externo»: de menor a mayor, y al revés.
        const cabecera = lista(page, t.reglas).locator("thead th").filter({ hasText: t.externo });
        await cabecera.getByRole("button").click();
        await expect(cabecera).toHaveAttribute("aria-sort", "ascending");
        await expect(filas(page).first()).toContainText("22");
        await expect(filas(page).last()).toContainText("27015");
        await cabecera.getByRole("button").click();
        await expect(cabecera).toHaveAttribute("aria-sort", "descending");
        await expect(filas(page).first()).toContainText("27015");

        // Filtrar: solo lo que coincide.
        await page.getByLabel(t.filtrar).fill("27015");
        await expect(filas(page)).toHaveCount(1);
        await expect(filas(page).first()).toContainText("27015");
        await page.getByLabel(t.filtrar).fill("");
        await expect(filas(page)).toHaveCount(9);

        if (movil) {
          // A 360 px, apiladas: los datos debajo del nombre, sin tabla.
          const fila = filas(page).first();
          await expect(fila).toHaveCSS("display", "flex");
          const nombre = await fila.locator(".lista-nombre").boundingBox();
          const dato = await fila.locator(".lista-dato").first().boundingBox();
          expect(dato?.y ?? 0).toBeGreaterThan((nombre?.y ?? 0) + (nombre?.height ?? 0) - 1);
        }

        // Dos marcadas: la barra ofrece borrarlas, y pide confirmarlo con
        // el botón de peligro.
        for (const nombre of ["Regla 1", "Regla 2"]) {
          const fila = filas(page).filter({ hasText: nombre });
          await fila.hover();
          await fila.getByRole("button", { name: `Marcar ${nombre}` }).click();
        }
        const barra = page.getByRole("toolbar");
        await expect(barra).toContainText(t.marcadas);
        await expect(barra.getByRole("button", { name: "Activar", exact: true })).toBeVisible();
        await expect(barra.getByRole("button", { name: "Desactivar", exact: true })).toBeVisible();
        const borrar = barra.getByRole("button", { name: t.borrar, exact: true });
        await expect(borrar).toHaveClass(/b-pel/);
        await sinDesbordamiento(page, `marcadas ${sufijo}`);
        await captura(page, `puertos-marcadas-${sufijo}`, !movil);
        await barra.getByRole("button", { name: "Desmarcar todo" }).click();
        await expect(barra).toBeHidden();
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });

      test("puertos: la fila abierta estrecha la lista y «Borrar» pide confirmación", async ({
        page,
      }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/puertos");
        await asentar(page);
        const ancho0 = await anchoDeLaTabla(page);
        const fila = filas(page).filter({ hasText: "desc-1" });
        const id = (await fila.getAttribute("id"))?.replace(/^fila-/, "") ?? "";
        expect(id).not.toBe("");
        await fila.locator(".lista-nombre").click();
        await page.waitForURL((u) => u.pathname === `/ajustes/puertos/${id}`);
        const detalle = page.locator(".detalle-lateral");
        await expect(detalle).toBeVisible();
        await expect(page.locator("#detalle-titulo")).toHaveText("desc-1");
        await expect(page.locator("#detalle-titulo")).toBeFocused();
        if (!movil) {
          // La lista, a unos 360 px; las columnas de datos, fuera.
          const estrecha = await anchoDeLaTabla(page);
          expect(estrecha).toBeGreaterThan(330);
          expect(estrecha).toBeLessThan(390);
          await expect(fila.locator(".lista-dato").first()).toBeHidden();
          await expect(fila.locator(".lista-estado")).toBeVisible();
        } else {
          // En el móvil, a pantalla completa.
          const caja = await detalle.boundingBox();
          expect(caja?.width ?? 0).toBeGreaterThanOrEqual(ancho - 1);
        }
        await sinDesbordamiento(page, `fila abierta ${sufijo}`);
        await captura(page, `puertos-abierta-${sufijo}`, !movil);
        expect(await violacionesGraves(page), "fila abierta").toEqual([]);

        // «Borrar» desde el detalle: la revisión con el botón de peligro;
        // se cancela y no se borra nada.
        await detalle.getByRole("button", { name: "Borrar la regla desc-1" }).click();
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
        const confirmar = dialogo.getByRole("button", { name: t.borrar, exact: true });
        await expect(confirmar).toHaveClass(/b-pel/);
        await expect(dialogo.getByRole("button", { name: "Aplicar" })).toHaveCount(0);
        await dialogo.getByRole("button", { name: t.cancelar }).click();
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        await expect(filas(page).filter({ hasText: "desc-1" })).toHaveCount(1);
        // Cancelar lo deja como estaba: el detalle sigue abierto.
        await expect(detalle).toBeVisible();
        expect(new URL(page.url()).pathname).toBe(`/ajustes/puertos/${id}`);

        // «✕» (o, en el móvil, «‹ Puertos») lo cierra y la lista recupera su
        // ancho.
        if (movil) {
          await page.getByRole("button", { name: "‹ Puertos" }).click();
        } else {
          await page.getByRole("button", { name: "Cerrar el detalle" }).click();
        }
        await page.waitForURL((u) => u.pathname === "/ajustes/puertos");
        await expect(page.locator(".detalle-lateral")).toHaveCount(0);
        expect(await anchoDeLaTabla(page)).toBeCloseTo(ancho0, 0);
        // El foco vuelve a la fila.
        await expect(filas(page).filter({ hasText: "desc-1" })).toBeFocused();
        // Y Esc también.
        await page.keyboard.press("Enter");
        await page.waitForURL((u) => u.pathname === `/ajustes/puertos/${id}`);
        await page.keyboard.press("Escape");
        await page.waitForURL((u) => u.pathname === "/ajustes/puertos");
        await expect(filas(page).filter({ hasText: "desc-1" })).toBeFocused();
        // La regla nueva: al cerrar su detalle, el foco vuelve a «Abrir un
        // puerto».
        const abrir = page.getByRole("button", { name: "Abrir un puerto" });
        await abrir.click();
        await page.waitForURL((u) => u.pathname === "/ajustes/puertos/reglas");
        await page.keyboard.press("Escape");
        await page.waitForURL((u) => u.pathname === "/ajustes/puertos");
        await expect(abrir).toBeFocused();
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });

      test("puertos: «Borrar» con cambios sin guardar, un solo diálogo", async ({ page }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/puertos");
        await asentar(page);
        const fila = filas(page).filter({ hasText: "desc-1" });
        const id = (await fila.getAttribute("id"))?.replace(/^fila-/, "") ?? "";
        await fila.locator(".lista-nombre").click();
        await page.waitForURL((u) => u.pathname === `/ajustes/puertos/${id}`);
        const detalle = page.locator(".detalle-lateral");
        const nombre = detalle.getByLabel("Nombre", { exact: true });
        await nombre.fill("desc-1 cambiada");
        await detalle.getByRole("button", { name: "Borrar la regla desc-1" }).click();
        // Solo la revisión del borrado, no la pregunta de los cambios.
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
        await expect(dialogo).toHaveCount(1);
        await dialogo.getByRole("button", { name: t.cancelar }).click();
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        // Todo como estaba: el detalle abierto, con lo editado.
        expect(new URL(page.url()).pathname).toBe(`/ajustes/puertos/${id}`);
        await expect(nombre).toHaveValue("desc-1 cambiada");
        // Al cerrarlo, la pregunta de siempre; se descarta.
        await page.keyboard.press("Escape");
        await page
          .locator("dialog[open]")
          .getByRole("button", { name: "Descartar los cambios" })
          .click();
        await page.waitForURL((u) => u.pathname === "/ajustes/puertos");
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });

      test("red local: dispositivos e IP fijas en lista", async ({ page }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/red_local");
        await asentar(page);
        const dispositivos = lista(page, t.dispositivos);
        const fijas = lista(page, t.fijas);
        await expect(dispositivos.locator("tbody tr.lista-fila")).not.toHaveCount(0);
        await expect(fijas.locator("tbody tr.lista-fila")).toHaveCount(2);
        // Los dispositivos no tienen nada que encender ni marcar.
        await expect(dispositivos.getByRole("switch")).toHaveCount(0);
        await expect(dispositivos.locator(".marcar")).toHaveCount(0);
        await expect(fijas.getByRole("switch")).toHaveCount(2);
        await sinDesbordamiento(page, `red local ${sufijo}`);
        await captura(page, `red_local-lista-${sufijo}`, !movil);
        expect(await violacionesGraves(page), "red local").toEqual([]);
        // Las dos IP fijas marcadas.
        for (const fila of await fijas.locator("tbody tr.lista-fila").all()) {
          await fila.hover();
          await fila.locator(".marcar").click();
        }
        await expect(page.getByRole("toolbar")).toContainText(t.marcadas);
        await captura(page, `red_local-marcadas-${sufijo}`, !movil);
        await page.getByRole("button", { name: "Desmarcar todo" }).click();
        // Un dispositivo abierto: sus datos y qué hacer con su IP.
        const host2 = dispositivos.locator("tbody tr.lista-fila").filter({ hasText: "host-2" });
        await host2.locator(".lista-nombre").click();
        await page.waitForURL((u) => /^\/ajustes\/red_local\/[0-9a-f-]+$/.test(u.pathname));
        await expect(page.locator("#detalle-titulo")).toHaveText("host-2");
        await expect(page.locator(".detalle-lateral")).toContainText("192.168.1.62");
        await sinDesbordamiento(page, `dispositivo abierto ${sufijo}`);
        await captura(page, `red_local-abierta-${sufijo}`, !movil);
        expect(await violacionesGraves(page), "dispositivo abierto").toEqual([]);
        await page.keyboard.press("Escape");
        await page.waitForURL((u) => u.pathname === "/ajustes/red_local");
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}

// Con otro cambio en curso, el interruptor de una fila no se aplica: el
// aviso lo dice (409) y la fila vuelve a como estaba. Una sola combinación:
// cambia UPnP de verdad (y lo deja como estaba).
test.describe("listas: interruptor con otro cambio en curso", () => {
  test.use({ locale: "es-ES", viewport: { width: 1440, height: 900 } });
  test("la fila vuelve a su estado", async ({ page }) => {
    test.setTimeout(plazoDeTest(2));
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/ajustes/puertos");
    await asentar(page);
    const fila = filas(page).filter({ hasText: "desc-1" });
    const activa = fila.getByRole("switch", { name: "Regla desc-1 activa" });
    await expect(activa).toHaveAttribute("aria-checked", "true");
    const cambio = await cambiarUpnp(page);
    await activa.click();
    const error = page.locator(".avisos-cambio [role=alert] .aviso-cambio");
    await expect(error).toContainText("Hay otro cambio en curso");
    await expect(activa).toHaveAttribute("aria-checked", "true");
    await expect(fila.locator(".lista-estado")).toContainText("Sin dispositivo");
    await esperarFinal(page, cambio);
    // UPnP como estaba.
    await esperarFinal(page, await cambiarUpnp(page));
    // El 409 de la petición rechazada sale en la consola: es lo esperado.
    expect(
      errores.filter((e) => !e.includes("409")),
      "errores o avisos en la consola",
    ).toEqual([]);
  });
});
