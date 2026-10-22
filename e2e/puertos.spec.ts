import { expect, type Locator, type Page, test } from "@playwright/test";
import { PLAZO_CAMBIO, plazoDeTest } from "./cambios";
import { asentar, entrar, recogerErrores, sinDesbordamiento } from "./util";

// Puertos abiertos (/ajustes/puertos) a 360, 768, 1024, 1280 y 1440 px, en
// español y en inglés, contra router-sim:
// - la lista de reglas, con la regla cuyo dispositivo de destino ya no está
//   en la red marcada «Sin dispositivo» (y qué hacer, en su detalle), sin
//   que nada se salga de la página a ningún ancho;
// - «Abrir un puerto» abre el detalle de una regla nueva, que dice junto al
//   campo por qué no vale (el 8080 TCP ya lo abre esa regla; un puerto del
//   dispositivo que no existe, con su plegable abierto) sin enviar nada;
// - «TCP y UDP»: el resumen con un paso «TCP» y otro «UDP», que solo enseña
//   lo que cambia.
//
// Aplicar se hace en una sola combinación (inglés, 1440 px, la última):
// cada aplicación se vigila (30 s en producción; en el e2e,
// RP_E2E_VIGILANCIA_S). Se edita la regla de las fixtures (al puerto 8081),
// se abre el 8080 TCP a host-2, aparece en la lista, se desactiva con su
// interruptor (sin diálogo), se borra desde su detalle (confirmación con el
// botón de peligro) y la regla de las fixtures vuelve a su puerto. Las
// demás combinaciones abren el resumen y lo cancelan.

const textos = {
  es: {
    titulo: "Puertos abiertos",
    abrir: "Abrir un puerto",
    nombre: "Nombre",
    protocolo: "Protocolo",
    puerto: "Puerto de internet",
    dispositivo: "Dispositivo",
    interno: "Puerto en el dispositivo",
    guardar: "Guardar",
    cancelar: "Cancelar",
    aplicar: "Aplicar",
    borrar: "Borrar",
    descartar: "Descartar los cambios",
    revisar: "Revisa el cambio",
    hecho: "Hecho: el router tiene ya el cambio.",
    sinDestino: "Sin dispositivo",
    cerrada: "Cerrada",
    queHacer: "El router no conoce ningún dispositivo",
    solapa: "Ese puerto ya está abierto en otra regla con el mismo protocolo",
    pasoIgual: "Igual que la regla TCP, pero con el protocolo UDP.",
    pasoIgualSinTcp:
      "Con los mismos datos que la regla TCP de arriba, que no se aplicará, pero con el protocolo UDP.",
    nombreNuevo: "Servidor web",
    activaWeb: "Regla Servidor web activa",
    borrarWeb: "Borrar la regla Servidor web",
    cerrar: "Cerrar el detalle",
  },
  en: {
    titulo: "Open ports",
    abrir: "Open a port",
    nombre: "Name",
    protocolo: "Protocol",
    puerto: "Internet port",
    dispositivo: "Device",
    interno: "Port on the device",
    guardar: "Save",
    cancelar: "Cancel",
    aplicar: "Apply",
    borrar: "Delete",
    descartar: "Discard the changes",
    revisar: "Review the change",
    hecho: "Done: the router now has the change.",
    sinDestino: "No device",
    cerrada: "Closed",
    queHacer: "The router doesn't know any device",
    solapa: "That port is already open in another rule with the same protocol",
    pasoIgual: "Same as the TCP rule, but with the UDP protocol.",
    pasoIgualSinTcp:
      "With the same details as the TCP rule above, which will not be applied, but with the UDP protocol.",
    nombreNuevo: "Web server",
    activaWeb: "Rule Web server active",
    borrarWeb: "Delete the rule Web server",
    cerrar: "Close the detail",
  },
} as const;

type Textos = (typeof textos)["es" | "en"];

/** La lista de reglas. */
function reglas(page: Page, t: Textos): Locator {
  return page.locator("section.lista-seccion").filter({
    has: page.getByRole("heading", { level: 2, name: t.titulo, exact: true }),
  });
}

/** La fila de la regla que contiene `texto`. */
function fila(page: Page, t: Textos, texto: string): Locator {
  return reglas(page, t).locator("tbody tr.lista-fila").filter({ hasText: texto });
}

/** Sin funciones ni parámetros de la API en un texto visible. */
function sinApi(texto: string) {
  expect(texto).not.toMatch(/\bcb[A-Z]\w+/);
  expect(texto).not.toMatch(/l4_protocol|ext_l4|int_l4|int_ip_addr|adm_state|nat_port/);
}

/** Espera a que el cambio en curso termine bien (el aviso de la página). */
async function hecho(page: Page, t: Textos) {
  const aviso = page.locator(".avisos-cambio [role=status] .aviso-cambio");
  await expect(aviso).toContainText(t.hecho, { timeout: PLAZO_CAMBIO });
}

