//! Embute o ícone no executável.
//!
//! Explorer, barra de tarefas e o diálogo de propriedades só mostram um ícone
//! próprio se ele estiver nos recursos do .exe, e recurso se faz ligando um
//! .res ao executável. O .rc é escrito aqui no OUT_DIR com o caminho absoluto do
//! .ico e compilado pelo rc.exe do Windows SDK.
//!
//! Máquina sem o SDK (ou alvo que não seja MSVC): o build segue sem ícone
//! embutido, com um aviso, e o programa usa o ícone genérico. Nada mais depende
//! disso.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=res/zonesnap.ico");

    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let raiz = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(v) => PathBuf::from(v),
        Err(_) => return,
    };
    let ico = raiz.join("res").join("zonesnap.ico");
    if !ico.exists() {
        println!("cargo:warning=res/zonesnap.ico não existe: rode `cargo run --release --example icone`");
        return;
    }
    let rc = match rc_exe() {
        Some(rc) => rc,
        None => {
            println!("cargo:warning=sem rc.exe do Windows SDK: o executável fica sem ícone embutido");
            return;
        }
    };
    let out = match std::env::var("OUT_DIR") {
        Ok(v) => PathBuf::from(v),
        Err(_) => return,
    };

    // O rc.exe trata barra invertida como escape dentro da string, então o
    // caminho vai com barras normais.
    let caminho = ico.display().to_string().replace('\\', "/");
    let fonte = out.join("icone.rc");
    if std::fs::write(&fonte, format!("1 ICON \"{caminho}\"\n")).is_err() {
        return;
    }
    let res = out.join("icone.res");
    let saida = Command::new(&rc)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res)
        .arg(&fonte)
        .status();
    match saida {
        Ok(s) if s.success() && res.exists() => {
            println!("cargo:rustc-link-arg-bins={}", res.display());
        }
        Ok(s) => println!("cargo:warning=rc.exe saiu com {s}; executável sem ícone"),
        Err(e) => println!("cargo:warning=não consegui rodar rc.exe: {e}"),
    }
}

/// O rc.exe mais novo dos Windows Kits instalados.
fn rc_exe() -> Option<PathBuf> {
    let mut achados: Vec<(u64, PathBuf)> = Vec::new();
    for base in ["ProgramFiles(x86)", "ProgramFiles"] {
        let Ok(base) = std::env::var(base) else {
            continue;
        };
        let bin = Path::new(&base).join("Windows Kits").join("10").join("bin");
        let Ok(entradas) = std::fs::read_dir(&bin) else {
            continue;
        };
        for entrada in entradas.flatten() {
            let rc = entrada.path().join("x64").join("rc.exe");
            if rc.is_file() {
                let versao = entrada
                    .file_name()
                    .to_string_lossy()
                    .replace('.', "")
                    .parse::<u64>()
                    .unwrap_or(0);
                achados.push((versao, rc));
            }
        }
    }
    achados.sort();
    achados.pop().map(|(_, rc)| rc)
}
