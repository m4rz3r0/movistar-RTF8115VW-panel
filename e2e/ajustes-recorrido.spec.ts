import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
import { CONTRASENA, USUARIO, recorridos } from "./instancias";
import { asentar, hidratada, recogerErrores, sinDesbordamiento } from "./util";

// Criterio de éxito 1 del spec de Ajustes: alguien que no conoce este router
// cambia la contraseña WiFi, apaga el WPS, fija la IP de un dispositivo,
// abre un puerto y desactiva la gestión remota del operador (TR-069), sin
// documentación y sin ver ningún nombre de la API (`cb…`) fuera de los
// detalles técnicos plegados. Un recorrido seguido, en español a 360 px y en
// inglés a 1440 px, cada uno en su propio panel (instancias.ts): los cinco
// cambios se aplican de verdad contra router-sim y no se deshacen.
//
// La gestión remota se pone «Apagada», la opción recomendada para quien
// prioriza su privacidad, y su recomendación tiene la forma «Recomendado si
// …. A favor: …. En contra: …».

const textos = {
  es: {
    wifi: "WiFi",
    redLocal: "Red local",
    puertos: "Puertos",
    seguridad: "Seguridad",
    red24: "Red WiFi 2,4 GHz",
    abrirRed24: "Abrir Red WiFi 2,4 GHz",
    guardarCambios: "Guardar cambios",
    quitarWps: "Quitar WPS",
    contrasena: "Contraseña",
    wps: "WPS",
    no: "No",
    editar: "Editar",
    guardar: "Guardar",
    aplicar: "Aplicar",
    cerrar: "Cerrar",
    revisar: "Revisa el cambio",
    hecho: "Hecho: el router tiene ya el cambio.",
    dispositivos: "Dispositivos conectados",
    fijas: "IP fijas",
    fijarHost2: "Fijar esta IP: 192.168.1.62 (host-2)",
    yaFija: "Ya tiene IP fija: 192.168.1.62",
    reglas: "Puertos abiertos",
    abrir: "Abrir un puerto",
    nombre: "Nombre",
    protocolo: "Protocolo",
    puerto: "Puerto de internet",
    dispositivo: "Dispositivo",
    nombreRegla: "Servidor web",
    gestion: "Gestión remota del operador",
    abrirGestion: "Abrir Gestión remota del operador",
    gestionCampo: "Gestión remota",
    apagada: "Apagada",
    recomendada: /^Recomendado: Apagada Recomendado si [^.]+\.$/,
    favor: "A favor",
    contra: "En contra",
    historial: "/historial",
  },
  en: {
    wifi: "WiFi",
    redLocal: "Home network",
    puertos: "Ports",
    seguridad: "Security",
    red24: "2.4 GHz WiFi network",
    abrirRed24: "Open 2.4 GHz WiFi network",
    guardarCambios: "Save changes",
    quitarWps: "Turn off WPS",
    contrasena: "Password",
    wps: "WPS",
    no: "No",
    editar: "Edit",
    guardar: "Save",
    aplicar: "Apply",
    cerrar: "Close",
    revisar: "Review the change",
    hecho: "Done: the router now has the change.",
    dispositivos: "Connected devices",
    fijas: "Fixed IPs",
    fijarHost2: "Fix this IP: 192.168.1.62 (host-2)",
    yaFija: "Already has a fixed IP: 192.168.1.62",
    reglas: "Open ports",
    abrir: "Open a port",
    nombre: "Name",
    protocolo: "Protocol",
    puerto: "Internet port",
    dispositivo: "Device",
    nombreRegla: "Web server",
    gestion: "Operator's remote management",
    abrirGestion: "Open Operator's remote management",
    gestionCampo: "Remote management",
    apagada: "Off",
    recomendada: /^Recommended: Off Recommended if [^.]+\.$/,
    favor: "Pros",
    contra: "Cons",
    historial: "/historial",
  },
} as const;

type Textos = (typeof textos)["es" | "en"];

/** La lista cuyo título es exactamente `titulo`. */
function lista(page: Page, titulo: string): Locator {
  return page.locator("section.lista-seccion").filter({
    has: page.getByRole("heading", { level: 2, name: titulo, exact: true }),
  });
}

/** La fila de la lista `titulo` que contiene `texto`. */
function fila(page: Page, titulo: string, texto: string): Locator {
  return lista(page, titulo).locator("tbody tr.lista-fila").filter({ hasText: texto });
}

/**
 * Ningún nombre de la API a la vista: el texto que se ve de la página (lo de
 * los detalles técnicos plegados no se ve) no nombra funciones `cb…` ni
 * páginas de la web básica.
 */
