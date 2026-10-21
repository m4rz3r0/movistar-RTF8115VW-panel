import { availableParallelism, tmpdir } from "node:os";
import { join } from "node:path";

// Paneles que arranca el e2e (cada uno con su simulador y su base de datos),
// en puertos propios: no chocan con una vista previa en 3000/3100/8091.
//
// - «instalado-k»: uno por worker de Playwright, con el usuario e2e y las
//   credenciales del router importadas del entorno (RP_ROUTER_*), como un
//   servidor que se actualiza; además, un grupo de dos pasos ya aplicado en
//   el Historial. Cada worker usa solo el suyo (el idioma de serie, la zona,
//   las reglas, el retardo del sim y el cambio en curso del gestor no se
//   comparten entre workers); dentro del worker los tests van en serie.
// - «nuevo-*»: recién instalados (sin usuarios ni router), uno por cada
//   pasada del asistente, que es de un solo uso. Solo los usa
//   instalar.spec.ts, que corre entero en un worker.
//
// Puertos: los instalados en E2E_PORT + 0..WORKERS-1 y los nuevos a
// continuación (igual con E2E_SIM_PORT para los simuladores).

const base = Number(process.env.E2E_PORT ?? 3195);
const baseSim = Number(process.env.E2E_SIM_PORT ?? 8195);

/**
 * Workers de Playwright (E2E_WORKERS; 1, en serie). Por defecto, uno por
 * cada 3 hilos de la CPU, entre 1 y 4: cada worker mueve un Chromium, un
 * panel y un simulador, y de 4 en adelante la pasada apenas gana (el camino
 * crítico son los tests largos de un mismo fichero).
 */
export const WORKERS = workers(process.env.E2E_WORKERS);

function workers(valor: string | undefined): number {
  if (valor === undefined || valor === "") {
    return Math.min(4, Math.max(1, Math.floor(availableParallelism() / 3)));
  }
  const n = Number(valor);
  if (!Number.isInteger(n) || n < 1) {
    throw new Error(`E2E_WORKERS debe ser un entero mayor que cero: ${valor}`);
  }
  return n;
}

/**
 * El worker de este proceso: Playwright pone TEST_PARALLEL_INDEX (0 ..
 * WORKERS-1; el worker que sustituye a uno caído hereda su índice) y vuelve
 * a evaluar la configuración en cada worker. En el proceso principal no
 * está: 0.
 */
const indice = Number(process.env.TEST_PARALLEL_INDEX ?? 0);

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

/** Un panel instalado por worker. */
export const instalados: Instancia[] = Array.from({ length: WORKERS }, (_, k) =>
  instancia(`instalado-${k}`, "instalado", k),
);

/** El panel instalado de este worker. */
export const instalado = instalados[indice];
if (instalado === undefined) {
  throw new Error(
    `TEST_PARALLEL_INDEX ${indice} fuera de los ${WORKERS} workers: elige cuántos con E2E_WORKERS, no con --workers`,
  );
}

/** Un panel recién instalado por cada ancho e idioma del asistente. */
export const nuevos = {
  "es-360": instancia("nuevo-es-360", "nuevo", WORKERS + 0),
  "es-1440": instancia("nuevo-es-1440", "nuevo", WORKERS + 1),
  "en-360": instancia("nuevo-en-360", "nuevo", WORKERS + 2),
  "en-1440": instancia("nuevo-en-1440", "nuevo", WORKERS + 3),
};

export const todas: Instancia[] = [...instalados, ...Object.values(nuevos)];

/** Usuario del panel «instalado». */
export const USUARIO = "e2e";
export const CONTRASENA = "e2e-contrasena";
