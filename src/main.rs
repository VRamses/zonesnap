#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod civ;
mod config;
mod httpd;
mod janela;
mod jogos;
mod overlay;

use config::{Config, Zone};
use overlay::{hit_zone, zone_rect, MonGeom, Overlay};
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, SystemTime};

use windows_sys::Win32::Foundation::{
    BOOL, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, ShellExecuteW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS_MAIN: &str = "ZoneSnapMain";
const MUTEX_NAME: &str = "ZoneSnap.InstanciaUnica";
const WM_DRAG_START: u32 = WM_APP + 1;
const WM_DRAG_END: u32 = WM_APP + 2;
const WM_TRAY: u32 = WM_APP + 3;
const WM_ABRIR: u32 = WM_APP + 5;
const TIMER_HOVER: usize = 1;
const TIMER_CFG: usize = 2;
const TIMER_VIGIA: usize = 3;
const TIMER_PREVIA: usize = 4;
const WM_PREVIA: u32 = WM_APP + 4;

static MAIN: AtomicIsize = AtomicIsize::new(0);
static STATE: OnceLock<Mutex<AppState>> = OnceLock::new();
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

struct AppState {
    cfg: Config,
    cfg_path: PathBuf,
    cfg_mtime: Option<SystemTime>,
    layout: usize,
    enabled: bool,
    drag: isize,
    mon: Option<MonGeom>,
    hover: Option<usize>,
    visible: bool,
    overlay: Overlay,
    jogos: jogos::Jogos,
    /// Última conclusão sobre o ini do jogo, para o log não repetir a mesma frase.
    ini_visto: String,
}

pub fn log(msg: &str) {
    let path = LOG_PATH.get_or_init(|| config::config_dir().join("zonesnap.log"));
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(md) = std::fs::metadata(path) {
        if md.len() > 1_000_000 {
            let _ = std::fs::remove_file(path);
        }
    }
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "[{now}] {msg}");
    }
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn fill_wide(dst: &mut [u16], s: &str) {
    let units: Vec<u16> = s.encode_utf16().collect();
    let n = units.len().min(dst.len().saturating_sub(1));
    dst[..n].copy_from_slice(&units[..n]);
    dst[n] = 0;
}

fn state() -> MutexGuard<'static, AppState> {
    STATE
        .get()
        .expect("estado")
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn main_hwnd() -> HWND {
    MAIN.load(Ordering::Relaxed) as HWND
}

fn cursor_pos() -> POINT {
    let mut pt = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut pt);
    }
    pt
}

fn monitor_at(pt: POINT) -> MonGeom {
    unsafe {
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if hmon.is_null() || GetMonitorInfoW(hmon, &mut mi) == 0 {
            let w = GetSystemMetrics(SM_CXSCREEN);
            let h = GetSystemMetrics(SM_CYSCREEN);
            let r = RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            };
            return MonGeom { rect: r, work: r };
        }
        MonGeom {
            rect: mi.rcMonitor,
            work: mi.rcWork,
        }
    }
}

fn zones_of(cfg: &Config, layout: usize) -> Vec<Zone> {
    cfg.layouts
        .get(layout)
        .or_else(|| cfg.layouts.first())
        .map(|l| l.zones.clone())
        .unwrap_or_default()
}

fn window_class(hwnd: isize) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd as HWND, buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize])
}

fn ignorable(hwnd: isize) -> bool {
    let cls = window_class(hwnd);
    matches!(
        cls.as_str(),
        "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "Windows.UI.Core.CoreWindow"
    )
}

unsafe fn snap_window(hwnd: isize, mon: &MonGeom, zone: &Zone, gap: i32) -> bool {
    if hwnd == 0 {
        return false;
    }
    let h = hwnd as HWND;
    if IsWindow(h) == 0 {
        return false;
    }
    if IsIconic(h) != 0 || IsZoomed(h) != 0 {
        ShowWindow(h, SW_RESTORE);
    }
    let target = zone_rect(mon, zone, gap);
    let mut wr: RECT = std::mem::zeroed();
    if GetWindowRect(h, &mut wr) == 0 {
        return false;
    }
    let mut ef: RECT = std::mem::zeroed();
    let hr = DwmGetWindowAttribute(
        h,
        DWMWA_EXTENDED_FRAME_BOUNDS as u32,
        &mut ef as *mut RECT as *mut core::ffi::c_void,
        std::mem::size_of::<RECT>() as u32,
    );
    let mut outer = target;
    if hr == 0 && ef.right > ef.left && ef.bottom > ef.top {
        outer.left -= ef.left - wr.left;
        outer.top -= ef.top - wr.top;
        outer.right -= ef.right - wr.right;
        outer.bottom -= ef.bottom - wr.bottom;
    }
    let ok = SetWindowPos(
        h,
        std::ptr::null_mut(),
        outer.left,
        outer.top,
        outer.right - outer.left,
        outer.bottom - outer.top,
        SWP_NOZORDER | SWP_NOACTIVATE,
    ) != 0;
    let mut after: RECT = std::mem::zeroed();
    GetWindowRect(h, &mut after);
    log(&format!(
        "snap hwnd=0x{hwnd:X} rect=({},{},{},{}) ok={}",
        after.left,
        after.top,
        after.right,
        after.bottom,
        if ok { 1 } else { 0 }
    ));
    ok
}

