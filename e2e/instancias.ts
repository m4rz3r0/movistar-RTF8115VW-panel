import { tmpdir } from "node:os";
import { join } from "node:path";

// Paneles que arranca el e2e (cada uno con su simulador y su base de datos),
// en puertos propios: no chocan con una vista previa en 3000/3100/8091.
//
// - «instalado»: con el usuario e2e y las credenciales del router importadas
//   del entorno (RP_ROUTER_*), como un servidor que se actualiza; además, un
//   grupo de dos pasos ya aplicado en el Historial.
// - «nuevo-*»: recién instalados (sin usuarios ni router), uno por cada
//   pasada del asistente, que es de un solo uso.

const base = Number(process.env.E2E_PORT ?? 3195);
const baseSim = Number(process.env.E2E_SIM_PORT ?? 8195);

export interface Instancia {
  nombre: string;
  modo: "instalado" | "nuevo";
  puerto: number;
  puertoSim: number;
  /** Carpeta de la base de datos (y del token del asistente). */
  dir: string;
  /** Origen del panel (localhost: Chromium acepta allí la cookie Secure). */
  url: string;
  urlSim: string;
}

function instancia(nombre: string, modo: Instancia["modo"], n: number): Instancia {
  const puerto = base + n;
  const puertoSim = baseSim + n;
  return {
    nombre,
    modo,
    puerto,
    puertoSim,
    dir: join(tmpdir(), `routerpanel-e2e-${puerto}`),
    url: `http://localhost:${puerto}`,
    urlSim: `http://127.0.0.1:${puertoSim}`,
  };
}

export const instalado = instancia("instalado", "instalado", 0);

/** Un panel recién instalado por cada ancho e idioma del asistente. */
export const nuevos = {
  "es-360": instancia("nuevo-es-360", "nuevo", 1),
  "es-1440": instancia("nuevo-es-1440", "nuevo", 2),
  "en-360": instancia("nuevo-en-360", "nuevo", 3),
  "en-1440": instancia("nuevo-en-1440", "nuevo", 4),
};

export const todas: Instancia[] = [instalado, ...Object.values(nuevos)];

/** Usuario del panel «instalado». */
export const USUARIO = "e2e";
export const CONTRASENA = "e2e-contrasena";
