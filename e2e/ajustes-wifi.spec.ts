import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
import { asentar, entrar, quieta, recogerErrores, sinDesbordamiento, violacionesGraves } from "./util";

// Ajustes → WiFi en rejilla (la pestaña modelo del rediseño), en español a
// 360, 1024 y 1440 px y en los dos temas, contra router-sim:
// - marcar dos redes con la marca redonda y ver «2 marcadas» en la barra;
// - abrir una tarjeta con Intro (foco al título del detalle, dirección
//   /ajustes/wifi/red-5), Esc la cierra y el foco vuelve a la tarjeta;
// - cambios sin guardar: otra tarjeta o «Atrás» preguntan antes;
// - una lectura fallida: su tarjeta dice «Sin leer» y el resto funciona;
// - el interruptor con efecto (apagar la radio de 5 GHz) pregunta antes.
// En dos combinaciones (1440 px en oscuro y 360 px en claro) se aplica de
// verdad: «Quitar WPS» en dos redes con su «Aplicando…», «Comprobando…» y
// «Deshacer», y el interruptor sin efecto (encender la red de invitados),
// que se aplica sin preguntar; cada uno se deshace después. En las demás,
// el diálogo se cancela. Axe sin violaciones serias en cada estado.

const t = {
  red24: "Red WiFi 2,4 GHz",
  red5: "Red WiFi 5 GHz",
  invitados: "Red de invitados",
  radio5: "Radio de 5 GHz",
  barra: "Acciones para lo marcado",
  quitarWps: "Quitar WPS",
  dosMarcadas: "2 marcadas",
  desmarcar: "Desmarcar todo",
  aplicar: "Aplicar",
  cancelar: "Cancelar",
  deshacer: "Deshacer",
  revisar: "Revisa el cambio",
  volver: "Volver a como estaba",
  aplicando: "Aplicando…",
  comprobando: "Comprobando…",
  hecho: "Hecho: el router tiene ya el cambio.",
  sinGuardar: "Tienes cambios sin guardar",
  seguir: "Seguir editando",
  descartar: "Descartar los cambios",
  nombre: "Nombre de la red",
  sinLeer: "Sin leer",
  volverALeer: "Volver a leer",
  efecto: "Al aplicarlo",
  encendida: "Encendida",
  apagada: "Apagada",
  motivo: "No se ha podido leer este ajuste del router. Prueba de nuevo en un momento.",
};

/** La tarjeta cuyo título es exactamente `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.tarjeta-el").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/** La insignia de estado de una tarjeta. */
function insignia(card: Locator): Locator {
  return card.locator(".tarjeta-insignia .ins");
}

/** La marca redonda de una tarjeta. */
function marca(card: Locator, titulo: string): Locator {
  return card.getByRole("button", { name: `Marcar ${titulo}`, exact: true });
}

/** Retrasa la próxima confirmación: deja ver «Aplicando…». */
async function confirmarDespacio(page: Page) {
  await page.route(
    "**/api/confirmar_ajuste",
    async (r) => {
      await new Promise((ok) => setTimeout(ok, 1_500));
      await r.continue();
    },
    { times: 1 },
  );
}

