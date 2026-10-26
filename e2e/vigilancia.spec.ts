import { expect, type Locator, type Page, test } from "@playwright/test";
import { cambiarUpnp, cambiarVigilado, estadoCambio } from "./cambios";
import { CONTRASENA, USUARIO, vigilancia } from "./instancias";
import {
  asentar,
  hidratada,
  quieta,
  recogerErrores,
  sinDesbordamiento,
  violacionesGraves,
} from "./util";
import { capturaOpcional } from "./util";

// La vigilancia y la cola, en un panel propio que vigila 5 min
// (instancias.ts), así que nada termina solo durante la prueba:
// - Ajustes → WiFi a 1440 px: encender la red de invitados (se vigila: el
//   panel no sabe si llega al router por cable); mientras, encender la de
//   2,4 GHz y la de 5 GHz esperan en la cola («En cola», «Quitar de la
//   cola»). «Quitar de la cola» cancela la de 5 GHz; «Todo bien, terminar»
//   da por buena la de invitados y la de 2,4 GHz se aplica detrás.
// - Historial a 360 px: un cambio vigilándose y otro en cola, con «Todo
//   bien, terminar» y «Quitar de la cola» en su fila (44 px de alto en el
//   móvil); lo que anota cada uno.
// Capturas en capturas/vigilancia/.

const CAPTURAS = "capturas/vigilancia";

const t = {
  red24: "Red WiFi 2,4 GHz",
  red5: "Red WiFi 5 GHz",
  invitados: "Red de invitados",
  terminar: "Todo bien, terminar",
  quitar: "Quitar de la cola",
  deshacer: "Deshacer",
  comprobando: "Comprobando…",
  enCola: "En cola",
  enColaAviso: "En cola (puesto 1)",
  hecho: "Hecho: el router tiene ya el cambio.",
  noAplicado: "No se ha aplicado",
  encendida: "Encendida",
  apagada: "Apagada",
  terminada: "e2e dio la vigilancia por terminada antes de tiempo",
  quitado: "e2e lo quitó de la cola",
};

test.use({ locale: "es-ES", baseURL: vigilancia.url });
test.describe.configure({ mode: "serial" });

/** Entra en el panel de la vigilancia. */
async function entrarEnSuPanel(page: Page) {
  await page.goto("/login");
  await hidratada(page);
  await page.waitForLoadState("networkidle");
  await page.locator("#usuario").fill(USUARIO);
  await page.locator("#contrasena").fill(CONTRASENA);
  await page.locator("form button[type=submit]").click();
  await page.waitForURL((url) => url.pathname === "/");
}

/** La tarjeta cuyo título es exactamente `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.tarjeta-el").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/** La insignia de estado de una tarjeta. */
function insignia(card: Locator): Locator {
  return card.locator(".tarjeta-insignia .ins");
}

/** La línea del aviso del cambio que dice `texto`. */
function linea(page: Page, texto: string): Locator {
  return page.locator(".avisos-cambio [role=status] .aviso-cambio").filter({ hasText: texto });
}

