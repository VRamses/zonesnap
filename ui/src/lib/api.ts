// Cliente da API local do ZoneSnap. O token vem na URL (?t=...) e vai em
// todo pedido no cabeçalho X-ZoneSnap; sem ele a API responde 403.

const token = new URLSearchParams(window.location.search).get("t") ?? "";

export type Zona = [number, number, number, number];

export type Layout = {
  name: string;
  zones: Zona[];
};

export type Jogo = {
  processo: string;
  nome: string;
  largura: number;
  altura: number;
  x: number;
  y: number;
  vigiar: boolean;
  borda: boolean;
  alinhar_cliente: boolean;
  preparar: string;
};

export type Config = {
  layouts: Layout[];
  layout: number;
  gap: number;
  show_overlay: boolean;
  jogo_largura: number;
  jogos: Jogo[];
};

export type Ini = {
  caminho: string | null;
  existe: boolean;
  jogo_aberto: boolean;
  fullscreen: number | null;
  largura: number | null;
  altura: number | null;
  alvo_largura: number;
  alvo_altura: number;
  divergente: boolean;
  pode_corrigir: boolean;
  frase: string;
};

export type JogoEstado = Jogo & { preso: boolean; aberto: boolean; ini?: Ini | null };

// Atalho que o Windows aceitou registrar. `ok: false` é combinação que outro
// programa já usa, e a interface mostra a tecla riscada.
export type Atalho = { tecla: string; oque: string; ok: boolean };

export type Estado = {
  versao: string;
  ativo: boolean;
  pausado: boolean;
  gap: number;
  mostrar_previa: boolean;
  layout: number;
  layout_nome: string;
  layouts: Layout[];
  jogo_largura: number;
  jogos: JogoEstado[];
  presos: number;
  video: number;
  atalhos?: Atalho[];
  monitor: { largura: number; altura: number };
  caminho_config: string;
  caminho_log: string | null;
  civ_ini: string | null;
};

type Resposta<T> = T & { ok: boolean; erro?: string; mensagem?: string };

async function pedir<T>(caminho: string, init?: RequestInit): Promise<Resposta<T>> {
  const r = await fetch(caminho, {
    ...init,
    headers: {
      "X-ZoneSnap": token,
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });
  const texto = await r.text();
  let corpo: unknown = {};
  try {
    corpo = JSON.parse(texto);
  } catch {
    corpo = { ok: false, erro: `resposta inesperada do app (${r.status})` };
  }
  return corpo as Resposta<T>;
}

export function estaAutorizado() {
  return token.length > 0;
}

export async function lerEstado() {
  return pedir<Estado>("/api/estado");
}

export const lerConfig = () => pedir<Config>("/api/config");

export const salvarConfig = (cfg: Config) =>
  pedir<{ estado: Estado }>("/api/config", {
    method: "POST",
    body: JSON.stringify(cfg),
  });

export const acao = (nome: string, processo?: string) =>
  pedir<{ estado: Estado }>("/api/acao", {
    method: "POST",
    body: JSON.stringify({ acao: nome, processo }),
  });
