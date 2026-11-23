import { useEffect, useMemo, useState } from "react";
import { Button } from "./components/ui/button";
import { Ponto } from "./components/pecas";
import { SecaoLayout } from "./secoes/layout";
import { SecaoJogos } from "./secoes/jogos";
import { SecaoAtalhos } from "./secoes/atalhos";
import { SecaoPrograma } from "./secoes/programa";
import {
  acao,
  estaAutorizado,
  lerConfig,
  lerEstado,
  salvarConfig,
  type Config,
  type Estado,
  type Jogo,
} from "./lib/api";

type Secao = "layout" | "jogos" | "atalhos" | "programa";

const SECOES: { id: Secao; nome: string; descricao: string }[] = [
  {
    id: "layout",
    nome: "Layout",
    descricao: "A tela em escala. O que você escolhe aqui vale para o próximo encaixe.",
  },
  {
    id: "jogos",
    nome: "Jogos",
    descricao: "Perfis que o ZoneSnap encaixa sozinho quando o processo aparece.",
  },
  {
    id: "atalhos",
    nome: "Atalhos",
    descricao: "As teclas registradas neste computador, com o que cada uma faz.",
  },
  {
    id: "programa",
    nome: "Programa",
    descricao: "Versão, licença e onde ficam os arquivos.",
  },
];

function secaoInicial(): Secao {
  const alvo = window.location.hash.replace("#", "");
  return SECOES.some((s) => s.id === alvo) ? (alvo as Secao) : "layout";
}

function Marca() {
  return (
    <svg width="26" height="26" viewBox="0 0 26 26" aria-hidden="true" fill="none">
      <path d="M2 3.5h22" stroke="currentColor" strokeWidth="1" opacity="0.45" />
      <path d="M2 1.5v2M13 1.5v2M24 1.5v2" stroke="currentColor" strokeWidth="1" opacity="0.45" />
      <rect
        x="1.75"
        y="7.75"
        width="22.5"
        height="13.5"
        rx="2"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <rect x="3.75" y="9.75" width="12.5" height="9.5" fill="currentColor" opacity="0.9" />
      <rect x="18.75" y="9.75" width="3.5" height="9.5" fill="currentColor" opacity="0.35" />
    </svg>
  );
}