/** Aplica el diálogo abierto con `boton` y espera a que el cambio termine. */
async function aplicar(page: Page, t: Textos, boton: string = t.aplicar) {
  const dialogo = page.locator("dialog[open]");
  await dialogo.getByRole("button", { name: boton, exact: true }).click();
  await expect(page.locator("dialog[open]")).toHaveCount(0);
  await hecho(page, t);
}

/** Abre la regla de `texto` y espera su detalle. */
async function abrir(page: Page, t: Textos, texto: string): Promise<Locator> {
  await fila(page, t, texto).locator(".lista-nombre").click();
  const detalle = page.locator(".detalle-lateral");
  await expect(detalle).toBeVisible();
  return detalle;
}

/** Edita el puerto de internet de desc-1, lo aplica y cierra su detalle. */
async function puertoDeDesc1(page: Page, t: Textos, puerto: string) {
  const detalle = await abrir(page, t, "desc-1");
  const form = detalle.locator("form.regla-formulario");
  await form.getByLabel(t.puerto, { exact: true }).fill(puerto);
  await form.getByRole("button", { name: t.guardar }).click();
  const dialogo = page.locator("dialog[open]");
  await expect(dialogo).toContainText(`${t.titulo}: desc-1`);
  // Solo cambian los puertos de internet (y los del dispositivo, que los
  // siguen): ni el nombre ni el destino.
  await expect(dialogo).not.toContainText("192.168.1.75");
  await aplicar(page, t);
  await expect(fila(page, t, "desc-1")).toContainText(puerto);
  await page.getByRole("button", { name: t.cerrar }).click();
  await expect(page.locator(".detalle-lateral")).toHaveCount(0);
}

// Cada test entra con su sesión y deja el simulador como estaba: se
// reparten entre los workers (cada uno con su panel, ver instancias.ts), y
// el fichero deja de ir entero en uno.
test.describe.configure({ mode: "parallel" });

