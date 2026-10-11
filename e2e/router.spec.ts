import { expect, test } from "@playwright/test";
import { asentar, entrar, recogerErrores } from "./util";

// Sección «Router»: el enlace de la navegación lleva a «Conexión con el
// router» y «Atrás» vuelve a la página de antes (sin quedarse atrapado en
// una redirección). Desde «Ajustes del panel» hay un enlace a la conexión.

test.describe("sección Router", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("Router desde Estado y Atrás vuelve a Estado", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await asentar(page);
    await page.getByRole("navigation", { name: "Principal" }).getByRole("link", { name: "Router" }).click();
    await page.waitForURL((url) => url.pathname === "/router/conexion");
    await expect(page.locator("main h1")).toHaveText("Conexión con el router");
    await page.goBack();
    await page.waitForURL((url) => url.pathname === "/");
    await expect(page.locator("main h1").first()).toHaveText("Estado");
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });

  test("/router lleva a la conexión y Atrás no se queda en ella", async ({ page }) => {
    await entrar(page);
    await asentar(page);
    // Navegación del lado del cliente a /router (como un enlace antiguo).
    await page.evaluate(() => {
      const a = document.createElement("a");
      a.href = "/router";
      document.querySelector("main")?.appendChild(a);
      a.click();
    });
    await page.waitForURL((url) => url.pathname === "/router/conexion");
    await page.goBack();
    await page.waitForURL((url) => url.pathname === "/");
  });

  test("Ajustes del panel enlaza con la conexión", async ({ page }) => {
    await entrar(page);
    await page.goto("/panel");
    await asentar(page);
    await page.locator("main .pagina-sub a[href='/router/conexion']").click();
    await page.waitForURL((url) => url.pathname === "/router/conexion");
    await expect(page.locator("main h1")).toHaveText("Conexión con el router");
  });
});
