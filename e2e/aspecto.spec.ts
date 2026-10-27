import { expect, type Page, test } from "@playwright/test";
import {
  asentar,
  entrar,
  quieta,
  recogerErrores,
  sinDesbordamiento,
  violacionesGraves,
} from "./util";
import { capturaOpcional } from "./util";

// Aspecto de Estado, Mejoras, Historial, Biblioteca, Configuración del router y
// Ajustes del panel con el sistema visual nuevo, en
// español a 360 y 1440 px y en los dos temas, contra router-sim: axe sin
// violaciones serias (entre ellas, el contraste), sin desbordamiento
// horizontal, cada botón de `main` en la escala única (.b-pri, .b-sec,
// .b-rec, .b-txt, .b-pel) salvo los interruptores y las marcas, sin
// etiquetas en mayúsculas, las tarjetas con el radio de las de Ajustes y,
// en Estado, cada red WiFi con su icono de emisión. Las comprobaciones son
// «soft»: un fallo dice todo lo que falta en cada página. Solo mira el aspecto:
// los e2e de cada página siguen probando lo que hacen.

/** Una página: su dirección y lo que tiene que haber cuando ya tiene datos. */
interface Pagina {
  nombre: string;
  ruta: string;
  contenido: string;
}

const paginas: Pagina[] = [
  { nombre: "estado", ruta: "/", contenido: "main .rejilla-indicadores" },
  { nombre: "mejoras", ruta: "/mejoras", contenido: "main .tarjeta-lote" },
  { nombre: "historial", ruta: "/historial", contenido: "main .hist-lista" },
  { nombre: "biblioteca", ruta: "/explorador", contenido: "main .tarjeta-enlace" },
  { nombre: "biblioteca-area", ruta: "/explorador/wifi", contenido: "main .funcion-item" },
  {
    nombre: "biblioteca-funcion",
    ruta: "/explorador/wifi/cbGetWifiAccPt",
    contenido: "main .cabecera-funcion",
  },
  { nombre: "router", ruta: "/router/conexion", contenido: "main .ajustes-panel" },
  { nombre: "panel", ruta: "/panel", contenido: "main .ajustes-panel" },
];

const ESCALA = ["b-pri", "b-sec", "b-rec", "b-txt", "b-pel"];
const CAPTURAS = "capturas/rediseno-6";

/** Botones visibles de `main` fuera de la escala única (ni interruptor ni marca). */
async function botonesFueraDeEscala(page: Page): Promise<string[]> {
  return page.locator("main button:visible").evaluateAll(
    (bs, escala) =>
      bs
        .filter(
          (b) =>
            b.getAttribute("role") !== "switch" &&
            !b.classList.contains("marcar") &&
            !escala.some((c) => b.classList.contains(c)),
        )
        .map((b) => `${b.className || "(sin clase)"}: ${b.textContent?.trim()}`),
    ESCALA,
  );
}

/** Elementos visibles de `main` con el texto en mayúsculas por CSS. */
async function enMayusculas(page: Page): Promise<string[]> {
  return page.locator("main *:visible").evaluateAll((els) =>
    els
      .filter((e) => getComputedStyle(e).textTransform === "uppercase")
      .map((e) => `${e.tagName.toLowerCase()}.${e.className}`),
  );
}

/** Tarjetas de `main` que no llevan el radio de las tarjetas (14 px). */
async function tarjetasSinRadio(page: Page): Promise<string[]> {
  return page.locator("main .tarjeta:visible").evaluateAll((ts) =>
    ts
      .filter((t) => getComputedStyle(t).borderTopLeftRadius !== "14px")
      .map((t) => `${t.className}: ${getComputedStyle(t).borderTopLeftRadius}`),
  );
}

test.describe.configure({ mode: "parallel" });

