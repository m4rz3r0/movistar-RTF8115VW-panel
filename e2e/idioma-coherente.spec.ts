import { expect, test, type Browser, type BrowserContext, type Page } from "@playwright/test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { CONTRASENA, USUARIO, instalado } from "./instancias";
import { asentar, recogerErrores, sinDesbordamiento } from "./util";

// Una página en español no lleva ningún texto en inglés: ni al pintarla el
// servidor (sin JavaScript), ni tras hidratar, ni después de hablar con el
// servidor (errores, diálogo de reautenticación). Se compara el texto
// visible (y aria-label, title, placeholder) con los textos ingleses de los
// .ftl que no existen en español.

const raiz = join(__dirname, "..", "crates", "i18n", "locales");

function mensajes(idioma: string): Map<string, string> {
  const m = new Map<string, string>();
  for (const f of readdirSync(join(raiz, idioma))) {
    for (const linea of readFileSync(join(raiz, idioma, f), "utf8").split("\n")) {
      const r = /^([a-z0-9-]+)\s*=\s*(.+)$/.exec(linea);
      if (r) m.set(r[1], r[2].trim());
    }
  }
  return m;
}

/** Trozos fijos (sin `{ $x }`) de los textos ingleses que no salen en español. */
const ingleses: { clave: string; re: RegExp; texto: string }[] = (() => {
  const es = mensajes("es");
  const en = mensajes("en");
  const todoEs = [...es.values()].join("\n");
  const out: { clave: string; re: RegExp; texto: string }[] = [];
  for (const [clave, valor] of en) {
    if (es.get(clave) === valor) continue;
    for (const trozo of valor.split(/\{[^}]*\}/)) {
      const t = trozo.trim().replace(/[:.,…]+$/, "").trim();
      if (t.length < 4 || !/[a-z]{3}/i.test(t) || todoEs.includes(t)) continue;
      const escapado = t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      out.push({ clave, texto: t, re: new RegExp(`(^|[^\\p{L}])${escapado}($|[^\\p{L}])`, "u") });
    }
  }
  return out;
})();

/** Textos de la página (sin los marcados con otro `lang`, como «English»). */
async function textos(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const out: string[] = [];
    const lang = document.documentElement.lang;
    const otroIdioma = (el: Element | null) => {
      const l = el?.closest("[lang]")?.getAttribute("lang");
      return l !== undefined && l !== null && l !== lang;
    };
    const w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    while (w.nextNode()) {
      const p = w.currentNode.parentElement;
      if (!p || p.closest("script,style,template") || otroIdioma(p)) continue;
      const t = (w.currentNode.textContent ?? "").trim();
      if (t) out.push(t);
    }
    for (const el of document.body.querySelectorAll("[aria-label],[title],[placeholder]")) {
      if (otroIdioma(el)) continue;
      for (const a of ["aria-label", "title", "placeholder"]) {
        const v = el.getAttribute(a);
        if (v) out.push(v);
      }
    }
    return out;
  });
}

/** Falla si hay algún texto inglés en la página, que debe estar en español. */
async function sinIngles(page: Page, donde: string) {
  await expect(page.locator("html")).toHaveAttribute("lang", "es");
  const encontrados = new Set<string>();
  for (const t of await textos(page)) {
    for (const i of ingleses) if (i.re.test(t)) encontrados.add(`${i.clave}: «${t}»`);
  }
  expect([...encontrados], `${donde}: textos en inglés`).toEqual([]);
}

async function entrarCon(page: Page) {
  await page.goto("/login");
  await asentar(page);
  await page.locator("#usuario").fill(USUARIO);
  await page.locator("#contrasena").fill(CONTRASENA);
  await page.locator("form button[type=submit]").click();
  await page.waitForURL((url) => url.pathname === "/");
}

/** La misma página pintada solo por el servidor (sin JavaScript). */
async function sinJs(browser: Browser, ctx: BrowserContext, locale: string, ruta: string) {
  const otro = await browser.newContext({ locale, javaScriptEnabled: false, baseURL: instalado.url });
  await otro.addCookies(await ctx.cookies());
  const p = await otro.newPage();
  await p.goto(ruta);
  return { p, cerrar: () => otro.close() };
}

const rutas = ["/", "/historial", "/explorador", "/explorador/lan_dhcp", "/panel", "/router/conexion"];

