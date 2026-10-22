import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Páginas de Ajustes con las tarjetas de siempre (/ajustes/<tema>; las que
// van en rejilla las prueban ajustes-wifi.spec.ts y ajustes-temas.spec.ts)
// a 360 y 1440 px, en español y en inglés, contra router-sim: abrir Red
// local, «Más opciones», cambiar la duración de las direcciones del DHCP,
// ver el resumen «antes → después» sin nombres de la API y, en móvil, el
// selector de temas. El diálogo: el foco
// entra en su título y vuelve a «Guardar»; Escape no lo cierra mientras
// aplica ni mientras prepara el deshacer. «Cancelar» de la tarjeta deja los
// controles (un <select> tocado varias veces) como los leyó la página.
//
// Aplicar, esperar «aplicado» y deshacer se hace en una sola combinación
// (inglés, 1440 px): cada aplicación se vigila (2 min en producción; en el
// e2e, RP_E2E_VIGILANCIA_S, 10 s por defecto, ver instancias.ts), y los
// plazos salen de ella (cambios.ts). Las demás llegan
// hasta el resumen y lo cancelan.
//
// Cada test entra con su propia sesión y deja el panel como estaba, así que
// se reparten entre los workers (cada uno con su panel, ver instancias.ts):
// este fichero es el más largo y, entero en un worker, sería el camino
// crítico de la pasada.
test.describe.configure({ mode: "parallel" });

const textos = {
  es: {
    dhcp: "Reparto de direcciones (DHCP)",
    campo: "Duración de cada dirección",
    primero: "Reparto automático",
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
    puertos: "Puertos",
  },
  en: {
    dhcp: "Address handout (DHCP)",
    campo: "Duration of each address",
    primero: "Automatic handout",
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
  expect(texto).not.toMatch(/lease_time|Dhcpv4/);
}

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 1440]) {
    const aplicaDeVerdad = idioma === "en" && ancho === 1440;
    test.describe(`ajustes ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("Red local: cambiar la duración de las direcciones", async ({ page }) => {
        // Aplica y deshace: dos cambios.
        if (aplicaDeVerdad) test.setTimeout(plazoDeTest(2));
        const t = textos[idioma];
        const errores = recogerErrores(page);
        await entrar(page);

        // 1. Abrir Red local (en móvil, con el selector de temas) y «Más
        // opciones», donde está el DHCP.
        if (ancho < 640) {
          await page.goto("/ajustes/puertos");
          await asentar(page);
          await expect(page.locator("nav.temas")).toBeHidden();
          await page.locator("#ajustes-tema").selectOption("red_local");
          await page.waitForURL((u) => u.pathname === "/ajustes/red_local");
        } else {
          await page.goto("/ajustes/red_local");
          await asentar(page);
          await expect(page.locator("#ajustes-tema")).toBeHidden();
          await expect(page.locator("nav.temas a[aria-current=page]")).toHaveAttribute(
            "href",
            "/ajustes/red_local",
          );
        }
        const dhcp = tarjeta(page, t.dhcp);
        const mas = page.locator("details.mas-opciones");
        await expect(dhcp).toBeHidden();
        await mas.locator(":scope > summary").click();
        await expect(dhcp).toBeVisible();
        await asentar(page);
        await sinDesbordamiento(page, `red local ${idioma} ${ancho}`);
        await page.screenshot({
          path: `capturas/ajustes-red-local-${idioma}-${ancho}.png`,
          fullPage: true,
        });
        const fila = dhcp
          .locator(".ajuste-resumen .ajuste-fila")
          .filter({ has: page.getByText(t.campo, { exact: true }) })
          // El valor (sin la marca de lo recomendado, que va detrás).
          .locator("dd > span")
          .first();
        const antes = (await fila.innerText()).trim();
        expect(antes.length).toBeGreaterThan(0);

        // 2. Cambiarla a la otra opción.
        await dhcp.getByRole("button", { name: t.editar }).click();
        // El foco, al primer campo (el reparto, encendido o no).
        await expect(dhcp.getByRole("switch", { name: t.primero, exact: true })).toBeFocused();
        const campo = dhcp.getByLabel(t.campo, { exact: true });
        const leida = await campo.inputValue();
        const otra = leida === "doce_horas" ? "dia" : "doce_horas";
        await campo.selectOption(otra);
        const despues = (await campo.locator("option:checked").innerText()).trim();
        const guardar = dhcp.getByRole("button", { name: t.guardar });
        await guardar.click();

        // 3. El resumen: «antes → después», sin nombres de la API; el foco
        // en su título.
        const dialogo = page.locator("dialog[open]");
        await expect(dialogo).toBeVisible();
        await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
        await expect(dialogo.locator("#grupo-titulo")).toBeFocused();
        const cambio = dialogo.locator("tbody tr");
        await expect(cambio).toHaveCount(1);
        await expect(cambio.locator("th")).toHaveText(t.campo);
        await expect(cambio.locator("td").nth(0)).toHaveText(antes);
        await expect(cambio.locator("td").nth(1)).toHaveText(despues);
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
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: PLAZO_CAMBIO });
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
          await expect(dialogo.locator("tbody td").nth(0)).toHaveText(despues);
          await expect(dialogo.locator("tbody td").nth(1)).toHaveText(antes);
          await dialogo.getByRole("button", { name: t.deshacer }).click();
          await expect(dialogo.getByText(t.hecho)).toBeVisible({ timeout: PLAZO_CAMBIO });
          await dialogo.getByRole("button", { name: t.cerrar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          // La página relee el tema y el foco va al título de la tarjeta (eso
          // prueba la relectura: el valor de antes ya estaba antes de aplicar).
          await expect(fila).toHaveText(antes);
          await expect(dhcp.getByRole("heading", { level: 2 })).toBeFocused();
        } else {
          // Cancelar: el foco vuelve a «Guardar» y nada se escribe.
          await dialogo.getByRole("button", { name: t.cancelar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          await expect(guardar).toBeFocused();
          // El selector, tocado varias veces.
          await campo.selectOption(leida);
          await campo.selectOption(otra);
          // «Cancelar» de la tarjeta: al volver a editar, el control tiene lo
          // que leyó la página.
          await dhcp.getByRole("button", { name: t.cancelar }).click();
          await dhcp.getByRole("button", { name: t.editar }).click();
          await expect(campo).toHaveValue(leida);
          await dhcp.getByRole("button", { name: t.cancelar }).click();
        }

        // 5. «Más opciones» se cierra.
        await mas.locator(":scope > summary").click();
        await expect(dhcp).toBeHidden();

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
