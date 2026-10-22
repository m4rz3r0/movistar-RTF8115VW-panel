import { expect, type Locator, type Page, type Route, test } from "@playwright/test";
import { asentar, entrar, recogerErrores } from "./util";

// Detalles de Mejoras en el panel del worker (no en el de Mejoras, que
// comparten otros ficheros): lo que el servidor responde se cambia en el
// navegador cuando hace falta un estado concreto, y nada se aplica.
// - La revisión marca en el selector de DNS el proveedor con el que el
//   servidor preparó el lote (el que ya estaba puesto), no siempre Quad9.
// - Cada casilla de la revisión se nombra con su paso y su elemento.
// - El error de una tarjeta se borra al volver a leer.
// - Si la lectura de la página falla, la barra lateral no se queda con la
//   cuenta anterior.

interface Paso {
  clave: string;
  estado: unknown;
}
interface Lote {
  id: string;
  estado: unknown;
  pasos: Paso[];
}

/** La tarjeta del lote cuyo título es `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.tarjeta-lote").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/**
 * Cambia cada lectura de `leer_mejoras`: cada lote con un paso pendiente
 * (así «Revisar y aplicar» está activo, lea lo que lea el panel del worker)
 * y, en Seguridad, el espejo sin leer si `sinLeer` (sale «Volver a leer»).
 */
async function forzarPendientes(page: Page, sinLeer = false) {
  await page.route("**/api/leer_mejoras", async (r) => {
    const respuesta = await r.fetch();
    const m = await respuesta.json();
    for (const l of m.lotes as Lote[]) {
      const primero = l.id === "seguridad" ? "upnp" : "dns_casa";
      for (const p of l.pasos) {
        if (p.clave === primero) p.estado = "Pendiente";
        if (sinLeer && p.clave === "espejo") {
          p.estado = { SinLeer: { motivo: "No se pudo leer la pestaña Sistema." } };
        }
      }
      l.estado = { Pendientes: { pendientes: 1, total: l.pasos.length } };
    }
    await r.fulfill({ response: respuesta, json: m });
  });
}

/** `/mejoras`, desde el menú (la lectura pasa por las rutas cambiadas). */
async function irAMejoras(page: Page) {
  await asentar(page);
  await page.locator('nav a[href="/mejoras"]').click();
  await page.waitForURL((u) => u.pathname === "/mejoras");
}

/** Abre la revisión del lote `titulo`. */
async function revisar(page: Page, titulo: string): Promise<Locator> {
  await tarjeta(page, titulo).getByRole("button", { name: "Revisar y aplicar" }).click();
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo.locator("#grupo-titulo")).toHaveText("Revisa el cambio");
  return dialogo;
}

/** La petición sigue con un cuerpo que el servidor rechaza con un error de verdad. */
function conCuerpo(cuerpo: string) {
  return (r: Route) => r.continue({ postData: cuerpo });
}

/** Los errores de la consola, sin el aviso de Chromium por las respuestas de error que se piden a propósito. */
function sinErroresDeRed(errores: string[]): string[] {
  return errores.filter((e) => !e.startsWith("Failed to load resource"));
}

test.describe("Mejoras: detalles de la revisión y de las tarjetas", () => {
  test.use({ locale: "es-ES", viewport: { width: 1440, height: 900 } });

  test("el selector de DNS marca el proveedor ya puesto", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await forzarPendientes(page);
    // El servidor dice que preparó el lote con Cloudflare (el que tendría el
    // router); el selector lo marca a él.
    await page.route("**/api/preparar_lote", async (r) => {
      const respuesta = await r.fetch();
      const g = await respuesta.json();
      if (respuesta.ok()) g.proveedor_dns = "cloudflare";
      await r.fulfill({ response: respuesta, json: g });
    });
    await irAMejoras(page);
    const dialogo = await revisar(page, "Privacidad máxima");
    await expect(dialogo.getByRole("radio", { name: "Cloudflare", exact: true })).toBeChecked();
    await expect(dialogo.getByRole("radio", { name: "Quad9", exact: true })).not.toBeChecked();
    await dialogo.getByRole("button", { name: "Cancelar" }).click();
    await page.unrouteAll({ behavior: "wait" });
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });

  test("cada casilla se nombra con su paso y su red", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await forzarPendientes(page);
    await irAMejoras(page);
    const dialogo = await revisar(page, "Seguridad recomendada");
    for (const nombre of [
      "Quitar el WPS · Red WiFi 5 GHz",
      "Cifrar la WiFi con WPA2-AES · Red WiFi 5 GHz",
      "Quitar el WPS · Red WiFi 2,4 GHz",
    ]) {
      await expect(dialogo.getByRole("checkbox", { name: nombre, exact: true })).toHaveCount(1);
    }
    await dialogo.getByRole("button", { name: "Cancelar" }).click();
    await page.unrouteAll({ behavior: "wait" });
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });

  test("el error de una tarjeta se borra al volver a leer", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await forzarPendientes(page, true);
    // Un lote que no existe: el servidor responde con su error.
    await page.route(
      "**/api/preparar_lote",
      conCuerpo(JSON.stringify({ id: "otro", opciones: {} })),
    );
    await irAMejoras(page);
    const seguridad = tarjeta(page, "Seguridad recomendada");
    await seguridad.getByRole("button", { name: "Revisar y aplicar" }).click();
    const error = seguridad.locator(".lote-pie [role=alert]");
    await expect(error).toBeVisible();
    const fresca = page.waitForResponse(
      (r) =>
        r.url().endsWith("/api/leer_mejoras") &&
        /"fresca":\s*true/.test(r.request().postData() ?? ""),
    );
    await seguridad.getByRole("button", { name: "Volver a leer" }).click();
    await fresca;
    await expect(
      tarjeta(page, "Seguridad recomendada").getByRole("button", { name: "Volver a leer" }),
    ).toBeVisible();
    await expect(error).toHaveCount(0);
    await page.unrouteAll({ behavior: "wait" });
    expect(sinErroresDeRed(errores), "errores o avisos en la consola").toEqual([]);
  });

  test("sin lectura, la barra no se queda con la cuenta anterior", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await forzarPendientes(page);
    await irAMejoras(page);
    const cuenta = page.locator("nav .cuenta");
    await expect(cuenta).toBeVisible();
    // La siguiente lectura de la página falla.
    await page.unrouteAll({ behavior: "wait" });
    await page.route("**/api/leer_mejoras", conCuerpo("no es JSON"));
    await page.locator('nav a[href="/historial"]').click();
    await page.waitForURL((u) => u.pathname === "/historial");
    await page.locator('nav a[href="/mejoras"]').click();
    await page.waitForURL((u) => u.pathname === "/mejoras");
    await expect(page.locator("main p.estado-error[role=alert]")).toBeVisible();
    await expect(cuenta).toHaveCount(0);
    await page.unrouteAll({ behavior: "wait" });
    expect(sinErroresDeRed(errores), "errores o avisos en la consola").toEqual([]);
  });
});
