import { expect, test, type Page } from "@playwright/test";

// Sin desbordamiento horizontal a 360 px (y capturas a 360 y 1440) en las
// páginas principales, contra router-sim.

const paginas = [
  { nombre: "estado", ruta: "/", listo: "main h1, main h2" },
  {
    nombre: "explorador-reservas",
    ruta: "/explorador/lan_dhcp/cbDhcpV4SvrPoolGetStatic",
    listo: "main h1",
  },
  { nombre: "historial", ruta: "/historial", listo: "main h1" },
];

const anchos = [
  { nombre: "movil", width: 360, height: 800 },
  { nombre: "escritorio", width: 1440, height: 900 },
];

async function entrar(page: Page) {
  await page.goto("/login");
  await page.waitForLoadState("networkidle");
  await page.getByLabel("Usuario").fill("e2e");
  await page.getByLabel("Contraseña").fill("e2e-contrasena");
  await page.getByRole("button", { name: "Entrar" }).click();
  await page.waitForURL((url) => url.pathname === "/");
}

/** Elementos que sobresalen por la derecha (para el mensaje de error). */
async function desbordados(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const ancho = document.documentElement.clientWidth;
    const fuera: string[] = [];
    for (const el of document.body.querySelectorAll("*")) {
      const r = el.getBoundingClientRect();
      if (r.width > 0 && r.right > ancho + 0.5) {
        const id = el.id ? `#${el.id}` : "";
        const clase = el.className && typeof el.className === "string"
          ? `.${el.className.trim().split(/\s+/).join(".")}`
          : "";
        fuera.push(`${el.tagName.toLowerCase()}${id}${clase} → ${Math.round(r.right)}px`);
      }
    }
    return fuera.slice(0, 15);
  });
}

for (const ancho of anchos) {
  test.describe(`${ancho.nombre} (${ancho.width} px)`, () => {
    test.use({ viewport: { width: ancho.width, height: ancho.height } });

    for (const p of paginas) {
      test(`${p.nombre} sin desbordamiento horizontal`, async ({ page }) => {
        const errores: string[] = [];
        page.on("console", (m) => {
          if (m.type() === "error" || m.type() === "warning") errores.push(m.text());
        });
        page.on("pageerror", (e) => errores.push(String(e)));

        await entrar(page);
        await page.goto(p.ruta);
        await page.locator(p.listo).first().waitFor();
        // Estado: hasta que llegue el primer sondeo del router (sim).
        await expect(page.getByText("Esperando el primer dato")).toHaveCount(0, {
          timeout: 20_000,
        });
        // Hidratación y primera carga de datos (las páginas sondean, así que
        // nunca queda la red del todo quieta).
        await page.waitForLoadState("networkidle", { timeout: 5_000 }).catch(() => {});
        await page.waitForTimeout(1_000);

        await page.screenshot({
          path: `capturas/${p.nombre}-${ancho.width}.png`,
          fullPage: true,
        });

        const { scroll, inner } = await page.evaluate(() => ({
          scroll: document.documentElement.scrollWidth,
          inner: window.innerWidth,
        }));
        expect(
          scroll,
          `scrollWidth ${scroll} > innerWidth ${inner}; sobresalen: ${(await desbordados(page)).join(", ")}`,
        ).toBeLessThanOrEqual(inner);
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    }
  });
}