fn on_drag_start(hwnd: isize) {
    let mut st = state();
    if !st.enabled || hwnd == 0 || hwnd == st.overlay.hwnd() || ignorable(hwnd) {
        return;
    }
    st.drag = hwnd;
    let pt = cursor_pos();
    let mon = monitor_at(pt);
    let zones = zones_of(&st.cfg, st.layout);
    let gap = st.cfg.gap;
    st.mon = Some(mon);
    st.hover = None;
    st.visible = false;
    if st.cfg.show_overlay && !zones.is_empty() {
        st.visible = st.overlay.show(&mon, &zones, None, gap);
        if st.visible {
            unsafe {
                SetTimer(main_hwnd(), TIMER_HOVER, 25, None);
            }
        }
    }
}

fn on_tick() {
    let mut st = state();
    if st.drag == 0 || !st.visible {
        return;
    }
    let pt = cursor_pos();
    let mon = monitor_at(pt);
    let zones = zones_of(&st.cfg, st.layout);
    let gap = st.cfg.gap;
    let trocou = match st.mon {
        Some(m) => m.rect.left != mon.rect.left || m.rect.top != mon.rect.top,
        None => true,
    };
    if trocou {
        st.mon = Some(mon);
        st.hover = None;
        st.visible = st.overlay.show(&mon, &zones, None, gap);
    }
    let hv = hit_zone(&mon, &zones, gap, pt);
    if hv != st.hover {
        st.hover = hv;
        st.overlay.show(&mon, &zones, hv, gap);
    }
}

fn on_drag_end() {
    unsafe {
        KillTimer(main_hwnd(), TIMER_HOVER);
    }
    let mut st = state();
    let drag = st.drag;
    st.drag = 0;
    if st.visible {
        st.overlay.hide();
        st.visible = false;
    }
    st.hover = None;
    st.mon = None;
    if drag == 0 || !st.enabled {
        return;
    }
    let pt = cursor_pos();
    let mon = monitor_at(pt);
    let zones = zones_of(&st.cfg, st.layout);
    let gap = st.cfg.gap;
    if let Some(i) = hit_zone(&mon, &zones, gap, pt) {
        if let Some(z) = zones.get(i) {
            unsafe {
                snap_window(drag, &mon, z, gap);
            }
        }
    }
}

unsafe extern "system" fn win_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _idobject: i32,
    _idchild: i32,
    _idthread: u32,
    _time: u32,
) {
    if hwnd.is_null() {
        return;
    }
    let msg = if event == EVENT_SYSTEM_MOVESIZESTART {
        WM_DRAG_START
    } else if event == EVENT_SYSTEM_MOVESIZEEND {
        WM_DRAG_END
    } else {
        return;
    };
    let main = main_hwnd();
    if main.is_null() {
        return;
    }
    PostMessageW(main, msg, hwnd as WPARAM, 0);
}

fn tip_text(st: &AppState) -> String {
    let name = st
        .cfg
        .layouts
        .get(st.layout)
        .map(|l| l.name.as_str())
        .unwrap_or("sem layout");
    let presos = st.jogos.quantidade();
    let extra = if presos > 0 {
        format!(" - {presos} preso{}", if presos > 1 { "s" } else { "" })
    } else {
        String::new()
    };
    if st.jogos.pausado {
        format!("ZoneSnap - jogos pausados ({name})")
    } else if st.enabled {
        format!("ZoneSnap - {name}{extra}")
    } else {
        format!("ZoneSnap - pausado ({name}){extra}")
    }
}

unsafe fn tray_add(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
    let tip = tip_text(&state());
    fill_wide(&mut nid.szTip, &tip);
    if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
        log("tray: NIM_ADD falhou");
    }
}

unsafe fn tray_update(hwnd: HWND, st: &AppState) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_TIP;
    let tip = tip_text(st);
    fill_wide(&mut nid.szTip, &tip);
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

