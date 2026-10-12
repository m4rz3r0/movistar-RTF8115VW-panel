import { expect, test, type Page } from "@playwright/test";
import { asentar, entrar, recogerErrores } from "./util";

// Zona horaria de la instancia: se elige en «Ajustes del panel», el servidor
// la pone en <html data-zona> y las fechas salen en ella. Con la zona en
// Canarias ninguna página da error de hidratación (el servidor y el
// navegador calculan las mismas horas). Al terminar se vuelve a la
// peninsular, la de por defecto.

const paginas = [
  { ruta: "/", titulo: "Estado" },
  { ruta: "/historial", titulo: "Historial" },
  { ruta: "/explorador", titulo: "Biblioteca" },
  { ruta: "/panel", titulo: "Ajustes del panel" },
  { ruta: "/router/conexion", titulo: "Conexión con el router" },
];

/** Elige `valor` en el selector de zona y guarda (la página se recarga). */
async function guardarZona(page: Page, valor: string, desfase?: string) {
  await page.goto("/panel");
  await asentar(page);
  await page.locator("#panel-zona").selectOption(valor);
  if (desfase) await page.locator("#panel-zona-otra").selectOption(desfase);
  await Promise.all([
    page.waitForEvent("load"),
    page.locator("section[aria-labelledby=panel-panel] button[type=submit]").click(),
  ]);
  await asentar(page);
}

/** Minutos desde medianoche de ahora en `zona` (IANA). */
async function minutosAhora(page: Page, zona: string): Promise<number> {
  return page.evaluate((zona) => {
    const partes = new Intl.DateTimeFormat("en-GB", {
      timeZone: zona,
      hour: "2-digit",
      minute: "2-digit",
      hourCycle: "h23",
    }).formatToParts(new Date());
    const n = (t: string) => Number(partes.find((p) => p.type === t)?.value);
    return n("hour") * 60 + n("minute");
  }, zona);
}

test.describe("zona horaria", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("en Canarias, horas de Canarias y sin errores de hidratación", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await asentar(page);
    await expect(page.locator("html")).toHaveAttribute("data-zona", "Europe/Madrid");
    try {
      await guardarZona(page, "Atlantic/Canary");
      await expect(page.locator("html")).toHaveAttribute("data-zona", "Atlantic/Canary");
      await expect(page.locator("#panel-zona")).toHaveValue("Atlantic/Canary");
      for (const p of paginas) {
        await page.goto(p.ruta);
        await expect(page.locator("main h1").first()).toHaveText(p.titulo);
        await expect(page.locator("html")).toHaveAttribute("data-zona", "Atlantic/Canary");
        await asentar(page);
      }
      // La hora del dato de Estado (de hace unos segundos), en Canarias.
      await page.goto("/");
      await asentar(page);
      const sub = await page.locator("main .pagina-sub").first().textContent();
      const m = /(\d\d):(\d\d):\d\d/.exec(sub ?? "");
      expect(m, `hora en «${sub}»`).not.toBeNull();
      const dato = Number(m![1]) * 60 + Number(m![2]);
      const ahora = await minutosAhora(page, "Atlantic/Canary");
      const diferencia = Math.min((ahora - dato + 1440) % 1440, (dato - ahora + 1440) % 1440);
      expect(diferencia, `dato ${sub}, ahora ${ahora} min`).toBeLessThanOrEqual(2);

      // «Otra»: un desfase fijo, con su propio selector.
      await guardarZona(page, "otra", "UTC+05:45");
      await expect(page.locator("html")).toHaveAttribute("data-zona", "UTC+05:45");
      await expect(page.locator("#panel-zona")).toHaveValue("otra");
      await expect(page.locator("#panel-zona-otra")).toHaveValue("UTC+05:45");
      await page.goto("/");
      await asentar(page);
    } finally {
      await guardarZona(page, "Europe/Madrid");
    }
    await expect(page.locator("html")).toHaveAttribute("data-zona", "Europe/Madrid");
    await expect(page.locator("#panel-zona-otra")).toHaveCount(0);
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });
});