for (const idioma of ["es", "en"] as const) {
  for (const ancho of [360, 768, 1024, 1280, 1440]) {
    const aplicaDeVerdad = idioma === "en" && ancho === 1440;
    test.describe(`puertos ${idioma} a ${ancho} px`, () => {
      test.use({
        locale: idioma === "es" ? "es-ES" : "en-GB",
        viewport: { width: ancho, height: 900 },
      });

      test("puertos abiertos", async ({ page }) => {
        // Cinco cambios: desc-1 a 8081, la regla nueva, desactivarla,
        // borrarla y desc-1 a 8080.
        if (aplicaDeVerdad) test.setTimeout(plazoDeTest(5));
        const t = textos[idioma];
        const errores = recogerErrores(page);
        await entrar(page);
        await page.goto("/ajustes/puertos");
        await asentar(page);
        await expect(reglas(page, t)).toBeVisible();
        // La regla de las fixtures va a una IP que no es de nadie.
        const desc1 = fila(page, t, "desc-1");
        await expect(desc1.locator(".lista-estado")).toContainText(t.sinDestino);
        await expect(desc1).toContainText("8080");
        await expect(desc1).toContainText("TCP");
        await sinDesbordamiento(page, `puertos ${idioma} ${ancho}`);
        sinApi(await reglas(page, t).innerText());
        await page.screenshot({ path: `capturas/puertos-${idioma}-${ancho}.png`, fullPage: true });
        // Su detalle dice qué hacer.
        const detalle1 = await abrir(page, t, "desc-1");
        await expect(detalle1).toContainText(t.queHacer);
        await page.keyboard.press("Escape");
        await expect(page.locator(".detalle-lateral")).toHaveCount(0);

        // 8080 TCP a host-2: ya lo abre desc-1. El error, junto al puerto,
        // sin enviar.
        await reglas(page, t).getByRole("button", { name: t.abrir }).click();
        await page.waitForURL((u) => u.pathname === "/ajustes/puertos/reglas");
        const form = page.locator(".detalle-lateral form.regla-formulario");
        await expect(page.locator("#detalle-titulo")).toBeFocused();
        await form.getByLabel(t.nombre, { exact: true }).fill(t.nombreNuevo);
        await form.getByLabel(t.protocolo, { exact: true }).selectOption("tcp");
        const puerto = form.getByLabel(t.puerto, { exact: true });
        await puerto.fill("8080");
        await form.getByLabel(t.dispositivo, { exact: true }).selectOption("192.168.1.62");
        let preparado = false;
        page.on("request", (r) => {
          if (r.url().endsWith("/api/preparar_ajuste")) preparado = true;
        });
        const guardar = form.getByRole("button", { name: t.guardar });
        await guardar.click();
        await expect(form.getByText(t.solapa)).toBeVisible();
        await expect(puerto).toBeFocused();
        await expect(puerto).toHaveAttribute("aria-invalid", "true");
        expect(preparado).toBe(false);
        await sinDesbordamiento(page, `formulario ${idioma} ${ancho}`);
        sinApi(await form.innerText());

        if (aplicaDeVerdad) {
          // «Cancelar» con cambios sin guardar pregunta antes; se descartan.
          await form.getByRole("button", { name: t.cancelar }).click();
          await page.locator("dialog[open]").getByRole("button", { name: t.descartar }).click();
          await page.waitForURL((u) => u.pathname === "/ajustes/puertos");
          // desc-1 pasa al 8081 (deja libre el 8080).
          await puertoDeDesc1(page, t, "8081");
          // Ahora sí: 8080 TCP a host-2.
          await reglas(page, t).getByRole("button", { name: t.abrir }).click();
          await form.getByLabel(t.nombre, { exact: true }).fill(t.nombreNuevo);
          await form.getByLabel(t.puerto, { exact: true }).fill("8080");
          await form.getByLabel(t.dispositivo, { exact: true }).selectOption("192.168.1.62");
          await form.getByRole("button", { name: t.guardar }).click();
          const dialogo = page.locator("dialog[open]");
          await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
          await expect(dialogo).toContainText(`${t.titulo}: ${t.nombreNuevo}`);
          await expect(dialogo.getByText("192.168.1.62")).toBeVisible();
          sinApi(await dialogo.innerText());
          await aplicar(page, t);
          // La regla nueva, en la lista (ya releída), hacia host-2 y sin aviso.
          const nueva = fila(page, t, t.nombreNuevo);
          await expect(nueva).toHaveCount(1);
          await page.getByRole("button", { name: t.cerrar }).click();
          await expect(page.locator(".detalle-lateral")).toHaveCount(0);
          await expect(nueva).toContainText("8080");
          await expect(nueva).toContainText("host-2");
          await expect(nueva.locator(".lista-estado")).not.toContainText(t.sinDestino);

          // Su interruptor se aplica sin diálogo.
          const activa = nueva.getByRole("switch", { name: t.activaWeb });
          await expect(activa).toHaveAttribute("aria-checked", "true");
          await activa.click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
          await hecho(page, t);
          await expect(fila(page, t, t.nombreNuevo).locator(".lista-estado")).toContainText(
            t.cerrada,
          );

          // Borrarla desde su detalle (confirmación con el botón de
          // peligro) y dejar desc-1 como estaba.
          const detalle = await abrir(page, t, t.nombreNuevo);
          await detalle.getByRole("button", { name: t.borrarWeb }).click();
          await expect(page.locator("dialog[open]")).toBeVisible();
          await expect(
            page.locator("dialog[open]").getByRole("button", { name: t.borrar, exact: true }),
          ).toHaveClass(/b-pel/);
          await aplicar(page, t, t.borrar);
          await expect(fila(page, t, t.nombreNuevo)).toHaveCount(0);
          await puertoDeDesc1(page, t, "8080");
        } else {
          // Un puerto del dispositivo que no existe, con su plegable
          // cerrado: el plegable se abre para enseñar el error.
          await puerto.fill("9000");
          const plegable = form.locator("details.regla-interno");
          await plegable.locator("summary").click();
          const interno = form.getByLabel(t.interno, { exact: true });
          await interno.fill("70000");
          await plegable.locator("summary").click();
          await expect(plegable).not.toHaveAttribute("open", "");
          await guardar.click();
          await expect(plegable).toHaveAttribute("open", "");
          await expect(interno).toBeFocused();
          await expect(interno).toHaveAttribute("aria-invalid", "true");
          expect(preparado).toBe(false);
          await interno.fill("");

          // «TCP y UDP»: un paso «TCP» y otro «UDP» que solo enseña lo
          // que cambia.
          await form.getByLabel(t.protocolo, { exact: true }).selectOption("ambos");
          await guardar.click();
          const dialogo = page.locator("dialog[open]");
          await expect(dialogo).toBeVisible();
          await expect(dialogo.locator("#grupo-titulo")).toHaveText(t.revisar);
          await expect(dialogo).toContainText(`${t.titulo}: ${t.nombreNuevo}`);
          const pasos = dialogo.locator("fieldset.grupo-paso");
          await expect(pasos).toHaveCount(2);
          await expect(pasos.nth(0).locator("legend label")).toHaveText("TCP");
          await expect(pasos.nth(1).locator("legend label")).toHaveText("UDP");
          await expect(pasos.nth(1)).toContainText(t.pasoIgual);
          // Sin el paso TCP, la nota del UDP lo dice; al volver a marcarlo,
          // como antes.
          const tcp = pasos.nth(0).locator("legend input[type=checkbox]");
          await tcp.uncheck();
          await expect(pasos.nth(1).locator(".grupo-paso-nota")).toHaveText(t.pasoIgualSinTcp);
          await tcp.check();
          await expect(pasos.nth(1).locator(".grupo-paso-nota")).toHaveText(t.pasoIgual);
          await expect(pasos.nth(1).locator("tbody tr")).toHaveCount(1);
          await expect(pasos.nth(0).getByText("192.168.1.62")).toBeVisible();
          sinApi(await dialogo.innerText());
          await sinDesbordamiento(page, `diálogo ${idioma} ${ancho}`);
          await page.screenshot({ path: `capturas/puertos-dialogo-${idioma}-${ancho}.png` });
          await dialogo.getByRole("button", { name: t.cancelar }).click();
          await expect(page.locator("dialog[open]")).toHaveCount(0);
        }
        expect(errores).toEqual([]);
      });
    });
  }
}
