import { expect, test } from "@playwright/test";
import { CONTRASENA } from "./instancias";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Sección «Configuración del router»: el enlace de la navegación lleva a
// «Conexión con el router» y «Atrás» vuelve a la página de antes (sin
// quedarse atrapado en una redirección). «Ajustes del panel» (se abre con el
// nombre de usuario) enlaza con la conexión.

test.describe("sección Configuración del router", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("Configuración del router desde Estado y Atrás vuelve a Estado", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await asentar(page);
    await page
      .getByRole("navigation", { name: "Principal" })
      .getByRole("link", { name: "Configuración del router" })
      .click();
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

  test("/recomendaciones lleva a Tus preferencias y Atrás no se queda en ella", async ({ page }) => {
    await entrar(page);
    await asentar(page);
    // Navegación del lado del cliente a /recomendaciones (como un enlace antiguo).
    await page.evaluate(() => {
      const a = document.createElement("a");
      a.href = "/recomendaciones";
      document.querySelector("main")?.appendChild(a);
      a.click();
    });
    await page.waitForURL((url) => url.pathname === "/recomendaciones/preferencias");
    await page.goBack();
    await page.waitForURL((url) => url.pathname === "/");
  });

  test("Ajustes del panel enlaza con la conexión", async ({ page }) => {
    await entrar(page);
    await asentar(page);
    await page.getByRole("link", { name: "Ajustes del panel" }).click();
    await page.waitForURL((url) => url.pathname === "/panel");
    await asentar(page);
    await page.locator("main .pagina-sub a[href='/router/conexion']").click();
    await page.waitForURL((url) => url.pathname === "/router/conexion");
    await expect(page.locator("main h1")).toHaveText("Conexión con el router");
  });
});

// Red del router detectada en «Conexión con el router»: el resumen a 360 y
// 1440 px, en español y en inglés, sin desbordamiento ni errores.
const red = {
  es: { titulo: "Red del router", internet: "Internet: ip2 (PPPoE)", telefonia: "Telefonía: sí" },
  en: { titulo: "Router network", internet: "Internet: ip2 (PPPoE)", telefonia: "Phone: yes" },
} as const;

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1440]) {
    test.describe(`red del router ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("muestra la red detectada", async ({ page }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/router/conexion");
        await asentar(page);
        const t = red[idioma];
        const tarjeta = page.locator("section[aria-labelledby=router-red]");
        await expect(tarjeta.locator("h2")).toHaveText(t.titulo);
        await expect(tarjeta.locator(".red-resumen")).toContainText(t.internet);
        await expect(tarjeta.locator(".red-resumen")).toContainText(t.telefonia);
        await expect(tarjeta.locator("#router-red-conexion option")).toHaveCount(4);
        await sinDesbordamiento(page, `red del router ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/router-red-${idioma}-${ancho}.png`, fullPage: true });
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}

test.describe("elegir la conexión a internet", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("pide la contraseña, guarda y vuelve a la automática", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/router/conexion");
    await asentar(page);
    const tarjeta = page.locator("section[aria-labelledby=router-red]");
    await tarjeta.locator("#router-red-conexion").selectOption("ip4");
    await tarjeta.getByRole("button", { name: "Guardar la conexión" }).click();
    const dialogo = page.locator("dialog[open]");
    await expect(dialogo).toBeVisible();
    await dialogo.locator("#reautenticar-contrasena").fill(CONTRASENA);
    await dialogo.locator("button[type=submit]").click();
    await expect(tarjeta.locator(".aviso-formulario")).toHaveText("Conexión a internet guardada.");

    // Tras recargar, sigue elegida; y se deja como estaba (la automática).
    await page.reload();
    await asentar(page);
    await expect(tarjeta.locator("#router-red-conexion")).toHaveValue("ip4");
    await tarjeta.locator("#router-red-conexion").selectOption("");
    await tarjeta.getByRole("button", { name: "Guardar la conexión" }).click();
    await expect(tarjeta.locator(".aviso-formulario")).toHaveText("Conexión a internet guardada.");
    await tarjeta.getByRole("button", { name: "Volver a detectar" }).click();
    await expect(tarjeta.locator(".aviso-formulario")).toHaveText("Red del router detectada de nuevo.");
    await expect(tarjeta.locator(".red-resumen")).toContainText("Internet: ip2 (PPPoE)");
    // El 401 que abre el diálogo de reautenticación es el esperado.
    expect(
      errores.filter((e) => !/status of 401/.test(e)),
      "errores o avisos en la consola",
    ).toEqual([]);
  });
});
