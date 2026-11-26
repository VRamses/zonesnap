//! Desenha o ícone do ZoneSnap e grava `res/zonesnap.ico`.
//!
//!     cargo run --release --example icone
//!
//! O desenho é vetorial e refeito em cada tamanho, em vez de reduzir uma imagem
//! grande: assim o traço acompanha o tamanho, e o ícone de 16 px, que é o da
//! bandeja, sai com contorno de 1 px em vez de borrado.
//!
//! Com `--previa` também grava `target/previa-icone.png`, com os tamanhos
//! pequenos ampliados quatro vezes e os tamanhos reais embaixo, só para conferir.

use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};

/// Tamanhos guardados no .ico. A bandeja usa 16 e 32; o Explorer usa até 256.
const TAMANHOS: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

// Cores da interface (ui/src/styles/app.css).
const FUNDO: (u8, u8, u8) = (0x10, 0x14, 0x1b);
const CLARO: (u8, u8, u8) = (0xe9, 0xee, 0xf6);
const ACENTO: (u8, u8, u8) = (0x3b, 0x7b, 0xf6);
const APAGADO: (u8, u8, u8) = (0x2b, 0x36, 0x48);

fn main() {
    let raiz = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = raiz.join("res");
    std::fs::create_dir_all(&dir).expect("criar res");

    let mut imagens = Vec::new();
    for t in TAMANHOS {
        let pm = desenhar(t);
        // O de 256 vira PNG: em BMP cru pesaria 262 KB sozinho, e o Explorer é
        // quem lê esse tamanho.
        if t >= 128 {
            imagens.push((t, pm.encode_png().expect("png"), true));
        } else {
            imagens.push((t, dib(&pm), false));
        }
    }
    let ico = montar_ico(&imagens);
    let destino = dir.join("zonesnap.ico");
    std::fs::write(&destino, &ico).expect("gravar ico");
    println!(
        "res/zonesnap.ico: {} bytes, {} tamanhos",
        ico.len(),
        imagens.len()
    );

    if std::env::args().any(|a| a == "--previa") {
        let previa = previa_de(&TAMANHOS[..7]);
        let dir = raiz.join("target");
        std::fs::create_dir_all(&dir).ok();
        let caminho = dir.join("previa-icone.png");
        std::fs::write(&caminho, previa.encode_png().expect("png")).expect("gravar previa");
        println!("target/previa-icone.png: {}x{}", previa.width(), previa.height());
    }
}

/// O ícone: quadrado escuro com a "tela" clara dividida em duas zonas, a da
/// esquerda acesa. É a ideia do programa em dois elementos.
fn desenhar(t: u32) -> Pixmap {
    let mut pm = Pixmap::new(t, t).expect("pixmap");
    let f = t as f32;
    let mut p = Paint::default();
    p.anti_alias = true;

    // Em 16 e 20 px a moldura come metade do desenho e o cinza da zona apagada
    // some no fundo escuro. Nesses tamanhos a folga encolhe e o cinza clareia;
    // do 24 para cima o desenho é o mesmo, com a cor da interface.
    let pequeno = t < 24;
    let apagado = if pequeno {
        (0x4a, 0x5c, 0x78)
    } else {
        APAGADO
    };

    // fundo arredondado, com 1 px de folga nas bordas do ícone
    let m = (0.015 * f).max(0.5);
    let fundo = rounded_rect(m, m, f - m, f - m, 0.21 * f).expect("fundo");
    cor(&mut p, FUNDO, 255);
    pm.fill_path(&fundo, &p, FillRule::Winding, Transform::identity(), None);

    // a tela, clara, que vira a moldura das zonas
    let a = (if pequeno { 0.13 } else { 0.18 }) * f;
    let b = f - a;
    let tela = rounded_rect(a, a, b, b, 0.07 * f).expect("tela");
    cor(&mut p, CLARO, 255);
    pm.fill_path(&tela, &p, FillRule::Winding, Transform::identity(), None);

    // duas zonas: 42% e o resto, separadas pela folga da moldura
    let g = (if pequeno { 0.05 } else { 0.055 }) * f;
    let xm = a + 0.42 * (b - a);
    let r = 0.035 * f;
    let esquerda = rounded_rect(a + g, a + g, xm - g * 0.5, b - g, r).expect("zona esquerda");
    cor(&mut p, ACENTO, 255);
    pm.fill_path(&esquerda, &p, FillRule::Winding, Transform::identity(), None);
    let direita = rounded_rect(xm + g * 0.5, a + g, b - g, b - g, r).expect("zona direita");
    cor(&mut p, apagado, 255);
    pm.fill_path(&direita, &p, FillRule::Winding, Transform::identity(), None);

    pm
}

fn cor(p: &mut Paint, (r, g, b): (u8, u8, u8), a: u8) {
    p.set_color_rgba8(r, g, b, a);
}

fn rounded_rect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + r, y0);
    pb.line_to(x1 - r, y0);
    pb.quad_to(x1, y0, x1, y0 + r);
    pb.line_to(x1, y1 - r);
    pb.quad_to(x1, y1, x1 - r, y1);
    pb.line_to(x0 + r, y1);
    pb.quad_to(x0, y1, x0, y1 - r);
    pb.line_to(x0, y0 + r);
    pb.quad_to(x0, y0, x0 + r, y0);
    pb.close();
    pb.finish()
}

