use serde::de::{self, Deserializer};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Copy, Debug)]
pub struct Zone {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Zone {
    fn new(x: f32, y: f32, w: f32, h: f32) -> Zone {
        Zone { x, y, w, h }
    }
}

impl Serialize for Zone {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        [self.x, self.y, self.w, self.h].serialize(s)
    }
}

impl<'de> Deserialize<'de> for Zone {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Zone, D::Error> {
        let v = <[f32; 4]>::deserialize(d)?;
        if v.iter().any(|n| !n.is_finite()) {
            return Err(de::Error::custom("zona com numero invalido"));
        }
        Ok(Zone::new(v[0], v[1], v[2], v[3]))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub zones: Vec<Zone>,
}

/// Regra por processo: prende a janela de um jogo numa zona fixa do monitor.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Jogo {
    /// Nome do executável, como o Windows reporta. A comparação ignora caixa.
    pub processo: String,
    pub nome: String,
    /// Largura em pixels. 0 usa `jogo_largura` da configuração.
    pub largura: i32,
    /// Altura em pixels. 0 usa a altura do monitor.
    pub altura: i32,
    pub x: i32,
    pub y: i32,
    /// Reaplica a zona quando a janela se mexe.
    pub vigiar: bool,
    /// Mantém a barra de título.
    pub borda: bool,
    /// Prende pela área cliente, sem mexer no estilo. Para jogo que mede o
    /// cursor pelo tamanho do cliente (Civ V).
    pub alinhar_cliente: bool,
    /// Receita de preparação: "" ou "civ5".
    pub preparar: String,
}

impl Default for Jogo {
    fn default() -> Jogo {
        Jogo {
            processo: String::new(),
            nome: String::new(),
            largura: 0,
            altura: 0,
            x: 0,
            y: 0,
            vigiar: true,
            borda: false,
            alinhar_cliente: false,
            preparar: String::new(),
        }
    }
}

