import { expect, type Locator, type Page, test } from "@playwright/test";
import { CONTRASENA } from "./instancias";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Páginas de Ajustes (/ajustes/<tema>) a 360 y 1440 px, en español y en
// inglés, contra router-sim: abrir WiFi, cambiar el nombre de la red de
// 5 GHz, ver el resumen «antes → después» sin nombres de la API, «Más
// opciones» y, en móvil, el selector de temas. El diálogo: el foco entra en
// su título y vuelve a «Guardar»; Escape no lo cierra mientras aplica ni
// mientras prepara el deshacer. «Cancelar» de la tarjeta deja los controles
// (también un <select> tocado varias veces) como los leyó la página.
//
// Aplicar, esperar «aplicado» y deshacer se hace en una sola combinación
// (inglés, 1440 px, la última, para que un fallo no deje un cambio vigilándose
// a las demás): la WiFi es de riesgo medio y cada aplicación se vigila
// 2 min, así que repetirlo en las cuatro alargaría la pasada unos 16 min.
// Las demás llegan hasta el resumen y lo cancelan.

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
        if (aplicaDeVerdad) test.setTimeout(420_000);
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
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: 200_000 });
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
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: 200_000 });
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

// Secretos (en español y en inglés, a 360, 1024 y 1440 px): «Mostrar» la
// contraseña de la red de 5 GHz pide la del panel con el diálogo de
// reautenticación, enseña el valor con «Copiar» y, a los 30 s (con el reloj
// de Playwright), vuelve a la máscara y el valor sale del DOM. Al editar,
// «Cambiar» abre el campo con «Ver lo escrito». En Teléfono, la contraseña
// de la línea (el router no deja leerla) se pide al guardar. En el
// Historial, el paso que cambió la clave de 5 GHz (lo aplica e2e_grupo al
// arrancar) ofrece «Mostrar el valor anterior»; en una combinación, con un
// cambio en curso (el Historial se refresca cada 2 s): el diálogo abierto
// más de 2 s no rompe la página y el valor aguanta.
const CLAVE_5 = "clave-de-prueba-1";

const textosSecreto = {
  es: {
    red5: "Red WiFi 5 GHz",
    contrasena: "Contraseña",
    mostrar: /^Mostrar/,
    copiar: /^Copiar/,
    cuenta: "Se volverá a ocultar",
    reautenticar: "Confirma que eres tú",
    seguir: "Seguir",
    editar: "Editar",
    cambiar: /^Cambiar/,
    ver: "Ver lo escrito",
    cancelar: "Cancelar",
    guardar: "Guardar",
    linea: "Línea de teléfono",
    lineaActiva: "Línea encendida",
    lineaClave: "Contraseña de la línea",
    pideLaClave: "El router no deja leer esta clave: escríbela para guardar los cambios.",
    anterior: /^Mostrar el valor anterior/,
    revelado: "Se mostró «Contraseña» de Red WiFi 5 GHz",
  },
  en: {
    red5: "5 GHz WiFi network",
    contrasena: "Password",
    mostrar: /^Show(?! what I typed)/,
    copiar: /^Copy/,
    cuenta: "It will be hidden again",
    reautenticar: "Confirm it's you",
    seguir: "Continue",
    editar: "Edit",
    cambiar: /^Change/,
    ver: "Show what I typed",
    cancelar: "Cancel",
    guardar: "Save",
    linea: "Phone line",
    lineaActiva: "Line on",
    lineaClave: "Line password",
    pideLaClave: "The router won't let this password be read: type it to save the changes.",
    anterior: /^Show the previous value/,
    revelado: "Showed “Password” for 5 GHz WiFi network",
  },
} as const;

/** Un cambio de UPnP (riesgo bajo: se vigila 30 s), por las server functions. */
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

