import { expect, test } from "@playwright/test";
import { CONTRASENA, USUARIO, instalado } from "./instancias";

// Entrar sin JavaScript: lo que pasa en un móvil lento si se pulsa «Entrar»
// antes de que la página se hidrate (o si el wasm no carga). El formulario
// se envía tal cual y el servidor redirige: con la contraseña buena, a
// `next`; con una mala, de vuelta al formulario con el error a la vista y
// `next` conservado. Con la sesión ya abierta, /login lleva a la portada.

test.describe("login sin JavaScript", () => {
  test("entra y lleva a next; con una contraseña mala, el error y next", async ({ browser }) => {
    const opciones = { javaScriptEnabled: false, baseURL: instalado.url, locale: "es-ES" };
    const entra = await browser.newContext(opciones);
    const falla = await browser.newContext(opciones);
    try {
      // Con la buena, a next.
      const page = await entra.newPage();
      await page.goto("/login?next=%2Fhistorial");
      await page.locator("#usuario").fill(USUARIO);
      await page.locator("#contrasena").fill(CONTRASENA);
      await page.locator("form button[type=submit]").click();
      await page.waitForURL((u) => u.pathname === "/historial");
      await expect(page.locator("main h1").first()).toHaveText("Historial");
      // Ya dentro, /login lleva a la portada.
      await page.goto("/login");
      await expect(page).toHaveURL((u) => u.pathname === "/");

      // Con una mala, de vuelta al formulario con el error y next.
      const otra = await falla.newPage();
      await otra.goto("/login?next=%2Fhistorial");
      await otra.locator("#usuario").fill(USUARIO);
      await otra.locator("#contrasena").fill("no-es-la-buena");
      await otra.locator("form button[type=submit]").click();
      await otra.waitForURL((u) => u.pathname === "/login" && u.searchParams.has("__err"));
      await expect(otra.locator("p.error[role=alert]")).toHaveText("Usuario o contraseña incorrectos");
      await expect(otra.locator("input[name=next]")).toHaveValue("/historial");
      expect(new URL(otra.url()).searchParams.get("next")).toBe("/historial");

      // Deja el freno del login como estaba (el fallo cuenta 1 s para la IP
      // y el usuario): una contraseña buena lo borra, pasada la espera.
      await expect
        .poll(
          async () =>
            (
              await page.request.post("/api/reautenticar", {
                data: { contrasena: CONTRASENA },
                headers: { origin: instalado.url, "sec-fetch-site": "same-origin" },
              })
            ).status(),
          { timeout: 10_000, intervals: [500] },
        )
        .toBe(200);
    } finally {
      await entra.close();
      await falla.close();
    }
  });
});
