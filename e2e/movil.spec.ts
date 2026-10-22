import { expect, test } from "@playwright/test";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Sin desbordamiento horizontal a 360 px (y capturas a 360 y 1440) en las
// páginas principales del panel instalado, contra router-sim.

const paginas = [
  { nombre: "estado", ruta: "/", listo: "main h1, main h2" },
  {
    nombre: "explorador-reservas",
    ruta: "/explorador/lan_dhcp/cbDhcpV4SvrPoolGetStatic",
    listo: "main h1",
  },
  { nombre: "historial", ruta: "/historial", listo: "main h1" },
  { nombre: "ajustes-wifi", ruta: "/ajustes/wifi", listo: "article.tarjeta-el" },
  { nombre: "ajustes-sistema", ruta: "/ajustes/sistema", listo: "article.ajuste" },
  { nombre: "ajustes-seguridad", ruta: "/ajustes/seguridad", listo: "article.ajuste" },
  { nombre: "ajustes-del-panel", ruta: "/panel", listo: "#panel-idioma" },
  { nombre: "router-conexion", ruta: "/router/conexion", listo: "#panel-router-url" },
];

const anchos = [
  { nombre: "movil", width: 360, height: 800 },
  { nombre: "escritorio", width: 1440, height: 900 },
];

for (const ancho of anchos) {
  test.describe(`${ancho.nombre} (${ancho.width} px)`, () => {
    test.use({ viewport: { width: ancho.width, height: ancho.height } });

    for (const p of paginas) {
      test(`${p.nombre} sin desbordamiento horizontal`, async ({ page }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto(p.ruta);
        await page.locator(p.listo).first().waitFor();
        // Estado: hasta que llegue el primer sondeo del router (sim).
        await expect(page.getByText("Esperando el primer dato")).toHaveCount(0, {
          timeout: 20_000,
        });
        await asentar(page);

        await page.screenshot({
          path: `capturas/${p.nombre}-${ancho.width}.png`,
          fullPage: true,
        });
        await sinDesbordamiento(page, p.nombre);
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    }

    test("historial: pasos de un grupo desplegados", async ({ page }) => {
      const errores = recogerErrores(page);
      await entrar(page);
      await page.goto("/historial");
      // El grupo de dos pasos que aplicó e2e_grupo al arrancar.
      const grupo = page.locator("li.hist-cambio", {
        has: page.locator("ol.hist-pasos"),
      });
      await expect(grupo).toHaveCount(1);
      await asentar(page);
      await grupo.locator("summary.hist-cabeza").click();
      const pasos = grupo.locator("li.hist-paso");
      await expect(pasos).toHaveCount(2);
      for (const resumen of await pasos.locator("summary.hist-paso-cabeza").all()) {
        await resumen.click();
      }
      await expect(pasos.locator(".hist-paso-cuerpo").first()).toBeVisible();
      await expect(pasos.locator(".hist-paso-cuerpo").last()).toBeVisible();

      await page.screenshot({
        path: `capturas/historial-grupo-${ancho.width}.png`,
        fullPage: true,
      });
      await sinDesbordamiento(page, "historial con el grupo desplegado");
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  });
}
