//! Receita do Civilization V.
//!
//! O jogo lê `FullScreen`, `WindowResX` e `WindowResY` do
//! `GraphicsSettingsDX11.ini` só na inicialização e regrava o arquivo ao sair,
//! então a edição tem que acontecer com o jogo fechado. Sem isso, o cliente
//! nasce maior que a zona e o modo `alinhar_cliente` recusa o encaixe.

use std::fs;
use std::path::PathBuf;

/// Pasta "Documentos" de verdade: o Windows pode tê-la mudado de lugar
/// (OneDrive, pasta localizada), então não dá para montar o caminho na mão.
fn documentos() -> Option<PathBuf> {
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{SHGetKnownFolderPath, FOLDERID_Documents};
    let mut p: *mut u16 = std::ptr::null_mut();
    let hr = unsafe { SHGetKnownFolderPath(&FOLDERID_Documents, 0, std::ptr::null_mut(), &mut p) };
    if hr < 0 || p.is_null() {
        return None;
    }
    let mut n = 0usize;
    unsafe {
        while *p.add(n) != 0 {
            n += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
        CoTaskMemFree(p as *mut core::ffi::c_void);
        Some(PathBuf::from(s))
    }
}

fn pasta_jogo() -> Option<PathBuf> {
    let base = documentos()?;
    Some(base.join("My Games").join("Sid Meier's Civilization 5"))
}

pub fn ini_path() -> Option<PathBuf> {
    Some(pasta_jogo()?.join("GraphicsSettingsDX11.ini"))
}

fn processo_aberto() -> bool {
    crate::jogos::maior_janela("civilizationv_dx11.exe").is_some()
        || crate::jogos::maior_janela("civilizationv.exe").is_some()
}

fn valor_da_linha(linha: &str, chave: &str) -> Option<String> {
    let t = linha.trim();
    let (k, v) = t.split_once('=')?;
    if k.trim().eq_ignore_ascii_case(chave) {
        Some(v.trim().to_string())
    } else {
        None
    }
}

fn trocar_linha(linha: &str, chave: &str, valor: &str) -> Option<String> {
    valor_da_linha(linha, chave)?;
    let pos = linha.find('=')?;
    let esquerda = &linha[..pos];
    Some(format!("{esquerda}= {valor}"))
}

/// Ajusta o ini para o Civ V nascer em janela com o cliente igual à zona.
/// Devolve a mensagem do que foi feito, com o caminho do backup.
pub fn preparar(largura: i32, altura: i32) -> Result<String, String> {
    if processo_aberto() {
        return Err(
            "o Civilization V está aberto. Feche o jogo e chame de novo: ele regrava o ini ao sair."
                .to_string(),
        );
    }
    let ini = ini_path().ok_or_else(|| "pasta do usuário não encontrada".to_string())?;
    crate::log(&format!("civ: ini {}", ini.display()));
    if !ini.exists() {
        return Err(format!(
            "não achei {}. Abra o jogo uma vez em modo janela para ele criar o arquivo.",
            ini.display()
        ));
    }
    let texto = fs::read_to_string(&ini).map_err(|e| format!("falha ao ler o ini: {e}"))?;
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = ini.with_extension(format!("ini.bak-{epoch}"));

    let mut linhas: Vec<String> = texto.lines().map(|l| l.to_string()).collect();
    let mut achou_full = false;
    let mut achou_x = false;
    let mut achou_y = false;
    let mut indice_full = 0usize;
    for (i, l) in linhas.iter_mut().enumerate() {
        if let Some(nova) = trocar_linha(l, "FullScreen", "0") {
            *l = nova;
            achou_full = true;
            indice_full = i;
        }
        if let Some(nova) = trocar_linha(l, "WindowResX", &largura.to_string()) {
            *l = nova;
            achou_x = true;
        }
        if let Some(nova) = trocar_linha(l, "WindowResY", &altura.to_string()) {
            *l = nova;
            achou_y = true;
        }
    }
    if !achou_full {
        return Err("o ini não tem a chave FullScreen; não sei onde mexer".to_string());
    }
    let mut extras: Vec<String> = Vec::new();
    if !achou_x {
        extras.push(format!("WindowResX = {largura}"));
    }
    if !achou_y {
        extras.push(format!("WindowResY = {altura}"));
    }
    if !extras.is_empty() {
        let mut saida: Vec<String> = Vec::with_capacity(linhas.len() + extras.len());
        for (i, l) in linhas.iter().enumerate() {
            saida.push(l.clone());
            if i == indice_full {
                saida.extend(extras.iter().cloned());
            }
        }
        linhas = saida;
    }

    fs::copy(&ini, &backup).map_err(|e| format!("falha ao criar o backup: {e}"))?;
    let novo = linhas.join("\r\n") + "\r\n";
    fs::write(&ini, novo).map_err(|e| format!("falha ao gravar o ini: {e}"))?;
    Ok(format!(
        "Civ V em janela: FullScreen=0, WindowResX={largura}, WindowResY={altura}. Backup em {}.",
        backup.display()
    ))
}
