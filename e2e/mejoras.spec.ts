import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
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
          .filter({
            hasText: "Aislar entre sí los dispositivos de la red de invitados",
          });
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
        const cuenta = page.locator("nav.nav .cuenta");
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
    const cuenta = page.locator("nav.nav .cuenta");
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

// Revisar un lote en su diálogo, aplicarlo y deshacerlo, en el panel de
// Mejoras (que vuelve a las fixtures al deshacer). La revisión agrupa los
// pasos por pestaña, en el orden de las pestañas; la hora neutra (opcional)
// sale desmarcada; el selector de DNS vuelve a preparar el lote con el
// proveedor elegido. Un solo cambio, con un solo «Deshacer» en el aviso de la
// página; al terminar, la página y el contador se ponen al día.
const CAPTURAS_REVISION = "capturas/mejoras-5";
const HECHO = "Hecho: el router tiene ya el cambio.";

/** El contador de la barra lateral (en el móvil, abriendo el menú). */
async function esperarContador(page: Page, ancho: number, valor: string) {
  if (ancho <= 860) await page.locator("button.menu-boton").click();
  await expect(page.locator("nav.nav .cuenta")).toHaveText(valor);
  if (ancho <= 860) await page.locator("button.menu-boton").click();
}

/** Entradas del Historial del panel. */
async function entradasDelHistorial(page: Page): Promise<number> {
  const r = await page.request.post("/api/historial", { data: { pagina: 1 } });
  expect(r.status()).toBe(200);
  return (await r.json()).total;
}

/** Retardo de cada respuesta del simulador del panel de Mejoras, en ms. */
async function retardoDelSim(ms: number) {
  const r = await fetch(`${mejoras.urlSim}/__sim/retardo`, { method: "POST", body: String(ms) });
  expect(r.ok).toBe(true);
}

/** Los pasos del diálogo, con sus tablas. */
function pasosDe(dialogo: Locator): Locator {
  return dialogo.locator(".grupo-pasos").first();
}

/** Abre la revisión de Privacidad máxima. */
async function revisarPrivacidad(page: Page): Promise<Locator> {
  await page.goto("/mejoras");
  await asentar(page);
  const privacidad = tarjeta(page, "Privacidad máxima");
  await expect(privacidad.locator(".ins-aviso")).toHaveText("3 de 5 pendientes", {
    timeout: 20_000,
  });
  await privacidad.getByRole("button", { name: "Revisar y aplicar" }).click();
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo.locator("#grupo-titulo")).toHaveText("Revisa el cambio");
  return dialogo;
}

