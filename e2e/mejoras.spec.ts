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

// Lecturas que fallan (la respuesta de leer_mejoras se cambia en el
// navegador, como en ajustes-wifi.spec.ts): Privacidad «sin leer» del todo
// y Seguridad con pendientes y dos pasos sin leer. Las dos ofrecen «Volver a
// leer», que pide una lectura fresca.
const MOTIVO = "No se pudo leer la pestaña Sistema.";
const SIN_LEER = ["gestion_remota", "aviso_operador"];

interface Paso {
  clave: string;
  estado: unknown;
}
interface Lote {
  id: string;
  estado: unknown;
  pasos: Paso[];
}

for (const tema of ["oscuro", "claro"] as const) {
  test.describe(`Mejoras con lecturas que fallan en tema ${tema}`, () => {
    test.use({
      locale: "es-ES",
      baseURL: mejoras.url,
      viewport: { width: 1440, height: 900 },
      colorScheme: tema === "oscuro" ? "dark" : "light",
    });

    test("lote sin leer y pasos sin leer con «Volver a leer»", async ({ page }) => {
      test.setTimeout(90_000);
      const errores = recogerErrores(page);
      await entrarEnSuPanel(page);
      await page.route("**/api/leer_mejoras", async (r) => {
        const respuesta = await r.fetch();
        const m = await respuesta.json();
        for (const l of m.lotes as Lote[]) {
          for (const p of l.pasos) {
            if (SIN_LEER.includes(p.clave)) {
              p.estado = { SinLeer: { motivo: MOTIVO } };
            } else if (l.id === "privacidad") {
              p.estado = "Hecho";
            }
          }
          l.estado =
            l.id === "privacidad"
              ? { SinLeer: { motivo: "2 pasos no se pudieron leer; lo demás ya está." } }
              : { Pendientes: { pendientes: 3, total: 9 } };
        }
        await r.fulfill({ response: respuesta, json: m });
      });
      // Navegación en el navegador: la lectura pasa por la ruta cambiada.
      await asentar(page);
      await page.locator('nav a[href="/mejoras"]').click();
      await page.waitForURL((u) => u.pathname === "/mejoras");
      const seguridad = tarjeta(page, "Seguridad recomendada");
      const privacidad = tarjeta(page, "Privacidad máxima");
      await expect(privacidad.locator(".ins-off")).toHaveText("Sin leer");
      await expect(privacidad).toContainText("2 pasos no se pudieron leer; lo demás ya está.");
      await expect(privacidad.getByRole("button", { name: "Revisar y aplicar" })).toBeDisabled();
      await expect(seguridad.locator(".ins-aviso")).toHaveText("3 de 9 pendientes");
      await expect(seguridad.locator("li.paso-sin-leer")).toHaveCount(2);
      for (const t of [seguridad, privacidad]) {
        await expect(t.getByRole("button", { name: "Volver a leer" })).toBeVisible();
      }
      await sinDesbordamiento(page, `sin leer ${tema}`);
      expect.soft(await violacionesGraves(page), `sin leer ${tema}`).toEqual([]);
      await quieta(page);
      await page.screenshot({
        path: `${CAPTURAS}/mejoras-sin-leer-1440-${tema}.png`,
        fullPage: true,
        animations: "disabled",
      });
      const fresca = page.waitForRequest(
        (r) => r.url().endsWith("/api/leer_mejoras") && /"fresca":\s*true/.test(r.postData() ?? ""),
      );
      await seguridad.getByRole("button", { name: "Volver a leer" }).click();
      await fresca;
      await page.unrouteAll({ behavior: "wait" });
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  });
}

// El contador no relanza su consulta en cada cambio de página: se pide al
// cargar el marco (en el servidor) y no vuelve a pedirse mientras la cuenta
// valga (30 s, como la caché de lecturas). En /mejoras sale del total de la
// propia página.
test.describe("contador sin consultas de más", () => {
  test.use({ locale: "es-ES", baseURL: mejoras.url, viewport: { width: 1440, height: 900 } });

  test("navegar no vuelve a contar", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrarEnSuPanel(page);
    await asentar(page);
    const cuenta = page.locator("nav .cuenta");
    await expect(cuenta).toHaveText("6");
    const consultas: string[] = [];
    page.on("request", (r) => {
      if (r.url().endsWith("/api/contar_mejoras")) consultas.push(r.url());
    });
    for (const ruta of ["/historial", "/explorador", "/mejoras", "/"]) {
      await page.locator(`nav a[href="${ruta}"]`).click();
      await page.waitForURL((u) => u.pathname === ruta);
      await asentar(page);
      await expect(cuenta).toHaveText("6");
    }
    expect(consultas, "consultas del contador al navegar").toEqual([]);
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });
});
