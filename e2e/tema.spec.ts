import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { asentar, entrar, recogerErrores } from "./util";

// Tema de la interfaz (automático, oscuro, claro): se elige en «Ajustes del
// panel → Tema», cambia al momento sin recargar y se guarda por usuario. El
// servidor pinta <html data-tema> con él (sin destello) y, en automático,
// decide el CSS con la preferencia del sistema, sin JavaScript. Las páginas
// pasan axe sin violaciones serias ni críticas en los dos temas. Al
// terminar se vuelve a «automático», el de por defecto.

/** Fondo de la página (color calculado del <body>). */
async function fondo(page: Page) {
  return page.evaluate(() => getComputedStyle(document.body).backgroundColor);
}

const FONDO_OSCURO = "rgb(15, 18, 21)";
const FONDO_CLARO = "rgb(243, 244, 246)";

/** Elige `tema` en «Ajustes del panel» y espera a que se guarde. */
async function elegirTema(page: Page, tema: string) {
  if (new URL(page.url()).pathname !== "/panel") {
    await page.goto("/panel");
    await asentar(page);
  }
  const seccion = page.locator("section[aria-labelledby=panel-tema]");
  const guardado = page.waitForResponse((r) => r.url().endsWith("/api/guardar_tema_ui"));
  await seccion.locator(`input[name=tema][value=${tema}]`).check();
  expect((await guardado).status()).toBe(200);
  await expect(page.locator("html")).toHaveAttribute("data-tema", tema);
}

/** Violaciones serias o críticas de axe en la página actual. */
async function violacionesGraves(page: Page) {
  const { violations, passes } = await new AxeBuilder({ page }).analyze();
  // Que axe ha mirado la página de verdad (entre otras, el contraste).
  expect(passes.map((p) => p.id)).toContain("color-contrast");
  return violations
    .filter((v) => v.impact === "serious" || v.impact === "critical")
    .map((v) => `${v.id} (${v.impact}): ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

test.describe("tema de la interfaz", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("cambia al momento sin recargar, se guarda y lo pinta el servidor", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/panel");
    await asentar(page);
    await expect(page.locator("html")).toHaveAttribute("data-tema", "automatico");
    const seccion = page.locator("section[aria-labelledby=panel-tema]");
    await expect(seccion.locator("input[name=tema]")).toHaveCount(3);
    await expect(seccion.locator("input[value=automatico]")).toBeChecked();
    // El navegador de las pruebas prefiere oscuro: automático pinta oscuro.
    expect(await fondo(page)).toBe(FONDO_OSCURO);
    try {
      // Una marca en la ventana: si la página se recargase, desaparecería.
      await page.evaluate(() => ((window as unknown as { sinRecargar: boolean }).sinRecargar = true));
      await elegirTema(page, "claro");
      expect(
        await page.evaluate(() => document.documentElement.dataset.tema),
      ).toBe("claro");
      expect(await fondo(page)).toBe(FONDO_CLARO);
      await elegirTema(page, "oscuro");
      expect(await fondo(page)).toBe(FONDO_OSCURO);
      expect(
        await page.evaluate(() => (window as unknown as { sinRecargar?: boolean }).sinRecargar),
        "la página no se recarga al cambiar el tema",
      ).toBe(true);

      // El servidor ya lo manda puesto: sin destello al cargar.
      await elegirTema(page, "claro");
      const html = await (await page.request.get("/")).text();
      expect(html).toContain('data-tema="claro"');
      await page.reload();
      await asentar(page);
      await expect(seccion.locator("input[value=claro]")).toBeChecked();
      expect(await fondo(page)).toBe(FONDO_CLARO);
    } finally {
      await elegirTema(page, "automatico");
    }
    // Automático sigue al sistema, solo con CSS.
    await page.emulateMedia({ colorScheme: "light" });
    expect(await fondo(page)).toBe(FONDO_CLARO);
    await page.emulateMedia({ colorScheme: "dark" });
    expect(await fondo(page)).toBe(FONDO_OSCURO);
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });

  test("axe sin violaciones serias ni críticas en los dos temas", async ({ page }) => {
    await entrar(page);
    try {
      for (const tema of ["oscuro", "claro"]) {
        await elegirTema(page, tema);
        for (const ruta of ["/", "/ajustes/wifi"]) {
          await page.goto(ruta);
          await asentar(page);
          // Con su contenido: los datos de Estado o las tarjetas de WiFi.
          const contenido =
            ruta === "/" ? page.locator("main .rejilla-indicadores") : page.locator("main .emision-red");
          await expect(contenido.first()).toBeVisible({ timeout: 20_000 });
          expect(await violacionesGraves(page), `${ruta} en tema ${tema}`).toEqual([]);
        }
      }
    } finally {
      await elegirTema(page, "automatico");
    }
  });
});