export default function App() {
  const [cfg, setCfg] = useState<Config | null>(null);
  const [salvo, setSalvo] = useState<string>("");
  const [estado, setEstado] = useState<Estado | null>(null);
  const [msg, setMsg] = useState<string>("");
  const [recados, setRecados] = useState<Record<string, string>>({});
  const [ocupado, setOcupado] = useState(false);
  const [secao, setSecao] = useState<Secao>(secaoInicial);
  const [adicionando, setAdicionando] = useState(false);
  const [novo, setNovo] = useState({ processo: "", nome: "" });

  const sujo = cfg !== null && JSON.stringify(cfg) !== salvo;

  async function carregar() {
    const [c, e] = await Promise.all([lerConfig(), lerEstado()]);
    if (c && "layouts" in (c as object)) {
      setCfg(c as Config);
      setSalvo(JSON.stringify(c));
    } else if (c && !c.ok) {
      setMsg(c.erro ?? "não consegui ler a configuração");
    }
    if (e && "monitor" in (e as object)) setEstado(e as Estado);
  }

  useEffect(() => {
    if (!estaAutorizado()) {
      setMsg("Falta o token no endereço. Abra pelo menu do ícone: Interface.");
      return;
    }
    void carregar();
  }, []);

  useEffect(() => {
    if (!estaAutorizado()) return;
    const t = setInterval(() => {
      void lerEstado().then((e) => {
        if (e && "monitor" in (e as object)) setEstado(e as Estado);
      });
    }, 2500);
    return () => clearInterval(t);
  }, []);

  // A seção vive no endereço: #jogos abre direto na aba, e recarregar não
  // devolve a pessoa para o começo.
  useEffect(() => {
    if (window.location.hash !== `#${secao}`) {
      window.history.replaceState(null, "", `#${secao}`);
    }
  }, [secao]);

  const atual = useMemo(
    () => SECOES.find((s) => s.id === secao) ?? SECOES[0],
    [secao],
  );

  function mexer(alteracao: (c: Config) => Config) {
    setCfg((atual) => (atual ? alteracao(atual) : atual));
    setMsg("");
  }

  function mexerJogo(indice: number, jogo: Jogo) {
    mexer((c) => ({
      ...c,
      jogos: c.jogos.map((j, i) => (i === indice ? jogo : j)),
    }));
  }

  function remover(indice: number) {
    const jogo = cfg?.jogos[indice];
    mexer((c) => ({ ...c, jogos: c.jogos.filter((_, i) => i !== indice) }));
    setMsg(
      jogo
        ? `Perfil de ${jogo.nome || jogo.processo} removido. Salve para confirmar.`
        : "",
    );
  }

  async function salvar() {
    if (!cfg) return;
    setOcupado(true);
    const r = await salvarConfig(cfg);
    setOcupado(false);
    if (r.ok) {
      setSalvo(JSON.stringify(cfg));
      setMsg("Configuração salva.");
      if (r.estado) setEstado(r.estado);
    } else {
      setMsg(r.erro ?? "não consegui salvar");
    }
  }

  function descartar() {
    void carregar();
    setMsg("Alterações descartadas.");
  }

  async function executar(nome: string, processo?: string) {
    setOcupado(true);
    const r = await acao(nome, processo);
    setOcupado(false);
    if (r.estado) setEstado(r.estado);
    if (r.ok) {
      setMsg(r.mensagem ?? "feito");
      if (processo) {
        setRecados((a) => ({ ...a, [processo]: r.mensagem ?? "" }));
      }
    } else {
      if (processo) {
        setRecados((a) => ({ ...a, [processo]: r.erro ?? "não deu" }));
      }
      setMsg(r.erro ?? "não deu");
    }
  }

  function adicionar() {
    if (!cfg || novo.processo.trim() === "") return;
    const jogo: Jogo = {
      processo: novo.processo.trim(),
      nome: novo.nome.trim() || novo.processo.trim(),
      largura: 1200,
      altura: 800,
      x: 0,
      y: 0,
      vigiar: true,
      borda: false,
      alinhar_cliente: false,
      preparar: "",
    };
    mexer((c) => ({ ...c, jogos: [...c.jogos, jogo] }));
    setNovo({ processo: "", nome: "" });
    setAdicionando(false);
  }

  const ativo = estado?.ativo ?? true;
  const pausado = estado?.pausado ?? false;
  const presos = estado?.presos ?? 0;

  const situacao = pausado
    ? { texto: "Vigilância pausada", cor: "alerta" as const }
    : ativo
      ? { texto: "Vigilância ativa", cor: "ok" as const }
      : { texto: "Programa desligado", cor: "neutro" as const };

  if (!estaAutorizado()) {
    return (
      <div className="flex h-screen items-center justify-center p-8">
        <p className="max-w-[46ch] text-center text-[13px] leading-relaxed text-[var(--zs-dim)]">
          Falta o token no endereço. Abra a interface pelo menu do ícone do
          ZoneSnap, na bandeja do Windows.
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-[var(--zs-bg)]">
      <aside className="flex w-[236px] shrink-0 flex-col border-r border-[var(--zs-line)] bg-[var(--zs-rail)]">
        <div className="flex items-center gap-2.5 px-4 pt-4 pb-4">
          <span className="text-[var(--zs-ink)]">
            <Marca />
          </span>
          <div className="min-w-0">
            <p className="text-[13px] leading-tight font-semibold">ZoneSnap</p>
            <p className="zs-num text-[10px] leading-tight text-[var(--zs-dim)]">
              v{estado?.versao ?? "0.0.0"}
            </p>
          </div>
        </div>

        <nav aria-label="Seções" className="flex flex-col gap-0.5 px-2">
          {SECOES.map((s) => (
            <button
              key={s.id}
              type="button"
              className="zs-item-trilho"
              data-ativo={s.id === secao}
              aria-current={s.id === secao ? "page" : undefined}
              onClick={() => setSecao(s.id)}
            >
              {s.nome}
            </button>
          ))}
        </nav>

        <div className="mt-auto border-t border-[var(--zs-line)] px-3 pt-3 pb-3">
          <div className="flex items-center gap-2 px-1 pb-2.5 text-[11px] text-[var(--zs-dim)]">
            <Ponto cor={situacao.cor} />
            <span>{situacao.texto}</span>
            {presos > 0 && (
              <span className="zs-num ml-auto text-[var(--zs-ink)]">{presos}</span>
            )}
          </div>
          <Button
            size="sm"
            variant="outline"
            className="w-full"
            onClick={() => void executar("pausar")}
            disabled={ocupado}
          >
            {pausado ? "Retomar" : "Pausar"}
          </Button>
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-[58px] shrink-0 items-center justify-between gap-4 border-b border-[var(--zs-line)] px-6">
          <div className="min-w-0">
            <h1 className="text-[15px] leading-tight font-semibold">{atual.nome}</h1>
            <p className="truncate text-[11px] leading-tight text-[var(--zs-dim)]">
              {atual.descricao}
            </p>
          </div>
          <div className="shrink-0">
            {secao === "layout" && (
              <Button
                size="sm"
                variant="outline"
                onClick={() => void executar("previa")}
                disabled={ocupado}
              >
                Ver prévia
              </Button>
            )}
            {secao === "jogos" && (
              <Button
                size="sm"
                onClick={() => setAdicionando(true)}
                disabled={adicionando || ocupado}
              >
                Adicionar jogo
              </Button>
            )}
          </div>
        </header>

        <div className="zs-rolagem min-h-0 flex-1 px-6 py-5">
          {!cfg ? (
            <p className="text-[12px] text-[var(--zs-dim)]">Carregando…</p>
          ) : secao === "layout" ? (
            <SecaoLayout cfg={cfg} estado={estado} mexer={mexer} />
          ) : secao === "jogos" ? (
            <SecaoJogos
              cfg={cfg}
              estado={estado}
              adicionando={adicionando}
              novo={novo}
              setNovo={setNovo}
              adicionar={adicionar}
              fecharAdicao={() => setAdicionando(false)}
              mexerJogo={mexerJogo}
              remover={remover}
              prender={(p) => void executar("prender", p)}
              soltar={(p) => void executar("soltar", p)}
              preparar={(p) => void executar("preparar_civ", p)}
              alternarVideo={() => void executar("video")}
              recados={recados}
            />
          ) : secao === "atalhos" ? (
            <SecaoAtalhos atalhos={estado?.atalhos} />
          ) : (
            <SecaoPrograma estado={estado} />
          )}
        </div>

        <footer className="flex h-[52px] shrink-0 items-center gap-3 border-t border-[var(--zs-line)] px-6">
          <p className="min-w-0 flex-1 truncate text-[11px] text-[var(--zs-dim)]">
            {msg ||
              (sujo ? "alterações não salvas neste computador" : "tudo salvo")}
          </p>
          {sujo && (
            <Button size="sm" variant="ghost" onClick={descartar} disabled={ocupado}>
              Descartar
            </Button>
          )}
          {sujo && (
            <Button size="sm" onClick={() => void salvar()} disabled={ocupado}>
              Salvar
            </Button>
          )}
        </footer>
      </main>
    </div>
  );
}
