//! Control del simulador desde las pruebas (`SimControl`).

use crate::LAN_PORT;
use crate::estado::{HangAfterWrite, Inner};
use crate::lecturas::{read_node, valor_de};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

/// Control del simulador desde las pruebas. Barato de clonar.
#[derive(Clone)]
pub struct SimControl {
    pub(crate) inner: Arc<Inner>,
}

impl SimControl {
    /// Si `true`, el router deja de responder (las peticiones quedan colgadas).
    pub fn set_unresponsive(&self, on: bool) {
        self.inner.unresponsive.store(on, Ordering::SeqCst);
    }

    /// Retarda cada respuesta `retardo` (cero, sin retardo), como la latencia
    /// del router real. También se cambia por HTTP: `POST /__sim/retardo`
    /// con los milisegundos en el cuerpo (lo usa el e2e).
    pub fn set_retardo(&self, retardo: Duration) {
        let ms = u64::try_from(retardo.as_millis()).unwrap_or(u64::MAX);
        self.inner.retardo_ms.store(ms, Ordering::SeqCst);
    }

    /// Caduca todas las sesiones (básicas y avanzadas).
    pub fn invalidate_sessions(&self) {
        self.inner.state().invalidate_sessions();
    }

    /// Cambia el `opr_status` en `cbGetWanIPIntf` de la conexión a internet
    /// de las fixtures (la de la ruta por defecto; `ip2` en las de serie): 1
    /// arriba, 0 abajo.
    pub fn set_wan_up(&self, up: bool) {
        self.inner.state().wan_up = up;
    }

    /// La próxima escritura responde `Result=1` con este `ErrorString`.
    ///
    /// Una llamada con `validation_only=1` (la validación previa que hace la
    /// página WiFi) también lo consume, como cuando el router rechaza la
    /// validación.
    pub fn fail_next_write(&self, err: &str) {
        self.inner.state().fail_next = Some(err.to_owned());
    }

    /// La próxima escritura real responde OK pero no se aplica.
    ///
    /// Las llamadas con `validation_only=1` no lo consumen: nunca aplican
    /// nada, así que el efecto recae en la escritura real que las sigue.
    pub fn ignore_next_write(&self) {
        self.inner.state().ignore_next = true;
    }

    /// Si `on`, la lectura `name` (p. ej. `cbGetTr69Config`) responde
    /// `Result=1` hasta que se desactive.
    pub fn fail_reads(&self, name: &str, on: bool) {
        let mut st = self.inner.state();
        if on {
            st.failing_reads.insert(name.to_owned());
        } else {
            st.failing_reads.remove(name);
        }
    }

    /// Si `on`, la lectura `name` con el parámetro `param=valor` (p. ej. el
    /// punto de acceso 1 de `cbGetWifiAccPt`) responde `Result=1` hasta que
    /// se desactive; con otros parámetros responde como siempre.
    pub fn fail_reads_con(&self, name: &str, param: &str, valor: &str, on: bool) {
        let mut st = self.inner.state();
        let clave = (name.to_owned(), param.to_owned(), valor.to_owned());
        if on {
            st.failing_reads_con.insert(clave);
        } else {
            st.failing_reads_con.remove(&clave);
        }
    }

    /// Tras la próxima escritura real (no de solo validación), la lectura
    /// `name` empieza a fallar como con [`SimControl::fail_reads`] hasta que
    /// se desactive con `fail_reads(name, false)`.
    pub fn fail_reads_after_next_write(&self, name: &str) {
        self.inner.state().fail_reads_after_write = Some(name.to_owned());
    }

    /// La próxima escritura real no recibe respuesta (la petición se queda
    /// colgada; el cliente agota su tiempo de espera). Con `aplicar`, el
    /// router la aplica antes (p. ej. reinicia un servicio y no contesta);
    /// sin él, no aplica nada (se cortó antes de llegar). Las peticiones
    /// siguientes se responden con normalidad.
    pub fn no_answer_next_write(&self, aplicar: bool) {
        let mut st = self.inner.state();
        st.no_answer_next = true;
        if !aplicar {
            st.ignore_next = true;
        }
    }

    /// La próxima reserva DHCP nueva (`cbDhcpV4SvrPoolSetStatic` con id -1)
    /// se crea dos veces (dos ids nuevos con los mismos valores).
    pub fn create_twice_next_write(&self) {
        self.inner.state().create_twice = true;
    }

    /// Tras responder a la próxima escritura real, el router deja de
    /// responder (como [`SimControl::set_unresponsive`]) hasta que se llame a
    /// `set_unresponsive(false)`.
    pub fn hang_after_next_write(&self) {
        self.inner.state().hang_after_write = HangAfterWrite::Armed;
    }

    /// La escritura real (no de solo validación) número `n` contando desde
    /// ahora (1: la próxima) responde `Result=1` y no se aplica. Se puede
    /// armar varias veces para que fallen varias.
    pub fn fail_write_n(&self, n: usize) {
        let mut st = self.inner.state();
        let objetivo = st.real_writes + n;
        st.fail_write_at.insert(objetivo);
    }

