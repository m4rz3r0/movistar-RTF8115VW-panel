import { expect, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, ajusteDe, cambiarUpnp, esperarFinal, plazoDeTest } from "./cambios";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// «Cómo estaba al instalar» (/ajustes/foto-de-partida) contra router-sim: el
// panel instalado del e2e tiene foto de partida y, después, UPnP y la clave
// de la red de 5 GHz cambiados (e2e_grupo). Se entra desde la cabecera de
// Puertos, se vuelve UPnP a su valor de la foto con el diálogo de siempre
// (Esc no lo cierra mientras confirma) y se deja como estaba; a 360 px, la
// clave que cambió se ve con el ojo; en
// Teléfono, volver a un valor pide la clave de la línea (el router no la
// deja leer).
//
// Cada test deja el panel como estaba: se reparten entre los workers.
test.describe.configure({ mode: "parallel" });

const RUTA = "/ajustes/foto-de-partida";
const UPNP = "Apertura automática de puertos (UPnP)";

/** Llama a la server function `nombre` con la sesión de la página. */
async function api(page: Page, nombre: string, cuerpo: unknown) {
  const r = await page.request.post(`/api/${nombre}`, { data: cuerpo });
  expect(r.status(), `${nombre}: ${await r.text()}`).toBe(200);
  return r.json();
}

/** ¿UPnP es distinto del de la foto? */
async function upnpCambiado(page: Page): Promise<boolean> {
  const c = await api(page, "comparar_con_partida", {});
  return c.diferencias.some((d: { ajuste: string }) => d.ajuste === "puertos.upnp");
}

/** UPnP encendido ahora. */
async function upnpActivo(page: Page): Promise<boolean> {
  const tema = await api(page, "leer_tema", { tema: "puertos" });
  const a = ajusteDe(tema, "puertos.upnp");
  return a.campos.find((c) => c.id === "activo")?.valor.Booleano as boolean;
}

/** Sin funciones ni parámetros de la API en un texto visible. */
function sinApi(texto: string) {
  expect(texto).not.toMatch(/\bcb[A-Z]\w+/);
  expect(texto).not.toMatch(/adm_state|key_phrase|\w+\.asp\b/);
}

test.use({ locale: "es-ES" });

test("volver UPnP a como estaba al instalar", async ({ page }) => {
  // Hasta tres cambios: provocar la diferencia, volver y dejarlo como estaba.
  test.setTimeout(plazoDeTest(3));
  await page.setViewportSize({ width: 1440, height: 900 });
  const errores = recogerErrores(page);
  await entrar(page);
  const original = await upnpActivo(page);
  if (!(await upnpCambiado(page))) await esperarFinal(page, await cambiarUpnp(page));

  // Desde la cabecera de un tema.
  await page.goto("/ajustes/puertos");
  await asentar(page);
  await page.getByRole("link", { name: "Ver qué ha cambiado desde que instalaste el panel" }).click();
  await page.waitForURL((u) => u.pathname === RUTA);
  await asentar(page);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Cómo estaba al instalar");
  await expect(page.getByText(/^Comparado con cómo estaba el router el /)).toBeVisible();
  const tema = page.locator("section.foto-tema").filter({
    has: page.getByRole("heading", { level: 2, name: "Puertos", exact: true }),
  });
  const tarjeta = tema.locator("article.foto-ajuste").filter({
    has: page.getByRole("heading", { level: 3, name: UPNP, exact: true }),
  });
  await expect(tarjeta).toBeVisible();
  await expect(tarjeta.locator(".foto-valores dt")).toHaveText(["Al instalar", "Ahora"]);
  sinApi(await page.locator("main").innerText());
  await sinDesbordamiento(page, "foto 1440");

  // «Volver a este valor»: el diálogo de siempre, y aplicado.
  await tarjeta.getByRole("button", { name: "Volver a este valor" }).click();
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo.locator("#grupo-titulo")).toHaveText("Revisa el cambio");
  sinApi(await dialogo.innerText());
  // Mientras confirma, Esc no cierra el diálogo.
  await page.route(
    "**/api/confirmar_ajuste",
    async (r) => {
      await new Promise((ok) => setTimeout(ok, 1_500));
      await r.continue();
    },
    { times: 1 },
  );
  await dialogo.getByRole("button", { name: "Aplicar", exact: true }).click();
  await expect(dialogo.getByRole("button", { name: "Aplicando…" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialogo).toBeVisible();
  await expect(dialogo.getByText("Hecho: el router tiene ya el cambio.")).toBeVisible({
    timeout: PLAZO_CAMBIO,
  });
  await dialogo.getByRole("button", { name: "Cerrar", exact: true }).click();
  await expect(dialogo).toBeHidden();
  // Ya no sale (la página vuelve a leer) y el foco va al título.
  await expect(tarjeta).toHaveCount(0);
  await expect(page.locator("#foto-titulo")).toBeFocused();
  expect(await upnpCambiado(page)).toBe(false);

  // Como estaba antes del test.
  if ((await upnpActivo(page)) !== original) await esperarFinal(page, await cambiarUpnp(page));
  expect(errores).toEqual([]);
});

test("a 360 px, la clave que cambió con el ojo", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 800 });
  const errores = recogerErrores(page);
  await entrar(page);
  await page.goto(RUTA);
  await asentar(page);
  await sinDesbordamiento(page, "foto 360");
  const red5 = page.locator("article.foto-ajuste").filter({
    has: page.getByRole("heading", { level: 3, name: "Red WiFi 5 GHz", exact: true }),
  });
  const fila = red5.locator(".foto-fila").filter({ hasText: "Contraseña" });
  const ahora = fila.locator(".foto-valores > div").nth(1);
  const campo = ahora.locator("input.secreto-valor");
  await expect(campo).toHaveAttribute("type", "password");
  await expect(campo).toHaveAttribute("readonly", "");
  await ahora.getByRole("button", { name: "Mostrar Contraseña" }).click();
  await expect(campo).toHaveAttribute("type", "text");
  await expect(campo).toHaveValue("clave-de-prueba-1");
  await sinDesbordamiento(page, "foto 360 con el ojo abierto");
  expect(errores).toEqual([]);
});