for (const ancho of [360, 1440]) {
  for (const tema of ["oscuro", "claro"] as const) {
    const sufijo = `${ancho}-${tema}`;
    test.describe(`aspecto a ${ancho} px en tema ${tema}`, () => {
      test.use({
        locale: "es-ES",
        viewport: { width: ancho, height: 900 },
        // El tema del usuario es «automático»: sigue al del sistema.
        colorScheme: tema === "oscuro" ? "dark" : "light",
      });

      test("cinco páginas con el sistema visual nuevo", async ({ page }) => {
        test.setTimeout(120_000);
        const errores = recogerErrores(page);
        await entrar(page);
        for (const p of paginas) {
          await page.goto(p.ruta);
          await asentar(page);
          await expect(page.locator(p.contenido).first()).toBeVisible({ timeout: 20_000 });
          await sinDesbordamiento(page, `${p.nombre} ${sufijo}`);
          expect.soft(await botonesFueraDeEscala(page), `${p.nombre}: botones`).toEqual([]);
          expect.soft(await enMayusculas(page), `${p.nombre}: mayúsculas`).toEqual([]);
          expect.soft(await tarjetasSinRadio(page), `${p.nombre}: tarjetas`).toEqual([]);
          if (p.nombre === "estado") {
            // Cada red WiFi con su icono de emisión, del color de su estado:
            // verde las que emiten («Activa», insignia verde) y gris las demás.
            const redes = page.locator("section[aria-labelledby=wifi-titulo] tbody tr");
            const n = await redes.count();
            expect.soft(n).toBeGreaterThan(0);
            await expect.soft(redes.locator("svg.emision")).toHaveCount(n);
            const emiten = await redes.locator(".ins-ok").count();
            await expect.soft(redes.locator("svg.emision-ok")).toHaveCount(emiten);
            await expect.soft(redes.locator("svg.emision-off")).toHaveCount(n - emiten);
          }
          await quieta(page);
          await capturaOpcional(page, {
            path: `${CAPTURAS}/${p.nombre}-${sufijo}.png`,
            fullPage: true,
            animations: "disabled",
          });
          expect.soft(await violacionesGraves(page), `${p.nombre} ${sufijo}`).toEqual([]);
        }
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}

// Ajustes con listas a 360 px, en los dos temas: axe sin violaciones serias
// con la página tal cual, con el detalle de la primera fila abierto y sus
// «Detalles técnicos» desplegados, y con el resumen de un cambio. Una tabla que se desplaza en horizontal
// tiene que poder recibir el foco (scrollable-region-focusable).
const listas = [
  { nombre: "red_local", ruta: "/ajustes/red_local" },
  { nombre: "puertos", ruta: "/ajustes/puertos" },
];

for (const tema of ["oscuro", "claro"] as const) {
  test.describe(`Ajustes con listas a 360 px en tema ${tema}`, () => {
    test.use({
      locale: "es-ES",
      viewport: { width: 360, height: 900 },
      colorScheme: tema === "oscuro" ? "dark" : "light",
    });

    test("axe con la lista, el detalle abierto y el resumen de un cambio", async ({ page }) => {
      test.setTimeout(120_000);
      const errores = recogerErrores(page);
      await entrar(page);
      for (const l of listas) {
        await page.goto(l.ruta);
        await asentar(page);
        const fila = page.locator("main tr.lista-fila").first();
        await expect(fila).toBeVisible({ timeout: 20_000 });
        await sinDesbordamiento(page, `${l.nombre} 360-${tema}`);
        expect.soft(await violacionesGraves(page), `${l.nombre} 360-${tema}`).toEqual([]);
        await fila.locator(".lista-nombre").click();
        const detalle = page.locator(".detalle-lateral");
        await expect(detalle).toBeVisible();
        for (const d of await detalle.locator("details.detalles-tecnicos").all()) {
          await d.locator("summary").click();
        }
        expect
          .soft(await violacionesGraves(page), `${l.nombre} 360-${tema} con el detalle`)
          .toEqual([]);
      }
      // El resumen de un cambio, con su tabla de antes y después: el de
      // quitar el WPS de dos redes (se cancela, no se aplica).
      await page.goto("/ajustes/wifi");
      await asentar(page);
      for (const red of ["Red WiFi 2,4 GHz", "Red WiFi 5 GHz"]) {
        await page.getByRole("button", { name: `Marcar ${red}`, exact: true }).click();
      }
      const barra = page.getByRole("toolbar", { name: "Acciones para lo marcado" });
      await barra.getByRole("button", { name: "Quitar WPS" }).click();
      const dialogo = page.locator("dialog[open]");
      await expect(dialogo.locator(".tabla-scroll").first()).toBeVisible();
      await sinDesbordamiento(page, `resumen del cambio 360-${tema}`);
      expect.soft(await violacionesGraves(page), `resumen del cambio 360-${tema}`).toEqual([]);
      await dialogo.getByRole("button", { name: "Cancelar" }).click();
      await expect(page.locator("dialog[open]")).toHaveCount(0);
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  });
}