/** Aplica el diálogo abierto y sigue el cambio en la tarjeta y en el aviso. */
async function aplicarYSeguir(page: Page, boton: string, cards: Locator[]) {
  const dialogo = page.locator("dialog[open]");
  await confirmarDespacio(page);
  await dialogo.getByRole("button", { name: boton, exact: true }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
  for (const c of cards) await expect(insignia(c)).toHaveText(t.aplicando);
  for (const c of cards) await expect(insignia(c)).toHaveText(t.comprobando, { timeout: 15_000 });
  const aviso = page.locator(".avisos-cambio .aviso-cambio");
  await expect(aviso).toContainText(t.hecho, { timeout: PLAZO_CAMBIO });
  await expect(aviso.getByRole("button", { name: t.deshacer, exact: true })).toBeVisible();
  return aviso;
}

const CAPTURAS = "capturas/rediseno-3";

test.describe.configure({ mode: "parallel" });

for (const ancho of [360, 1024, 1440]) {
  for (const tema of ["oscuro", "claro"] as const) {
    const deVerdad = (ancho === 1440 && tema === "oscuro") || (ancho === 360 && tema === "claro");
    const sufijo = `${ancho}-${tema}`;
    test.describe(`WiFi en rejilla a ${ancho} px en tema ${tema}`, () => {
      test.use({
        locale: "es-ES",
        viewport: { width: ancho, height: 900 },
        colorScheme: tema === "oscuro" ? "dark" : "light",
      });

      test("marcar, detalle con el teclado y cambios sin guardar", async ({ page }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/wifi");
        await asentar(page);
        await expect(page.locator("article.tarjeta-el")).toHaveCount(5);
        await sinDesbordamiento(page, `wifi ${sufijo}`);
        const red24 = tarjeta(page, t.red24);
        const red5 = tarjeta(page, t.red5);

        // La marca es un círculo, nunca una casilla cuadrada.
        const marca24 = marca(red24, t.red24);
        expect(await marca24.evaluate((e) => getComputedStyle(e).borderRadius)).toBe("50%");
        await marca24.click();
        await expect(marca24).toHaveAttribute("aria-pressed", "true");
        await marca(red5, t.red5).click();
        const barra = page.getByRole("toolbar", { name: t.barra });
        await expect(barra).toBeVisible();
        await expect(barra.locator(".barra-acciones-n")).toHaveText(t.dosMarcadas);
        await expect(barra.getByRole("button", { name: t.quitarWps })).toBeVisible();
        await expect(red24).toHaveClass(/marcada/);
        await sinDesbordamiento(page, `dos marcadas ${sufijo}`);
        await quieta(page);
        await page.screenshot({ path: `${CAPTURAS}/wifi-marcadas-${sufijo}.png`, fullPage: ancho > 900, animations: "disabled" });
        expect(await violacionesGraves(page), "dos marcadas").toEqual([]);

        // «Quitar WPS»: un solo cambio con un paso por red (aquí se cancela).
        await barra.getByRole("button", { name: t.quitarWps }).click();
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
        await expect(dialogo.locator(".grupo-paso-titulo label")).toHaveText([t.red24, t.red5]);
        await sinDesbordamiento(page, `diálogo del lote ${sufijo}`);
        await dialogo.getByRole("button", { name: t.cancelar }).click();
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        await barra.getByRole("button", { name: t.desmarcar }).click();
        await expect(barra).toBeHidden();

        // Teclado: flechas, espacio marca, Intro abre y Esc cierra.
        await page.locator("#elemento-red-24").focus();
        await page.keyboard.press("ArrowRight");
        await expect(page.locator("#elemento-red-5")).toBeFocused();
        await page.keyboard.press("Space");
        await expect(barra).toBeVisible();
        await page.keyboard.press("Space");
        await expect(barra).toBeHidden();
        await page.keyboard.press("Enter");
        await page.waitForURL((u) => u.pathname === "/ajustes/wifi/red-5");
        const titulo = page.locator("#detalle-titulo");
        await expect(titulo).toHaveText(t.red5);
        await expect(titulo).toBeFocused();
        await sinDesbordamiento(page, `detalle ${sufijo}`);
        await quieta(page);
        await page.screenshot({ path: `${CAPTURAS}/wifi-detalle-${sufijo}.png`, fullPage: ancho > 900, animations: "disabled" });
        expect(await violacionesGraves(page), "detalle").toEqual([]);
        await page.keyboard.press("Escape");
        await page.waitForURL((u) => u.pathname === "/ajustes/wifi");
        await expect(page.locator(".detalle")).toHaveCount(0);
        await expect(page.locator("#elemento-red-5")).toBeFocused();

        // Cambios sin guardar: otra tarjeta y «Atrás» preguntan antes.
        await page.keyboard.press("Enter");
        await page.waitForURL((u) => u.pathname === "/ajustes/wifi/red-5");
        const nombre = page.locator(".detalle").getByLabel(t.nombre, { exact: true });
        const antes = await nombre.inputValue();
        await nombre.fill("RED-9");
        const pregunta = page.locator("dialog[open]");
        if (ancho > 900) {
          await red24.getByRole("heading", { level: 2 }).click();
          await expect(pregunta.locator("h2")).toHaveText(t.sinGuardar);
          await pregunta.getByRole("button", { name: t.seguir }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          await expect(page).toHaveURL(/\/ajustes\/wifi\/red-5$/);
          await expect(nombre).toHaveValue("RED-9");
          // ✕ también pregunta; seguir editando lo deja todo como estaba.
          await page.locator(".detalle").getByRole("button", { name: "Cerrar el detalle" }).click();
          await expect(pregunta.locator("h2")).toHaveText(t.sinGuardar);
          await pregunta.getByRole("button", { name: t.seguir }).click();
          await expect(nombre).toHaveValue("RED-9");
        }
        await page.goBack();
        await expect(pregunta.locator("h2")).toHaveText(t.sinGuardar);
        await sinDesbordamiento(page, `sin guardar ${sufijo}`);
        expect(await violacionesGraves(page), "sin guardar").toEqual([]);
        await pregunta.getByRole("button", { name: t.descartar }).click();
        await expect(page.locator(".detalle")).toHaveCount(0);
        await page.waitForURL((u) => u.pathname === "/ajustes/wifi");
        // Descartado de verdad: al volver a abrirla, lo que se leyó.
        await red5.getByRole("button", { name: `Abrir ${t.red5}` }).click();
        await expect(nombre).toHaveValue(antes);
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });

      test("una lectura fallida solo afecta a su tarjeta", async ({ page }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/internet");
        await asentar(page);
        // La red de 5 GHz no se puede leer (en la lectura que hace la página
        // al cambiar de tema).
        await page.route("**/api/leer_tema", async (r) => {
          const respuesta = await r.fetch();
          const pagina = await respuesta.json();
          if (pagina.tema === "wifi") {
            const e = pagina.elementos.find((x: { id: string }) => x.id === "red-5");
            const sinLeer = { SinLeer: t.motivo };
            Object.assign(e, {
              lectura: sinLeer,
              estado: "peligro",
              texto_estado: t.sinLeer,
              interruptor: null,
              datos: [],
              recomendacion: null,
              acciones: [],
            });
            for (const a of e.ajustes) Object.assign(a, { estado: sinLeer, campos: [] });
          }
          await r.fulfill({ response: respuesta, json: pagina });
        });
        if (ancho < 640) {
          await page.locator("#ajustes-tema").selectOption("wifi");
        } else {
          await page.locator('nav.temas a[href="/ajustes/wifi"]').click();
        }
        await page.waitForURL((u) => u.pathname === "/ajustes/wifi");
        const red5 = tarjeta(page, t.red5);
        await expect(insignia(red5)).toHaveText(t.sinLeer);
        await expect(red5).toContainText(t.motivo);
        await expect(red5.getByRole("button", { name: t.volverALeer })).toBeVisible();
        await expect(red5.getByRole("switch")).toHaveCount(0);
        // El resto, como siempre.
        await expect(page.getByRole("switch")).toHaveCount(4);
        const red24 = tarjeta(page, t.red24);
        await marca(red24, t.red24).click();
        await marca(red5, t.red5).click();
        const barra = page.getByRole("toolbar", { name: t.barra });
        await expect(barra.getByRole("button", { name: t.quitarWps })).toBeVisible();
        await expect(barra).toContainText(`No entra lo que no se ha podido leer: ${t.red5}.`);
        await sinDesbordamiento(page, `sin leer ${sufijo}`);
        await quieta(page);
        await page.screenshot({ path: `${CAPTURAS}/wifi-sin-leer-${sufijo}.png`, fullPage: ancho > 900, animations: "disabled" });
        expect(await violacionesGraves(page), "sin leer").toEqual([]);
        await page.unrouteAll({ behavior: "wait" });
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });

      test("acciones en lote e interruptores", async ({ page }) => {
        // Quitar WPS y encender la red de invitados, cada uno deshecho.
        if (deVerdad) test.setTimeout(plazoDeTest(4));
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/wifi");
        await asentar(page);
        const dialogo = page.locator("dialog[open]");

        // Con efecto (apagar la radio corta su WiFi): pregunta antes.
        const radio5 = tarjeta(page, t.radio5);
        await radio5.getByRole("switch", { name: `${t.radio5} encendida` }).click();
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
        await expect(dialogo).toContainText(t.efecto);
        await expect(dialogo.locator("tbody td").nth(0)).toHaveText(t.encendida);
        await expect(dialogo.locator("tbody td").nth(1)).toHaveText(t.apagada);
        await quieta(page);
        await page.screenshot({ path: `${CAPTURAS}/wifi-interruptor-con-efecto-${sufijo}.png`, animations: "disabled" });
        await dialogo.getByRole("button", { name: t.cancelar }).click();
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        await expect(radio5.getByRole("switch")).toHaveAttribute("aria-checked", "true");
        if (!deVerdad) {
          expect(errores, "errores o avisos en la consola").toEqual([]);
          return;
        }

        // «Quitar WPS» en dos redes: «Aplicando…», «Comprobando…», «Deshacer».
        const red24 = tarjeta(page, t.red24);
        const red5 = tarjeta(page, t.red5);
        await marca(red24, t.red24).click();
        await marca(red5, t.red5).click();
        const barra = page.getByRole("toolbar", { name: t.barra });
        await barra.getByRole("button", { name: t.quitarWps }).click();
        await expect(dialogo.locator("fieldset.grupo-paso")).toHaveCount(2);
        let aviso = await aplicarYSeguir(page, t.aplicar, [red24, red5]);
        await quieta(page);
        await page.screenshot({ path: `${CAPTURAS}/wifi-aviso-${sufijo}.png`, animations: "disabled" });
        expect(await violacionesGraves(page), "aviso").toEqual([]);
        await aviso.getByRole("button", { name: t.deshacer, exact: true }).click();
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.volver);
        await expect(dialogo.locator("fieldset.grupo-paso")).toHaveCount(2);
        aviso = await aplicarYSeguir(page, t.deshacer, [red24, red5]);
        await barra.getByRole("button", { name: t.desmarcar }).click();

        // Sin efecto (encender no corta nada): se aplica sin preguntar.
        const invitados = tarjeta(page, t.invitados);
        const interruptor = invitados.getByRole("switch");
        await expect(interruptor).toHaveAttribute("aria-checked", "false");
        await confirmarDespacio(page);
        await invitados.getByRole("switch", { name: `${t.invitados} apagada` }).click();
        await expect(insignia(invitados)).toHaveText(t.aplicando);
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        await expect(interruptor).toHaveAttribute("aria-checked", "true");
        await expect(insignia(invitados)).toHaveText(t.comprobando, { timeout: 15_000 });
        await expect(aviso).toContainText(
          `${t.invitados}: Red encendida, ${t.apagada} → ${t.encendida}`,
        );
        await expect(aviso).toContainText(t.hecho, { timeout: PLAZO_CAMBIO });
        await expect(insignia(invitados)).toHaveText(t.encendida);
        await aviso.getByRole("button", { name: t.deshacer, exact: true }).click();
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.volver);
        await aplicarYSeguir(page, t.deshacer, [invitados]);
        await expect(insignia(invitados)).toHaveText(t.apagada);
        await expect(interruptor).toHaveAttribute("aria-checked", "false");
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}
