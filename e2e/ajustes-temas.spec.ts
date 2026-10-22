import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
import { CONTRASENA, USUARIO, temas } from "./instancias";
import {
  asentar,
  entrar,
  hidratada,
  quieta,
  recogerErrores,
  sinDesbordamiento,
  violacionesGraves,
} from "./util";

// Internet, Seguridad, Teléfono y Sistema en rejilla (como la WiFi), en
// español a 360 y 1440 px y en los dos temas, contra router-sim: la franja
// de resumen con sus cuatro datos, las tarjetas, abrir una (foco al título
// del detalle y su dirección), editar un campo y guardarlo (el diálogo de
// revisión), y Esc cierra el detalle. En dos combinaciones (1440 px en
// oscuro y 360 px en claro) el guardado se aplica de verdad, en un panel
// propio (instancias.ts) y en serie (el gestor aplica un cambio a la vez);
// en las demás, el diálogo se cancela y «Descartar» lo deja como estaba.
// Además, la recomendación de la tarjeta: «Usar un servidor de hora neutro»
// prepara el cambio a hora.roa.es (se cancela) y la gestión remota
// encendida pide apagarla, como riesgo. Axe sin violaciones serias en la
// rejilla y con el detalle abierto.

const t = {
  guardarCambios: "Guardar cambios",
  descartar: "Descartar",
  aplicar: "Aplicar",
  cancelar: "Cancelar",
  deshacer: "Deshacer",
  revisar: "Revisa el cambio",
  hecho: "Hecho: el router tiene ya el cambio.",
  horaNeutra: "Usar un servidor de hora neutro",
  apagarRemota: "Apagar la gestión remota",
};

/** Una pestaña: su tarjeta que se abre y cómo se edita un campo en su detalle. */
interface Caso {
  tema: string;
  nombre: string;
  tarjetas: number;
  elemento: string;
  id: string;
  /** Cambia un campo del detalle (a otro valor que el que tiene). */
  editar: (detalle: Locator, sufijo: string) => Promise<void>;
}

/** Pulsa el interruptor `nombre` del detalle. */
function pulsar(nombre: string) {
  return async (detalle: Locator) => {
    await detalle.getByRole("switch", { name: nombre, exact: true }).click();
  };
}

const casos: Caso[] = [
  {
    tema: "internet",
    nombre: "Internet",
    tarjetas: 2,
    elemento: "Relé DNS",
    id: "dns",
    editar: pulsar("Relé DNS activado"),
  },
  {
    tema: "seguridad",
    nombre: "Seguridad",
    tarjetas: 3,
    elemento: "Gestión remota del operador",
    id: "gestion-remota",
    editar: async (detalle) => {
      const campo = detalle.getByLabel("Cada cuánto", { exact: true });
      const leido = await campo.inputValue();
      await campo.selectOption(leido === "dia" ? "semana" : "dia");
    },
  },
  {
    tema: "telefono",
    nombre: "Teléfono",
    tarjetas: 4,
    elemento: "Teclas y fax en las llamadas",
    id: "llamadas",
    editar: pulsar("Fax por internet"),
  },
  {
    tema: "sistema",
    nombre: "Sistema",
    tarjetas: 3,
    elemento: "Hora del router",
    id: "hora",
    editar: async (detalle, sufijo) => {
      await detalle
        .getByLabel("Servidor de hora de reserva", { exact: true })
        .fill(`reserva-${sufijo}.example`);
    },
  },
];

/** La tarjeta cuyo título es exactamente `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.tarjeta-el").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

const CAPTURAS = "capturas/rediseno-4";

/** Entra en el panel de los temas (`baseURL`), que no es el del worker. */
async function entrarEnSuPanel(page: Page) {
  await page.goto("/login");
  await hidratada(page);
  await page.waitForLoadState("networkidle");
  await page.locator("#usuario").fill(USUARIO);
  await page.locator("#contrasena").fill(CONTRASENA);
  await page.locator("form button[type=submit]").click();
  await page.waitForURL((url) => url.pathname === "/");
}

/** La rejilla del tema: resumen, tarjetas, sin desbordamiento y axe. */
async function rejilla(page: Page, c: Caso, sufijo: string, ancho: number) {
  await page.goto(`/ajustes/${c.tema}`);
  await asentar(page);
  await expect(page.locator("article.tarjeta-el")).toHaveCount(c.tarjetas);
  await expect(page.locator(".resumen .resumen-dato")).toHaveCount(4);
  await expect(page.locator(".tema-lectura")).toContainText(/Leído (ahora mismo|hace)/);
  await sinDesbordamiento(page, `${c.tema} ${sufijo}`);
  // Nada del resumen se sale de su caja (un servidor de hora largo a 360 px).
  const salen = await page
    .locator(".resumen .resumen-dato")
    .evaluateAll((ds) => ds.filter((d) => d.scrollWidth > d.clientWidth).map((d) => d.textContent));
  expect(salen, `${c.tema} ${sufijo}: el resumen se sale`).toEqual([]);
  await quieta(page);
  await page.screenshot({
    path: `${CAPTURAS}/${c.tema}-${sufijo}.png`,
    fullPage: ancho > 900,
    animations: "disabled",
  });
  expect(await violacionesGraves(page), `${c.tema}: rejilla`).toEqual([]);
}