async function sinApiALaVista(page: Page, donde: string) {
  const texto = await page.locator("body").innerText();
  expect(texto, `${donde}: un nombre de la API a la vista`).not.toMatch(/\bcb[A-Z]\w+/);
  expect(texto, `${donde}: una página de la web básica a la vista`).not.toMatch(
    /\bte_\w+\.(asp|cmd|cgi|wl)\b/,
  );
}

/**
 * El diálogo del resumen en la rejilla (WiFi): sin la API a la vista;
 * aplica (el diálogo se cierra) y espera a que el aviso del cambio diga
 * «hecho».
 */
async function aplicarEnRejilla(page: Page, t: Textos, donde: string) {
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
  await sinApiALaVista(page, `${donde}: resumen`);
  await sinDesbordamiento(page, `${donde}: resumen`);
  await dialogo.getByRole("button", { name: t.aplicar }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
  const aviso = page.locator(".avisos-cambio [role=status] .aviso-cambio");
  await expect(aviso).not.toContainText(t.hecho);
  await expect(aviso).toContainText(t.hecho, { timeout: PLAZO_CAMBIO });
}

/**
 * Cierra el detalle abierto con «✕» (o, en el móvil, con «‹ Tema»). Tras un
 * cambio, la página vuelve a leer el tema y, mientras tanto, está inerte: se
 * reintenta hasta que la pulsación llega.
 */
async function cerrarDetalle(page: Page) {
  await expect(async () => {
    await page.locator(".detalle :is(.detalle-cerrar, .detalle-volver):visible").click();
    await expect(page.locator(".detalle")).toHaveCount(0, { timeout: 1_000 });
  }).toPass();
}

/** Va a un tema con las pestañas o, en móvil, con el selector de temas. */
async function irA(page: Page, ancho: number, tema: string, nombre: string) {
  if (ancho < 640) {
    await page.locator("#ajustes-tema").selectOption(tema);
  } else {
    await page.locator("nav.temas").getByRole("link", { name: nombre, exact: true }).click();
  }
  await page.waitForURL((u) => u.pathname === `/ajustes/${tema}`);
  await asentar(page);
  await sinApiALaVista(page, tema);
  await sinDesbordamiento(page, tema);
}

// Cada pasada en su panel: pueden ir en workers distintos.
test.describe.configure({ mode: "parallel" });

const pasadas = [
  { idioma: "es", locale: "es-ES", ancho: 360 },
  { idioma: "en", locale: "en-GB", ancho: 1440 },
] as const;

for (const { idioma, locale, ancho } of pasadas) {
  const panel = recorridos[idioma];
  const t = textos[idioma];

  test.describe(`recorrido ${idioma} a ${ancho} px`, () => {
    // Un clic que no llega falla pronto, no al agotar el plazo del test.
    test.use({
      locale,
      viewport: { width: ancho, height: 900 },
      baseURL: panel.url,
      actionTimeout: 15_000,
    });

    test("contraseña WiFi, WPS, IP fija, puerto y gestión remota", async ({ page }) => {
      test.setTimeout(plazoDeTest(5));
      const errores = recogerErrores(page);
      await page.goto("/login");
      await hidratada(page);
      await page.waitForLoadState("networkidle");
      await page.locator("#usuario").fill(USUARIO);
      await page.locator("#contrasena").fill(CONTRASENA);
      await page.locator("form button[type=submit]").click();
      await page.waitForURL((url) => url.pathname === "/");
      await page.goto("/ajustes/wifi");
      await asentar(page);
      await sinApiALaVista(page, "wifi");

      // 1. La contraseña de la red WiFi de 2,4 GHz, en su detalle.
      const red24 = page.locator("article.tarjeta-el").filter({
        has: page.getByRole("heading", { level: 2, name: t.red24, exact: true }),
      });
      await red24.getByRole("button", { name: t.abrirRed24, exact: true }).click();
      const detalle = page.locator(".detalle");
      await detalle.getByLabel(t.contrasena, { exact: true }).fill("una-frase-larga-de-prueba");
      await detalle.getByRole("button", { name: t.guardarCambios }).click();
      const dialogo = page.locator("dialog[open]");
      await expect(dialogo.locator("tbody tr")).toHaveCount(1);
      await expect(dialogo.locator("tbody th")).toHaveText(t.contrasena);
      // El secreto no sale en claro en el resumen.
      expect(await dialogo.innerText()).not.toContain("una-frase-larga-de-prueba");
      await aplicarEnRejilla(page, t, "contraseña WiFi");

      // 2. El WPS de esa red, apagado (el recomendado): el botón de la
      // recomendación de su tarjeta, con el detalle cerrado («Atrás»).
      await page.goBack();
      await expect(detalle).toHaveCount(0);
      await red24.getByRole("button", { name: t.quitarWps, exact: true }).click();
      await expect(dialogo.locator("tbody th")).toHaveText(t.wps);
      await aplicarEnRejilla(page, t, "WPS");
      await red24.getByRole("button", { name: t.abrirRed24, exact: true }).click();
      const wps = detalle.getByRole("switch", { name: t.wps, exact: true });
      await expect(wps).not.toBeChecked();
      await page.goBack();
      await expect(detalle).toHaveCount(0);

      // 3. La IP de un dispositivo conectado, fija, desde su detalle en la
      // lista de dispositivos.
      await irA(page, ancho, "red_local", t.redLocal);
      await fila(page, t.dispositivos, "host-2").locator(".lista-nombre").click();
      const lateral = page.locator(".detalle-lateral");
      await lateral.getByRole("button", { name: t.fijarHost2, exact: true }).click();
      await expect(dialogo).toContainText(`${t.fijas}: host-2`);
      await aplicarEnRejilla(page, t, "IP fija");
      await expect(lateral.locator(".chip-ya-fija").filter({ hasText: t.yaFija })).toBeVisible();
      await expect(fila(page, t.fijas, "192.168.1.62")).toHaveCount(1);
      await cerrarDetalle(page);

      // 4. Un puerto abierto hacia ese dispositivo.
      await irA(page, ancho, "puertos", t.puertos);
      await lista(page, t.reglas).getByRole("button", { name: t.abrir }).click();
      const form = lateral.locator("form.regla-formulario");
      await form.getByLabel(t.nombre, { exact: true }).fill(t.nombreRegla);
      await form.getByLabel(t.protocolo, { exact: true }).selectOption("tcp");
      await form.getByLabel(t.puerto, { exact: true }).fill("8443");
      await form.getByLabel(t.dispositivo, { exact: true }).selectOption("192.168.1.62");
      await form.getByRole("button", { name: t.guardar }).click();
      await expect(dialogo).toContainText(`${t.reglas}: ${t.nombreRegla}`);
      await aplicarEnRejilla(page, t, "puerto");
      // El detalle de la regla nueva se cierra solo al aplicarse.
      await expect(lateral).toHaveCount(0);
      const regla = fila(page, t.reglas, t.nombreRegla);
      await expect(regla).toContainText("8443");
      await expect(regla).toContainText("host-2");

      // 5. La gestión remota del operador (TR-069), apagada, en su detalle
      // (Seguridad va en rejilla).
      await irA(page, ancho, "seguridad", t.seguridad);
      const gestion = page.locator("article.tarjeta-el").filter({
        has: page.getByRole("heading", { level: 2, name: t.gestion, exact: true }),
      });
      await gestion.getByRole("button", { name: t.abrirGestion, exact: true }).click();
      await detalle.getByLabel(t.gestionCampo, { exact: true }).selectOption("apagada");
      // La recomendación: para quién es y, plegado, a favor y en contra.
      const reco = detalle.locator(".recomendacion").first();
      await expect(reco.locator(":scope > p")).toHaveText(t.recomendada);
      await reco.locator("details.recomendacion-pros > summary").click();
      // A favor y en contra, cada uno en su bloque con su encabezado.
      const pros = reco.locator("details.recomendacion-pros .pros-contras");
      await expect(pros.locator(".pros-contras-favor dt")).toHaveText(t.favor);
      await expect(pros.locator(".pros-contras-favor dd")).toHaveText(/^\S.+\.$/);
      await expect(pros.locator(".pros-contras-contra dt")).toHaveText(t.contra);
      await expect(pros.locator(".pros-contras-contra dd")).toHaveText(/^\S.+\.$/);
      await sinDesbordamiento(page, "gestión remota");
      await detalle.getByRole("button", { name: t.guardarCambios }).click();
      await expect(dialogo.locator("tbody td").nth(1)).toHaveText(t.apagada);
      await aplicarEnRejilla(page, t, "gestión remota");
      await expect(gestion.locator(".tarjeta-datos dd").first()).toContainText(t.apagada);
      await page.screenshot({ path: `capturas/recorrido-${idioma}-${ancho}.png`, fullPage: true });

      // El Historial: los cinco cambios, sin la API a la vista.
      await page.goto(t.historial);
      await asentar(page);
      await expect(page.locator("li.hist-cambio")).toHaveCount(5 + 2);
      await sinApiALaVista(page, "historial");
      await sinDesbordamiento(page, "historial");
      expect(errores, "errores o avisos en la consola").toEqual([]);
    });
  });
}
