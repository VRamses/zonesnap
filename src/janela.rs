//! A interface em janela própria do Windows.
//!
//! Em vez de mandar o navegador abrir uma aba, esta janela carrega a mesma
//! página servida em 127.0.0.1 dentro de um WebView2, o componente de
//! navegador que já vem no Windows 11 (Edge WebView2 Runtime). Para o usuário
//! é uma janela comum: aparece na barra de tarefas e no Alt+Tab, abre no
//! tamanho da tela e fecha sem parar o programa, que continua no ícone ao lado
//! do relógio.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Once;

use webview2_com::Microsoft::Web::WebView2::Win32::{
    CreateCoreWebView2EnvironmentWithOptions, ICoreWebView2Controller,
    ICoreWebView2Controller2, ICoreWebView2Environment, ICoreWebView2EnvironmentOptions,
    COREWEBVIEW2_COLOR,
};
use webview2_com::{
    CreateCoreWebView2ControllerCompletedHandler, CreateCoreWebView2EnvironmentCompletedHandler,
};
use windows::core::{HSTRING, Interface};
use windows::Win32::Foundation::{HWND as HwndWeb, RECT as RectWeb};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS_UI: &str = "ZoneSnapJanela";
const LARGURA: i32 = 1240;
const ALTURA: i32 = 900;

static JANELA: AtomicIsize = AtomicIsize::new(0);
static CLASSE: Once = Once::new();

thread_local! {
    /// O controlador do WebView2 vive no thread da interface, como manda o COM:
    /// tudo aqui roda na mesma linha de execução do laço de mensagens.
    static WEBVIEW: RefCell<Option<ICoreWebView2Controller>> = RefCell::new(None);
}

