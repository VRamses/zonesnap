//! Perfis de jogo: prende a janela de um processo numa zona fixa do monitor.
//!
//! Duas formas de prender:
//!
//! - **moldura removida** (`borda = false`): tira `WS_CAPTION`, `WS_THICKFRAME`
//!   e `WS_MAXIMIZE` e põe a janela na zona. Serve para quase todo jogo.
//! - **pela área cliente** (`alinhar_cliente = true`): não mexe no estilo e
//!   calcula o tamanho da janela a partir das margens medidas na hora, para o
//!   cliente cair exatamente na zona. É o modo do Civilization V, que mede o
//!   cursor pelo tamanho do cliente: se o cliente não fecha, a imagem estica e
//!   o clique desloca.

use std::collections::{HashMap, HashSet};

use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    ClientToScreen, EnumDisplayDevicesW, GetMonitorInfoW, MonitorFromWindow, DISPLAY_DEVICEW,
    MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClientRect, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_RESTORE, WS_CAPTION, WS_MAXIMIZE, WS_THICKFRAME,
};

use crate::config::Jogo;

#[derive(Clone, Copy, Debug)]
pub struct Alvo {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Alvo {
    fn difere(self, outro: Alvo) -> bool {
        (self.x - outro.x).abs() > 1
            || (self.y - outro.y).abs() > 1
            || (self.w - outro.w).abs() > 1
            || (self.h - outro.h).abs() > 1
    }
}

#[derive(Clone, Copy)]
struct Pin {
    style: isize,
    janela: RECT,
    alvo: Alvo,
    cliente: bool,
}

#[derive(Default)]
pub struct Jogos {
    pins: HashMap<isize, Pin>,
    /// Processos que o usuário soltou de propósito: o vigia não prende de
    /// novo até um pedido explícito.
    soltos: HashSet<String>,
    falhas: HashMap<String, u32>,
    avisados: HashSet<String>,
    pub pausado: bool,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let fim = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..fim])
}

pub fn monitor_da_janela(hwnd: isize) -> Option<RECT> {
    unsafe {
        let hmon = MonitorFromWindow(hwnd as HWND, MONITOR_DEFAULTTONEAREST);
        if hmon.is_null() {
            return None;
        }
        let mut mi: MONITORINFOEXW = std::mem::zeroed();
        mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmon, &mut mi.monitorInfo) == 0 {
            return None;
        }
        Some(mi.monitorInfo.rcMonitor)
    }
}

/// Nome do monitor e descrição do adaptador. A descrição é o que revela o
/// monitor virtual do Sunshine (`VDD`, `virtual`, `indirect`).
pub fn descricao_monitor(hwnd: isize) -> Option<(String, String)> {
    unsafe {
        let hmon = MonitorFromWindow(hwnd as HWND, MONITOR_DEFAULTTONEAREST);
        if hmon.is_null() {
            return None;
        }
        let mut mi: MONITORINFOEXW = std::mem::zeroed();
        mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmon, &mut mi.monitorInfo) == 0 {
            return None;
        }
        let device = from_wide(&mi.szDevice);
        let mut dd: DISPLAY_DEVICEW = std::mem::zeroed();
        dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let nome = wide(&device);
        let mut desc = String::new();
        if EnumDisplayDevicesW(nome.as_ptr(), 0, &mut dd, 0) != 0 {
            desc = from_wide(&dd.DeviceString);
        }
        Some((device, desc))
    }
}

pub fn monitor_virtual(hwnd: isize) -> bool {
    match descricao_monitor(hwnd) {
        Some((_, desc)) => {
            let d = desc.to_lowercase();
            d.contains("virtual") || d.contains("vdd") || d.contains("mtt") || d.contains("indirect")
        }
        None => false,
    }
}

pub fn processo_da_janela(hwnd: isize) -> String {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd as HWND, &mut pid);
        if pid == 0 {
            return String::new();
        }
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 1024];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut n);
        CloseHandle(h);
        if ok == 0 {
            return String::new();
        }
        let full = from_wide(&buf[..n as usize]);
        let nome = full.rsplit(['\\', '/']).next().unwrap_or(&full);
        nome.to_lowercase()
    }
}

fn flag_removida() -> isize {
    WS_CAPTION as isize | WS_THICKFRAME as isize | WS_MAXIMIZE as isize
}

fn rect_da_janela(hwnd: isize) -> Option<RECT> {
    let mut r: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd as HWND, &mut r) } == 0 {
        return None;
    }
    Some(r)
}