for (const [ancho, tema] of [
  [1440, "oscuro"],
  [360, "claro"],
] as const) {
  const sufijo = `${ancho}-${tema}`;
  test.describe(`Revisión de un lote a ${ancho} px en tema ${tema}`, () => {
    test.use({
      locale: "es-ES",
      baseURL: mejoras.url,
      viewport: { width: ancho, height: 900 },
      colorScheme: tema === "oscuro" ? "dark" : "light",
    });

    test("revisar, aplicar y deshacer", async ({ page }) => {
      test.setTimeout(plazoDeTest(2));
      const errores = recogerErrores(page);
      await entrarEnSuPanel(page);
      const dialogo = await revisarPrivacidad(page);

      // Por pestañas, en el orden de las pestañas.
      await expect(dialogo.locator("h3.grupo-pestana-titulo")).toHaveText([
        "Red local",
        "Seguridad",
        "Sistema",
      ]);
      // La hora neutra, opcional, sale desmarcada; lo demás que cambia, marcado.
      await expect(dialogo.getByRole("checkbox", { name: "Hora del router" })).not.toBeChecked();
      const aviso = dialogo.getByRole("checkbox", { name: "Aviso periódico al operador" });
      await expect(aviso).toBeChecked();
      // Quad9, por defecto; con DNS4EU, la tabla dice sus servidores.
      await expect(dialogo.getByRole("radio", { name: "Quad9", exact: true })).toBeChecked();
      await expect(dialogo).toContainText("9.9.9.9");
      await dialogo.getByRole("radio", { name: "DNS4EU", exact: true }).check();
      await expect(pasosDe(dialogo)).toContainText("86.54.11.100");
      await expect(dialogo).not.toContainText("9.9.9.9");
      await expect(dialogo.getByRole("button", { name: "Aplicar", exact: true })).toBeEnabled();

      await sinDesbordamiento(page, `revisión ${sufijo}`);
      expect.soft(await violacionesGraves(page), `revisión ${sufijo}`).toEqual([]);
      await quieta(page);
      await page.screenshot({
        path: `${CAPTURAS_REVISION}/revision-${sufijo}.png`,
        animations: "disabled",
      });

      // Sin el aviso periódico: se aplica lo demás, como un solo cambio.
      await aviso.uncheck();
      await dialogo.getByRole("button", { name: "Aplicar", exact: true }).click();
      await expect(page.locator("dialog[open]")).toHaveCount(0);
      const cambio = page.locator(".avisos-cambio [role=status] .aviso-cambio");
      await expect(cambio).toContainText(HECHO, { timeout: PLAZO_CAMBIO });
      const privacidad = tarjeta(page, "Privacidad máxima");
      const seguridad = tarjeta(page, "Seguridad recomendada");
      await expect(privacidad.locator(".ins-aviso")).toHaveText("1 de 5 pendientes");
      await expect(seguridad.locator(".ins-aviso")).toHaveText("4 de 11 pendientes");
      await esperarContador(page, ancho, "4");

      // Un solo «Deshacer» lo devuelve todo.
      await cambio.getByRole("button", { name: "Deshacer", exact: true }).click();
      await expect(dialogo.locator("#grupo-titulo")).toHaveText("Volver a como estaba");
      await dialogo.getByRole("button", { name: "Deshacer", exact: true }).click();
      await expect(page.locator("dialog[open]")).toHaveCount(0);
      await expect(privacidad.locator(".ins-aviso")).toHaveText("3 de 5 pendientes", {
        timeout: PLAZO_CAMBIO,
      });
      await expect(seguridad.locator(".ins-aviso")).toHaveText("5 de 11 pendientes");
      await expect(cambio).toContainText(HECHO);
      await esperarContador(page, ancho, "6");
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  });
}

test.describe("Revisión de un lote: casos raros", () => {
  test.use({ locale: "es-ES", baseURL: mejoras.url, viewport: { width: 1440, height: 900 } });

  test("Esc mientras prepara", async ({ page }) => {
    // Con el retardo, preparar el lote de nuevo lee el router despacio.
    test.setTimeout(120_000);
    const errores = recogerErrores(page);
    await entrarEnSuPanel(page);
    const dialogo = await revisarPrivacidad(page);
    await retardoDelSim(1_500);
    try {
      const preparar = page.waitForResponse((r) => r.url().endsWith("/api/preparar_lote"), {
        timeout: 90_000,
      });
      await dialogo.getByRole("radio", { name: "Cloudflare", exact: true }).check();
      await expect(dialogo.getByRole("button", { name: "Cancelar" })).toBeDisabled();
      await page.keyboard.press("Escape");
      await expect(page.locator("dialog[open]")).toHaveCount(1);
      await preparar;
    } finally {
      await retardoDelSim(0);
    }
    await expect(pasosDe(dialogo)).toContainText("1.1.1.1");
    await dialogo.getByRole("button", { name: "Cancelar" }).click();
    await expect(page.locator("dialog[open]")).toHaveCount(0);
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });

  test("todo desmarcado", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrarEnSuPanel(page);
    const antes = await entradasDelHistorial(page);
    const dialogo = await revisarPrivacidad(page);
    const casillas = dialogo.locator("input[type=checkbox]:enabled");
    for (const c of await casillas.all()) await c.uncheck();
    await dialogo.getByRole("button", { name: "Aplicar", exact: true }).click();
    await expect(dialogo.locator(".dialogo-cuerpo > .error-formulario")).toHaveText(
      "Marca al menos un paso para aplicar el cambio.",
    );
    await expect(page.locator("dialog[open]")).toHaveCount(1);
    expect(await entradasDelHistorial(page)).toBe(antes);
    await dialogo.getByRole("button", { name: "Cancelar" }).click();
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });
});

// Desde Ajustes: la recomendación de una tarjeta dice de qué lote forma
// parte su ajuste y lleva a Mejoras (solo lee: las fixtures siguen igual).
const CAPTURAS_AJUSTES = "capturas/mejoras-6";

/** La tarjeta de Ajustes cuyo título es `titulo`. */
function tarjetaDeAjustes(page: Page, titulo: string): Locator {
  return page.locator("article.tarjeta-el").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

for (const ancho of [360, 1440]) {
  test.describe(`Enlace desde Ajustes a ${ancho} px`, () => {
    test.use({ locale: "es-ES", baseURL: mejoras.url, viewport: { width: ancho, height: 900 } });

    test("enlace desde Ajustes", async ({ page }) => {
      const errores = recogerErrores(page);
      await entrarEnSuPanel(page);
      await page.goto("/ajustes/wifi");
      await asentar(page);
      const red24 = tarjetaDeAjustes(page, "Red WiFi 2,4 GHz");
      const rec = red24.locator(".tarjeta-rec");
      await expect(rec).toContainText("Forma parte de «Seguridad recomendada»");
      // Una sola recomendación en la tarjeta, con su enlace dentro.
      await expect(rec).toHaveCount(1);
      const enlace = rec.getByRole("link", { name: "Ver en Mejoras" });
      await expect(enlace).toHaveAttribute("href", "/mejoras");
      await sinDesbordamiento(page, `ajustes wifi ${ancho}`);
      expect.soft(await violacionesGraves(page), `ajustes wifi ${ancho}`).toEqual([]);
      await quieta(page);
      await red24.screenshot({
        path: `${CAPTURAS_AJUSTES}/tarjeta-wifi-${ancho}.png`,
        animations: "disabled",
      });
      await enlace.click();
      await page.waitForURL((u) => u.pathname === "/mejoras");
      await expect(page.locator("main h1")).toHaveText("Mejoras recomendadas para tu router");
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  });
}

// Un cambio aplicado o deshecho desde Ajustes pone al día el contador de la
// barra lateral al momento, sin cambiar de página.
test.describe("contador tras un cambio en Ajustes", () => {
  test.use({ locale: "es-ES", baseURL: mejoras.url, viewport: { width: 1440, height: 900 } });

  test("quitar el WPS desde Ajustes", async ({ page }) => {
    test.setTimeout(plazoDeTest(2));
    const errores = recogerErrores(page);
    await entrarEnSuPanel(page);
    await page.goto("/ajustes/wifi");
    await asentar(page);
    const cuenta = page.locator("nav.nav .cuenta");
    await expect(cuenta).toHaveText("6");
    const rec = tarjetaDeAjustes(page, "Red WiFi 2,4 GHz").locator(".tarjeta-rec");
    await rec.getByRole("button", { name: "Quitar WPS" }).click();
    const dialogo = page.locator("dialog[open]");
    await expect(dialogo.locator("#grupo-titulo")).toHaveText("Revisa el cambio");
    await dialogo.getByRole("button", { name: "Aplicar", exact: true }).click();
    const cambio = page.locator(".avisos-cambio [role=status] .aviso-cambio");
    await expect(cambio).toContainText(HECHO, { timeout: PLAZO_CAMBIO });
    await expect(cuenta).toHaveText("5", { timeout: PLAZO_CAMBIO });
    // Deshacerlo también lo pone al día (y deja las fixtures como estaban).
    await cambio.getByRole("button", { name: "Deshacer", exact: true }).click();
    await expect(dialogo.locator("#grupo-titulo")).toHaveText("Volver a como estaba");
    await dialogo.getByRole("button", { name: "Deshacer", exact: true }).click();
    await expect(page.locator("dialog[open]")).toHaveCount(0);
    await expect(cuenta).toHaveText("6", { timeout: PLAZO_CAMBIO });
    await expect(cambio).toContainText(HECHO);
    await expect(page).toHaveURL(/\/ajustes\/wifi$/);
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });
});