pub fn default_civ() -> Jogo {
    Jogo {
        processo: "CivilizationV_DX11.exe".to_string(),
        nome: "Civilization V (DX11)".to_string(),
        largura: 2300,
        altura: 1440,
        x: 0,
        y: 0,
        vigiar: true,
        borda: false,
        alinhar_cliente: true,
        preparar: "civ5".to_string(),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub enabled: bool,
    pub show_overlay: bool,
    pub gap: i32,
    pub layout: usize,
    pub layouts: Vec<Layout>,
    /// Largura padrão de um perfil de jogo que não define a sua.
    pub jogo_largura: i32,
    pub jogos: Vec<Jogo>,
}

impl Default for Config {
    fn default() -> Config {
        default_config()
    }
}

fn layout(name: &str, zones: &[(f32, f32, f32, f32)]) -> Layout {
    Layout {
        name: name.to_string(),
        zones: zones
            .iter()
            .map(|(x, y, w, h)| Zone::new(*x, *y, *w, *h))
            .collect(),
    }
}

pub fn default_config() -> Config {
    let third = 1.0 / 3.0;
    Config {
        enabled: true,
        show_overlay: true,
        gap: 8,
        layout: 0,
        jogo_largura: 2300,
        jogos: vec![default_civ()],
        layouts: vec![
            layout(
                "Duas metades",
                &[(0.0, 0.0, 0.5, 1.0), (0.5, 0.0, 0.5, 1.0)],
            ),
            layout(
                "Três colunas",
                &[
                    (0.0, 0.0, third, 1.0),
                    (third, 0.0, third, 1.0),
                    (third * 2.0, 0.0, third, 1.0),
                ],
            ),
            layout(
                "Quatro colunas",
                &[
                    (0.0, 0.0, 0.25, 1.0),
                    (0.25, 0.0, 0.25, 1.0),
                    (0.5, 0.0, 0.25, 1.0),
                    (0.75, 0.0, 0.25, 1.0),
                ],
            ),
            layout(
                "Foco 70/30",
                &[(0.0, 0.0, 0.7, 1.0), (0.7, 0.0, 0.3, 1.0)],
            ),
            layout(
                "Foco 60/20/20",
                &[
                    (0.0, 0.0, 0.6, 1.0),
                    (0.6, 0.0, 0.2, 1.0),
                    (0.8, 0.0, 0.2, 1.0),
                ],
            ),
            layout(
                "Centro 25/50/25",
                &[
                    (0.0, 0.0, 0.25, 1.0),
                    (0.25, 0.0, 0.5, 1.0),
                    (0.75, 0.0, 0.25, 1.0),
                ],
            ),
            layout(
                "Grade 2x2",
                &[
                    (0.0, 0.0, 0.5, 0.5),
                    (0.5, 0.0, 0.5, 0.5),
                    (0.0, 0.5, 0.5, 0.5),
                    (0.5, 0.5, 0.5, 0.5),
                ],
            ),
        ],
    }
}

pub const HELP: &str = "Cada zona é uma lista [x, y, largura, altura] em frações de 0 a 1 da área útil do monitor. \
Exemplo: [0, 0, 0.5, 1] é a metade esquerda inteira; [0.5, 0, 0.5, 1] é a metade direita. \
Na tela 3440x1440, cada metade fica com 1720 px; em outra resolução a proporção é a mesma. \
gap é o espaço em pixels entre as janelas (8 = 4 px de respiro em cada lado). \
layout é o índice do layout inicial (0 = primeiro da lista). \
Edite e salve: o app recarrega sozinho em até 2 segundos, sem reiniciar. \
Para voltar ao padrão, apague este arquivo e abra o app de novo.";

pub const HELP_JOGOS: &str = "jogos é a lista de perfis: um bloco por processo, que prende a janela do jogo numa zona fixa \
(por exemplo, 2300x1440 no canto superior esquerdo, com o resto da tela livre para o navegador). \
processo é o nome do executável, como aparece no Gerenciador de Tarefas; a comparação ignora maiúsculas. \
largura/altura em pixels: 0 usa jogo_largura e a altura do monitor; x e y são o deslocamento dentro do monitor. \
vigiar reaplica a zona quando o jogo tenta se mexer. borda mantém a barra de título (jogo que não aceita sem moldura). \
alinhar_cliente prende pela área cliente, sem mexer no estilo: é o modo do Civ V, que mede o cursor pelo cliente. \
preparar aceita \"civ5\", que ajusta o GraphicsSettingsDX11.ini com o jogo fechado (com backup).";

impl Config {
    pub fn sanitize(&mut self) {
        if self.layouts.is_empty() {
            *self = default_config();
            return;
        }
        if self.layout >= self.layouts.len() {
            self.layout = 0;
        }
        self.gap = self.gap.clamp(0, 64);
        for l in &mut self.layouts {
            for z in &mut l.zones {
                z.x = z.x.clamp(0.0, 1.0);
                z.y = z.y.clamp(0.0, 1.0);
                z.w = z.w.clamp(0.01, 1.0 - z.x);
                z.h = z.h.clamp(0.01, 1.0 - z.y);
            }
            l.zones.retain(|z| z.w > 0.0 && z.h > 0.0);
        }
        self.layouts.retain(|l| !l.zones.is_empty());
        if self.layouts.is_empty() {
            *self = default_config();
        }
        self.jogo_largura = self.jogo_largura.clamp(0, 16384);
        for j in &mut self.jogos {
            j.largura = j.largura.clamp(0, 16384);
            j.altura = j.altura.clamp(0, 16384);
            j.x = j.x.clamp(-16384, 16384);
            j.y = j.y.clamp(-16384, 16384);
            j.processo = j.processo.trim().to_string();
            if j.nome.trim().is_empty() {
                j.nome = j.processo.clone();
            }
            if j.largura > 0 && j.largura < 200 {
                j.largura = 200;
            }
            if j.altura > 0 && j.altura < 200 {
                j.altura = 200;
            }
        }
        self.jogos.retain(|j| !j.processo.is_empty());
    }
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("ZoneSnap")
}

pub fn default_config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_or_create(path: &Path) -> (Config, Option<SystemTime>) {
    if !path.exists() {
        let cfg = default_config();
        write_default(path, &cfg);
        let mt = fs::metadata(path).ok().and_then(|m| m.modified().ok());
        return (cfg, mt);
    }
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            crate::log(&format!("config: falha ao ler {}: {e}", path.display()));
            return (default_config(), None);
        }
    };
    match serde_json::from_str::<Config>(&text) {
        Ok(mut cfg) => {
            if !text.contains("\"jogos\"") {
                cfg.jogos = vec![default_civ()];
                cfg.jogo_largura = 2300;
                let _ = save(path, &cfg);
                crate::log("config: perfil do Civ V adicionado ao arquivo antigo");
            }
            cfg.sanitize();
            let mt = fs::metadata(path).ok().and_then(|m| m.modified().ok());
            (cfg, mt)
        }
        Err(e) => {
            crate::log(&format!("config: JSON invalido em {}: {e}", path.display()));
            (
                default_config(),
                fs::metadata(path).ok().and_then(|m| m.modified().ok()),
            )
        }
    }
}

