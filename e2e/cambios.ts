import { expect, type Page } from "@playwright/test";
import { VIGILANCIA_S } from "./instancias";

// Cambios en el router (el simulador) por las server functions, para las
// pruebas que necesitan uno en curso o recién aplicado sin pasar por la
// interfaz.

/**
 * Lo que puede tardar un cambio hasta «hecho»: su vigilancia (la de riesgo
 * alto es el doble de VIGILANCIA_S) y 30 s para escribir y releer.
 */
export const PLAZO_CAMBIO = (2 * VIGILANCIA_S + 30) * 1_000;

/** Plazo de un test que aplica `cambios` cambios: 1 min para lo demás y PLAZO_CAMBIO por cambio. */
export function plazoDeTest(cambios: number): number {
  return 60_000 + cambios * PLAZO_CAMBIO;
}

/** Llama a la server function `nombre` con la sesión de la página. */
async function api(page: Page, nombre: string, cuerpo: unknown) {
  const r = await page.request.post(`/api/${nombre}`, { data: cuerpo });
  expect(r.status(), `${nombre}: ${await r.text()}`).toBe(200);
  return r.json();
}

/** Un ajuste de la página de un tema (`leer_tema`), esté en el elemento que esté. */
type AjusteLeido = { id: string; campos: { id: string; valor: Record<string, unknown> }[] };

/** El ajuste `id` de la página de un tema leída con `leer_tema`. */
export function ajusteDe(pagina: { elementos: { ajustes: AjusteLeido[] }[] }, id: string): AjusteLeido {
  const a = pagina.elementos.flatMap((e) => e.ajustes).find((x) => x.id === id);
  if (!a) throw new Error(`falta el ajuste ${id}`);
  return a;
}

/**
 * Cambia la apertura automática de puertos (UPnP) al valor contrario: un
 * cambio de riesgo bajo, que se vigila RP_E2E_VIGILANCIA_S. Devuelve su id.
 * Otra llamada lo deja como estaba.
 */
export async function cambiarUpnp(page: Page): Promise<number> {
  const tema = await api(page, "leer_tema", { tema: "puertos" });
  const upnp = ajusteDe(tema, "puertos.upnp");
  const activo = upnp.campos.find((c) => c.id === "activo")?.valor.Booleano;
  const grupo = await api(page, "preparar_ajuste", {
    p: { ajuste: "puertos.upnp", instancia: null, borrar: false, valores: { activo: String(!activo) } },
  });
  return api(page, "confirmar_ajuste", { token: grupo.token, desmarcados: [] });
}

/** Estado del cambio `id` («Vigilando», «Aplicado»…). */
export async function estadoCambio(page: Page, id: number): Promise<string> {
  const r = await page.request.post("/api/estado_cambio", { data: { id } });
  return (await r.json()).estado;
}

/** Espera a que el cambio `id` termine. */
export async function esperarFinal(page: Page, id: number) {
  await expect
    .poll(() => estadoCambio(page, id), { timeout: PLAZO_CAMBIO, intervals: [1_000] })
    .toBe("Aplicado");
}

/**
 * Abre el puerto `puerto` (TCP) hacia host-2 con una regla nueva llamada
 * `nombre`, sin pasar por la interfaz. Devuelve el id del cambio.
 */
export async function crearRegla(page: Page, nombre: string, puerto: number): Promise<number> {
  const p = String(puerto);
  const grupo = await api(page, "preparar_ajuste", {
    p: {
      ajuste: "puertos.reglas",
      instancia: null,
      borrar: false,
      valores: {
        nombre,
        protocolo: "tcp",
        externo_desde: p,
        externo_hasta: p,
        destino: "192.168.1.62",
        interno_desde: p,
        interno_hasta: p,
        activa: "true",
      },
      esperados: {},
    },
  });
  return api(page, "confirmar_ajuste", { token: grupo.token, desmarcados: [] });
}
