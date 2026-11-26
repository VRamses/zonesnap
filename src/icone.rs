//! O ícone do ZoneSnap dentro do processo.
//!
//! O desenho mora em `examples/icone.rs`, que grava `res/zonesnap.ico`; o
//! `build.rs` embute esse arquivo no executável como grupo de ícones 1. Aqui só
//! se pede ao Windows o tamanho certo: a bandeja usa o ícone pequeno da métrica
//! do sistema, e a janela e o Alt+Tab usam o grande. O Windows escolhe dentro do
//! .ico a entrada mais próxima do tamanho pedido.
//!
//! Sem o recurso embutido (build feita sem o rc.exe do Windows SDK), cai no
//! ícone genérico em vez de ficar sem ícone nenhum.

use std::ffi::c_void;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, LoadIconW, LoadImageW, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR,
    SM_CXICON, SM_CXSMICON,
};

/// Id do grupo de ícones no recurso (o `build.rs` gera o .rc com esse id).
const ID: usize = 1;

/// Ícone no tamanho da bandeja.
pub fn pequeno() -> *mut c_void {
    unsafe { icone(GetSystemMetrics(SM_CXSMICON)) }
}

/// Ícone no tamanho da janela, do Alt+Tab e da barra de tarefas.
pub fn grande() -> *mut c_void {
    unsafe { icone(GetSystemMetrics(SM_CXICON)) }
}

unsafe fn icone(tamanho: i32) -> *mut c_void {
    let inst = GetModuleHandleW(std::ptr::null());
    let h = LoadImageW(
        inst,
        ID as *const u16,
        IMAGE_ICON,
        tamanho,
        tamanho,
        LR_DEFAULTCOLOR,
    );
    if !h.is_null() {
        return h;
    }
    crate::log(&format!(
        "icone: o recurso de {tamanho} px não carregou; usando o genérico"
    ));
    LoadIconW(std::ptr::null_mut(), IDI_APPLICATION)
}