fn cliente(hwnd: isize) -> Option<(Alvo, i32, i32, i32, i32)> {
    let mut cr: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetClientRect(hwnd as HWND, &mut cr) } == 0 {
        return None;
    }
    let mut pt = POINT { x: 0, y: 0 };
    if unsafe { ClientToScreen(hwnd as HWND, &mut pt) } == 0 {
        return None;
    }
    let r = rect_da_janela(hwnd)?;
    let alvo = Alvo {
        x: pt.x,
        y: pt.y,
        w: cr.right - cr.left,
        h: cr.bottom - cr.top,
    };
    let ml = pt.x - r.left;
    let mt = pt.y - r.top;
    let mr = r.right - (pt.x + alvo.w);
    let mb = r.bottom - (pt.y + alvo.h);
    Some((alvo, ml, mt, mr, mb))
}

fn alvo_pretendido(jogo: &Jogo, largura_padrao: i32, mon: &RECT) -> Alvo {
    let w = if jogo.largura > 0 {
        jogo.largura
    } else {
        largura_padrao.max(200)
    };
    let h = if jogo.altura > 0 {
        jogo.altura
    } else {
        mon.bottom - mon.top
    };
    Alvo {
        x: mon.left + jogo.x,
        y: mon.top + jogo.y,
        w,
        h,
    }
}

impl Jogos {
    pub fn new() -> Jogos {
        Jogos::default()
    }

    pub fn quantidade(&self) -> usize {
        self.pins.len()
    }

    pub fn presos_processo(&self, processo: &str) -> usize {
        let chave = processo.to_lowercase();
        self.pins
            .keys()
            .filter(|h| processo_da_janela(**h) == chave)
            .count()
    }