unsafe fn tray_remove(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe fn open_config(path: &PathBuf) {
    let op = wide("open");
    let p = wide(&path.to_string_lossy());
    ShellExecuteW(
        std::ptr::null_mut(),
        op.as_ptr(),
        p.as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
        SW_SHOWNORMAL,
    );
}

/// Abre a interface na janela própria do programa. Se o WebView2 não estiver
/// disponível, cai no navegador para não deixar ninguém sem interface.
fn abrir_interface() {
    let Some(url) = httpd::endereco() else {
        log("interface: endereço não encontrado");
        return;
    };
    match janela::abrir(&url) {
        Ok(()) => log(&format!("interface na janela do programa em {url}")),
        Err(e) => {
            log(&format!("janela do programa: {e}; abrindo no navegador"));
            httpd::abrir();
        }
    }
}

/// Pede à instância que já está rodando para mostrar a janela da interface.
/// Usado quando um atalho com `--ui` é disparado com o programa já aberto.
unsafe fn pedir_interface() {
    unsafe extern "system" fn procurar(hwnd: HWND, l: LPARAM) -> BOOL {
        let mut buf = [0u16; 64];
        let n = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if n > 0 {
            let nome = String::from_utf16_lossy(&buf[..n as usize]);
            if nome == CLASS_MAIN {
                *(l as *mut HWND) = hwnd;
                return 0;
            }
        }
        1
    }
    let mut achado: HWND = std::ptr::null_mut();
    EnumWindows(Some(procurar), &mut achado as *mut HWND as LPARAM);
    if !achado.is_null() {
        PostMessageW(achado, WM_ABRIR, 0, 0);
    }
}

fn reload_config(st: &mut AppState) {
    let (cfg, mt) = config::load_or_create(&st.cfg_path);
    st.layout = cfg.layout.min(cfg.layouts.len().saturating_sub(1));
    st.enabled = cfg.enabled;
    st.cfg_mtime = mt;
    st.cfg = cfg;
    log(&format!(
        "config recarregada gap={} layouts={} layout={}",
        st.cfg.gap,
        st.cfg.layouts.len(),
        st.layout
    ));
}

fn reload_if_changed() {
    let mut st = state();
    let mt = std::fs::metadata(&st.cfg_path)
        .ok()
        .and_then(|m| m.modified().ok());
    if mt.is_some() && mt != st.cfg_mtime {
        reload_config(&mut st);
        unsafe {
            tray_update(main_hwnd(), &st);
        }
    }
}

fn on_hotkey(id: i32) {
    match id {
        1 => {
            let mut st = state();
            let n = st.cfg.layouts.len().max(1);
            st.layout = (st.layout + 1) % n;
            let nome = st
                .cfg
                .layouts
                .get(st.layout)
                .map(|l| l.name.clone())
                .unwrap_or_default();
            log(&format!("layout {} ({})", st.layout, nome));
            unsafe {
                tray_update(main_hwnd(), &st);
            }
        }
        2 => {
            let mut st = state();
            st.enabled = !st.enabled;
            log(&format!("enabled -> {}", st.enabled));
            unsafe {
                tray_update(main_hwnd(), &st);
            }
        }
        10..=18 => {
            let idx = (id - 10) as usize;
            let st = state();
            let hwnd = unsafe { GetForegroundWindow() };
            if hwnd.is_null() {
                return;
            }
            let alvo = hwnd as isize;
            if ignorable(alvo) || alvo == st.overlay.hwnd() {
                return;
            }
            let mut wr: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetWindowRect(hwnd, &mut wr);
            }
            let mon = monitor_at(POINT {
                x: wr.left,
                y: wr.top,
            });
            let zones = zones_of(&st.cfg, st.layout);
            if let Some(z) = zones.get(idx) {
                unsafe {
                    snap_window(alvo, &mon, z, st.cfg.gap);
                }
            }
        }
        20 => {
            let hwnd = unsafe { GetForegroundWindow() } as isize;
            let proc = jogos::processo_da_janela(hwnd);
            match executar_acao("prender", Some(&proc)) {
                Ok(m) => log(&format!("atalho prender: {m}")),
                Err(e) => log(&format!("atalho prender: {e}")),
            }
        }
        21 => {
            let hwnd = unsafe { GetForegroundWindow() } as isize;
            let proc = jogos::processo_da_janela(hwnd);
            match executar_acao("soltar", Some(&proc)) {
                Ok(m) => log(&format!("atalho soltar: {m}")),
                Err(e) => log(&format!("atalho soltar: {e}")),
            }
        }
        22 => {
            let hwnd = unsafe { GetForegroundWindow() } as isize;
            if hwnd == 0 {
                log("atalho area livre: sem janela em foco");
                return;
            }
            let proc = jogos::processo_da_janela(hwnd);
            let largura = {
                let st = state();
                st.cfg
                    .jogos
                    .iter()
                    .find(|j| j.processo.to_lowercase() == proc)
                    .map(|j| {
                        if j.largura > 0 {
                            j.largura
                        } else {
                            st.cfg.jogo_largura
                        }
                    })
                    .unwrap_or(st.cfg.jogo_largura)
            };
            let mut wr: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetWindowRect(hwnd as HWND, &mut wr);
            }
            let mon = monitor_at(POINT {
                x: wr.left,
                y: wr.top,
            });
            let x = mon.rect.left + largura;
            let w = (mon.rect.right - x).max(200);
            let h = mon.rect.bottom - mon.rect.top;
            unsafe {
                SetWindowPos(
                    hwnd as HWND,
                    std::ptr::null_mut(),
                    x,
                    mon.rect.top,
                    w,
                    h,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            log(&format!("area livre: {proc} em {w}x{h}+{x}"));
        }
        23 => match executar_acao("pausar", None) {
            Ok(m) => log(&format!("atalho pausar: {m}")),
            Err(e) => log(&format!("atalho pausar: {e}")),
        },
        _ => {}
    }
}

unsafe fn tray_menu(hwnd: HWND) {
    let st = state();
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    let enabled_label = wide(if st.enabled {
        "Ativado (clique para pausar)"
    } else {
        "Pausado (clique para ativar)"
    });
    let flags = MF_STRING | if st.enabled { MF_CHECKED } else { 0 };
    AppendMenuW(menu, flags, 1, enabled_label.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    let sub = CreatePopupMenu();
    if !sub.is_null() {
        for (i, l) in st.cfg.layouts.iter().enumerate() {
            let name = wide(&l.name);
            let f = MF_STRING | if i == st.layout { MF_CHECKED } else { 0 };
            AppendMenuW(sub, f, 100 + i, name.as_ptr());
        }
        let sub_label = wide("Layout");
        AppendMenuW(menu, MF_POPUP, sub as usize, sub_label.as_ptr());
    }
    let interface = wide("Abrir interface");
    let navegador = wide("Abrir no navegador");
    AppendMenuW(menu, MF_STRING, 5, interface.as_ptr());
    AppendMenuW(menu, MF_STRING, 9, navegador.as_ptr());
    let subj = CreatePopupMenu();
    if !subj.is_null() {
        for (i, j) in st.cfg.jogos.iter().enumerate() {
            let preso = st.jogos.presos_processo(&j.processo) > 0;
            let texto = wide(&format!(
                "{} - {}",
                j.nome,
                if preso { "preso" } else { "livre" }
            ));
            AppendMenuW(subj, MF_STRING, 300usize + i, texto.as_ptr());
        }
        if st.cfg.jogos.is_empty() {
            let vazio = wide("(sem perfis)");
            AppendMenuW(subj, MF_STRING | MF_GRAYED, 0, vazio.as_ptr());
        }
        AppendMenuW(subj, MF_SEPARATOR, 0, std::ptr::null());
        let pausar = wide("Pausar vigilância dos jogos");
        let f = MF_STRING | if st.jogos.pausado { MF_CHECKED } else { 0 };
        AppendMenuW(subj, f, 6, pausar.as_ptr());
        let todas = wide("Soltar todas as janelas presas");
        AppendMenuW(subj, MF_STRING, 7, todas.as_ptr());
        if st
            .cfg
            .jogos
            .iter()
            .any(|j| j.preparar.eq_ignore_ascii_case("civ5"))
        {
            AppendMenuW(subj, MF_SEPARATOR, 0, std::ptr::null());
            let ini = wide("Conferir o .ini do Civ V");
            AppendMenuW(subj, MF_STRING, 8, ini.as_ptr());
        }
        let jlabel = wide("Jogos");
        AppendMenuW(menu, MF_POPUP, subj as usize, jlabel.as_ptr());
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    let abrir = wide("Abrir config.json");
    let recarregar = wide("Recarregar config");
    AppendMenuW(menu, MF_STRING, 2, abrir.as_ptr());
    AppendMenuW(menu, MF_STRING, 3, recarregar.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    let sair = wide("Sair");
    AppendMenuW(menu, MF_STRING, 4, sair.as_ptr());

    // TrackPopupMenu roda um laço modal e despacha WM_TIMER daqui mesmo.
    // Se a trava do estado estiver na mão neste ponto, o primeiro timer que
    // chamar state() trava o programa para sempre. O menu já está montado:
    // solta a trava antes de abrir, e cada ação pega a sua.
    drop(st);

    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_RIGHTBUTTON,
        pt.x,
        pt.y,
        0,
        hwnd,
        std::ptr::null(),
    );
    DestroyMenu(menu);
    PostMessageW(hwnd, WM_NULL, 0, 0);

    match cmd {
        1 => on_hotkey(2),
        2 => {
            let caminho = state().cfg_path.clone();
            open_config(&caminho);
        }
        3 => {
            let mut st = state();
            reload_config(&mut st);
            tray_update(hwnd, &st);
        }
        4 => {
            let mut st = state();
            let n = st.jogos.soltar_todas();
            log(&format!("saindo: soltou {n} janela(s)"));
            janela::fechar();
            tray_remove(hwnd);
            DestroyWindow(hwnd);
        }
        100..=200 => {
            let idx = (cmd - 100) as usize;
            let mut st = state();
            if idx < st.cfg.layouts.len() {
                st.layout = idx;
                tray_update(hwnd, &st);
            }
        }
        5 => abrir_interface(),
        9 => {
            if let Some(url) = httpd::abrir() {
                log(&format!("interface aberta no navegador em {url}"));
            }
        }
        6 => {
            let mut st = state();
            st.jogos.pausado = !st.jogos.pausado;
            log(&format!("jogos: pausado={}", st.jogos.pausado));
            tray_update(hwnd, &st);
        }
        7 => {
            let mut st = state();
            let n = st.jogos.soltar_todas();
            log(&format!("jogos: soltou {n} janela(s)"));
            tray_update(hwnd, &st);
        }
        8 => match executar_acao("preparar_civ", None) {
            Ok(m) => {
                log(&format!("menu ini: {m}"));
                aviso(&m);
            }
            Err(e) => {
                log(&format!("menu ini: {e}"));
                aviso(&e);
            }
        },
        300..=399 => {
            let idx = (cmd - 300) as usize;
            let proc = {
                let st = state();
                st.cfg.jogos.get(idx).map(|j| j.processo.clone())
            };
            if let Some(p) = proc {
                match executar_acao("prender", Some(&p)) {
                    Ok(m) => log(&format!("menu jogos: {m}")),
                    Err(e) => log(&format!("menu jogos: {e}")),
                }
            }
        }
        _ => {}
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DRAG_START => {
            on_drag_start(wparam as isize);
            0
        }
        WM_DRAG_END => {
            on_drag_end();
            0
        }
        WM_TIMER => {
            if wparam == TIMER_HOVER {
                on_tick();
            } else if wparam == TIMER_CFG {
                reload_if_changed();
                vigiar_ini();
            } else if wparam == TIMER_VIGIA {
                vigiar_jogos();
            } else if wparam == TIMER_PREVIA {
                esconder_previa();
            }
            0
        }
        WM_PREVIA => {
            mostrar_previa();
            0
        }
        WM_HOTKEY => {
            on_hotkey(wparam as i32);
            0
        }
        WM_TRAY => {
            let ev = (lparam as u32) & 0xFFFF;
            if ev == WM_RBUTTONUP || ev == WM_CONTEXTMENU || ev == WM_LBUTTONUP {
                tray_menu(hwnd);
            }
            0
        }
        WM_ABRIR => {
            // Pedido de outra instância (por exemplo, atalho com --ui quando
            // este programa já está rodando): mostra a janela, em vez de abrir
            // outra aba de navegador.
            abrir_interface();
            0
        }
        WM_DESTROY => {
            // saindo por qualquer caminho (Sair, encerrar sessao, taskkill amigavel):
            // devolve as janelas presas ao estilo e ao retangulo originais.
            let n = state().jogos.soltar_todas();
            if n > 0 {
                log(&format!("encerrando: soltou {n} janela(s)"));
            }
            tray_remove(hwnd);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let i = args.iter().position(|a| a == flag)?;
    args.get(i + 1).map(|s| s.as_str())
}

struct FindCtx {
    needle: String,
    found: isize,
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam as *mut FindCtx);
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let mut buf = [0u16; 512];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len > 0 {
        let s = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
        if s.contains(&ctx.needle) {
            ctx.found = hwnd as isize;
            return 0;
        }
    }
    1
}

fn find_window(needle: &str) -> isize {
    let mut ctx = FindCtx {
        needle: needle.to_lowercase(),
        found: 0,
    };
    unsafe {
        EnumWindows(Some(enum_cb), &mut ctx as *mut FindCtx as LPARAM);
    }
    ctx.found
}

fn cli_preview(args: &[String]) {
    let secs: u64 = arg_value(args, "--seconds")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    let (cfg, _) = config::load_or_create(&config::default_config_path());
    let inst = unsafe { GetModuleHandleW(std::ptr::null()) };
    let ov = match Overlay::create(inst as *mut core::ffi::c_void) {
        Some(o) => o,
        None => {
            log("preview: overlay nao criado");
            return;
        }
    };
    let pt = cursor_pos();
    let mon = monitor_at(pt);
    let zones = zones_of(&cfg, cfg.layout);
    let ok = ov.show(&mon, &zones, Some(0), cfg.gap);
    log(&format!(
        "preview ativo mon=({},{},{},{}) zonas={} ok={}",
        mon.rect.left,
        mon.rect.top,
        mon.rect.right,
        mon.rect.bottom,
        zones.len(),
        if ok { 1 } else { 0 }
    ));
    std::thread::sleep(Duration::from_secs(secs));
    ov.hide();
}

fn cli_snap(args: &[String], pos: usize) {
    let idx: usize = args
        .get(pos + 1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let (cfg, _) = config::load_or_create(&config::default_config_path());
    let hwnd = match arg_value(args, "--title") {
        Some(t) => find_window(t),
        None => unsafe { GetForegroundWindow() as isize },
    };
    if hwnd == 0 {
        log("snap: janela nao encontrada");
        return;
    }
    let mut wr: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetWindowRect(hwnd as HWND, &mut wr);
    }
    let mon = monitor_at(POINT {
        x: wr.left,
        y: wr.top,
    });
    let zones = zones_of(&cfg, cfg.layout);
    match zones.get(idx) {
        Some(z) => unsafe {
            snap_window(hwnd, &mon, z, cfg.gap);
        },
        None => log(&format!("snap: zona {idx} nao existe neste layout")),
    }
}

unsafe fn register_hotkeys(hwnd: HWND) {
    let mods = MOD_WIN | MOD_ALT;
    if RegisterHotKey(hwnd, 1, mods, 'L' as u32) == 0 {
        log("hotkey Win+Alt+L indisponivel");
    }
    if RegisterHotKey(hwnd, 2, mods, 'Z' as u32) == 0 {
        log("hotkey Win+Alt+Z indisponivel");
    }
    for i in 0..9u32 {
        if RegisterHotKey(hwnd, 10 + i as i32, mods | MOD_SHIFT, '1' as u32 + i) == 0 {
            log(&format!("hotkey Win+Alt+Shift+{} indisponivel", i + 1));
        }
    }
    let ctrl_alt = MOD_CONTROL | MOD_ALT;
    for (id, vk, nome) in [
        (20, 'Z', "Ctrl+Alt+Z"),
        (21, 'X', "Ctrl+Alt+X"),
        (22, 'B', "Ctrl+Alt+B"),
        (23, 'S', "Ctrl+Alt+S"),
    ] {
        if RegisterHotKey(hwnd, id, ctrl_alt, vk as u32) != 0 {
            continue;
        }
        if RegisterHotKey(hwnd, id, ctrl_alt | MOD_SHIFT, vk as u32) != 0 {
            log(&format!("hotkey {nome} ocupada; usei Ctrl+Alt+Shift+{vk}"));
        } else {
            log(&format!("hotkey {nome} e Ctrl+Alt+Shift+{vk} indisponiveis"));
        }
    }
}

fn aviso(msg: &str) {
    let t = wide("ZoneSnap");
    let m = wide(msg);
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            m.as_ptr(),
            t.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

fn vigiar_jogos() {
    let mut st = state();
    let antes = st.jogos.quantidade();
    let perfis = st.cfg.jogos.clone();
    let largura = st.cfg.jogo_largura;
    st.jogos.vigiar(&perfis, largura);
    if st.jogos.quantidade() != antes {
        unsafe {
            tray_update(main_hwnd(), &st);
        }
    }
}

/// Confere o ini do jogo a cada cinco passadas do timer de configuração
/// (10 segundos) e só escreve no log quando a conclusão muda, para não
/// transformar o arquivo num eco.
static INI_TICK: AtomicUsize = AtomicUsize::new(0);

fn vigiar_ini() {
    let volta = INI_TICK.fetch_add(1, Ordering::Relaxed) + 1;
    if volta % 5 != 0 {
        return;
    }
    let alvo = {
        let st = state();
        st.cfg
            .jogos
            .iter()
            .find(|j| j.preparar.eq_ignore_ascii_case("civ5"))
            .map(|j| (j.largura, j.altura, j.nome.clone()))
    };
    let Some((largura, altura, nome)) = alvo else {
        return;
    };
    let e = civ::ler();
    let assinatura = format!(
        "{}|{}|{}|{}|{}",
        e.existe,
        e.jogo_aberto,
        e.fullscreen.map(|v| v.to_string()).unwrap_or_default(),
        e.largura.map(|v| v.to_string()).unwrap_or_default(),
        e.altura.map(|v| v.to_string()).unwrap_or_default()
    );
    let mut st = state();
    if st.ini_visto == assinatura {
        return;
    }
    let frase = e.frase(largura, altura);
    log(&format!("ini de {nome}: {frase}"));
    st.ini_visto = assinatura;
}

fn mostrar_previa() {
    let mut st = state();
    let pt = cursor_pos();
    let mon = monitor_at(pt);
    let zones = zones_of(&st.cfg, st.layout);
    st.visible = st.overlay.show(&mon, &zones, Some(0), st.cfg.gap);
    st.mon = Some(mon);
    st.hover = Some(0);
    unsafe {
        SetTimer(main_hwnd(), TIMER_PREVIA, 2000, None);
    }
}

fn esconder_previa() {
    unsafe {
        KillTimer(main_hwnd(), TIMER_PREVIA);
    }
    let mut st = state();
    st.overlay.hide();
    st.visible = false;
    st.hover = None;
    st.mon = None;
}

pub fn config_json() -> serde_json::Value {
    serde_json::to_value(&state().cfg).unwrap_or(serde_json::json!({}))
}

pub fn estado_json() -> serde_json::Value {
    let st = state();
    let layout_nome = st
        .cfg
        .layouts
        .get(st.layout)
        .map(|l| l.name.clone())
        .unwrap_or_default();
    let layouts: Vec<serde_json::Value> = st
        .cfg
        .layouts
        .iter()
        .map(|l| serde_json::json!({"nome": l.name, "zonas": l.zones}))
        .collect();
    let jogos: Vec<serde_json::Value> = st
        .cfg
        .jogos
        .iter()
        .map(|j| {
            let mut v = serde_json::json!({
                "processo": j.processo,
                "nome": j.nome,
                "largura": j.largura,
                "altura": j.altura,
                "x": j.x,
                "y": j.y,
                "vigiar": j.vigiar,
                "borda": j.borda,
                "alinhar_cliente": j.alinhar_cliente,
                "preparar": j.preparar,
                "preso": st.jogos.presos_processo(&j.processo) > 0,
                "aberto": jogos::maior_janela(&j.processo).is_some(),
            });
            // O ini do jogo diz, na hora, se ele vai nascer no tamanho da zona.
            if j.preparar.eq_ignore_ascii_case("civ5") {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("ini".to_string(), civ::resumo_json(j.largura, j.altura));
                }
            }
            v
        })
        .collect();
    let (largura_mon, altura_mon) = unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
        )
    };
    serde_json::json!({
        "versao": env!("CARGO_PKG_VERSION"),
        "ativo": st.enabled,
        "pausado": st.jogos.pausado,
        "gap": st.cfg.gap,
        "mostrar_previa": st.cfg.show_overlay,
        "layout": st.layout,
        "layout_nome": layout_nome,
        "layouts": layouts,
        "jogo_largura": st.cfg.jogo_largura,
        "jogos": jogos,
        "presos": st.jogos.quantidade(),
        "monitor": {"largura": largura_mon, "altura": altura_mon},
        "caminho_config": st.cfg_path.display().to_string(),
        "caminho_log": LOG_PATH.get().map(|p| p.display().to_string()),
        "civ_ini": civ::ini_path().map(|p| p.display().to_string()),
    })
}

pub fn recarregar() {
    let mut st = state();
    reload_config(&mut st);
    unsafe {
        tray_update(main_hwnd(), &st);
    }
}

pub fn executar_acao(acao: &str, processo: Option<&str>) -> Result<String, String> {
    match acao {
        "prender" => {
            let p = processo
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "informe o processo".to_string())?;
            let (perfis, largura) = {
                let st = state();
                (st.cfg.jogos.clone(), st.cfg.jogo_largura)
            };
            let h = {
                let mut st = state();
                let h = st.jogos.pinar_processo(p, &perfis, largura)?;
                unsafe {
                    tray_update(main_hwnd(), &st);
                }
                h
            };
            Ok(format!("{p} preso (janela 0x{h:X})"))
        }
        "soltar" => {
            let p = processo
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "informe o processo".to_string())?;
            let mut st = state();
            let n = st.jogos.soltar_processo(p);
            unsafe {
                tray_update(main_hwnd(), &st);
            }
            Ok(format!("soltou {n} janela(s) de {p}"))
        }
        "soltar_todas" => {
            let mut st = state();
            let n = st.jogos.soltar_todas();
            unsafe {
                tray_update(main_hwnd(), &st);
            }
            Ok(format!("soltou {n} janela(s)"))
        }
        "pausar" => {
            let mut st = state();
            st.jogos.pausado = !st.jogos.pausado;
            let pausado = st.jogos.pausado;
            unsafe {
                tray_update(main_hwnd(), &st);
            }
            Ok(if pausado {
                "vigilância dos jogos pausada".to_string()
            } else {
                "vigilância dos jogos ativa".to_string()
            })
        }
        "recarregar" => {
            let mut st = state();
            reload_config(&mut st);
            unsafe {
                tray_update(main_hwnd(), &st);
            }
            Ok("configuração recarregada".to_string())
        }
        "previa" => {
            unsafe {
                PostMessageW(main_hwnd(), WM_PREVIA, 0, 0);
            }
            Ok("prévia das zonas por 2 segundos".to_string())
        }
        "preparar_civ" => {
            let (w, h) = {
                let st = state();
                let j = st
                    .cfg
                    .jogos
                    .iter()
                    .find(|j| j.preparar.eq_ignore_ascii_case("civ5"));
                let w = j
                    .map(|j| j.largura)
                    .filter(|v| *v > 0)
                    .unwrap_or(st.cfg.jogo_largura);
                let h = j
                    .map(|j| j.altura)
                    .filter(|v| *v > 0)
                    .unwrap_or_else(|| unsafe { GetSystemMetrics(SM_CYSCREEN) });
                (w, h)
            };
            civ::preparar(w, h)
        }
        "abrir_config" => {
            let p = {
                let st = state();
                st.cfg_path.clone()
            };
            unsafe {
                open_config(&p);
            }
            Ok(p.display().to_string())
        }
        "abrir_pasta" => {
            let p = config::config_dir();
            unsafe {
                open_config(&p);
            }
            Ok(p.display().to_string())
        }
        "" => Err("ação vazia".to_string()),
        outra => Err(format!("ação desconhecida: {outra}")),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    unsafe {
        SetProcessDpiAwarenessContext((-4isize) as *mut core::ffi::c_void);
        // Apartamento single-thread: exigência do WebView2 da janela da
        // interface, que roda no mesmo thread da fila de mensagens.
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
    }

    if args.iter().any(|a| a == "--preview") {
        cli_preview(&args);
        return;
    }
    if let Some(pos) = args.iter().position(|a| a == "--snap") {
        cli_snap(&args, pos);
        return;
    }

    if args.iter().any(|a| a == "--preparar-civ") {
        let (cfg, _) = config::load_or_create(&config::default_config_path());
        let j = cfg
            .jogos
            .iter()
            .find(|j| j.preparar.eq_ignore_ascii_case("civ5"))
            .cloned();
        let largura = j
            .as_ref()
            .map(|j| j.largura)
            .filter(|v| *v > 0)
            .unwrap_or(cfg.jogo_largura);
        let altura = j
            .as_ref()
            .map(|j| j.altura)
            .filter(|v| *v > 0)
            .unwrap_or_else(|| unsafe { GetSystemMetrics(SM_CYSCREEN) });
        let msg = match civ::preparar(largura, altura) {
            Ok(m) => m,
            Err(e) => format!("Falhou: {e}"),
        };
        log(&format!("preparar-civ: {msg}"));
        aviso(&msg);
        return;
    }

    // Diagnóstico do ini sem subir o app nem tocar no arquivo.
    if args.iter().any(|a| a == "--checar-ini") {
        let (cfg, _) = config::load_or_create(&config::default_config_path());
        let j = cfg
            .jogos
            .iter()
            .find(|j| j.preparar.eq_ignore_ascii_case("civ5"))
            .cloned();
        let largura = j
            .as_ref()
            .map(|j| j.largura)
            .filter(|v| *v > 0)
            .unwrap_or(cfg.jogo_largura);
        let altura = j
            .as_ref()
            .map(|j| j.altura)
            .filter(|v| *v > 0)
            .unwrap_or_else(|| unsafe { GetSystemMetrics(SM_CYSCREEN) });
        let e = civ::ler();
        let num = |v: Option<i32>| v.map(|n| n.to_string()).unwrap_or_else(|| "?".to_string());
        log(&format!("checar-ini: {}", e.frase(largura, altura)));
        println!(
            "ini existe={} jogo_aberto={} fullscreen={} janela={}x{} alvo={}x{} divergente={} pode_corrigir={}",
            e.existe,
            e.jogo_aberto,
            num(e.fullscreen),
            num(e.largura),
            num(e.altura),
            largura,
            altura,
            e.divergente(largura, altura),
            e.pode_corrigir(largura, altura)
        );
        return;
    }

    unsafe {
        let nome = wide(MUTEX_NAME);
        let mtx = CreateMutexW(std::ptr::null(), 1, nome.as_ptr());
        if mtx.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
            log("ja existe uma instancia rodando");
            if args.iter().any(|a| a == "--ui") {
                pedir_interface();
                log("pedi a interface para a instancia que ja estava rodando");
            }
            return;
        }

        let inst = GetModuleHandleW(std::ptr::null());
        let cfg_path = config::default_config_path();
        let (cfg, mt) = config::load_or_create(&cfg_path);
        let layout = cfg.layout.min(cfg.layouts.len().saturating_sub(1));
        let enabled = cfg.enabled;
        let overlay = match Overlay::create(inst as *mut core::ffi::c_void) {
            Some(o) => o,
            None => {
                log("overlay: falha ao criar janela");
                return;
            }
        };
        let _ = STATE.set(Mutex::new(AppState {
            cfg,
            cfg_path,
            cfg_mtime: mt,
            layout,
            enabled,
            drag: 0,
            mon: None,
            hover: None,
            visible: false,
            overlay,
            jogos: jogos::Jogos::new(),
            ini_visto: String::new(),
        }));

        let class = wide(CLASS_MAIN);
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
        };
        RegisterClassW(&wc);
        let main = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("ZoneSnap").as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            inst,
            std::ptr::null(),
        );
        if main.is_null() {
            log("janela principal: falha");
            return;
        }
        MAIN.store(main as isize, Ordering::Relaxed);

        tray_add(main);
        register_hotkeys(main);
        let hook = SetWinEventHook(
            EVENT_SYSTEM_MOVESIZESTART,
            EVENT_SYSTEM_MOVESIZEEND,
            std::ptr::null_mut(),
            Some(win_event),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        if hook.is_null() {
            log("hook: SetWinEventHook falhou");
        }
        SetTimer(main, TIMER_CFG, 2000, None);
        SetTimer(main, TIMER_VIGIA, 1200, None);

        {
            let mut st = state();
            let perfis = st.cfg.jogos.clone();
            st.jogos.marcar_abertos(&perfis);
        }

        match httpd::iniciar() {
            Some((porta, token)) => log(&format!(
                "interface em http://127.0.0.1:{porta} token={token}"
            )),
            None => log("interface: nenhuma porta livre entre 47121 e 47159"),
        }
        if args.iter().any(|a| a == "--ui") {
            abrir_interface();
        }

        log(&format!(
            "iniciando layout={} v={}",
            layout,
            env!("CARGO_PKG_VERSION")
        ));

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