fn num(v: f32) -> String {
    let mut s = format!("{v:.6}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

pub fn save(path: &Path, cfg: &Config) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut v = serde_json::to_value(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if let serde_json::Value::Object(m) = &mut v {
        m.insert(
            "_ajuda".to_string(),
            serde_json::Value::String(HELP.to_string()),
        );
        m.insert(
            "_ajuda_jogos".to_string(),
            serde_json::Value::String(HELP_JOGOS.to_string()),
        );
    }
    let text = serde_json::to_string_pretty(&v)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, text + "\n")
}

fn write_default(path: &Path, cfg: &Config) {    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let q = |s: &str| serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string());
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"_ajuda\": {},\n", q(HELP)));
    out.push_str(&format!("  \"enabled\": {},\n", cfg.enabled));
    out.push_str(&format!("  \"show_overlay\": {},\n", cfg.show_overlay));
    out.push_str(&format!("  \"gap\": {},\n", cfg.gap));
    out.push_str(&format!("  \"layout\": {},\n", cfg.layout));
    out.push_str("  \"layouts\": [\n");
    for (i, l) in cfg.layouts.iter().enumerate() {
        out.push_str(&format!("    {{\"name\": {}, \"zones\": [", q(&l.name)));
        for (j, z) in l.zones.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!(
                "[{}, {}, {}, {}]",
                num(z.x),
                num(z.y),
                num(z.w),
                num(z.h)
            ));
        }
        out.push_str("]}");
        if i + 1 < cfg.layouts.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ],\n");
    out.push_str(&format!("  \"jogo_largura\": {},\n", cfg.jogo_largura));
    out.push_str(&format!("  \"_ajuda_jogos\": {},\n", q(HELP_JOGOS)));
    out.push_str("  \"jogos\": [\n");
    for (i, j) in cfg.jogos.iter().enumerate() {
        out.push_str(&format!(
            "    {{\"processo\": {}, \"nome\": {}, \"largura\": {}, \"altura\": {}, \"x\": {}, \"y\": {}, \"vigiar\": {}, \"borda\": {}, \"alinhar_cliente\": {}, \"preparar\": {}}}",
            q(&j.processo),
            q(&j.nome),
            j.largura,
            j.altura,
            j.x,
            j.y,
            j.vigiar,
            j.borda,
            j.alinhar_cliente,
            q(&j.preparar)
        ));
        if i + 1 < cfg.jogos.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    let _ = fs::write(path, out);}
