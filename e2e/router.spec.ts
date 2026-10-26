import { expect, type Page, test } from "@playwright/test";
import { CONTRASENA, instalado } from "./instancias";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";
import { capturaOpcional } from "./util";

// Sección «Configuración del router»: el enlace de la navegación lleva a
// «Conexión con el router» y «Atrás» vuelve a la página de antes (sin
// quedarse atrapado en una redirección). «Ajustes del panel» (se abre con el
// nombre de usuario) enlaza con la conexión.

test.describe("sección Configuración del router", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("Configuración del router desde Estado y Atrás vuelve a Estado", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await asentar(page);
    await page
      .getByRole("navigation", { name: "Principal" })
      .getByRole("link", { name: "Configuración del router" })
      .click();
    await page.waitForURL((url) => url.pathname === "/router/conexion");
    await expect(page.locator("main h1")).toHaveText("Conexión con el router");
    await page.goBack();
    await page.waitForURL((url) => url.pathname === "/");
    await expect(page.locator("main h1").first()).toHaveText("Estado");
    expect(errores, "errores o avisos en la consola").toEqual([]);
  });

  test("/router lleva a la conexión y Atrás no se queda en ella", async ({ page }) => {
    await entrar(page);
    await asentar(page);
    // Navegación del lado del cliente a /router (como un enlace antiguo).
    await page.evaluate(() => {
      const a = document.createElement("a");
      a.href = "/router";
      document.querySelector("main")?.appendChild(a);
      a.click();
    });
    await page.waitForURL((url) => url.pathname === "/router/conexion");
    await page.goBack();
    await page.waitForURL((url) => url.pathname === "/");
  });

  for (const ruta of ["/recomendaciones", "/recomendaciones/preferencias"]) {
    test(`${ruta} lleva al Estado y Atrás no se queda en ella`, async ({ page }) => {
      await entrar(page);
      await page.goto("/historial");
      await asentar(page);
      // Navegación del lado del cliente (como un enlace antiguo).
      await page.evaluate((href) => {
        const a = document.createElement("a");
        a.href = href;
        document.querySelector("main")?.appendChild(a);
        a.click();
      }, ruta);
      await page.waitForURL((url) => url.pathname === "/");
      await page.goBack();
      await page.waitForURL((url) => url.pathname === "/historial");
    });
  }

  test("Ajustes del panel enlaza con la conexión", async ({ page }) => {
    await entrar(page);
    await asentar(page);
    await page.getByRole("link", { name: "Ajustes del panel" }).click();
    await page.waitForURL((url) => url.pathname === "/panel");
    await asentar(page);
    await page.locator("main .pagina-sub a[href='/router/conexion']").click();
    await page.waitForURL((url) => url.pathname === "/router/conexion");
    await expect(page.locator("main h1")).toHaveText("Conexión con el router");
  });
});