    /// Como [`SimControl::hang_after_next_write`], pero con la escritura real
    /// número `n` contando desde ahora (1: la próxima).
    pub fn hang_after_write_n(&self, n: usize) {
        let mut st = self.inner.state();
        st.hang_at_write = Some(st.real_writes + n);
    }

    /// Tráfico de internet de un equipo en el puerto LAN 4: `rx` bytes de
    /// bajada y `tx` de subida. En `cbGetStatistic` suben la conexión a
    /// internet (rx, tx; `ip2` en las fixtures de serie) y, del lado LAN,
    /// `eth0.4` al revés (lo que baja sale por el puerto: tx). El router real
    /// apenas los cuenta en la WAN (reenvío por hardware); el panel mide en
    /// los puertos LAN.
    pub fn counters_bump(&self, rx: u64, tx: u64) {
        if let Some(wan) = &self.inner.wan {
            self.counters_bump_iface(wan, rx, tx);
        }
        self.counters_bump_iface(LAN_PORT, tx, rx);
    }

    /// Suma bytes a `rx_byte`/`tx_byte` de la interfaz `name` de
    /// `cbGetStatistic`.
    pub fn counters_bump_iface(&self, name: &str, rx: u64, tx: u64) {
        let mut st = self.inner.state();
        let e = st.counters_extra.entry(name.to_owned()).or_default();
        e.0 = e.0.saturating_add(rx);
        e.1 = e.1.saturating_add(tx);
    }

    /// Cambia la contraseña que acepta el login (las sesiones abiertas
    /// siguen valiendo).
    pub fn set_password(&self, password: &str) {
        self.inner.fijar_password(password.to_owned());
    }

    /// El próximo `cbModifyPwd` responde OK sin cambiar nada (el router lo
    /// acepta pero la contraseña sigue siendo la anterior).
    pub fn modify_pwd_sin_efecto(&self) {
        self.inner.state().modify_pwd_sin_efecto = true;
    }

    /// El próximo `cbModifyPwd` responde OK, pero el router tarda
    /// `retraso` en aplicarlo: hasta entonces sigue entrando la anterior y
    /// la nueva no. Lo aplica el primer inicio de sesión después.
    pub fn modify_pwd_con_retraso(&self, retraso: Duration) {
        self.inner.state().modify_pwd_retraso = Some(retraso);
    }

    /// El próximo `cbModifyPwd` responde OK, pero los `inicios` inicios de
    /// sesión básicos siguientes aún no ven la nueva (entra la anterior); la
    /// aplica el que va después. Para probar sin carreras un router que la
    /// aplica justo al acabar el plazo del panel. Manda sobre
    /// `modify_pwd_con_retraso`.
    pub fn modify_pwd_tras_inicios(&self, inicios: u32) {
        self.inner.state().modify_pwd_tras_inicios = Some(inicios);
    }

    /// El inicio de sesión (básico y avanzado) con `password` falla aunque
    /// sea la vigente: simula un formato que el router no acepta al entrar.
    pub fn login_falla_con(&self, password: &str) {
        self.inner.state().login_falla_con = Some(password.to_owned());
    }

    /// Valor actual de `campo` en la lectura `lectura` (sin parámetros), con
    /// lo escrito hasta ahora: el primer nodo que lo tiene. Para las pruebas
    /// que buscan secretos en el HTML.
    pub fn fixture_valor(&self, lectura: &str, campo: &str) -> Option<String> {
        let st = self.inner.state();
        let nodo = read_node(&self.inner, &st, lectura, &[]).ok()?;
        valor_de(&nodo, campo)
    }

    /// Usuarios de la web del router como `(nombre, papel)`, en el orden de
    /// `cbGetUserList` (con lo escrito hasta ahora).
    pub fn usuarios(&self) -> Vec<(String, String)> {
        self.inner
            .usuarios()
            .iter()
            .map(|u| (u.nombre.clone(), u.grupo.clone()))
            .collect()
    }

    /// Contraseña actual del usuario `nombre` (`None` si no existe).
    pub fn contrasena_de(&self, nombre: &str) -> Option<String> {
        self.inner
            .usuarios()
            .iter()
            .find(|u| u.nombre == nombre)
            .map(|u| u.contrasena.clone())
    }

    /// Cambia la versión de firmware (`SWVersion`) que da `cbGetSummary`.
    pub fn set_firmware(&self, firmware: &str) {
        self.inner.state().firmware = Some(firmware.to_owned());
    }

    /// Escrituras recibidas con `sessionKey` válido (sin él), en orden. No
    /// incluye las llamadas de solo validación (`validation_only=1`). Las de
    /// la UI básica (`GET /cgi-bin/<x>.cmd|.cgi|.wl?…`) aparecen con el
    /// nombre del fichero y los parámetros de la URL (incluido `action`).
    pub fn writes(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.inner.state().writes.clone()
    }

    /// Lecturas avanzadas (`cb*.xml`) recibidas con sesión válida, con sus
    /// parámetros, en orden; también las que fallan con
    /// [`SimControl::fail_reads`].
    pub fn reads(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.inner.state().reads.clone()
    }
}