test("Ajustes: terminar la vigilancia y la cola en el aviso", async ({ page }) => {
  test.setTimeout(120_000);
  const errores = recogerErrores(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await entrarEnSuPanel(page);
  await page.goto("/ajustes/wifi");
  await asentar(page);
  const invitados = tarjeta(page, t.invitados);
  const red24 = tarjeta(page, t.red24);
  const red5 = tarjeta(page, t.red5);

  // Se vigila: «Comprobando…» y, en el aviso, «Todo bien, terminar».
  await invitados.getByRole("switch", { name: `${t.invitados} apagada` }).click();
  await expect(insignia(invitados)).toHaveText(t.comprobando, { timeout: 15_000 });
  const primero = linea(page, t.invitados);
  await expect(primero.getByRole("button", { name: t.terminar })).toBeVisible();

  // Mientras, otros dos: esperan en la cola, cada uno con su línea.
  await red24.getByRole("switch", { name: `${t.red24} apagada` }).click();
  const segundo = linea(page, t.red24);
  await expect(segundo).toContainText(t.enColaAviso, { timeout: 15_000 });
  await expect(insignia(red24)).toHaveText(t.enCola);
  await expect(red24.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  await red5.getByRole("switch", { name: `${t.red5} apagada` }).click();
  const tercero = linea(page, t.red5);
  await expect(tercero).toContainText("En cola (puesto 2)", { timeout: 15_000 });
  await expect(tercero.getByRole("button", { name: t.quitar })).toBeVisible();
  await quieta(page);
  await capturaOpcional(page, { path: `${CAPTURAS}/aviso-vigilando-y-cola-1440.png`, animations: "disabled" });
  expect(await violacionesGraves(page), "aviso con la cola").toEqual([]);

  // «Quitar de la cola»: no se aplica y la tarjeta vuelve a como estaba.
  await tercero.getByRole("button", { name: t.quitar }).click();
  await expect(tercero).toContainText(t.noAplicado);
  await expect(insignia(red5)).toHaveText(t.apagada);
  await expect(red5.getByRole("switch")).toHaveAttribute("aria-checked", "false");

  // «Todo bien, terminar»: aplicado ya, y el de la cola se aplica detrás.
  await primero.getByRole("button", { name: t.terminar }).click();
  await expect(primero).toContainText(t.hecho);
  await expect(primero.getByRole("button", { name: t.deshacer })).toBeVisible();
  // La página vuelve a leer el router mientras el siguiente se prepara (las
  // peticiones al router van en serie): «Actualizando…» unos segundos.
  await expect(insignia(invitados)).toHaveText(t.encendida, { timeout: 15_000 });
  await expect(insignia(red24)).toHaveText(t.comprobando, { timeout: 15_000 });
  await expect(segundo.getByRole("button", { name: t.terminar })).toBeVisible({ timeout: 15_000 });
  await quieta(page);
  await capturaOpcional(page, { path: `${CAPTURAS}/aviso-terminado-y-siguiente-1440.png`, animations: "disabled" });
  await segundo.getByRole("button", { name: t.terminar }).click();
  await expect(segundo).toContainText(t.hecho);
  await expect(insignia(red24)).toHaveText(t.encendida);
  await sinDesbordamiento(page, "aviso 1440");
  expect(errores, "errores o avisos en la consola").toEqual([]);
});

test("Historial: terminar y quitar de la cola desde la fila, en el móvil", async ({ page }) => {
  test.setTimeout(120_000);
  const errores = recogerErrores(page);
  await page.setViewportSize({ width: 360, height: 800 });
  await entrarEnSuPanel(page);
  // Uno vigilándose (la reserva de IP toca el DHCP) y otro en la cola.
  const vigilado = await cambiarVigilado(page);
  const enCola = await cambiarUpnp(page);
  expect(await estadoCambio(page, vigilado)).toBe("Vigilando");
  expect(await estadoCambio(page, enCola)).toBe("EnCola");
  await page.goto("/historial");
  await asentar(page);
  const filaVigilada = page.locator(`#cambio-${vigilado}`);
  const filaEnCola = page.locator(`#cambio-${enCola}`);
  const terminar = filaVigilada.getByRole("button", { name: t.terminar });
  const quitar = filaEnCola.getByRole("button", { name: t.quitar });
  await expect(terminar).toBeVisible();
  await expect(quitar).toBeVisible();
  for (const b of [terminar, quitar]) {
    const caja = await b.boundingBox();
    expect(caja?.height ?? 0, "zona táctil de 44 px").toBeGreaterThanOrEqual(44);
  }
  await sinDesbordamiento(page, "historial 360");
  await quieta(page);
  await capturaOpcional(page, { path: `${CAPTURAS}/historial-vigilando-y-cola-360.png`, fullPage: true });
  expect(await violacionesGraves(page), "historial con la cola").toEqual([]);

  await quitar.click();
  await expect(filaEnCola).toContainText(t.quitado);
  await expect(filaEnCola.getByRole("button", { name: t.quitar })).toHaveCount(0);
  await terminar.click();
  await expect(filaVigilada).toContainText(t.terminada);
  await expect(filaVigilada.getByRole("button", { name: t.terminar })).toHaveCount(0);
  expect(await estadoCambio(page, vigilado)).toBe("Aplicado");
  expect(await estadoCambio(page, enCola)).toBe("Cancelado");
  await quieta(page);
  await capturaOpcional(page, { path: `${CAPTURAS}/historial-terminado-y-quitado-360.png`, fullPage: true });
  expect(errores, "errores o avisos en la consola").toEqual([]);
});
