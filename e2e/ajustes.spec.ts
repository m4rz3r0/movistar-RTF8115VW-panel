import { expect, type Locator, type Page, test } from "@playwright/test";
import { asentarDespacio as asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Páginas de Ajustes (/ajustes/<tema>) a 360 y 1440 px, en español y en
// inglés, contra router-sim: abrir WiFi, cambiar el nombre de la red de
// 5 GHz, ver el resumen «antes → después» sin nombres de la API, «Más
// opciones» y, en móvil, el selector de temas. El diálogo: el foco entra en
// su título y vuelve a «Guardar»; Escape no lo cierra mientras aplica ni
// mientras prepara el deshacer. «Cancelar» de la tarjeta deja los controles
// (también un <select> tocado varias veces) como los leyó la página.
//
// Aplicar, esperar «aplicado» y deshacer se hace en una sola combinación
// (inglés, 1440 px): cada aplicación se vigila (2 min en producción; en el
// e2e, RP_E2E_VIGILANCIA_S, 10 s por defecto, ver playwright.config.ts), y
// los plazos de abajo cuentan con esa vigilancia corta. Las demás llegan
// hasta el resumen y lo cancelan.
//
// Cada test entra con su propia sesión y deja el panel como estaba, así que
// se reparten entre los workers (cada uno con su panel, ver instancias.ts):
// este fichero es el más largo y, entero en un worker, sería el camino
// crítico de la pasada.
test.describe.configure({ mode: "parallel" });

const textos = {
  es: {
    red5: "Red WiFi 5 GHz",
    nombre: "Nombre de la red",
    seguridad: "Seguridad",
    aplicando: "Aplicando…",
    preparando: "Preparando…",
    editar: "Editar",
    guardar: "Guardar",
    cancelar: "Cancelar",
    aplicar: "Aplicar",
    deshacer: "Deshacer",
    cerrar: "Cerrar",
    revisar: "Revisa el cambio",
    volver: "Volver a como estaba",
    hecho: "Hecho: el router tiene ya el cambio.",
    mas: "Más opciones",
    red5Avanzada: "Red WiFi 5 GHz: más opciones",
    puertos: "Puertos",
  },
  en: {
    red5: "5 GHz WiFi network",
    nombre: "Network name",
    seguridad: "Security",
    aplicando: "Applying…",
    preparando: "Preparing…",
    editar: "Edit",
    guardar: "Save",
    cancelar: "Cancel",
    aplicar: "Apply",
    deshacer: "Undo",
    cerrar: "Close",
    revisar: "Review the change",
    volver: "Put it back as it was",
    hecho: "Done: the router now has the change.",
    mas: "More options",
    red5Avanzada: "5 GHz WiFi network: more options",
    puertos: "Ports",
  },
} as const;

/** La tarjeta cuyo título es exactamente `titulo`. */
function tarjeta(page: Page, titulo: string): Locator {
  return page.locator("article.ajuste").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/** Sin funciones ni parámetros de la API en un texto visible. */
function sinApi(texto: string) {
  expect(texto).not.toMatch(/\bcb[A-Z]\w+/);
  expect(texto).not.toMatch(/\bssid\b|key_phrase|authentication/);
}

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1440]) {
    const aplicaDeVerdad = idioma === "en" && ancho === 1440;
    test.describe(`ajustes ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("WiFi: cambiar el nombre de la red de 5 GHz", async ({ page }) => {
        if (aplicaDeVerdad) test.setTimeout(120_000);
        const t = textos[idioma];
        const errores = recogerErrores(page);
        await entrar(page);

        // 1. Abrir WiFi (en móvil, con el selector de temas).
        if (ancho < 640) {
          await page.goto("/ajustes/puertos");
          await asentar(page);
          await expect(page.locator("nav.temas")).toBeHidden();
          await page.locator("#ajustes-tema").selectOption("wifi");
          await page.waitForURL((u) => u.pathname === "/ajustes/wifi");
        } else {
          await page.goto("/ajustes/wifi");
          await asentar(page);
          await expect(page.locator("#ajustes-tema")).toBeHidden();
          await expect(page.locator("nav.temas a[aria-current=page]")).toHaveAttribute(
            "href",
            "/ajustes/wifi",
          );
        }
        const red5 = tarjeta(page, t.red5);
        await expect(red5).toBeVisible();
        await asentar(page);
        await sinDesbordamiento(page, `wifi ${idioma} ${ancho}`);
        const antes = (await red5.locator(".ajuste-fila dd").first().innerText()).trim();
        expect(antes.length).toBeGreaterThan(0);

        // 2. Cambiar el nombre.
        await red5.getByRole("button", { name: t.editar }).click();
        const nombre = red5.getByLabel(t.nombre, { exact: true });
        await expect(nombre).toBeFocused();
        const nuevo = `prueba-e2e-${idioma}-${ancho}`;
        await nombre.fill(nuevo);
        const guardar = red5.getByRole("button", { name: t.guardar });
        await guardar.click();

        // 3. El resumen: «antes → después», sin nombres de la API; el foco
        // en su título.
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo).toBeVisible();
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
        await expect(dialogo.locator("#grupo-titulo")).toBeFocused();
        const fila = dialogo.locator("tbody tr");
        await expect(fila).toHaveCount(1);
        await expect(fila.locator("th")).toHaveText(t.nombre);
        await expect(fila.locator("td").nth(0)).toHaveText(antes);
        await expect(fila.locator("td").nth(1)).toHaveText(nuevo);
        sinApi(await dialogo.innerText());
        await sinDesbordamiento(page, `diálogo ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/ajustes-dialogo-${idioma}-${ancho}.png` });

        if (aplicaDeVerdad) {
          // 4. Aplicar: mientras confirma, Escape no cierra; luego «aplicado»
          // y deshacer.
          await page.route(
            "**/api/confirmar_ajuste",
            async (r) => {
              await new Promise((ok) => setTimeout(ok, 1_500));
              await r.continue();
            },
            { times: 1 },
          );
          await dialogo.getByRole("button", { name: t.aplicar }).click();
          await expect(dialogo.getByRole("button", { name: t.aplicando })).toBeVisible();
          await page.keyboard.press("Escape");
          await expect(dialogo).toBeVisible();
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: 60_000 });
          // Deshacer: mientras prepara, Escape tampoco cierra.
          await page.route(
            "**/api/deshacer_ajuste",
            async (r) => {
              await new Promise((ok) => setTimeout(ok, 1_500));
              await r.continue();
            },
            { times: 1 },
          );
          await dialogo.getByRole("button", { name: t.deshacer }).click();
          await expect(dialogo.getByRole("button", { name: t.preparando })).toBeVisible();
          await page.keyboard.press("Escape");
          await expect(dialogo).toBeVisible();
          await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.volver);
          await expect(dialogo.locator("#grupo-titulo")).toBeFocused();
          await expect(dialogo.locator("tbody td").nth(0)).toHaveText(nuevo);
          await expect(dialogo.locator("tbody td").nth(1)).toHaveText(antes);
          await dialogo.getByRole("button", { name: t.deshacer }).click();
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: 60_000 });
          await dialogo.getByRole("button", { name: t.cerrar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          // La página relee el tema y el foco va al título de la tarjeta (eso
          // prueba la relectura: el nombre de antes ya estaba antes de aplicar).
          await expect(red5.locator(".ajuste-fila dd").first()).toHaveText(antes);
          await expect(red5.getByRole("heading", { level: 2 })).toBeFocused();
        } else {
          // Cancelar: el foco vuelve a «Guardar» y nada se escribe.
          await dialogo.getByRole("button", { name: t.cancelar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          await expect(guardar).toBeFocused();
          // Con el nombre aún cambiado, la seguridad tocada varias veces.
          const seguridad = red5.getByLabel(t.seguridad, { exact: true });
          await seguridad.selectOption("abierta");
          await seguridad.selectOption("wpa2");
          await seguridad.selectOption("abierta");
          // «Cancelar» de la tarjeta: al volver a editar, los controles
          // tienen lo que leyó la página.
          await red5.getByRole("button", { name: t.cancelar }).click();
          await red5.getByRole("button", { name: t.editar }).click();
          await expect(nombre).toHaveValue(antes);
          await expect(seguridad).toHaveValue("wpa2");
          await red5.getByRole("button", { name: t.cancelar }).click();
        }

        // 5. «Más opciones» abre y cierra.
        const mas = page.locator("details.mas-opciones");
        const avanzada = tarjeta(page, t.red5Avanzada);
        await expect(avanzada).toBeHidden();
        await mas.locator(":scope > summary").click();
        await expect(avanzada).toBeVisible();
        await sinDesbordamiento(page, `más opciones ${idioma} ${ancho}`);
        await page.screenshot({
          path: `capturas/ajustes-wifi-${idioma}-${ancho}.png`,
          fullPage: true,
        });
        await mas.locator(":scope > summary").click();
        await expect(avanzada).toBeHidden();

        // 6. Las pestañas (o el selector en móvil) llevan a otro tema.
        if (ancho < 640) {
          await page.locator("#ajustes-tema").selectOption("puertos");
        } else {
          await page.locator("nav.temas").getByRole("link", { name: t.puertos }).click();
        }
        await page.waitForURL((u) => u.pathname === "/ajustes/puertos");
        await expect(page.locator("article.ajuste").first()).toBeVisible();
        await sinDesbordamiento(page, `puertos ${idioma} ${ancho}`);
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}

// Secretos (en español y en inglés, a 360, 1024 y 1440 px). Cada secreto es
// un campo de contraseña ya relleno, censurado, con un ojo que lo muestra u
// oculta al instante, sin pedir la contraseña del panel y sin ocultarse
// solo; los que se pueden escribir se editan en el mismo campo. Se prueba en
// la WiFi (censurado, visible, con el teclado y editando), en Teléfono (la
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

/** Un cambio de UPnP (riesgo bajo: se vigila RP_E2E_VIGILANCIA_S), por las server functions. */
async function cambiarUpnp(page: Page): Promise<number> {
  const api = async (nombre: string, cuerpo: unknown) => {
    const r = await page.request.post(`/api/${nombre}`, { data: cuerpo });
    expect(r.status(), `${nombre}: ${await r.text()}`).toBe(200);
    return r.json();
  };
  const tema = await api("leer_tema", { tema: "puertos" });
  const upnp = tema.ajustes.find((a: { id: string }) => a.id === "puertos.upnp");
  const activo = upnp.campos.find((c: { id: string }) => c.id === "activo").valor.Booleano;
  const grupo = await api("preparar_ajuste", {
    p: { ajuste: "puertos.upnp", instancia: null, borrar: false, valores: { activo: String(!activo) } },
  });
  return api("confirmar_ajuste", { token: grupo.token, desmarcados: [] });
}

/** Estado del cambio `id` («Vigilando», «Aplicado»…). */
async function estadoCambio(page: Page, id: number): Promise<string> {
  const r = await page.request.post("/api/estado_cambio", { data: { id } });
  return (await r.json()).estado;
}

/** Espera a que el cambio `id` termine. */
async function esperarFinal(page: Page, id: number) {
  await expect
    .poll(() => estadoCambio(page, id), { timeout: 60_000, intervals: [1_000] })
    .toBe("Aplicado");
}

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
        if (conCambioEnCurso || guardaLaVoz) test.setTimeout(120_000);
        const t = textosSecreto[idioma];
        const errores = recogerErrores(page);
        await page.clock.install();
        await entrar(page);

        // WiFi: censurado, con el valor ya en el campo.
        await page.goto("/ajustes/wifi");
        await asentar(page);
        const red5 = tarjeta(page, t.red5);
        const fila = red5.locator(".ajuste-resumen .ajuste-fila").filter({ hasText: t.contrasena });
        const campo = fila.locator("input.secreto-valor");
        await expect(campo).toHaveAttribute("type", "password");
        await expect(campo).toHaveValue(CLAVE_5);
        await expect(campo).toHaveAttribute("readonly", "");
        await expect(campo).toHaveAttribute("autocomplete", "off");
        expect(await aLaVista(page.locator("main"))).not.toContain(CLAVE_5);
        await sinDesbordamiento(page, `secreto censurado ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-censurado-${idioma}-${ancho}.png` });

        // El ojo lo muestra al instante, sin pedir la contraseña del panel.
        // Su nombre dice de qué campo es.
        const ojo = fila.getByRole("button", { name: t.ojoContrasena, exact: true });
        await expect(ojo).toHaveAttribute("aria-pressed", "false");
        await ojo.click();
        await expect(campo).toHaveAttribute("type", "text");
        await expect(fila.getByRole("button", { name: t.ocultar })).toHaveAttribute("aria-pressed", "true");
        await expect(page.locator("dialog[open]")).toHaveCount(0);
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

        // Editar: el mismo campo, relleno y editable, con su ojo.
        await red5.getByRole("button", { name: t.editar }).click();
        const edicion = red5.locator(".ajuste-edicion");
        const editable = edicion.getByLabel(t.contrasena, { exact: true });
        await expect(editable).toHaveValue(CLAVE_5);
        await expect(editable).toHaveAttribute("type", "password");
        await expect(editable).toHaveAttribute("autocomplete", "new-password");
        await expect(editable).not.toHaveAttribute("readonly", "");
        await editable.fill("clave-de-prueba-2");
        await edicion.getByRole("button", { name: t.ver }).click();
        await expect(editable).toHaveAttribute("type", "text");
        await sinDesbordamiento(page, `secreto editando ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-editando-${idioma}-${ancho}.png` });
        // «Cancelar» lo deja como estaba.
        await red5.getByRole("button", { name: t.cancelar }).click();
        await red5.getByRole("button", { name: t.editar }).click();
        await expect(edicion.getByLabel(t.contrasena, { exact: true })).toHaveValue(CLAVE_5);
        await red5.getByRole("button", { name: t.cancelar }).click();

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
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: 60_000 });
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
          // vigilancia corta del e2e, el cambio podría haber terminado ya y
          // la espera no probaría nada: tiene que seguir vigilándose.
          expect(await estadoCambio(page, enCurso), "el cambio sigue en curso").toBe("Vigilando");
          await page.waitForTimeout(5_000);
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
