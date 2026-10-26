import { expect, test } from "@playwright/test";
import { PLAZO_CAMBIO, cambiarUpnp, esperarFinal, estadoCambio, plazoDeTest } from "./cambios";
import { CONTRASENA, USUARIO, instalado } from "./instancias";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";
import { capturaOpcional } from "./util";

// La Biblioteca (una función de lectura con un parámetro) abierta justo
// después de aplicar un cambio, con el router lento: el servidor pinta la
// página mientras relee lo que el cambio dejó viejo. Tiene que hidratarse sin
// errores y seguir respondiendo («Leer de nuevo» lee). Antes, el resultado de
// la lectura llegaba en el HTML del servidor y, según lo que tardase el
// router, el navegador no lo encontraba al hidratar y el wasm moría
// (pánico de hidratación en explorador.rs).

async function retardo(ms: number) {
  const r = await fetch(`${instalado.urlSim}/__sim/retardo`, { method: "POST", body: String(ms) });
  expect(r.status).toBe(204);
}

test.describe("Biblioteca tras un cambio", () => {
  test.use({ viewport: { width: 1440, height: 900 } });
  test.afterEach(async () => {
    await retardo(0);
  });

  test("se hidrata y lee justo después de aplicar un cambio", async ({ page }) => {
    // El cambio y el que lo deshace.
    test.setTimeout(plazoDeTest(2));
    const errores = recogerErrores(page);
    await entrar(page);
    const id = await cambiarUpnp(page);
    let fallo: unknown = null;
    try {
      await esperarFinal(page, id);
      await retardo(150);
      for (let i = 0; i < 3; i++) {
        await page.goto("/explorador/wifi/cbGetWifiAccPt");
        // Hidratada y con sus primeras cargas: un error de hidratación de lo
        // que va dentro de un Suspense sale después de la marca.
        await asentar(page);
        expect(errores, `errores al abrir la Biblioteca (${i + 1})`).toEqual([]);
      }
      await expect(page.locator(".aviso-sin-hidratar")).toBeHidden();
      // Sigue respondiendo: «Leer de nuevo» con el parámetro lee la red 1.
      await page.locator("#lectura-1").fill("1");
      await page.getByRole("button", { name: "Leer de nuevo" }).click();
      await expect(page.locator(".resultado-lectura input.secreto-valor").first()).not.toHaveValue("", {
        timeout: 15_000,
      });
    } catch (e) {
      fallo = e;
      throw e;
    } finally {
      // Lo deja como estaba, sin tapar el error de arriba si lo hubo: espera
      // a que el primer cambio termine (con otro en curso, el segundo
      // esperaría en la cola) y solo lo deshace si quedó aplicado.
      try {
        await retardo(0);
        await expect
          .poll(() => estadoCambio(page, id), { timeout: PLAZO_CAMBIO, intervals: [1_000] })
          .not.toMatch(/^(Preparado|Aplicando|Vigilando)$/);
        if ((await estadoCambio(page, id)) === "Aplicado") {
          await esperarFinal(page, await cambiarUpnp(page));
        }
      } catch (e) {
        if (fallo === null) throw e;
      }
    }
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });

  test("sin JavaScript, dice que la página lo necesita", async ({ browser }) => {
    const ctx = await browser.newContext({
      javaScriptEnabled: false,
      baseURL: instalado.url,
      locale: "es-ES",
    });
    try {
      const page = await ctx.newPage();
      // Se entra sin JavaScript (el servidor redirige a next).
      await page.goto("/login?next=%2Fexplorador%2Fwifi%2FcbGetWifiAccPt");
      await page.locator("#usuario").fill(USUARIO);
      await page.locator("#contrasena").fill(CONTRASENA);
      await page.locator("form button[type=submit]").click();
      await page.waitForURL((u) => u.pathname === "/explorador/wifi/cbGetWifiAccPt");
      const aviso = page.locator(".aviso-sin-hidratar");
      await expect(aviso).toBeVisible({ timeout: 10_000 });
      await expect(aviso).toContainText("La Biblioteca lee del router desde el navegador y necesita JavaScript.");
      // El <noscript> va en el aviso. Chromium con javaScriptEnabled: false
      // sigue tratándolo como texto oculto (solo desactiva la ejecución), así
      // que aquí se comprueba que está, no que se vea.
      expect(await aviso.locator("noscript").textContent()).toContain(
        "Tu navegador tiene JavaScript desactivado.",
      );
    } finally {
      await ctx.close();
    }
  });
});

// Solo se censuran los secretos de verdad: en la lectura de TR-069, la
// dirección del servidor y el usuario se ven en claro; las contraseñas van
// con el ojo, y en el XML, censuradas hasta pulsar «Mostrar claves en el XML».
for (const ancho of [360, 1440]) {
  test.describe(`Biblioteca a ${ancho} px: datos sensibles en claro`, () => {
    test.use({ viewport: { width: ancho, height: 900 }, locale: "es-ES" });

    test("el usuario en claro, la contraseña con el ojo y el XML con su botón", async ({ page }) => {
      const errores = recogerErrores(page);
      await entrar(page);
      const area = "tr069";
      await page.goto(`/explorador/${area}/cbGetTr69Config`);
      await asentar(page);
      const resultado = page.locator(".resultado-lectura");
      const tabla = resultado.locator("table").first();
      await expect(tabla).toBeVisible({ timeout: 15_000 });
      const columna = async (nombre: string) =>
        (await tabla.locator("thead th").allInnerTexts()).findIndex((t) => t.trim() === nombre);
      const celda = async (nombre: string) =>
        tabla.locator("tbody tr").first().locator("td").nth(await columna(nombre));
      // El usuario y la URL, texto a la vista; sin campo censurado.
      for (const nombre of ["acs_username", "acs_URL", "connectionRequestUsername"]) {
        const c = await celda(nombre);
        await expect(c.locator("input")).toHaveCount(0);
        await expect(c).toHaveText("X-REDACTED-X");
      }
      // La contraseña, censurada y con su ojo.
      const clave = (await celda("acs_password")).locator("input.secreto-valor");
      await expect(clave).toHaveAttribute("type", "password");
      await expect(clave).toHaveValue("X-REDACTED-X");

      // El XML: censurado hasta pulsar el botón, que lo vuelve a censurar.
      const xml = resultado.locator("details.crudo");
      await xml.locator("summary").click();
      const pre = xml.locator("pre");
      await expect(pre).toContainText("<acs_username>X-REDACTED-X</acs_username>");
      await expect(pre).toContainText("<acs_password>••••••</acs_password>");
      const mostrar = xml.getByRole("button", { name: "Mostrar claves en el XML" });
      await expect(mostrar).toHaveAttribute("aria-pressed", "false");
      await mostrar.click();
      await expect(pre).toContainText("<acs_password>X-REDACTED-X</acs_password>");
      await expect(pre).not.toContainText("••••••");
      const ocultar = xml.getByRole("button", { name: "Ocultar claves en el XML" });
      await expect(ocultar).toHaveAttribute("aria-pressed", "true");
      await sinDesbordamiento(page, `biblioteca sensibles ${ancho}`);
      await capturaOpcional(page, { path: `capturas/biblioteca-sensibles-${ancho}.png`, fullPage: true });
      await ocultar.click();
      await expect(pre).toContainText("<acs_password>••••••</acs_password>");
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  });
}
