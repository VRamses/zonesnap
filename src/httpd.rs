//! Servidor local da interface.
//!
//! Escuta só em 127.0.0.1, exige o token da sessão no cabeçalho
//! `X-ZoneSnap` e serve a página embutida no executável. O token e a porta
//! ficam em `%APPDATA%\ZoneSnap` para que uma segunda instância consiga
//! reabrir a interface em vez de subir outro servidor.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HTML: &str = include_str!("../ui/dist/index.html");

fn misturar(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

pub fn novo_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id() as u64;
    format!("{:016x}{:016x}", misturar(nanos), misturar(nanos ^ pid))
}

fn gravar(nome: &str, texto: &str) {
    let dir = crate::config::config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(nome), texto);
}

pub fn ler_arquivo(nome: &str) -> Option<String> {
    std::fs::read_to_string(crate::config::config_dir().join(nome))
        .ok()
        .map(|s| s.trim().to_string())
}

/// Sobe o servidor e devolve (porta, token).
pub fn iniciar() -> Option<(u16, String)> {
    let token = novo_token();
    let mut escolhida = 0u16;
    let mut listener = None;
    for porta in 47121u16..47160 {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, porta));
        if let Ok(l) = TcpListener::bind(addr) {
            escolhida = porta;
            listener = Some(l);
            break;
        }
    }
    let listener = listener?;
    gravar("ui.token", &token);
    gravar("ui.port", &escolhida.to_string());
    let t = token.clone();
    std::thread::spawn(move || {
        for conexao in listener.incoming() {
            if let Ok(s) = conexao {
                let t = t.clone();
                std::thread::spawn(move || atender(s, &t));
            }
        }
    });
    Some((escolhida, token))
}

struct Pedido {
    metodo: String,
    caminho: String,
    headers: HashMap<String, String>,
    corpo: Vec<u8>,
}

fn ler(mut s: &TcpStream) -> Option<Pedido> {
    let mut buf = Vec::new();
    let mut bloco = [0u8; 2048];
    let limite = 1_048_576usize;
    let cabecalho_fim;
    loop {
        let n = s.read(&mut bloco).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&bloco[..n]);
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            cabecalho_fim = p;
            break;
        }
        if buf.len() > 65536 {
            return None;
        }
    }
    let cabecalho = String::from_utf8_lossy(&buf[..cabecalho_fim]).to_string();
    let mut linhas = cabecalho.split("\r\n");
    let primeira = linhas.next()?;
    let mut partes = primeira.split_whitespace();
    let metodo = partes.next()?.to_string();
    let caminho = partes.next()?.to_string();
    let mut headers = HashMap::new();
    for l in linhas {
        if let Some((k, v)) = l.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }
    let mut corpo = buf[cabecalho_fim + 4..].to_vec();
    let tamanho: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if tamanho > limite {
        return None;
    }
    while corpo.len() < tamanho {
        let n = s.read(&mut bloco).ok()?;
        if n == 0 {
            break;
        }
        corpo.extend_from_slice(&bloco[..n]);
    }
    Some(Pedido {
        metodo,
        caminho,
        headers,
        corpo,
    })
}

fn responder(s: &mut TcpStream, codigo: u16, tipo: &str, corpo: &[u8]) {
    let motivo = match codigo {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Erro",
    };
    let cabecalho = format!(
        "HTTP/1.1 {codigo} {motivo}\r\nContent-Type: {tipo}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        corpo.len()
    );
    let _ = s.write_all(cabecalho.as_bytes());
    let _ = s.write_all(corpo);
    let _ = s.flush();
}

fn json(s: &mut TcpStream, codigo: u16, v: serde_json::Value) {
    responder(
        s,
        codigo,
        "application/json; charset=utf-8",
        v.to_string().as_bytes(),
    );
}

fn atender(mut s: TcpStream, token: &str) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(8)));
    let Some(p) = ler(&s) else { return };
    let caminho = p.caminho.split('?').next().unwrap_or("/").to_string();

    if caminho == "/" || caminho == "/index.html" {
        responder(&mut s, 200, "text/html; charset=utf-8", HTML.as_bytes());
        return;
    }
    if !caminho.starts_with("/api/") {
        responder(&mut s, 404, "text/plain; charset=utf-8", b"nao encontrado");
        return;
    }
    let enviado = p.headers.get("x-zonesnap").map(|v| v.as_str()).unwrap_or("");
    if enviado != token {
        json(
            &mut s,
            403,
            serde_json::json!({"ok": false, "erro": "token inválido"}),
        );
        return;
    }

    match (p.metodo.as_str(), caminho.as_str()) {
        ("GET", "/api/estado") => json(&mut s, 200, crate::estado_json()),
        ("GET", "/api/config") => json(&mut s, 200, crate::config_json()),
        ("POST", "/api/config") => {
            let texto = String::from_utf8_lossy(&p.corpo).to_string();
            match serde_json::from_str::<serde_json::Value>(&texto) {
                Ok(v) => match serde_json::from_value::<crate::config::Config>(v) {
                    Ok(mut cfg) => {
                        cfg.sanitize();
                        let caminho_cfg = crate::config::default_config_path();
                        if let Err(e) = crate::config::save(&caminho_cfg, &cfg) {
                            json(
                                &mut s,
                                200,
                                serde_json::json!({"ok": false, "erro": format!("falha ao gravar: {e}")}),
                            );
                            return;
                        }
                        crate::recarregar();
                        json(
                            &mut s,
                            200,
                            serde_json::json!({"ok": true, "estado": crate::estado_json()}),
                        );
                    }
                    Err(e) => json(
                        &mut s,
                        200,
                        serde_json::json!({"ok": false, "erro": format!("config inválida: {e}")}),
                    ),
                },
                Err(e) => json(
                    &mut s,
                    200,
                    serde_json::json!({"ok": false, "erro": format!("JSON inválido: {e}")}),
                ),
            }
        }
        ("POST", "/api/acao") => {
            let v: serde_json::Value =
                serde_json::from_slice(&p.corpo).unwrap_or(serde_json::json!({}));
            let acao = v.get("acao").and_then(|a| a.as_str()).unwrap_or("");
            let processo = v.get("processo").and_then(|a| a.as_str());
            match crate::executar_acao(acao, processo) {
                Ok(msg) => json(
                    &mut s,
                    200,
                    serde_json::json!({"ok": true, "mensagem": msg, "estado": crate::estado_json()}),
                ),
                Err(e) => json(
                    &mut s,
                    200,
                    serde_json::json!({"ok": false, "erro": e, "estado": crate::estado_json()}),
                ),
            }
        }
        _ => json(
            &mut s,
            405,
            serde_json::json!({"ok": false, "erro": "rota não existe"}),
        ),
    }
}

/// Abre a interface no navegador padrão, reusando a porta e o token gravados.
pub fn abrir() -> Option<String> {
    let porta = ler_arquivo("ui.port")?;
    let token = ler_arquivo("ui.token")?;
    let endereco = format!("http://127.0.0.1:{porta}/?t={token}");
    let op = crate::wide("open");
    let url = crate::wide(&endereco);
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            5,
        );
    }
    Some(endereco)
}
