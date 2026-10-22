import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, cambiarUpnp, esperarFinal, estadoCambio, plazoDeTest } from "./cambios";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

test.describe.configure({ mode: "parallel" });

/** La tarjeta de siempre cuyo título es exactamente `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.ajuste").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/** La tarjeta de la rejilla (WiFi) cuyo título es exactamente `titulo`. */
function tarjetaDeRejilla(page: Page, titulo: string): Locator {
  return page.locator("article.tarjeta-el").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

// Secretos (en español y en inglés, a 360, 1024 y 1440 px). Cada secreto es
// un campo de contraseña ya relleno, censurado, con un ojo que lo muestra u
// oculta al instante, sin pedir la contraseña del panel y sin ocultarse
// solo; los que se pueden escribir se editan en el mismo campo. Se prueba en
// la WiFi (censurado en la tarjeta, visible, con el teclado y editando en
// el detalle), en Teléfono (la
// clave que el router no deja leer: vacía, con su motivo, y pedida al
// guardar; en una combinación se guarda de verdad), en la Biblioteca y en el
// Historial (el valor anterior y el nuevo de la clave de 5 GHz, que cambia
// e2e_grupo al arrancar; en una combinación, con un cambio en curso: el
// Historial se refresca cada 2 s y lo desplegado aguanta).
const CLAVE_5 = "clave-de-prueba-1";

const textosSecreto = {
  es: {
    red5: "Red WiFi 5 GHz",
    contrasena: "Contraseña",
    ver: /^Mostrar /,
    ocultar: /^Ocultar /,
    editar: "Editar",
    cancelar: "Cancelar",
    guardar: "Guardar",
    abrir: "Abrir Red WiFi 5 GHz",
    descartar: "Descartar",
    aplicar: "Aplicar",
    hecho: "Hecho: el router tiene ya el cambio.",
    cerrar: "Cerrar",
    linea: "Línea de teléfono",
    lineaActiva: "Línea encendida",
    lineaClave: "Contraseña de la línea",
    sinLeer: "El router no deja leerla",
    pideLaClave: "El router no deja leer esta clave: escríbela para guardar los cambios.",
    leer: "Leer de nuevo",
    tecnico: "Detalles técnicos",
    ojoContrasena: "Mostrar Contraseña",
    sinDeshacer: "no se puede volver atrás",
    claveSinLeer: "(el router no deja leerla)",
    deshacer: "Deshacer",
  },
  en: {
    red5: "5 GHz WiFi network",
    contrasena: "Password",
    ver: /^Show /,
    ocultar: /^Hide /,
    editar: "Edit",
    cancelar: "Cancel",
    guardar: "Save",
    abrir: "Open 5 GHz WiFi network",
    descartar: "Discard",
    aplicar: "Apply",
    hecho: "Done: the router now has the change.",
    cerrar: "Close",
    linea: "Phone line",
    lineaActiva: "Line on",
    lineaClave: "Line password",
    sinLeer: "The router won't let it be read",
    pideLaClave: "The router won't let this password be read: type it to save the changes.",
    leer: "Read again",
    tecnico: "Technical details",
    ojoContrasena: "Show Password",
    sinDeshacer: "can't be put back",
    claveSinLeer: "(the router won't let it be read)",
    deshacer: "Undo",
  },
} as const;

/** El texto que se ve de un trozo de la página (sin los valores de los campos). */
async function aLaVista(l: Locator): Promise<string> {
  return l.innerText();
}

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1024, 1440]) {
    // Con un cambio en curso en el Historial y guardando de verdad la clave
    // de la voz, una sola combinación cada uno (cada cambio se vigila
    // RP_E2E_VIGILANCIA_S).
    const conCambioEnCurso = idioma === "en" && ancho === 1024;
    const guardaLaVoz = (idioma === "es" && ancho === 1024) || (idioma === "en" && ancho === 1440);
    test.describe(`secretos ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("el ojo en Ajustes, Teléfono, la Biblioteca y el Historial", async ({ page }) => {
        // La clave de la voz, un cambio; el de UPnP en curso, dos (lo deshace).
        test.setTimeout(plazoDeTest((guardaLaVoz ? 1 : 0) + (conCambioEnCurso ? 2 : 0)));
        const t = textosSecreto[idioma];
        const errores = recogerErrores(page);
        await page.clock.install();
        await entrar(page);

        // WiFi: censurado, con el valor ya en el campo.
        await page.goto("/ajustes/wifi");
        await asentar(page);
        const red5 = tarjetaDeRejilla(page, t.red5);
        const fila = red5.locator(".tarjeta-datos > div").filter({ hasText: t.contrasena });
        const campo = fila.locator("input.secreto-valor");
        await expect(campo).toHaveAttribute("type", "password");
        await expect(campo).toHaveValue(CLAVE_5);
        await expect(campo).toHaveAttribute("readonly", "");
        await expect(campo).toHaveAttribute("autocomplete", "off");
        expect(await aLaVista(page.locator("main"))).not.toContain(CLAVE_5);
        await sinDesbordamiento(page, `secreto censurado ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-censurado-${idioma}-${ancho}.png` });

        // El ojo lo muestra al instante, sin pedir la contraseña del panel
        // (ni abrir el detalle). Su nombre dice de qué campo es.
        const ojo = fila.getByRole("button", { name: t.ojoContrasena, exact: true });
        await expect(ojo).toHaveAttribute("aria-pressed", "false");
        await ojo.click();
        await expect(campo).toHaveAttribute("type", "text");
        await expect(fila.getByRole("button", { name: t.ocultar })).toHaveAttribute("aria-pressed", "true");
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        await expect(page.locator(".detalle")).toHaveCount(0);
        await sinDesbordamiento(page, `secreto visible ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-visible-${idioma}-${ancho}.png` });
        // No se oculta solo.
        await page.clock.runFor(60_000);
        await expect(campo).toHaveAttribute("type", "text");
        // Teclado: Tab desde el campo llega al ojo; Espacio e Intro lo cambian.
        await campo.focus();
        await page.keyboard.press("Tab");
        await expect(fila.getByRole("button", { name: t.ocultar })).toBeFocused();
        await page.keyboard.press("Space");
        await expect(campo).toHaveAttribute("type", "password");
        await page.keyboard.press("Enter");
        await expect(campo).toHaveAttribute("type", "text");

        // En el detalle: el mismo campo, relleno y editable, con su ojo.
        await red5.getByRole("button", { name: t.abrir, exact: true }).click();
        const detalle = page.locator(".detalle");
        const editable = detalle.getByLabel(t.contrasena, { exact: true });
        await expect(editable).toHaveValue(CLAVE_5);
        await expect(editable).toHaveAttribute("type", "password");
        await expect(editable).toHaveAttribute("autocomplete", "new-password");
        await expect(editable).not.toHaveAttribute("readonly", "");
        await editable.fill("clave-de-prueba-2");
        await detalle.getByRole("button", { name: t.ver }).first().click();
        await expect(editable).toHaveAttribute("type", "text");
        await sinDesbordamiento(page, `secreto editando ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-editando-${idioma}-${ancho}.png` });
        // «Descartar» lo deja como estaba.
        await detalle.getByRole("button", { name: t.descartar, exact: true }).click();
        await expect(editable).toHaveValue(CLAVE_5);

        // Al salir de la página, el valor ya no está en ella.
        await page.goto("/ajustes/puertos");
        await asentar(page);
        expect(await page.content()).not.toContain(CLAVE_5);

        // Teléfono: la clave que el router no deja leer sale vacía, con su
        // motivo, y se pide al guardar.
        await page.goto("/ajustes/telefono");
        await asentar(page);
        const linea = tarjeta(page, t.linea);
        const resumen = linea.locator(".ajuste-resumen .ajuste-fila").filter({ hasText: t.lineaClave });
        await expect(resumen.locator("input.secreto-valor")).toHaveValue("");
        await expect(resumen.locator("input.secreto-valor")).toHaveAttribute("placeholder", t.sinLeer);
        await linea.getByRole("button", { name: t.editar }).click();
        const lineaEdicion = linea.locator(".ajuste-edicion");
        const lineaClave = lineaEdicion.getByLabel(t.lineaClave, { exact: true });
        await expect(lineaClave).toHaveValue("");
        await lineaEdicion.getByRole("switch", { name: t.lineaActiva }).click();
        let preparados = 0;
        page.on("request", (r) => {
          if (r.url().endsWith("/api/preparar_ajuste")) preparados += 1;
        });
        await lineaEdicion.getByRole("button", { name: t.guardar }).click();
        await expect(lineaClave).toBeFocused();
        await expect(lineaEdicion.locator(".campo-error").filter({ hasText: t.pideLaClave })).toBeVisible();
        expect(preparados, "sin la clave no se pide nada al servidor").toBe(0);
        await sinDesbordamiento(page, `clave de la voz ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-voz-${idioma}-${ancho}.png` });
        if (guardaLaVoz) {
          // Con la clave escrita (y la línea como estaba), se guarda.
          await lineaEdicion.getByRole("switch", { name: t.lineaActiva }).click();
          await lineaClave.fill("clave-de-prueba-voz");
          await lineaEdicion.getByRole("button", { name: t.guardar }).click();
          const dialogo = page.locator("dialog[open]");
          await dialogo.getByRole("button", { name: t.aplicar }).click();
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: PLAZO_CAMBIO });
          await dialogo.getByRole("button", { name: t.cerrar }).click();
          // Sigue sin poder leerse: vacía otra vez.
          await expect(
            linea.locator(".ajuste-resumen .ajuste-fila").filter({ hasText: t.lineaClave }).locator("input"),
          ).toHaveValue("");
          // En el Historial no se ofrece deshacerlo, y se dice por qué; lo
          // de antes de la clave no se sabe.
          await page.goto("/historial");
          await asentar(page);
          const voz = page.locator("li.hist-cambio").filter({ hasText: t.linea }).first();
          await expect(voz.locator(".hist-sin-deshacer")).toContainText(t.sinDeshacer);
          await expect(voz.getByRole("button", { name: new RegExp(`^${t.deshacer}`) })).toHaveCount(0);
          await voz.locator("summary.hist-cabeza .hist-hora").click();
          await expect(voz.locator(".hist-detalle .tabla-scroll")).toContainText(t.claveSinLeer);
          await sinDesbordamiento(page, `historial sin deshacer ${idioma} ${ancho}`);
          await page.screenshot({ path: `capturas/secreto-sin-deshacer-${idioma}-${ancho}.png`, fullPage: true });
          await page.goto("/ajustes/telefono");
          await asentar(page);
        } else {
          await linea.getByRole("button", { name: t.cancelar }).click();
        }

        // Biblioteca: la celda secreta de una lectura, con el mismo ojo.
        await page.goto("/explorador/wifi/cbGetWifiAccPt");
        await asentar(page);
        await page.locator("#lectura-1").fill("1");
        await page.getByRole("button", { name: t.leer }).click();
        const celda = page.locator(".resultado-lectura input.secreto-valor").first();
        await expect(celda).toHaveAttribute("type", "password");
        await expect(celda).not.toHaveValue("");
        await page.locator(".resultado-lectura").getByRole("button", { name: t.ver }).first().click();
        await expect(celda).toHaveAttribute("type", "text");
        await sinDesbordamiento(page, `biblioteca ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-biblioteca-${idioma}-${ancho}.png` });

        // Historial: el cambio de la clave de 5 GHz, por su título; al
        // desplegarlo, el valor anterior y el nuevo con el ojo, sin
        // nombres de la API a la vista.
        let enCurso: number | null = null;
        if (conCambioEnCurso) enCurso = await cambiarUpnp(page);
        await page.goto("/historial");
        await asentar(page);
        // Con las filas plegadas, ningún nombre de la API a la vista (los de
        // las funciones van en los detalles técnicos de cada paso).
        expect(await page.locator("ol.hist-lista").innerText()).not.toMatch(/\bcb[A-Z]\w+/);
        // El de la clave (en la pasada completa hay otros de esta red: el nombre).
        const cambio = page
          .locator("li.hist-cambio")
          .filter({ hasText: t.red5 })
          .filter({ hasText: t.contrasena });
        await expect(cambio).toHaveCount(1);
        await cambio.locator("summary.hist-cabeza .hist-hora").click();
        const valores = cambio.locator(".hist-detalle input.secreto-valor");
        await expect(valores).toHaveCount(2);
        await expect(valores.nth(1)).toHaveValue(CLAVE_5);
        await expect(valores.nth(0)).not.toHaveValue("");
        await cambio.locator(".hist-detalle").getByRole("button", { name: t.ver }).nth(1).click();
        await expect(valores.nth(1)).toHaveAttribute("type", "text");
        const fuera = await cambio.locator(".hist-detalle > .tabla-scroll").innerText();
        expect(fuera).toContain(t.contrasena);
        expect(fuera).not.toMatch(/key_phrase|cbSet/);
        await expect(cambio.locator("details.detalles-tecnicos > summary")).toHaveText(t.tecnico);
        if (enCurso !== null) {
          // Aguanta los refrescos de 2 s del cambio en curso. Con la
          // vigilancia corta del e2e, el cambio podría terminar antes y la
          // espera no probaría nada: sigue vigilándose antes y después.
          expect(await estadoCambio(page, enCurso), "el cambio está en curso").toBe("Vigilando");
          await page.waitForTimeout(5_000);
          expect(await estadoCambio(page, enCurso), "sigue en curso tras la espera").toBe("Vigilando");
          await expect(valores.nth(1)).toHaveAttribute("type", "text");
          await expect(valores.nth(1)).toHaveValue(CLAVE_5);
        }
        await sinDesbordamiento(page, `historial ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-historial-${idioma}-${ancho}.png`, fullPage: true });
        if (enCurso !== null) {
          // Lo deja como estaba: espera al primero y deshace con otro.
          await esperarFinal(page, enCurso);
          await esperarFinal(page, await cambiarUpnp(page));
        }
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}
