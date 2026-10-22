import { expect, type Locator, type Page, test } from "@playwright/test";
import { CONTRASENA, mejoras, USUARIO } from "./instancias";
import {
  asentar,
  hidratada,
  quieta,
  recogerErrores,
  sinDesbordamiento,
  violacionesGraves,
} from "./util";

// La página Mejoras, en español a 360 y 1440 px y en los dos temas, contra
// router-sim con las fixtures sin tocar (en el panel de Mejoras, no en el del
// worker): las dos tarjetas con su insignia («5 de 11» y «3 de 5»), el paso
// de invitados que no aplica, «Más consejos» plegado, el contador de la
// barra lateral (6), axe sin violaciones serias y sin desbordamiento.

const CAPTURAS = "capturas/mejoras-4";

/** Entra en el panel de Mejoras (`baseURL`), que no es el del worker. */
async function entrarEnSuPanel(page: Page) {
  await page.goto("/login");
  await hidratada(page);
  await page.waitForLoadState("networkidle");
  await page.locator("#usuario").fill(USUARIO);
  await page.locator("#contrasena").fill(CONTRASENA);
  await page.locator("form button[type=submit]").click();
  await page.waitForURL((url) => url.pathname === "/");
}

/** La tarjeta del lote cuyo título es `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.tarjeta-lote").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

for (const ancho of [360, 1440]) {
  for (const tema of ["oscuro", "claro"] as const) {
    const sufijo = `${ancho}-${tema}`;
    test.describe(`Mejoras a ${ancho} px en tema ${tema}`, () => {
      test.use({
        locale: "es-ES",
        baseURL: mejoras.url,
        viewport: { width: ancho, height: 900 },
        colorScheme: tema === "oscuro" ? "dark" : "light",
      });

      test("tarjetas y contador", async ({ page }) => {
        test.setTimeout(90_000);
        const errores = recogerErrores(page);
        await entrarEnSuPanel(page);
        await page.goto("/mejoras");
        await asentar(page);
        await expect(page.locator("main h1")).toHaveText("Mejoras recomendadas para tu router");

        // Las dos tarjetas con sus insignias.
        await expect(page.locator("main article.tarjeta-lote")).toHaveCount(2, {
          timeout: 20_000,
        });
        const seguridad = tarjeta(page, "Seguridad recomendada");
        const privacidad = tarjeta(page, "Privacidad máxima");
        await expect(seguridad.locator(".ins-aviso")).toHaveText("5 de 11 pendientes");
        await expect(privacidad.locator(".ins-aviso")).toHaveText("3 de 5 pendientes");
        for (const t of [seguridad, privacidad]) {
          await expect(t.getByRole("button", { name: "Revisar y aplicar" })).toBeEnabled();
        }

        // El paso de invitados no aplica: la red está apagada.
        const invitados = seguridad
          .locator("li.paso")
          .filter({ hasText: "Aislar la red de invitados" });
        await expect(invitados).toContainText("No aplica");

        // «Más consejos», plegado, se abre.
        const consejos = seguridad.locator("details.lote-consejos");
        await expect(consejos).not.toHaveAttribute("open", "");
        await consejos.locator("summary").click();
        await expect(consejos).toHaveAttribute("open", "");
        await expect(consejos.locator("li").first()).toBeVisible();

        // El contador de la barra lateral (en el móvil, en el menú plegado).
        if (ancho <= 860) {
          await page.locator("button.menu-boton").click();
        }
        const cuenta = page.locator("nav .cuenta");
        await expect(cuenta).toHaveText("6");
        await expect(cuenta).toHaveAttribute("aria-label", "6 pasos pendientes");
        await expect(cuenta).toBeVisible();
        if (ancho <= 860) {
          await page.locator("button.menu-boton").click();
        }

        await sinDesbordamiento(page, `mejoras ${sufijo}`);
        const cabe = await page.evaluate(
          () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
        );
        expect(cabe, "scrollWidth <= clientWidth").toBe(true);
        expect.soft(await violacionesGraves(page), `mejoras ${sufijo}`).toEqual([]);
        await quieta(page);
        await page.screenshot({
          path: `${CAPTURAS}/mejoras-${sufijo}.png`,
          fullPage: true,
          animations: "disabled",
        });
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}
