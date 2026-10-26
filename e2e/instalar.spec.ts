import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { nuevos } from "./instancias";
import { asentar, recogerErrores, sinDesbordamiento } from "./util";
import { capturaOpcional } from "./util";

// Asistente de primera instalación en un panel recién instalado (base de
// datos vacía, simulador): el token del fichero, el administrador, el router,
// la lectura inicial y Estado con datos. A 360 y 1440 px, en español y en
// inglés (por Accept-Language), sin desbordamiento horizontal ni errores en
// la consola (hidratación). Cada pasada usa su propio panel: el asistente es
// de un solo uso.

const textos = {
  es: {
    titulo: "Configurar el panel",
    crear: "Crear el administrador",
    guardar: "Guardar y seguir",
    leer: "Leer la configuración",
    fin: "El panel ya está configurado.",
    red: "Red del router detectada:",
    telefonia: "Telefonía: sí",
    irEstado: "Ir al estado",
    estado: "Estado",
    esperando: "Esperando el primer dato",
  },
  en: {
    titulo: "Set up the panel",
    crear: "Create the administrator",
    guardar: "Save and continue",
    leer: "Read the configuration",
    fin: "The panel is set up.",
    red: "Router network detected:",
    telefonia: "Phone: yes",
    irEstado: "Go to Status",
    estado: "Status",
    esperando: "Waiting for the first data",
  },
};

const pasadas = [
  { idioma: "es", locale: "es-ES", ancho: 360 },
  { idioma: "es", locale: "es-ES", ancho: 1440 },
  { idioma: "en", locale: "en-GB", ancho: 360 },
  { idioma: "en", locale: "en-GB", ancho: 1440 },
] as const;

for (const { idioma, locale, ancho } of pasadas) {
  const panel = nuevos[`${idioma}-${ancho}`];
  const t = textos[idioma];

  test.describe(`asistente ${idioma} a ${ancho} px`, () => {
    test.use({ locale, viewport: { width: ancho, height: 900 }, baseURL: panel.url });

    test("instala y llega a Estado con datos", async ({ page }) => {
      const errores = recogerErrores(page);
      // Sin usuarios, cualquier página lleva al asistente.
      await page.goto("/");
      await page.waitForURL((url) => url.pathname === "/instalar");
      await expect(page.locator("html")).toHaveAttribute("lang", idioma);
      await expect(page.locator("h1")).toHaveText(t.titulo);
      await asentar(page);
      await sinDesbordamiento(page, "paso del administrador");
      await capturaOpcional(page, { path: `capturas/instalar-${idioma}-${ancho}.png`, fullPage: true });

      // El token, del fichero junto a la base de datos.
      const token = readFileSync(join(panel.dir, "instalar.token"), "utf8").trim();
      expect(token.length).toBeGreaterThan(20);
      await page.locator("#instalar-token").fill(token);
      await page.locator("#instalar-usuario").fill("admin");
      await page.locator("#instalar-contrasena").fill("una contraseña larga");
      await page.locator("#instalar-repetir").fill("una contraseña larga");
      await page.getByRole("button", { name: t.crear }).click();

      await page.locator("#instalar-router-url").fill(panel.urlSim);
      await page.locator("#instalar-router-usuario").fill("sim");
      await page.locator("#instalar-router-contrasena").fill("sim");
      await sinDesbordamiento(page, "paso del router");
      await page.getByRole("button", { name: t.guardar }).click();

      await page.getByRole("button", { name: t.leer }).click();
      await expect(page.getByText(t.fin)).toBeVisible({ timeout: 30_000 });
      // La red detectada del router, resumida.
      await expect(page.getByText(t.red)).toBeVisible();
      await expect(page.locator(".red-resumen")).toContainText("Internet: ip2 (PPPoE)");
      await expect(page.locator(".red-resumen")).toContainText(t.telefonia);
      // El final solo lleva al Estado: nada de «Tus preferencias».
      await expect(page.locator(".instalar-botones a")).toHaveCount(1);
      await expect(page.locator("a[href^='/recomendaciones']")).toHaveCount(0);
      await sinDesbordamiento(page, "fin del asistente");
      await capturaOpcional(page, { path: `capturas/instalar-fin-${idioma}-${ancho}.png`, fullPage: true });

      await page.getByRole("link", { name: t.irEstado }).click();
      await page.waitForURL((url) => url.pathname === "/");
      await expect(page.locator("main h1").first()).toHaveText(t.estado);
      await expect(page.getByText(t.esperando)).toHaveCount(0, { timeout: 20_000 });
      // Con datos: el resumen del router (modelo de las capturas).
      await expect(page.locator("main")).toContainText("RTF8115VW");
      await asentar(page);
      await sinDesbordamiento(page, "Estado tras instalar");
      expect(errores, "errores o avisos en la consola").toEqual([]);

      // El token era de un solo uso: el asistente ya no está.
      expect(() => readFileSync(join(panel.dir, "instalar.token"))).toThrow();
      const resp = await page.request.get("/instalar");
      expect(resp.status()).toBe(404);
    });
  });
}