/** Espera a que el cambio `id` termine (y el gestor quede libre). */
async function esperarFinal(page: Page, id: number) {
  await expect
    .poll(
      async () => {
        const r = await page.request.post("/api/estado_cambio", { data: { id } });
        return (await r.json()).estado;
      },
      { timeout: 90_000, intervals: [1_000] },
    )
    .toBe("Aplicado");
}

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1024, 1440]) {
    // Con un cambio en curso solo en una (cada cambio se vigila 30 s, y hay
    // que deshacerlo para dejar el simulador como estaba).
    const conCambioEnCurso = idioma === "en" && ancho === 1024;
    test.describe(`secretos ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("Mostrar, cuenta atrás, Cambiar, la clave de la voz y el valor anterior", async ({ page }) => {
        if (conCambioEnCurso) test.setTimeout(240_000);
        const t = textosSecreto[idioma];
        // El 401 que pide la contraseña es lo esperado; cualquier otro error
        // o aviso de la consola, no.
        const errores: string[] = [];
        page.on("console", (m) => {
          const url = m.location().url;
          if (url.endsWith("/favicon.ico") || /\/api\/revelar_(ajuste|anterior)$/.test(url)) return;
          if (m.type() === "error" || m.type() === "warning") errores.push(m.text());
        });
        page.on("pageerror", (e) => errores.push(String(e)));
        await page.clock.install();
        await entrar(page);
        await page.goto("/ajustes/wifi");
        await asentar(page);
        const red5 = tarjeta(page, t.red5);
        const fila = red5.locator(".ajuste-resumen .ajuste-fila").filter({ hasText: t.contrasena });
        await expect(fila.locator(".secreto")).toHaveText("••••••");
        await expect(red5).not.toContainText(CLAVE_5);
        expect(await page.content()).not.toContain(CLAVE_5);
        await page.screenshot({ path: `capturas/secreto-mascara-${idioma}-${ancho}.png` });

        // «Mostrar»: sin la ventana abierta, el diálogo de reautenticación.
        await fila.getByRole("button", { name: t.mostrar }).click();
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo.locator("h2")).toHaveText(t.reautenticar);
        await dialogo.locator("#reautenticar-contrasena").fill(CONTRASENA);
        await dialogo.getByRole("button", { name: t.seguir }).click();
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        const valor = fila.locator(".revelado");
        await expect(valor).toHaveText(CLAVE_5);
        await expect(fila.getByRole("button", { name: t.copiar })).toBeVisible();
        await expect(fila.locator(".solo-lector").filter({ hasText: t.cuenta })).toContainText("30");
        await sinDesbordamiento(page, `secreto mostrado ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-mostrado-${idioma}-${ancho}.png` });

        // A los 30 s, la máscara otra vez; el valor ya no está en la página.
        await page.clock.runFor(29_000);
        await expect(valor).toHaveText(CLAVE_5);
        await page.clock.runFor(1_500);
        await expect(fila.locator(".revelado")).toHaveCount(0);
        await expect(fila.locator(".secreto")).toHaveText("••••••");
        expect(await page.content()).not.toContain(CLAVE_5);

        // Editar: «•••••• · Mostrar · Cambiar»; «Cambiar» abre el campo.
        await red5.getByRole("button", { name: t.editar }).click();
        const edicion = red5.locator(".ajuste-edicion");
        const cambiar = edicion.getByRole("button", { name: t.cambiar });
        await expect(cambiar).toBeVisible();
        await expect(edicion.getByRole("button", { name: t.mostrar })).toBeVisible();
        await cambiar.click();
        const campo = edicion.getByLabel(t.contrasena, { exact: true });
        await expect(campo).toBeFocused();
        await expect(campo).toHaveAttribute("type", "password");
        await expect(campo).toHaveAttribute("autocomplete", "new-password");
        await campo.fill("clave-de-prueba-2");
        await edicion.getByRole("button", { name: t.ver }).click();
        await expect(campo).toHaveAttribute("type", "text");
        await sinDesbordamiento(page, `secreto en edición ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-edicion-${idioma}-${ancho}.png` });
        // «Cancelar» lo deja como estaba: otra vez la máscara y «Cambiar».
        await red5.getByRole("button", { name: t.cancelar }).click();
        await red5.getByRole("button", { name: t.editar }).click();
        await expect(edicion.getByRole("button", { name: t.cambiar })).toBeVisible();
        await expect(edicion.getByLabel(t.contrasena, { exact: true })).toHaveCount(0);
        await red5.getByRole("button", { name: t.cancelar }).click();

        // Teléfono: la contraseña de la línea (el router no deja leerla) está
        // abierta al editar y se pide al guardar; sin ella, nada se envía.
        await page.goto("/ajustes/telefono");
        await asentar(page);
        const linea = tarjeta(page, t.linea);
        await linea.getByRole("button", { name: t.editar }).click();
        const lineaEdicion = linea.locator(".ajuste-edicion");
        const lineaClave = lineaEdicion.getByLabel(t.lineaClave, { exact: true });
        await expect(lineaClave).toBeVisible();
        await expect(lineaEdicion.getByRole("button", { name: t.mostrar })).toHaveCount(0);
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
        await linea.getByRole("button", { name: t.cancelar }).click();

        // Historial, en otra sesión (sin la ventana de reautenticación):
        // «Mostrar el valor anterior» del paso que cambió la clave.
        let enCurso: number | null = null;
        await entrar(page);
        if (conCambioEnCurso) enCurso = await cambiarUpnp(page);
        await page.goto("/historial");
        await asentar(page);
        // (El botón está dentro del detalle plegado: se busca la lista.)
        const cambio = page.locator("li.hist-cambio", { has: page.locator("ul.hist-anteriores") });
        await expect(cambio).toHaveCount(1);
        // En la hora: el centro del resumen puede caer en el enlace a la función.
        await cambio.locator("summary.hist-cabeza .hist-hora").click();
        const anterior = cambio.locator(".hist-anteriores li");
        await expect(anterior).toContainText(t.contrasena);
        await expect(anterior.locator(".secreto")).toHaveText("••••••");
        await sinDesbordamiento(page, `historial con el valor anterior ${idioma} ${ancho}`);
        await page.screenshot({ path: `capturas/secreto-historial-${idioma}-${ancho}.png`, fullPage: true });
        await anterior.getByRole("button", { name: t.anterior }).click();
        await expect(dialogo.locator("h2")).toHaveText(t.reautenticar);
        await dialogo.locator("#reautenticar-contrasena").fill(CONTRASENA);
        // Con un cambio en curso, el Historial se refresca mientras el
        // diálogo sigue abierto.
        if (conCambioEnCurso) await page.waitForTimeout(4_500);
        await dialogo.getByRole("button", { name: t.seguir }).click();
        await expect(page.locator("dialog[open]")).toHaveCount(0);
        const mostrado = anterior.locator(".revelado");
        await expect(mostrado).toBeVisible();
        await expect(mostrado).not.toHaveText(CLAVE_5);
        if (conCambioEnCurso) {
          // Aguanta los refrescos de 2 s.
          await page.waitForTimeout(5_000);
          await expect(mostrado).toBeVisible();
        }
        await page.screenshot({ path: `capturas/secreto-historial-mostrado-${idioma}-${ancho}.png`, fullPage: true });
        // El revelado de Ajustes, con el ajuste y el campo (en lenguaje claro).
        const revelados = page.locator("li.hist-revelado");
        await expect(revelados.filter({ hasText: t.revelado }).first()).toBeVisible();
        expect(await revelados.allInnerTexts()).not.toContainEqual(expect.stringMatching(/key_phrase|cbGet|cbSet/));
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