/// Entrada de bitmap no formato que o Windows espera dentro do .ico: cabeçalho
/// de 40 bytes, pixels de baixo para cima em BGRA e a máscara AND zerada (quem
/// manda é o canal alfa).
fn dib(pm: &Pixmap) -> Vec<u8> {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let mascara_linha = (w + 31) / 32 * 4;
    let mut out = Vec::with_capacity(40 + w * h * 4 + mascara_linha * h);
    out.extend_from_slice(&40u32.to_le_bytes()); // tamanho do cabeçalho
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&((h * 2) as i32).to_le_bytes()); // XOR + AND
    out.extend_from_slice(&1u16.to_le_bytes()); // planos
    out.extend_from_slice(&32u16.to_le_bytes()); // bits por pixel
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&((w * h * 4 + mascara_linha * h) as u32).to_le_bytes());
    for _ in 0..4 {
        out.extend_from_slice(&0u32.to_le_bytes());
    }
    let px = pm.data();
    for y in (0..h).rev() {
        for x in 0..w {
            let i = (y * w + x) * 4;
            out.extend_from_slice(&[px[i + 2], px[i + 1], px[i], px[i + 3]]);
        }
    }
    out.resize(out.len() + mascara_linha * h, 0);
    out
}

fn montar_ico(imagens: &[(u32, Vec<u8>, bool)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // reservado
    out.extend_from_slice(&1u16.to_le_bytes()); // 1 = ícone
    out.extend_from_slice(&(imagens.len() as u16).to_le_bytes());

    let mut deslocamento = 6 + 16 * imagens.len();
    for (t, bytes, _) in imagens {
        let lado = if *t >= 256 { 0 } else { *t as u8 };
        out.push(lado);
        out.push(lado);
        out.push(0); // paleta
        out.push(0); // reservado
        out.extend_from_slice(&1u16.to_le_bytes()); // planos
        out.extend_from_slice(&32u16.to_le_bytes()); // bits por pixel
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&(deslocamento as u32).to_le_bytes());
        deslocamento += bytes.len();
    }
    for (_, bytes, _) in imagens {
        out.extend_from_slice(bytes);
    }
    out
}

/// Folha de conferência: os tamanhos pequenos ampliados quatro vezes em cima e
/// os tamanhos reais embaixo, sobre o cinza do Windows e sobre o azul da barra.
fn previa_de(tamanhos: &[u32]) -> Pixmap {
    let largura: u32 = tamanhos.iter().map(|t| t * 4 + 8).sum::<u32>() + 8;
    let linha1 = tamanhos.iter().max().copied().unwrap_or(0) * 4;
    let linha2 = tamanhos.iter().max().copied().unwrap_or(0);
    let altura = 8 + linha1 + 12 + linha2 + 8;
    let mut pm = Pixmap::new(largura, altura).expect("previa");
    let mut p = Paint::default();
    p.anti_alias = false;
    cor(&mut p, (0x20, 0x20, 0x20), 255);
    let tudo = rounded_rect(0.0, 0.0, largura as f32, altura as f32, 0.0).expect("fundo");
    pm.fill_path(&tudo, &p, FillRule::Winding, Transform::identity(), None);

    // faixa azul da barra de tarefas, atrás dos tamanhos reais
    let azul = rounded_rect(
        0.0,
        (8 + linha1 + 12) as f32,
        largura as f32,
        (8 + linha1 + 12 + linha2) as f32,
        0.0,
    )
    .expect("barra");
    cor(&mut p, (0x2b, 0x3a, 0x55), 255);
    pm.fill_path(&azul, &p, FillRule::Winding, Transform::identity(), None);

    let mut x = 8u32;
    for t in tamanhos {
        let icone = desenhar(*t);
        colar(&mut pm, &icone, x, 8, 4);
        colar(&mut pm, &icone, x, 8 + linha1 + 12, 1);
        x += t * 4 + 8;
    }
    pm
}

fn colar(alvo: &mut Pixmap, fonte: &Pixmap, x: u32, y: u32, escala: u32) {
    let (w, h) = (fonte.width(), fonte.height());
    let px = fonte.data();
    let (aw, ah) = (alvo.width(), alvo.height());
    let destino = alvo.data_mut();
    for yy in 0..h * escala {
        for xx in 0..w * escala {
            let (sx, sy) = (xx / escala, yy / escala);
            let i = ((sy * w + sx) * 4) as usize;
            let a = px[i + 3] as u32;
            if a == 0 {
                continue;
            }
            let (dx, dy) = (x + xx, y + yy);
            if dx >= aw || dy >= ah {
                continue;
            }
            let j = ((dy * aw + dx) * 4) as usize;
            for c in 0..3 {
                destino[j + c] =
                    ((px[i + c] as u32 * a + destino[j + c] as u32 * (255 - a)) / 255) as u8;
            }
            destino[j + 3] = 255;
        }
    }
}