/** Abre la tarjeta del caso: su dirección y el foco al título del detalle. */
async function abrir(page: Page, c: Caso): Promise<Locator> {
  await tarjeta(page, c.elemento)
    .getByRole("button", { name: `Abrir ${c.elemento}`, exact: true })
    .click();
  await page.waitForURL((u) => u.pathname === `/ajustes/${c.tema}/${c.id}`);
  const titulo = page.locator("#detalle-titulo");
  await expect(titulo).toHaveText(c.elemento);
  await expect(titulo).toBeFocused();
  return page.locator(".detalle");
}

test.describe.configure({ mode: "parallel" });

for (const ancho of [360, 1440]) {
  for (const tema of ["oscuro", "claro"] as const) {
    const deVerdad = (ancho === 1440 && tema === "oscuro") || (ancho === 360 && tema === "claro");
    if (deVerdad) continue;
    const sufijo = `${ancho}-${tema}`;
    test.describe(`temas en rejilla a ${ancho} px en tema ${tema}`, () => {
      test.use({
        locale: "es-ES",
        viewport: { width: ancho, height: 900 },
        colorScheme: tema === "oscuro" ? "dark" : "light",
      });

      for (const c of casos) {
        test(`${c.nombre}: abrir, editar y descartar`, async ({ page }) => {
          const errores = recogerErrores(page);
          await entrar(page);
          await rejilla(page, c, sufijo, ancho);
          const detalle = await abrir(page, c);
          await c.editar(detalle, sufijo);
          await sinDesbordamiento(page, `${c.tema} detalle ${sufijo}`);
          await quieta(page);
          await page.screenshot({
            path: `${CAPTURAS}/${c.tema}-detalle-${sufijo}.png`,
            fullPage: ancho > 900,
            animations: "disabled",
          });
          expect(await violacionesGraves(page), `${c.tema}: detalle`).toEqual([]);
          // «Guardar cambios» abre la revisión; aquí se cancela.
          await detalle.getByRole("button", { name: t.guardarCambios }).click();
          const dialogo = page.locator("dialog[open]");
          await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
          await expect(dialogo.locator("tbody tr")).toHaveCount(1);
          await sinDesbordamiento(page, `${c.tema} diálogo ${sufijo}`);
          await dialogo.getByRole("button", { name: t.cancelar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          // «Descartar» y Esc: el detalle se cierra sin preguntar.
          await detalle.getByRole("button", { name: t.descartar, exact: true }).click();
          await page.keyboard.press("Escape");
          await page.waitForURL((u) => u.pathname === `/ajustes/${c.tema}`);
          await expect(page.locator(".detalle")).toHaveCount(0);
          expect(errores, "errores o avisos en la consola").toEqual([]);
        });
      }

      test("las recomendaciones de las tarjetas", async ({ page }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        // Sistema: el servidor de hora del operador; el botón prepara hora.roa.es.
        await page.goto("/ajustes/sistema");
        await asentar(page);
        const hora = tarjeta(page, "Hora del router");
        const rec = hora.locator(".tarjeta-rec");
        await expect(rec).toContainText("Recomendado si");
        await rec.getByRole("button", { name: t.horaNeutra }).click();
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo.locator("tbody td").nth(1)).toHaveText("hora.roa.es");
        await dialogo.getByRole("button", { name: t.cancelar }).click();
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        // Seguridad: la gestión remota encendida es un riesgo (fondo ámbar).
        await page.goto("/ajustes/seguridad");
        await asentar(page);
        const remota = tarjeta(page, "Gestión remota del operador").locator(".tarjeta-rec");
        await expect(remota).toHaveClass(/riesgo/);
        await expect(remota.getByRole("button", { name: t.apagarRemota })).toBeVisible();
        // Teléfono: cada tarjeta con su glifo (no los arcos de la WiFi), el
        // interruptor del teléfono por internet con su nombre y, con
        // acciones en lote, la marca redonda.
        await page.goto("/ajustes/telefono");
        await asentar(page);
        await expect(page.locator("article.tarjeta-el")).toHaveCount(4);
        await expect(page.locator("article.tarjeta-el .glifo")).toHaveCount(4);
        await expect(page.locator("article.tarjeta-el .emision")).toHaveCount(0);
        await expect(
          tarjeta(page, "Servidores de teléfono del operador").getByRole("switch", {
            name: "Teléfono por internet encendido",
          }),
        ).toBeVisible();
        await expect(page.locator("article.tarjeta-el .marcar")).toHaveCount(4);
        // La contraseña del router no tiene un estado que decir: sin insignia.
        await page.goto("/ajustes/seguridad");
        await asentar(page);
        await expect(tarjeta(page, "Contraseña del router").locator(".ins")).toHaveCount(0);
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}

// Los guardados de verdad, en el panel de los temas y en serie: el gestor
// aplica un cambio a la vez, y cada combinación cambia los cuatro temas.
test.describe("temas en rejilla: guardados de verdad", () => {
  test.describe.configure({ mode: "serial" });
  test.use({ locale: "es-ES", baseURL: temas.url });

  for (const [ancho, tema] of [
    [1440, "oscuro"],
    [360, "claro"],
  ] as const) {
    const sufijo = `${ancho}-${tema}`;
    test.describe(`a ${ancho} px en tema ${tema}`, () => {
      test.use({
        viewport: { width: ancho, height: 900 },
        colorScheme: tema === "oscuro" ? "dark" : "light",
      });

      test("el interruptor del relé DNS, con deshacer", async ({ page }) => {
        test.setTimeout(plazoDeTest(2));
        const errores = recogerErrores(page);
        await entrarEnSuPanel(page);
        await page.goto("/ajustes/internet");
        await asentar(page);
        const dns = tarjeta(page, "Relé DNS");
        const interruptor = dns.getByRole("switch");
        const antes = await interruptor.getAttribute("aria-checked");
        const despues = antes === "true" ? "false" : "true";
        // Sin efecto: se aplica sin preguntar.
        await interruptor.click();
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        const aviso = page.locator(".avisos-cambio [role=status] .aviso-cambio");
        await expect(aviso).toContainText(t.hecho, { timeout: PLAZO_CAMBIO });
        await expect(interruptor).toHaveAttribute("aria-checked", despues);
        await expect(interruptor).toHaveAccessibleName(
          despues === "true" ? "Relé DNS activado" : "Relé DNS desactivado",
        );
        await sinDesbordamiento(page, `interruptor ${sufijo}`);
        // «Deshacer»: la revisión y, aplicada, como estaba.
        await aviso.getByRole("button", { name: t.deshacer, exact: true }).click();
        const dialogo = page.locator("dialog[open]");
        await dialogo.getByRole("button", { name: t.deshacer, exact: true }).click();
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        await expect(interruptor).toHaveAttribute("aria-checked", antes ?? "false", {
          timeout: PLAZO_CAMBIO,
        });
        await expect(aviso).toContainText(t.hecho, { timeout: PLAZO_CAMBIO });
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });

      for (const c of casos) {
        test(`${c.nombre}: abrir, editar y guardar`, async ({ page }) => {
          test.setTimeout(plazoDeTest(1));
          const errores = recogerErrores(page);
          await entrarEnSuPanel(page);
          await rejilla(page, c, sufijo, ancho);
          const detalle = await abrir(page, c);
          await c.editar(detalle, sufijo);
          await sinDesbordamiento(page, `${c.tema} detalle ${sufijo}`);
          await quieta(page);
          await page.screenshot({
            path: `${CAPTURAS}/${c.tema}-detalle-${sufijo}.png`,
            fullPage: ancho > 900,
            animations: "disabled",
          });
          expect(await violacionesGraves(page), `${c.tema}: detalle`).toEqual([]);
          await detalle.getByRole("button", { name: t.guardarCambios }).click();
          const dialogo = page.locator("dialog[open]");
          await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
          await expect(dialogo.locator("tbody tr")).toHaveCount(1);
          // Al terminar, la página vuelve a leer el tema.
          const relectura = page.waitForResponse((r) => r.url().endsWith("/api/leer_tema"), {
            timeout: PLAZO_CAMBIO,
          });
          await dialogo.getByRole("button", { name: t.aplicar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          const aviso = page.locator(".avisos-cambio [role=status] .aviso-cambio");
          await expect(aviso).toContainText(t.hecho, { timeout: PLAZO_CAMBIO });
          expect((await relectura).status()).toBe(200);
          await asentar(page);
          await expect(aviso.getByRole("button", { name: t.deshacer, exact: true })).toBeVisible();
          await sinDesbordamiento(page, `${c.tema} hecho ${sufijo}`);
          // Guardado: cerrar el detalle (✕; en el móvil, «‹ Tema») ya no
          // pregunta.
          const cerrar = ancho > 900 ? "Cerrar el detalle" : `‹ ${c.nombre}`;
          await page.locator(".detalle").getByRole("button", { name: cerrar }).click();
          await page.waitForURL((u) => u.pathname === `/ajustes/${c.tema}`);
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          expect(errores, "errores o avisos en la consola").toEqual([]);
        });
      }
    });
  }
});