// Red del router detectada en «Conexión con el router»: el resumen a 360 y
// 1440 px, en español y en inglés, sin desbordamiento ni errores.
const red = {
  es: { titulo: "Red del router", internet: "Internet: ip2 (PPPoE)", telefonia: "Telefonía: sí" },
  en: { titulo: "Router network", internet: "Internet: ip2 (PPPoE)", telefonia: "Phone: yes" },
} as const;

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1440]) {
    test.describe(`red del router ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("muestra la red detectada", async ({ page }) => {
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/router/conexion");
        await asentar(page);
        const t = red[idioma];
        const tarjeta = page.locator("section[aria-labelledby=router-red]");
        await expect(tarjeta.locator("h2")).toHaveText(t.titulo);
        await expect(tarjeta.locator(".red-resumen")).toContainText(t.internet);
        await expect(tarjeta.locator(".red-resumen")).toContainText(t.telefonia);
        await expect(tarjeta.locator("#router-red-conexion option")).toHaveCount(4);
        await sinDesbordamiento(page, `red del router ${idioma} ${ancho}`);
        await capturaOpcional(page, { path: `capturas/router-red-${idioma}-${ancho}.png`, fullPage: true });
        expect(errores, "errores o avisos en la consola").toEqual([]);
      });
    });
  }
}

test.describe("elegir la conexión a internet", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("pide la contraseña, guarda y vuelve a la automática", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/router/conexion");
    await asentar(page);
    const tarjeta = page.locator("section[aria-labelledby=router-red]");
    await tarjeta.locator("#router-red-conexion").selectOption("ip4");
    await tarjeta.getByRole("button", { name: "Guardar la conexión" }).click();
    const dialogo = page.locator("dialog[open]");
    await expect(dialogo).toBeVisible();
    await dialogo.locator("#reautenticar-contrasena").fill(CONTRASENA);
    await dialogo.locator("button[type=submit]").click();
    await expect(tarjeta.locator(".aviso-formulario")).toHaveText("Conexión a internet guardada.");

    // Tras recargar, sigue elegida; y se deja como estaba (la automática).
    await page.reload();
    await asentar(page);
    await expect(tarjeta.locator("#router-red-conexion")).toHaveValue("ip4");
    await tarjeta.locator("#router-red-conexion").selectOption("");
    await tarjeta.getByRole("button", { name: "Guardar la conexión" }).click();
    await expect(tarjeta.locator(".aviso-formulario")).toHaveText("Conexión a internet guardada.");
    await tarjeta.getByRole("button", { name: "Volver a detectar" }).click();
    await expect(tarjeta.locator(".aviso-formulario")).toHaveText("Red del router detectada de nuevo.");
    await expect(tarjeta.locator(".red-resumen")).toContainText("Internet: ip2 (PPPoE)");
    // El 401 que abre el diálogo de reautenticación es el esperado.
    expect(
      errores.filter((e) => !/status of 401/.test(e)),
      "errores o avisos en la consola",
    ).toEqual([]);
  });
});

// Cambiar la contraseña del router (Tarea 12): en «Conexión con el router»,
// junto a la tarjeta de la contraseña con la que entra el panel (rellena,
// censurada y con el ojo). Pide la contraseña del panel y confirma con un
// diálogo propio. Solo una prueba cambia de verdad la del simulador; las
// demás cancelan en el diálogo.

const NUEVA_ROUTER = "clave-nueva-de-prueba";
const SDD = "capturas";

function sinApiNiClave(texto: string) {
  expect(texto).not.toMatch(/\bcb[A-Z]\w+/);
  expect(texto).not.toMatch(/rqUser|rqOPwd|rqNPwd|user_profile/);
  expect(texto).not.toContain(NUEVA_ROUTER);
}

/** Escribe la nueva, pulsa «Cambiar» y, si lo pide, la contraseña del panel. */
async function pedirCambio(page: Page, nueva: string) {
  const tarjeta = page.locator("section#router-cambiar-contrasena");
  await tarjeta.locator("#router-contrasena-nueva").fill(nueva);
  await tarjeta.getByRole("button", { name: /^(Cambiar la contraseña|Change the password)$/ }).click();
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo).toBeVisible();
  if (await dialogo.locator("#reautenticar-contrasena").count()) {
    await dialogo.locator("#reautenticar-contrasena").fill(CONTRASENA);
    await dialogo.locator("button[type=submit]").click();
  }
  await expect(page.locator("dialog[open] #router-confirmar-titulo")).toBeVisible();
}

test.describe("cambiar la contraseña del router", () => {
  for (const ancho of [360, 1024, 1440]) {
    test(`es a ${ancho} px: las dos tarjetas, el ojo y la confirmación`, async ({ page }) => {
      await page.setViewportSize({ width: ancho, height: 900 });
      const errores = recogerErrores(page);
      await entrar(page);
      await page.goto("/router/conexion");
      await asentar(page);
      const cred = page.locator("section[aria-labelledby=router-credenciales]");
      const cambio = page.locator("section#router-cambiar-contrasena");
      await expect(cred.locator("h2")).toHaveText("Contraseña con la que el panel entra en el router");
      await expect(cambio.locator("h2")).toHaveText("Cambiar la contraseña del router");
      // La guardada llega rellena y censurada; el ojo la muestra.
      const guardada = cred.locator("#panel-router-contrasena");
      await expect(guardada).toHaveAttribute("type", "password");
      expect(await guardada.inputValue()).not.toBe("");
      await sinDesbordamiento(page, `tarjetas ${ancho}`);
      sinApiNiClave(await page.locator("main").innerText());
      await capturaOpcional(page, { path: `${SDD}/contrasena-tarjetas-es-${ancho}.png`, fullPage: true });

      // La nueva, con el ojo abierto para comprobarla.
      const nueva = cambio.locator("#router-contrasena-nueva");
      await expect(nueva).toHaveAttribute("autocomplete", "new-password");
      await expect(nueva).toHaveAttribute("data-1p-ignore", "");
      // Lo más que admite la web oficial del router.
      await expect(nueva).toHaveAttribute("maxlength", "32");
      await nueva.fill(NUEVA_ROUTER);
      await cambio.getByRole("button", { name: "Mostrar Contraseña nueva" }).click();
      await expect(nueva).toHaveAttribute("type", "text");
      await sinDesbordamiento(page, `ojo ${ancho}`);
      await capturaOpcional(cambio, { path: `${SDD}/contrasena-ojo-es-${ancho}.png` });

      // Una corta: el error, sin diálogo.
      await nueva.fill("corta");
      await cambio.getByRole("button", { name: "Cambiar la contraseña" }).click();
      await expect(cambio.locator(".error-formulario")).toContainText("demasiado corta");
      await expect(page.locator("dialog[open]")).toHaveCount(0);

      await pedirCambio(page, NUEVA_ROUTER);
      const dialogo = page.locator("dialog[open]");
      await expect(dialogo.locator("h2")).toHaveText("¿Cambiar la contraseña del router?");
      await expect(dialogo).toContainText("también desde su propia web");
      sinApiNiClave(await dialogo.innerText());
      await sinDesbordamiento(page, `diálogo ${ancho}`);
      await capturaOpcional(page, { path: `${SDD}/contrasena-dialogo-es-${ancho}.png` });
      await dialogo.getByRole("button", { name: "Cancelar" }).click();
      await expect(page.locator("dialog[open]")).toHaveCount(0);
      // No ha cambiado nada.
      await expect(cambio.locator(".aviso-formulario")).toBeEmpty();
      expect(
        errores.filter((e) => !/status of 401/.test(e)),
        "errores o avisos en la consola",
      ).toEqual([]);
    });
  }

  test("en a 1024 px: el error grave explica cómo recuperar el acceso", async ({ browser }) => {
    const ctx = await browser.newContext({
      locale: "en-GB",
      viewport: { width: 1024, height: 900 },
      colorScheme: "dark",
    });
    const page = await ctx.newPage();
    await entrar(page);
    await page.goto("/router/conexion");
    await asentar(page);
    await expect(page.locator("main h1")).toHaveText("Router connection");
    // El simulador no sabe dejar al panel fuera con las dos contraseñas a
    // la vez: la respuesta de un error de verdad (la nueva corta), con el
    // texto del error grave (el de router.ftl en inglés).
    const corta = "The new password is too short: it needs at least 8 characters.";
    const grave =
      "The panel cannot log in to the router with either the new password or the previous one. " +
      "To get access back, log in to the router's web interface (try the new one and, if that " +
      "fails, the previous one) and set by hand the password the panel has saved: press the eye " +
      'in "Password the panel uses to log in to the router", on this same page, to see it.';
    await page.route(
      "**/api/cambiar_contrasena_router",
      async (r) => {
        const real = await r.fetch({ postData: JSON.stringify({ nueva: "corta" }) });
        const texto = await real.text();
        expect(texto).toContain(corta);
        await r.fulfill({ response: real, status: 502, body: texto.replace(corta, grave) });
      },
      { times: 1 },
    );
    await pedirCambio(page, "otra-clave-de-prueba");
    await page.locator("dialog[open]").getByRole("button", { name: "Change the password" }).click();
    const cambio = page.locator("section#router-cambiar-contrasena");
    await expect(cambio.locator(".error-formulario")).toHaveText(grave);
    await expect(page.locator("dialog[open]")).toHaveCount(0);
    await sinDesbordamiento(page, "error grave en 1024");
    await capturaOpcional(cambio, { path: `${SDD}/contrasena-error-grave-en-1024.png` });
    await ctx.close();
  });

  test("Ajustes → Seguridad enlaza con la tarjeta sin repetir el formulario", async ({ page }) => {
    await page.setViewportSize({ width: 360, height: 900 });
    await entrar(page);
    // En el detalle de la contraseña del router, en la rejilla de Seguridad.
    await page.goto("/ajustes/seguridad/contrasena-router");
    await asentar(page);
    const enlace = page.getByRole("link", { name: "Cambiar la contraseña del router" });
    await expect(enlace).toBeVisible();
    await expect(page.locator("#router-contrasena-nueva")).toHaveCount(0);
    await sinDesbordamiento(page, "seguridad 360");
    await capturaOpcional(enlace.locator("xpath=ancestor::section[contains(@class, 'detalle-columna')]"), {
      path: `${SDD}/contrasena-enlace-seguridad-es-360.png`,
    });
    await enlace.click();
    await page.waitForURL((url) => url.pathname === "/router/conexion");
    // La vista llega hasta la tarjeta, aunque se pinte tras la carga.
    await expect(page.locator("section#router-cambiar-contrasena h2")).toBeInViewport();
    // También al abrir el enlace directamente (carga completa).
    await page.goto("/");
    await asentar(page);
    await page.goto("/router/conexion#router-cambiar-contrasena");
    await asentar(page);
    await expect(page.locator("section#router-cambiar-contrasena h2")).toBeInViewport();
  });

  test("es a 1440 px: la cambia en el simulador y el Estado sigue al día", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/router/conexion");
    await asentar(page);
    await pedirCambio(page, NUEVA_ROUTER);
    await page.locator("dialog[open]").getByRole("button", { name: "Cambiar la contraseña" }).click();
    const cambio = page.locator("section#router-cambiar-contrasena");
    await expect(cambio.locator(".aviso-formulario")).toContainText(
      "Contraseña del router cambiada",
      { timeout: 30_000 },
    );
    await expect(page.locator("dialog[open]")).toHaveCount(0);
    await expect(cambio.locator("h2")).toBeFocused();
    // La tarjeta de las credenciales ya tiene la nueva, también al recargar.
    const guardada = page.locator("#panel-router-contrasena");
    await expect(guardada).toHaveValue(NUEVA_ROUTER);
    await page.reload();
    await asentar(page);
    await expect(guardada).toHaveValue(NUEVA_ROUTER);
    await expect(guardada).toHaveAttribute("type", "password");

    // El Estado sigue actualizándose con la nueva: la hora de los datos
    // avanza y no hay error.
    await page.goto("/");
    await asentar(page);
    const sub = page.locator("main .pagina-sub").first();
    const antes = await sub.innerText();
    await expect(sub).not.toHaveText(antes, { timeout: 40_000 });
    await expect(page.getByText("No se pueden actualizar los datos")).toHaveCount(0);
    const salud = await (await page.request.get("/salud")).json();
    expect(salud.router).toBe("ok");

    // En el Historial, con su nombre y sin la API a la vista.
    await page.goto("/historial");
    await asentar(page);
    await expect(page.locator("main")).toContainText("Contraseña del router");
    expect(
      errores.filter((e) => !/status of 401/.test(e)),
      "errores o avisos en la consola",
    ).toEqual([]);
  });
});

// La tarjeta de las credenciales con otra dirección que llega al mismo
// simulador (localhost en lugar de 127.0.0.1) y la contraseña sin tocar:
// viaja la del campo, prueba, guarda y el panel sigue entrando. Después se
// deja como estaba.
test.describe("credenciales con otra dirección y la misma contraseña", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("prueba, guarda y vuelve a la de antes", async ({ page }) => {
    const errores = recogerErrores(page);
    await entrar(page);
    await page.goto("/router/conexion");
    await asentar(page);
    const cred = page.locator("section[aria-labelledby=router-credenciales]");
    const url = cred.locator("#panel-router-url");
    const antes = await url.inputValue();
    expect(antes).toBe(instalado.urlSim);
    const usuario = await cred.locator("#panel-router-usuario").inputValue();
    const clave = await cred.locator("#panel-router-contrasena").inputValue();
    const otra = `http://localhost:${instalado.puertoSim}`;
    try {
      for (const [destino, primera] of [
        [otra, true],
        [antes, false],
      ] as const) {
        await url.fill(destino);
        await cred.getByRole("button", { name: "Probar la conexión" }).click();
        await expect(cred).toContainText("RTF8115VW");
        await expect(cred.locator(".error-formulario")).toBeEmpty();
        await cred.getByRole("button", { name: "Guardar las credenciales" }).click();
        if (primera) {
          const dialogo = page.locator("dialog[open]");
          await expect(dialogo).toBeVisible();
          await dialogo.locator("#reautenticar-contrasena").fill(CONTRASENA);
          await dialogo.locator("button[type=submit]").click();
        }
        await expect(cred.locator(".aviso-formulario")).toHaveText(
          "Credenciales guardadas: el panel ya usa las nuevas.",
        );
        await page.reload();
        await asentar(page);
        await expect(url).toHaveValue(destino);
      }
      const salud = await (await page.request.get("/salud")).json();
      expect(salud.router).toBe("ok");
    } finally {
      // Pase lo que pase, la dirección de antes (por la API, como la página).
      const api = (fn: string, datos: object) =>
        page.request.post(`/api/${fn}`, {
          data: datos,
          headers: { origin: instalado.url, "sec-fetch-site": "same-origin" },
        });
      expect((await api("reautenticar", { contrasena: CONTRASENA })).status()).toBe(200);
      const r = await api("panel_guardar_router", { url: antes, usuario, contrasena: clave });
      expect(r.status(), await r.text()).toBe(200);
    }
    expect(
      errores.filter((e) => !/status of 401/.test(e)),
      "errores o avisos en la consola",
    ).toEqual([]);
  });
});

