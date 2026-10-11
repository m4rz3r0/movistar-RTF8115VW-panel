import { expect, test, type Page } from "@playwright/test";
import { instalado } from "./instancias";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Idioma de la interfaz: el selector de la cabecera cambia todo a inglés y
// la elección sobrevive a recargar; con la cookie en inglés, la hidratación
// no da errores (el servidor y el navegador pintan lo mismo).

/** Cabecera y título de las tres páginas principales, en inglés. */
async function enIngles(page: Page) {
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  const nav = page.getByRole("navigation", { name: "Main", exact: true });
  await expect(nav.getByRole("link", { name: "Status" })).toBeVisible();
  await expect(nav.getByRole("link", { name: "Explorer" })).toBeVisible();
  await expect(nav.getByRole("link", { name: "History" })).toBeVisible();
}

const paginas = [
  { ruta: "/", titulo: "Status" },
  { ruta: "/historial", titulo: "History" },
  { ruta: "/explorador", titulo: "Explorer" },
];

test.describe("idioma", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("el selector pasa a inglés y la elección se queda", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await asentar(page);
    await expect(page.locator("html")).toHaveAttribute("lang", "es");
    // Cambiar recarga la página con la cookie puesta.
    await Promise.all([
      page.waitForEvent("load"),
      page.locator("header .idioma-selector").selectOption("en"),
    ]);
    await enIngles(page);
    for (const p of paginas) {
      await page.goto(p.ruta);
      await expect(page.locator("main h1").first()).toHaveText(p.titulo);
      await enIngles(page);
    }
    await page.reload();
    await enIngles(page);
    await expect(page.locator("main h1").first()).toHaveText("Explorer");
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });

  for (const ancho of [360, 1440]) {
    test(`hidratación en inglés sin errores (${ancho} px)`, async ({ page, context }) => {
      await page.setViewportSize({ width: ancho, height: 900 });
      await context.addCookies([{ name: "rp_lang", value: "en", url: instalado.url }]);
      const errores = recogerErrores(page);
      await page.goto("/login");
      await expect(page.locator("html")).toHaveAttribute("lang", "en");
      await asentar(page);
      await page.locator("#usuario").fill("e2e");
      await page.locator("#contrasena").fill("e2e-contrasena");
      await page.getByRole("button", { name: "Sign in" }).click();
      await page.waitForURL((url) => url.pathname === "/");
      await expect(page.getByText("Waiting for the first data")).toHaveCount(0, {
        timeout: 20_000,
      });
      for (const p of paginas) {
        await page.goto(p.ruta);
        await expect(page.locator("main h1").first()).toHaveText(p.titulo);
        await asentar(page);
        await sinDesbordamiento(page, `${p.ruta} en inglés`);
      }
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  }
});