const combinaciones = [
  { nombre: "cookie es y navegador en inglés", locale: "en-US", cookie: "es" },
  { nombre: "sin cookie y navegador en español", locale: "es-ES", cookie: null },
];

test.describe("idioma coherente", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  for (const c of combinaciones) {
    test(`sin inglés en español: ${c.nombre}`, async ({ browser }) => {
      const ctx = await browser.newContext({ locale: c.locale, baseURL: instalado.url });
      if (c.cookie) await ctx.addCookies([{ name: "rp_lang", value: c.cookie, url: instalado.url }]);
      const page = await ctx.newPage();
      const errores = recogerErrores(page);
      await entrarCon(page);
      for (const ruta of rutas) {
        const { p, cerrar } = await sinJs(browser, ctx, c.locale, ruta);
        await sinIngles(p, `${ruta} (servidor)`);
        await cerrar();
        await page.goto(ruta);
        await asentar(page);
        await sinIngles(page, `${ruta} (hidratada)`);
      }

      // Tras hablar con el servidor: error de validación…
      await page.goto("/panel");
      await asentar(page);
      await page.locator("#panel-dns").fill("no-es-una-ip");
      await page.locator("section[aria-labelledby=panel-privacidad] button[type=submit]").click();
      await expect(page.locator("section[aria-labelledby=panel-privacidad] .error-formulario")).not.toBeEmpty();
      await sinIngles(page, "/panel con error");

      // …y diálogo de reautenticación con contraseña mala.
      await page.goto("/router/conexion");
      await asentar(page);
      await page.locator("#panel-router-contrasena").fill("sim");
      await page.locator("form.ajustes-formulario button[type=submit]").click();
      const dialogo = page.locator("dialog[open]");
      await expect(dialogo).toBeVisible();
      await sinIngles(page, "diálogo de reautenticación");
      await dialogo.locator("#reautenticar-contrasena").fill("no-es-la-buena");
      await dialogo.locator("button[type=submit]").click();
      await expect(dialogo.locator(".error-formulario")).not.toBeEmpty();
      await sinIngles(page, "reautenticación fallida");
      await dialogo.getByRole("button", { name: "Cancelar" }).click();
      expect(
        errores.filter((e) => !/status of (400|401|403)/.test(e)),
        "errores o avisos en la consola",
      ).toEqual([]);
      await ctx.close();
    });
  }

  // El fallo de la v0.2.1: con el navegador en inglés y sin idioma por
  // defecto, «Ajustes del panel» decía «Idioma por defecto: Español»; al
  // guardar «Español», la página seguía en inglés (y los errores del
  // servidor ya llegaban en español).
  test("idioma por defecto: sin fijar es el del navegador y al cambiarlo la página lo sigue", async ({
    browser,
  }) => {
    const ctx = await browser.newContext({ locale: "en-US", baseURL: instalado.url });
    const page = await ctx.newPage();
    const errores = recogerErrores(page);
    await entrarCon(page);
    await page.goto("/panel");
    await asentar(page);
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    const selector = page.locator("#panel-idioma");
    await expect(selector).toHaveValue("navegador");
    try {
      await selector.selectOption("es");
      await Promise.all([
        page.waitForEvent("load"),
        page.locator("section[aria-labelledby=panel-panel] button[type=submit]").click(),
      ]);
      await asentar(page);
      await expect(page.locator("main h1")).toHaveText("Ajustes del panel");
      await expect(page.locator("#panel-idioma")).toHaveValue("es");
      await sinIngles(page, "/panel tras fijar el español");
      // Navegar sin recargar también sigue en español.
      await page.getByRole("navigation", { name: "Principal" }).getByRole("link", { name: "Estado" }).click();
      await expect(page.locator("main h1").first()).toHaveText("Estado");
      await sinIngles(page, "/ tras navegar");
      await sinDesbordamiento(page, "estado");
    } finally {
      // Deja la instancia como estaba (otras pruebas la usan).
      await page.goto("/panel");
      await asentar(page);
      await page.locator("#panel-idioma").selectOption("navegador");
      await Promise.all([
        page.waitForEvent("load"),
        page.locator("section[aria-labelledby=panel-panel] button[type=submit]").click(),
      ]);
    }
    await asentar(page);
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    expect(errores, "errores o avisos en la consola").toEqual([]);
    await ctx.close();
  });
});
