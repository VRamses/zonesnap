//! Modo vídeo: esconde a moldura do navegador.
//!
//! O Opera, o Chrome, o Edge e o Brave desenham as abas e a barra de endereço
//! **dentro** da área cliente, então tirar a legenda do Windows não esconde
//! nada, e a tela cheia do navegador (F11) sempre toma o monitor inteiro. O que
//! funciona é medir a moldura pela janela filha que desenha a página
//! (`Chrome_RenderWidgetHostHWND`): a distância dela até a janela é exatamente
//! a altura das abas mais a barra de endereço. Aí a janela é empurrada para
//! cima por essa altura, com `y` negativo, e a página começa no topo do
//! monitor. O navegador continua inteiro, com as abas fora do campo de visão,
//! e um segundo toque devolve retângulo e estilo originais.
//!
//! Veio do `janelas.ahk`, onde rodava com `Ctrl+Alt+V`.

use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetAncestor, GetClassNameW, GetCursorPos, GetForegroundWindow,
    GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
    IsZoomed, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, WindowFromPoint,
    GA_ROOT, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSENDCHANGING,
    SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_RESTORE, WS_CAPTION, WS_MAXIMIZE, WS_THICKFRAME,
};

/// O que era a janela antes de entrar no modo vídeo, para devolver igual.
struct Salvo {
    estilo: isize,
    janela: RECT,
    processo: String,
    moldura: i32,
}

/// Uma entrada por janela no modo vídeo. Poucas janelas ao mesmo tempo, então
/// uma lista basta e evita depender de tabela de hash em memória estática.
static SALVOS: Mutex<Vec<(isize, Salvo)>> = Mutex::new(Vec::new());

/// Retângulo da página encontrado pela enumeração. A enumeração do Windows não
/// aceita contexto, então o resultado passa por aqui.
static PAGINA: Mutex<Option<RECT>> = Mutex::new(None);

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

unsafe extern "system" fn procurar_pagina(filho: HWND, _dados: LPARAM) -> i32 {
    let mut cls = [0u16; 128];
    let n = GetClassNameW(filho, cls.as_mut_ptr(), cls.len() as i32);
    if n <= 0 {
        return 1;
    }
    if String::from_utf16_lossy(&cls[..n as usize]) != "Chrome_RenderWidgetHostHWND" {
        return 1;
    }
    if IsWindowVisible(filho) == 0 {
        return 1;
    }
    let mut r: RECT = std::mem::zeroed();
    if GetWindowRect(filho, &mut r) == 0 {
        return 1;
    }
    // A página de uma janela de verdade tem centenas de pixels; filho pequeno
    // é outro pedaço do Chromium (barra de status, sombra) e não serve.
    if r.right - r.left < 200 || r.bottom - r.top < 200 {
        return 1;
    }
    if let Ok(mut p) = PAGINA.lock() {
        *p = Some(r);
    }
    0
}

/// Retângulo da página dentro da janela do navegador, ou `None` quando a
/// janela não é de um navegador Chromium.
fn pagina(hwnd: isize) -> Option<RECT> {
    if let Ok(mut p) = PAGINA.lock() {
        *p = None;
    }
    unsafe {
        EnumChildWindows(hwnd as HWND, Some(procurar_pagina), 0);
    }
    PAGINA.lock().ok().and_then(|p| *p)
}

fn posicionar(hwnd: isize, x: i32, y: i32, w: i32, h: i32) -> bool {
    // SWP_NOSENDCHANGING é o que impede o Chromium de limitar a altura da
    // janela ao tamanho do monitor, o que deixaria uma faixa descoberta no pé
    // da tela.
    (unsafe {
        SetWindowPos(
            hwnd as HWND,
            std::ptr::null_mut(),
            x,
            y,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOSENDCHANGING,
        )
    }) != 0
}