    pub fn prender(
        &mut self,
        hwnd: isize,
        jogo: &Jogo,
        largura_padrao: i32,
    ) -> Result<Alvo, String> {
        if hwnd == 0 || unsafe { IsWindow(hwnd as HWND) } == 0 {
            return Err("janela inválida".to_string());
        }
        if monitor_virtual(hwnd) {
            return Err("janela no monitor virtual; não mexo".to_string());
        }
        if unsafe { IsIconic(hwnd as HWND) } != 0 {
            unsafe {
                ShowWindow(hwnd as HWND, SW_RESTORE);
            }
        }
        let mon = monitor_da_janela(hwnd).ok_or_else(|| "monitor não encontrado".to_string())?;
        let alvo = alvo_pretendido(jogo, largura_padrao, &mon);
        let chave = jogo.processo.to_lowercase();

        // Janela maximizada não aceita posição nem mostra a barra de título
        // direito: volta ao tamanho normal antes de medir e mover.
        if unsafe { IsZoomed(hwnd as HWND) } != 0 {
            unsafe {
                ShowWindow(hwnd as HWND, SW_RESTORE);
            }
        }

        if jogo.alinhar_cliente {
            let antes = rect_da_janela(hwnd).ok_or_else(|| "sem retângulo".to_string())?;
            let info = cliente(hwnd).ok_or_else(|| "sem área cliente".to_string())?;
            let (_, ml, mt, mr, mb) = info;
            let wx = alvo.x - ml;
            let wy = alvo.y - mt;
            let ww = alvo.w + ml + mr;
            let wh = alvo.h + mt + mb;
            let ok = unsafe {
                SetWindowPos(
                    hwnd as HWND,
                    std::ptr::null_mut(),
                    wx,
                    wy,
                    ww,
                    wh,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            } != 0;
            let depois = cliente(hwnd).map(|c| c.0);
            match depois {
                Some(_) if !ok => {
                    self.contar_falha(&chave);
                    return Err("SetWindowPos recusado".to_string());
                }
                Some(c) if c.difere(alvo) => {
                    unsafe {
                        SetWindowPos(
                            hwnd as HWND,
                            std::ptr::null_mut(),
                            antes.left,
                            antes.top,
                            antes.right - antes.left,
                            antes.bottom - antes.top,
                            SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                    self.contar_falha(&chave);
                    return Err(format!(
                        "cliente ficou em {}x{} e a zona pede {}x{}; rode --preparar-civ e abra o jogo de novo",
                        c.w, c.h, alvo.w, alvo.h
                    ));
                }
                None => {
                    self.contar_falha(&chave);
                    return Err("cliente sumiu depois do movimento".to_string());
                }
                _ => {}
            }
            self.pins.insert(
                hwnd,
                Pin {
                    style: unsafe { GetWindowLongPtrW(hwnd as HWND, GWL_STYLE) },
                    janela: antes,
                    alvo,
                    cliente: true,
                },
            );
            self.soltos.remove(&chave);
            self.falhas.remove(&chave);
            self.avisados.remove(&chave);
            return Ok(alvo);
        }

        let style = unsafe { GetWindowLongPtrW(hwnd as HWND, GWL_STYLE) };
        let antes = rect_da_janela(hwnd).ok_or_else(|| "sem retângulo".to_string())?;
        if !jogo.borda {
            unsafe {
                SetWindowLongPtrW(hwnd as HWND, GWL_STYLE, style & !flag_removida());
            }
        }
        let ok = unsafe {
            SetWindowPos(
                hwnd as HWND,
                std::ptr::null_mut(),
                alvo.x,
                alvo.y,
                alvo.w,
                alvo.h,
                SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } != 0;
        if !ok {
            self.contar_falha(&chave);
            return Err("SetWindowPos recusado".to_string());
        }
        self.pins.insert(
            hwnd,
            Pin {
                style,
                janela: antes,
                alvo,
                cliente: false,
            },
        );
        self.soltos.remove(&chave);
        self.falhas.remove(&chave);
        self.avisados.remove(&chave);
        Ok(alvo)
    }

    fn contar_falha(&mut self, chave: &str) -> u32 {
        let n = self.falhas.entry(chave.to_string()).or_insert(0);
        *n += 1;
        *n
    }

    pub fn deve_avisar(&mut self, chave: &str) -> bool {
        let n = *self.falhas.get(chave).unwrap_or(&0);
        if n >= 4 && !self.avisados.contains(chave) {
            self.avisados.insert(chave.to_string());
            return true;
        }
        false
    }

    pub fn soltar(&mut self, hwnd: isize) -> bool {
        let pin = match self.pins.remove(&hwnd) {
            Some(p) => p,
            None => return false,
        };
        unsafe {
            SetWindowLongPtrW(hwnd as HWND, GWL_STYLE, pin.style);
            SetWindowPos(
                hwnd as HWND,
                std::ptr::null_mut(),
                pin.janela.left,
                pin.janela.top,
                pin.janela.right - pin.janela.left,
                pin.janela.bottom - pin.janela.top,
                SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        true
    }

    pub fn soltar_processo(&mut self, processo: &str) -> usize {
        let chave = processo.to_lowercase();
        let alvos: Vec<isize> = self
            .pins
            .keys()
            .copied()
            .filter(|h| processo_da_janela(*h) == chave)
            .collect();
        for h in &alvos {
            self.soltar(*h);
        }
        self.soltos.insert(chave);
        alvos.len()
    }

    pub fn soltar_todas(&mut self) -> usize {
        let alvos: Vec<isize> = self.pins.keys().copied().collect();
        for h in &alvos {
            self.soltar(*h);
        }
        alvos.len()
    }

    pub fn pinar_processo(
        &mut self,
        processo: &str,
        perfis: &[Jogo],
        largura_padrao: i32,
    ) -> Result<isize, String> {
        let chave = processo.to_lowercase();
        let jogo = perfis
            .iter()
            .find(|j| j.processo.to_lowercase() == chave)
            .ok_or_else(|| format!("sem perfil para {processo}"))?
            .clone();
        let hwnd = maior_janela(&chave)
            .ok_or_else(|| format!("{processo} não tem janela aberta"))?;
        self.prender(hwnd, &jogo, largura_padrao)?;
        Ok(hwnd)
    }

    /// Marca como "soltos" os perfis que já têm janela aberta agora, para o
    /// vigia não mexer num jogo que já estava rodando quando o app subiu.
    pub fn marcar_abertos(&mut self, perfis: &[Jogo]) {
        for j in perfis {
            if j.processo.is_empty() {
                continue;
            }
            if maior_janela(&j.processo).is_some() {
                self.soltos.insert(j.processo.to_lowercase());
                crate::log(&format!(
                    "jogo: {} já estava aberto; só prendo se você pedir",
                    j.processo
                ));
            }
        }
    }

    /// Reaplica a zona nas janelas presas e prende sozinho um perfil com
    /// `vigiar` quando o processo aparece. Ignora janela minimizada, arrasto
    /// com o mouse e o estado pausado.
    pub fn vigiar(&mut self, perfis: &[Jogo], largura_padrao: i32) {
        if self.pausado {
            return;
        }
        // Janelas novas de perfis com vigiar.
        let abertos = janelas_visiveis();
        for (hwnd, proc) in &abertos {
            if self.pins.contains_key(hwnd) || self.soltos.contains(proc) {
                continue;
            }
            if let Some(j) = perfis
                .iter()
                .find(|j| j.vigiar && j.processo.to_lowercase() == *proc)
            {
                if monitor_virtual(*hwnd) {
                    continue;
                }
                match self.prender(*hwnd, j, largura_padrao) {
                    Ok(a) => crate::log(&format!(
                        "jogo: encaixe automático {proc} hwnd=0x{hwnd:X} cliente={} alvo={}x{}+{}+{}",
                        if j.alinhar_cliente { 1 } else { 0 },
                        a.w,
                        a.h,
                        a.x,
                        a.y
                    )),
                    Err(e) => {
                        self.contar_falha(proc);
                        if self.deve_avisar(proc) {
                            crate::log(&format!("jogo: {proc} recusou a zona: {e}"));
                        }
                        continue;
                    }
                }
            }
        }

        // Reaplicação do que já está preso.
        let ativos: Vec<(isize, Pin)> = self.pins.iter().map(|(h, p)| (*h, *p)).collect();
        for (hwnd, pin) in ativos {
            if unsafe { IsWindow(hwnd as HWND) } == 0 {
                self.pins.remove(&hwnd);
                continue;
            }
            if unsafe { IsIconic(hwnd as HWND) } != 0 {
                continue;
            }
            if unsafe { GetForegroundWindow() } == hwnd as HWND
                && (unsafe { GetAsyncKeyState(VK_LBUTTON as i32) } as u16 & 0x8000) != 0
            {
                continue;
            }
            let atual = if pin.cliente {
                cliente(hwnd).map(|c| c.0)
            } else {
                rect_da_janela(hwnd).map(|r| Alvo {
                    x: r.left,
                    y: r.top,
                    w: r.right - r.left,
                    h: r.bottom - r.top,
                })
            };
            let Some(atual) = atual else { continue };
            if !atual.difere(pin.alvo) {
                continue;
            }
            let proc = processo_da_janela(hwnd);
            let Some(jogo) = perfis
                .iter()
                .find(|j| j.processo.to_lowercase() == proc)
                .cloned()
            else {
                continue;
            };
            match self.prender(hwnd, &jogo, largura_padrao) {
                Ok(a) => crate::log(&format!(
                    "jogo: reapontado {proc} hwnd=0x{hwnd:X} alvo={}x{}+{}+{}",
                    a.w, a.h, a.x, a.y
                )),
                Err(e) => {
                    let falhas = self.contar_falha(&proc);
                    if self.deve_avisar(&proc) {
                        crate::log(&format!("jogo: {proc} recusou de novo: {e}"));
                    }
                    if falhas >= 3 {
                        self.pins.remove(&hwnd);
                        self.soltos.insert(proc.clone());
                        crate::log(&format!(
                            "jogo: {proc} ignorado até você pedir; a janela não aceita {0}x{1}, então feche o jogo, ajuste o tamanho no perfil (ou rode --preparar-civ) e abra de novo",
                            pin.alvo.w, pin.alvo.h
                        ));
                    }
                }
            }
        }
    }
}

fn janelas_visiveis() -> Vec<(isize, String)> {
    struct Ctx {
        out: Vec<(isize, String)>,
    }
    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam as *mut Ctx);
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut buf = [0u16; 512];
        if GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) <= 0 {
            return 1;
        }
        let h = hwnd as isize;
        let proc = processo_da_janela(h);
        if !proc.is_empty() {
            ctx.out.push((h, proc));
        }
        1
    }
    let mut ctx = Ctx { out: Vec::new() };
    unsafe {
        EnumWindows(Some(cb), &mut ctx as *mut Ctx as LPARAM);
    }
    ctx.out
}

/// Maior janela visível do processo, que costuma ser a janela do jogo e não a
/// tela de abertura.
pub fn maior_janela(processo: &str) -> Option<isize> {
    let chave = processo.to_lowercase();
    let candidatos: Vec<(isize, i32)> = janelas_visiveis()
        .into_iter()
        .filter(|(_, p)| *p == chave)
        .filter_map(|(h, _)| {
            let r = rect_da_janela(h)?;
            let area = (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
            Some((h, area as i32))
        })
        .collect();
    candidatos.into_iter().max_by_key(|(_, a)| *a).map(|(h, _)| h)
}