/// Traz a janela para a frente, criando-a se ainda não existir.
///
/// A mensagem de erro volta para quem chamou registrar no log e cair no
/// navegador, o que acontece quando o WebView2 não está instalado.
pub fn abrir(url: &str) -> Result<(), String> {
    let atual = JANELA.load(Ordering::Relaxed);
    if atual != 0 {
        unsafe {
            let hwnd = atual as HWND;
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
        if let Some(ctrl) = atual_webview() {
            let _ = unsafe { ctrl.SetIsVisible(true) };
        }
        return Ok(());
    }

    unsafe { criar(url) }
}

/// Fecha a janela se estiver aberta, para não deixar o WebView2 segurando o
/// processo na hora de sair.
pub fn fechar() {
    let atual = JANELA.load(Ordering::Relaxed);
    if atual != 0 {
        unsafe {
            DestroyWindow(atual as HWND);
        }
    }
}

unsafe fn criar(url: &str) -> Result<(), String> {
    registrar_classe();
    let inst = GetModuleHandleW(std::ptr::null());

    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    let mut info: MONITORINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
    GetMonitorInfoW(monitor, &mut info);
    let area = info.rcWork;
    let largura = LARGURA.min(area.right - area.left - 80);
    let altura = ALTURA.min(area.bottom - area.top - 80);
    let x = area.left + ((area.right - area.left) - largura) / 2;
    let y = area.top + ((area.bottom - area.top) - altura) / 2;

    let classe = wide(CLASS_UI);
    let titulo = wide("ZoneSnap");
    let hwnd = CreateWindowExW(
        0,
        classe.as_ptr(),
        titulo.as_ptr(),
        WS_OVERLAPPEDWINDOW,
        x,
        y,
        largura,
        altura,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        inst,
        std::ptr::null(),
    );
    if hwnd.is_null() {
        return Err("não consegui criar a janela".into());
    }
    JANELA.store(hwnd as isize, Ordering::Relaxed);

    if let Err(e) = montar_webview(hwnd, url) {
        DestroyWindow(hwnd);
        JANELA.store(0, Ordering::Relaxed);
        return Err(e);
    }

    ShowWindow(hwnd, SW_SHOW);
    Ok(())
}

fn registrar_classe() {
    CLASSE.call_once(|| unsafe {
        let inst = GetModuleHandleW(std::ptr::null());
        let classe = wide(CLASS_UI);
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: crate::icone::grande(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            // O fundo escuro do tema, para a janela não piscar branca antes de
            // a página carregar.
            hbrBackground: CreateSolidBrush(0x0010_0c0a),
            lpszMenuName: std::ptr::null(),
            lpszClassName: classe.as_ptr(),
        };
        RegisterClassW(&wc);
    });
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            ajustar_limites(hwnd);
            0
        }
        WM_GETMINMAXINFO => {
            let info = &mut *(l as *mut MINMAXINFO);
            info.ptMinTrackSize.x = 640;
            info.ptMinTrackSize.y = 480;
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            soltar_webview();
            JANELA.store(0, Ordering::Relaxed);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

fn ajustar_limites(hwnd: HWND) {
    let mut rc: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetClientRect(hwnd, &mut rc) } == 0 {
        return;
    }
    if let Some(ctrl) = atual_webview() {
        let limites = RectWeb {
            left: 0,
            top: 0,
            right: rc.right,
            bottom: rc.bottom,
        };
        let _ = unsafe { ctrl.SetBounds(limites) };
    }
}

fn atual_webview() -> Option<ICoreWebView2Controller> {
    WEBVIEW.with(|w| w.borrow().clone())
}

fn soltar_webview() {
    WEBVIEW.with(|w| {
        if let Some(ctrl) = w.borrow_mut().take() {
            let _ = unsafe { ctrl.Close() };
        }
    });
}

fn montar_webview(hwnd: HWND, url: &str) -> Result<(), String> {
    let pasta = HSTRING::from(pasta_webview().to_string_lossy().to_string());
    let padrao = HSTRING::new();

    let ambiente: Rc<RefCell<Option<ICoreWebView2Environment>>> = Rc::new(RefCell::new(None));
    let guarda = ambiente.clone();

    CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe {
            CreateCoreWebView2EnvironmentWithOptions(
                &padrao,
                &pasta,
                None::<&ICoreWebView2EnvironmentOptions>,
                &handler,
            )
        }
        .map_err(Into::into)),
        Box::new(move |erro, env| {
            erro?;
            *guarda.borrow_mut() = env;
            Ok(())
        }),
    )
    .map_err(|e| format!("WebView2 indisponível ({e:?})"))?;

    let env = ambiente
        .borrow()
        .clone()
        .ok_or("WebView2 não devolveu o ambiente")?;
    let env_controller = env.clone();

    let controle: Rc<RefCell<Option<ICoreWebView2Controller>>> = Rc::new(RefCell::new(None));
    let guarda = controle.clone();
    let pai = HwndWeb(hwnd);

    CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe {
            env_controller.CreateCoreWebView2Controller(pai, &handler)
        }
        .map_err(Into::into)),
        Box::new(move |erro, ctrl| {
            erro?;
            *guarda.borrow_mut() = ctrl;
            Ok(())
        }),
    )
    .map_err(|e| format!("WebView2 não criou o controlador ({e:?})"))?;

    let ctrl = controle
        .borrow()
        .clone()
        .ok_or("WebView2 não devolveu o controlador")?;

    unsafe {
        let mut rc: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rc);
        let _ = ctrl.SetBounds(RectWeb {
            left: 0,
            top: 0,
            right: rc.right,
            bottom: rc.bottom,
        });

        if let Ok(c2) = ctrl.cast::<ICoreWebView2Controller2>() {
            let _ = c2.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR {
                A: 255,
                R: 10,
                G: 12,
                B: 16,
            });
        }

        let _ = ctrl.SetIsVisible(true);

        let wv = ctrl
            .CoreWebView2()
            .map_err(|e| format!("WebView2 sem página ({e:?})"))?;
        if let Ok(s) = wv.Settings() {
            // Janela de programa, não de navegador: sem menu de contexto da
            // web, sem ferramentas de desenvolvedor, sem barra de status e
            // sem zoom por Ctrl+roda.
            let _ = s.SetAreDefaultContextMenusEnabled(false);
            let _ = s.SetAreDevToolsEnabled(false);
            let _ = s.SetIsStatusBarEnabled(false);
            let _ = s.SetIsZoomControlEnabled(false);
        }

        wv.Navigate(&HSTRING::from(url))
            .map_err(|e| format!("WebView2 não navegou ({e:?})"))?;
    }

    WEBVIEW.with(|w| *w.borrow_mut() = Some(ctrl));
    Ok(())
}

fn pasta_webview() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let pasta = base.join("ZoneSnap").join("webview");
    let _ = std::fs::create_dir_all(&pasta);
    pasta
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