fn repintar_estilo(hwnd: isize, estilo: isize) {
    unsafe {
        SetWindowLongPtrW(hwnd as HWND, GWL_STYLE, estilo);
        SetWindowPos(
            hwnd as HWND,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

pub fn quantidade() -> usize {
    SALVOS.lock().map(|v| v.len()).unwrap_or(0)
}

fn em_modo(hwnd: isize) -> bool {
    SALVOS
        .lock()
        .map(|v| v.iter().any(|(h, _)| *h == hwnd))
        .unwrap_or(false)
}

/// A janela em foco, se for navegador; senão a janela sob o cursor; senão o
/// maior navegador aberto. O último caso é o que faz o botão da interface
/// funcionar: quem clica nele põe a própria interface em foco.
fn alvo() -> isize {
    let ativa = unsafe { GetForegroundWindow() } as isize;
    if ativa != 0 && pagina(ativa).is_some() {
        return ativa;
    }
    let mut pt = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut pt);
    }
    let sob = unsafe { WindowFromPoint(pt) };
    if !sob.is_null() {
        let raiz = unsafe { GetAncestor(sob, GA_ROOT) } as isize;
        if raiz != 0 && pagina(raiz).is_some() {
            return raiz;
        }
    }
    let maior = navegador_maior();
    if maior != 0 {
        return maior;
    }
    ativa
}

/// Navegadores que usam a mesma janela de página do Chromium.
const NAVEGADORES: [&str; 5] = [
    "opera.exe",
    "chrome.exe",
    "msedge.exe",
    "brave.exe",
    "vivaldi.exe",
];

fn eh_navegador(processo: &str) -> bool {
    NAVEGADORES.iter().any(|n| n.eq_ignore_ascii_case(processo))
}

/// Traz a janela para a frente. O Chromium só mantém a janela da página
/// visível quando a janela está na frente: em segundo plano ela some, e sem
/// isso o botão da interface nunca acharia a página, porque quem clica nele põe
/// a própria interface em foco.
fn focar(hwnd: isize) {
    unsafe {
        let atual = GetForegroundWindow();
        let mut pid = 0u32;
        let alvo = GetWindowThreadProcessId(hwnd as HWND, &mut pid);
        let frente = GetWindowThreadProcessId(atual, std::ptr::null_mut());
        let meu = GetCurrentThreadId();
        if frente != 0 {
            AttachThreadInput(meu, frente, 1);
        }
        if alvo != 0 {
            AttachThreadInput(meu, alvo, 1);
        }
        SetForegroundWindow(hwnd as HWND);
        if alvo != 0 {
            AttachThreadInput(meu, alvo, 0);
        }
        if frente != 0 {
            AttachThreadInput(meu, frente, 0);
        }
    }
}

fn navegador_maior() -> isize {
    let mut melhor = 0isize;
    let mut area = 0i64;
    for proc in NAVEGADORES {
        let Some(h) = crate::jogos::maior_janela(proc) else {
            continue;
        };
        let Some(r) = rect_da_janela(h) else {
            continue;
        };
        let a = (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
        if a > area {
            area = a;
            melhor = h;
        }
    }
    melhor
}

/// Alterna o modo vídeo na janela em foco.
pub fn alternar() -> Result<String, String> {
    let hwnd = alvo();
    if hwnd == 0 {
        return Err("não achei janela para o modo vídeo".to_string());
    }
    alternar_janela(hwnd)
}

/// Alterna o modo vídeo numa janela específica.
pub fn alternar_janela(hwnd: isize) -> Result<String, String> {
    if hwnd == 0 {
        return Err("janela inválida".to_string());
    }
    if em_modo(hwnd) {
        sair(hwnd)
    } else {
        entrar(hwnd)
    }
}

fn entrar(hwnd: isize) -> Result<String, String> {
    if unsafe { IsWindow(hwnd as HWND) } == 0 {
        return Err("janela inválida".to_string());
    }
    if crate::jogos::monitor_virtual(hwnd) {
        return Err("essa janela está no monitor virtual do Sunshine; não mexo".to_string());
    }
    // Maximizada não tem moldura para medir: volta ao tamanho normal primeiro.
    if unsafe { IsZoomed(hwnd as HWND) } != 0 {
        unsafe {
            ShowWindow(hwnd as HWND, SW_RESTORE);
        }
        sleep(Duration::from_millis(140));
    }
    let mon = crate::jogos::monitor_da_janela(hwnd)
        .ok_or_else(|| "não achei o monitor da janela".to_string())?;

    let processo = crate::jogos::processo_da_janela(hwnd);
    let mut pg = match pagina(hwnd) {
        Some(p) => p,
        None if eh_navegador(&processo) => {
            focar(hwnd);
            sleep(Duration::from_millis(200));
            pagina(hwnd).ok_or_else(|| {
                "não achei a página do navegador; traga a janela para a frente e tente de novo".to_string()
            })?
        }
        None => {
            return Err(
                "essa janela não tem página de navegador Chromium; o atalho serve para Opera, Chrome, Edge e Brave"
                    .to_string(),
            )
        }
    };
    let estilo = unsafe { GetWindowLongPtrW(hwnd as HWND, GWL_STYLE) };
    let mut janela = rect_da_janela(hwnd).ok_or_else(|| "sem retângulo".to_string())?;

    // A legenda do Windows atrapalha a medida: sai antes, volta na saída.
    let tinha_legenda = estilo & (WS_CAPTION as isize) != 0;
    if tinha_legenda {
        repintar_estilo(hwnd, estilo & !flag_removida());
        sleep(Duration::from_millis(150));
        pg = pagina(hwnd).ok_or_else(|| "perdi a página da janela".to_string())?;
        janela = rect_da_janela(hwnd).ok_or_else(|| "sem retângulo".to_string())?;
    }

    let dx = pg.left - janela.left;
    let dy = pg.top - janela.top;
    let direita = janela.right - pg.right;
    let baixo = janela.bottom - pg.bottom;

    if !(10..=300).contains(&dy) {
        if tinha_legenda {
            repintar_estilo(hwnd, estilo);
        }
        let cobre = janela.left <= mon.left
            && janela.top <= mon.top
            && janela.right >= mon.right
            && janela.bottom >= mon.bottom;
        if cobre {
            return Err(
                "essa janela está em tela cheia do navegador (F11); aperte F11 para sair e use o atalho de novo"
                    .to_string(),
            );
        }
        return Err(format!("medida estranha da moldura ({dy} px); não mexi"));
    }

    // A página passa a ocupar do topo do monitor até o pé da janela.
    let largura_pagina = janela.right - janela.left;
    let altura_pagina = janela.bottom - mon.top;

    if let Ok(mut v) = SALVOS.lock() {
        v.retain(|(h, _)| *h != hwnd);
        v.push((
            hwnd,
            Salvo {
                estilo,
                janela,
                processo,
                moldura: dy,
            },
        ));
    }

    let ok = posicionar(
        hwnd,
        janela.left - dx,
        mon.top - dy,
        largura_pagina + dx + direita,
        altura_pagina + dy + baixo,
    );
    if !ok {
        if let Ok(mut v) = SALVOS.lock() {
            v.retain(|(h, _)| *h != hwnd);
        }
        if tinha_legenda {
            repintar_estilo(hwnd, estilo);
        }
        return Err("o Windows recusou o movimento".to_string());
    }
    Ok(format!(
        "moldura escondida ({dy} px); a página ficou em {largura_pagina}x{altura_pagina}"
    ))
}

fn sair(hwnd: isize) -> Result<String, String> {
    let salvo = match SALVOS.lock() {
        Ok(mut v) => v
            .iter()
            .position(|(h, _)| *h == hwnd)
            .map(|i| v.remove(i).1),
        Err(_) => None,
    };
    let Some(s) = salvo else {
        return Err("essa janela não está no modo vídeo".to_string());
    };
    if unsafe { IsWindow(hwnd as HWND) } == 0 {
        return Ok("a janela já tinha sido fechada".to_string());
    }
    let j = s.janela;
    posicionar(hwnd, j.left, j.top, j.right - j.left, j.bottom - j.top);
    if s.estilo & (WS_CAPTION as isize) != 0 {
        repintar_estilo(hwnd, s.estilo);
    }
    Ok(format!(
        "moldura de volta ({} px): {} em {}x{}",
        s.moldura,
        s.processo,
        j.right - j.left,
        j.bottom - j.top
    ))
}

/// Devolve todas as janelas ao normal. Chamado ao sair do programa, para não
/// deixar navegador empurrado para fora da tela.
pub fn soltar_tudo() -> usize {
    let lista: Vec<(isize, Salvo)> = match SALVOS.lock() {
        Ok(mut v) => v.drain(..).collect(),
        Err(_) => return 0,
    };
    let mut n = 0;
    for (hwnd, s) in lista {
        if unsafe { IsWindow(hwnd as HWND) } == 0 {
            continue;
        }
        let j = s.janela;
        posicionar(hwnd, j.left, j.top, j.right - j.left, j.bottom - j.top);
        if s.estilo & (WS_CAPTION as isize) != 0 {
            repintar_estilo(hwnd, s.estilo);
        }
        n += 1;
    }
    n
}