// Con el panel en solo lectura, la tarjeta se ve desactivada y dice por
// qué. El panel del e2e tiene un firmware probado: los datos de la página
// llegan con «solo lectura» puesto (navegación del cliente, que los pide al
// servidor).
test("es a 360 px: en solo lectura la tarjeta se ve desactivada", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 900 });
  await entrar(page);
  await asentar(page);
  await page.route(
    "**/api/leer_conexion_router",
    async (r) => {
      const real = await r.fetch();
      const texto = (await real.text()).replace('"solo_lectura":false', '"solo_lectura":true');
      expect(texto).toContain('"solo_lectura":true');
      await r.fulfill({ response: real, body: texto });
    },
    { times: 1 },
  );
  await page.locator("button.menu-boton").click();
  await page.getByRole("link", { name: "Configuración del router" }).click();
  await page.waitForURL((u) => u.pathname === "/router/conexion");
  const cambio = page.locator("section#router-cambiar-contrasena");
  await expect(cambio).toContainText("No se puede cambiar mientras el panel esté en solo lectura");
  await expect(cambio.locator("#router-contrasena-nueva")).toBeDisabled();
  await expect(cambio.getByRole("button", { name: "Cambiar la contraseña" })).toBeDisabled();
  // Y se ve apagado, como los botones desactivados.
  await expect(cambio.locator(".secreto-campo")).toHaveCSS("opacity", "0.6");
  await sinDesbordamiento(page, "solo lectura 360");
  await capturaOpcional(cambio, { path: `${SDD}/contrasena-solo-lectura-es-360.png` });
  await page.evaluate(() => window.scrollTo(0, 0));
  await capturaOpcional(page, { path: `${SDD}/contrasena-solo-lectura-pagina-es-360.png`, fullPage: true });
});