test("Teléfono: volver a un valor pide la clave que el router no deja leer", async ({ page }) => {
  // Dos cambios: provocar la diferencia y volver.
  test.setTimeout(plazoDeTest(2));
  await page.setViewportSize({ width: 1024, height: 900 });
  const errores = recogerErrores(page);
  await entrar(page);
  // Cambia «Ahorro en los silencios» desde Ajustes (con la clave de la línea).
  const tema = await api(page, "leer_tema", { tema: "telefono" });
  const linea = ajusteDe(tema, "telefono.linea");
  const silencio = linea.campos.find((c) => c.id === "silencio")?.valor.Booleano;
  const grupo = await api(page, "preparar_ajuste", {
    p: {
      ajuste: "telefono.linea",
      instancia: null,
      borrar: false,
      valores: { silencio: String(!silencio), contrasena: "clave-de-prueba-3" },
    },
  });
  await esperarFinal(page, await api(page, "confirmar_ajuste", { token: grupo.token, desmarcados: [] }));

  await page.goto(RUTA);
  await asentar(page);
  const tarjeta = page.locator("article.foto-ajuste").filter({
    has: page.getByRole("heading", { level: 3, name: "Línea de teléfono", exact: true }),
  });
  const fila = tarjeta.locator(".foto-fila").filter({ hasText: "Ahorro en los silencios" });
  await expect(fila.getByText("El router no deja leer esta clave: escríbela para guardar los cambios.")).toBeVisible();
  // Sin la clave, no se prepara nada.
  await fila.getByRole("button", { name: "Volver a este valor" }).click();
  await expect(fila.getByText("Este campo no puede quedar vacío.")).toBeVisible();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
  // Con ella, el diálogo de siempre, y aplicado.
  const clave = fila.getByLabel("Contraseña de la línea", { exact: true });
  await expect(clave).toBeFocused();
  await clave.fill("clave-de-prueba-3");
  await fila.getByRole("button", { name: "Volver a este valor" }).click();
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo.locator("#grupo-titulo")).toHaveText("Revisa el cambio");
  sinApi(await dialogo.innerText());
  expect(await dialogo.innerText()).not.toContain("clave-de-prueba-3");
  await dialogo.getByRole("button", { name: "Aplicar", exact: true }).click();
  await expect(dialogo.getByText("Hecho: el router tiene ya el cambio.")).toBeVisible({
    timeout: PLAZO_CAMBIO,
  });
  await dialogo.getByRole("button", { name: "Cerrar", exact: true }).click();
  await expect(tarjeta).toHaveCount(0);
  await sinDesbordamiento(page, "foto teléfono 1024");
  expect(errores).toEqual([]);
});
