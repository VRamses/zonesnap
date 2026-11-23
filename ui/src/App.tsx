import { useEffect, useMemo, useState } from "react";
import { Button } from "./components/ui/button";
import { Switch } from "./components/ui/switch";
import {
  acao,
  estaAutorizado,
  lerConfig,
  lerEstado,
  salvarConfig,
  type Config,
  type Estado,
  type Ini,
  type Jogo,
} from "./lib/api";

const numero = (v: string, padrao = 0) => {
  const n = Number(v.replace(",", "."));
  return Number.isFinite(n) ? n : padrao;
};

function Marca() {
  return (
    <svg width="22" height="22" viewBox="0 0 22 22" aria-hidden="true">
      <rect x="1" y="4.5" width="20" height="13" rx="2" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <rect x="3" y="6.5" width="7.2" height="9" rx="1" fill="currentColor" opacity="0.85" />
      <rect x="11.6" y="6.5" width="7.4" height="4" rx="1" fill="currentColor" opacity="0.4" />
      <rect x="11.6" y="11.5" width="7.4" height="4" rx="1" fill="currentColor" opacity="0.4" />
    </svg>
  );
}

function Tela({
  largura,
  altura,
  zonas,
  gap,
  ativo,
}: {
  largura: number;
  altura: number;
  zonas: [number, number, number, number][];
  gap: number;
  ativo: boolean;
}) {
  const margem = 24;
  return (
    <svg
      viewBox={`0 0 ${largura} ${altura}`}
      className="block w-full"
      style={{ aspectRatio: `${largura} / ${altura}` }}
      role="img"
      aria-label={`Mapa do monitor: ${zonas.length} zonas`}
    >
      <rect
        x="1"
        y="1"
        width={largura - 2}
        height={altura - 2}
        rx="10"
        fill="#0a0c10"
        stroke="#2f3a48"
        strokeWidth="2"
      />
      {zonas.map((z, i) => {
        const x = z[0] * largura;
        const y = z[1] * altura;
        const w = z[2] * largura;
        const h = z[3] * altura;
        const t = ativo ? gap / 2 : 0;
        const larg = Math.max(0, w - t * 2);
        const altur = Math.max(0, h - t * 2);
        return (
          <g key={i}>
            <rect
              x={x + t}
              y={y + t}
              width={larg}
              height={altur}
              rx="8"
              fill={ativo ? "#1d3a76" : "#151a23"}
              fillOpacity={ativo ? 0.55 : 1}
              stroke={ativo ? "#3474f6" : "#2f3a48"}
              strokeWidth="2"
            />
            {larg > margem * 3 && altur > margem * 2 && (
              <text
                x={x + t + 18}
                y={y + t + 46}
                fill={ativo ? "#cfe0ff" : "#94a1b3"}
                fontSize="44"
                fontFamily="Segoe UI, system-ui, sans-serif"
              >
                {i + 1}
              </text>
            )}
          </g>
        );
      })}
    </svg>
  );
}

function Linha({
  rotulo,
  valor,
  aoMudar,
  passo = 1,
  dica,
}: {
  rotulo: string;
  valor: number;
  aoMudar: (n: number) => void;
  passo?: number;
  dica?: string;
}) {
  const id = `campo-${rotulo.replace(/\s+/g, "-").toLowerCase()}`;
  return (
    <div>
      <label htmlFor={id} className="zs-rotulo block pb-1">
        {rotulo}
      </label>
      <input
        id={id}
        className="zs-campo zs-num"
        type="number"
        inputMode="numeric"
        step={passo}
        value={valor}
        onChange={(e) => aoMudar(numero(e.target.value, valor))}
        aria-describedby={dica ? `${id}-dica` : undefined}
      />
      {dica && (
        <span id={`${id}-dica`} className="block pt-1 text-[11px] text-[var(--zs-dim)]">
          {dica}
        </span>
      )}
    </div>
  );
}

function Interruptor({
  rotulo,
  dica,
  valor,
  aoMudar,
}: {
  rotulo: string;
  dica: string;
  valor: boolean;
  aoMudar: (v: boolean) => void;
}) {
  const id = `int-${rotulo.replace(/\s+/g, "-").toLowerCase()}`;
  return (
    <div className="flex items-start justify-between gap-3 rounded-md border border-[var(--zs-line)] bg-[var(--zs-bg)] px-3 py-2">
      <div>
        <label htmlFor={id} className="block text-[13px] text-[var(--zs-ink)]">
          {rotulo}
        </label>
        <span className="block text-[11px] text-[var(--zs-dim)]">{dica}</span>
      </div>
      <Switch id={id} checked={valor} onCheckedChange={aoMudar} />
    </div>
  );
}

function CartaoJogo({
  jogo,
  aberto,
  preso,
  ini,
  aoMudar,
  aoRemover,
  aoPrender,
  aoSoltar,
  aoPreparar,
  mensagem,
}: {
  jogo: Jogo;
  aberto: boolean;
  preso: boolean;
  ini?: Ini | null;
  aoMudar: (j: Jogo) => void;
  aoRemover: () => void;
  aoPrender: () => void;
  aoSoltar: () => void;
  aoPreparar: () => void;
  mensagem?: string;
}) {
  return (
    <section className="flex flex-col gap-3 rounded-lg border border-[var(--zs-line)] bg-[var(--zs-surface)] p-3">
      <header className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="truncate text-[15px] font-semibold text-[var(--zs-ink)]">
            {jogo.nome || jogo.processo || "sem nome"}
          </h3>
          <p className="truncate font-mono text-[11px] text-[var(--zs-dim)]">{jogo.processo}</p>
        </div>
        <div className="flex shrink-0 items-center gap-2 text-[11px]">
          <span
            className="rounded-full border px-2 py-[2px]"
            style={{
              borderColor: preso ? "#3474f6" : "var(--zs-line)",
              color: preso ? "#cfe0ff" : "var(--zs-dim)",
            }}
          >
            {preso ? "encaixado" : "livre"}
          </span>
          <span
            className="rounded-full border px-2 py-[2px]"
            style={{
              borderColor: "var(--zs-line)",
              color: aberto ? "var(--zs-ok)" : "var(--zs-dim)",
            }}
          >
            {aberto ? "rodando" : "fechado"}
          </span>
        </div>
      </header>

      <div className="grid grid-cols-4 gap-2">
        <Linha rotulo="Largura" valor={jogo.largura} aoMudar={(n) => aoMudar({ ...jogo, largura: n })} dica="px" />
        <Linha rotulo="Altura" valor={jogo.altura} aoMudar={(n) => aoMudar({ ...jogo, altura: n })} dica="px" />
        <Linha rotulo="X" valor={jogo.x} aoMudar={(n) => aoMudar({ ...jogo, x: n })} dica="px" />
        <Linha rotulo="Y" valor={jogo.y} aoMudar={(n) => aoMudar({ ...jogo, y: n })} dica="px" />
      </div>

      <div className="grid gap-2 sm:grid-cols-3">
        <Interruptor
          rotulo="Prender ao abrir"
          dica="encaixa sozinho quando o processo aparece"
          valor={jogo.vigiar}
          aoMudar={(v) => aoMudar({ ...jogo, vigiar: v })}
        />
        <Interruptor
          rotulo="Barra de título"
          dica="deixe ligado para manter a moldura"
          valor={jogo.borda}
          aoMudar={(v) => aoMudar({ ...jogo, borda: v })}
        />
        <Interruptor
          rotulo="Cliente exato"
          dica="para jogo que nasce travado num tamanho"
          valor={jogo.alinhar_cliente}
          aoMudar={(v) => aoMudar({ ...jogo, alinhar_cliente: v })}
        />
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" onClick={aoPrender}>
          Prender agora
        </Button>
        <Button size="sm" variant="outline" onClick={aoSoltar}>
          Soltar
        </Button>
        {jogo.preparar === "civ5" && !ini && (
          <Button size="sm" variant="secondary" onClick={aoPreparar}>
            Ajustar o .ini do Civ V
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="ml-auto text-[var(--zs-dim)] hover:text-[var(--zs-ink)]"
          onClick={aoRemover}
        >
          Remover
        </Button>
      </div>

      {ini && (
        <div
          className="rounded-md border px-3 py-2 text-[12px] leading-snug"
          style={{
            borderColor: ini.divergente ? "var(--zs-alerta)" : "var(--zs-line)",
            background: "var(--zs-bg)",
            color: ini.divergente ? "var(--zs-ink)" : "var(--zs-dim)",
          }}
        >
          <p className="flex items-start gap-2">
            <span
              aria-hidden="true"
              className="mt-[5px] inline-block size-[7px] shrink-0 rounded-full"
              style={{ background: ini.divergente ? "var(--zs-alerta)" : "var(--zs-ok)" }}
            />
            <span>
              <span className="zs-rotulo mr-2">.ini do jogo</span>
              {ini.frase}
            </span>
          </p>
          {ini.divergente && (
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <Button
                size="sm"
                variant="secondary"
                onClick={aoPreparar}
                disabled={!ini.pode_corrigir}
              >
                {ini.jogo_aberto ? "Feche o jogo para corrigir" : "Corrigir o .ini agora"}
              </Button>
              {ini.caminho && (
                <span className="min-w-0 truncate font-mono text-[11px] text-[var(--zs-dim)]">
                  {ini.caminho}
                </span>
              )}
            </div>
          )}
        </div>
      )}

      {mensagem && (
        <p className="rounded-md border border-[var(--zs-line-forte)] bg-[var(--zs-bg)] px-3 py-2 text-[12px] text-[var(--zs-ink)]">
          {mensagem}
        </p>
      )}
    </section>
  );
}

export default function App() {
  const [cfg, setCfg] = useState<Config | null>(null);
  const [salvo, setSalvo] = useState<string>("");
  const [estado, setEstado] = useState<Estado | null>(null);
  const [msg, setMsg] = useState<string>("");
  const [recados, setRecados] = useState<Record<string, string>>({});
  const [ocupado, setOcupado] = useState(false);
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

  const layout = useMemo(() => {
    if (!cfg) return null;
    return cfg.layouts[cfg.layout] ?? cfg.layouts[0] ?? null;
  }, [cfg]);

  function mexer(alteracao: (c: Config) => Config) {
    setCfg((atual) => (atual ? alteracao(atual) : atual));
    setMsg("");
  }

  function mexerJogo(indice: number, jogo: Jogo) {
    mexer((c) => ({ ...c, jogos: c.jogos.map((j, i) => (i === indice ? jogo : j)) }));
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

  async function executar(nome: string, processo?: string) {
    setOcupado(true);
    const r = await acao(nome, processo);
    setOcupado(false);
    if (r.estado) setEstado(r.estado);
    if (r.ok) {
      setMsg(r.mensagem ?? "feito");
      if (processo) setRecados((a) => ({ ...a, [processo]: r.mensagem ?? "" }));
    } else {
      if (processo) setRecados((a) => ({ ...a, [processo]: r.erro ?? "não deu" }));
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
  }

  const ativo = estado?.ativo ?? true;
  const pausado = estado?.pausado ?? false;
  const [largMon, altMon] = [estado?.monitor.largura ?? 3440, estado?.monitor.altura ?? 1440];

  return (
    <div className="min-h-screen bg-[var(--zs-bg)]">
      <header className="sticky top-0 z-10 border-b border-[var(--zs-line)] bg-[var(--zs-bg)]/95 backdrop-blur">
        <div className="mx-auto flex max-w-[1180px] items-center gap-4 px-4 py-3">
          <div className="flex items-center gap-2 text-[var(--zs-ink)]">
            <Marca />
            <div>
              <h1 className="text-[15px] font-semibold leading-tight">ZoneSnap</h1>
              <p className="text-[11px] leading-tight text-[var(--zs-dim)]">
                zonas de encaixe e jogos no Windows
              </p>
            </div>
          </div>

          <div className="ml-2 flex items-center gap-2 text-[12px] text-[var(--zs-dim)]">
            <span
              className="inline-block size-2 rounded-full"
              style={{ background: pausado ? "var(--zs-alerta)" : "var(--zs-ok)" }}
            />
            {pausado ? "vigilância pausada" : ativo ? "vigilância ativa" : "desligado"}
            {estado && estado.presos > 0 && (
              <span className="text-[var(--zs-ink)]">· {estado.presos} encaixada(s)</span>
            )}
          </div>

          <div className="ml-auto flex items-center gap-2">
            <Button size="sm" variant="ghost" onClick={() => executar("previa")}>
              Ver prévia
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => executar("pausar")}
              aria-pressed={pausado}
            >
              {pausado ? "Retomar" : "Pausar"}
            </Button>
            <Button size="sm" onClick={salvar} disabled={!sujo || ocupado}>
              {sujo ? "Salvar alterações" : "Salvo"}
            </Button>
          </div>
        </div>
      </header>

      <main className="mx-auto grid max-w-[1180px] gap-4 px-4 py-4" aria-busy={ocupado}>
        {msg && (
          <p
            role="status"
            aria-live="polite"
            className="rounded-md border border-[var(--zs-line-forte)] bg-[var(--zs-surface)] px-3 py-2 text-[13px] text-[var(--zs-ink)]"
          >
            {msg}
          </p>
        )}

        <section className="rounded-lg border border-[var(--zs-line)] bg-[var(--zs-surface)] p-3">
          <div className="flex flex-wrap items-baseline justify-between gap-2 pb-2">
            <h2 className="text-[13px] font-semibold text-[var(--zs-ink)]">Monitor</h2>
            <span className="zs-num text-[12px] text-[var(--zs-dim)]">
              {largMon} x {altMon} px · {layout?.name ?? "sem layout"}
            </span>
          </div>

          <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_320px]">
            <div>
              <div className="rounded-md border border-[var(--zs-line)] bg-[var(--zs-bg)] p-2">
                {layout ? (
                  <Tela
                    largura={largMon}
                    altura={altMon}
                    zonas={layout.zones}
                    gap={cfg?.gap ?? 0}
                    ativo={!pausado}
                  />
                ) : (
                  <p className="p-6 text-center text-[13px] text-[var(--zs-dim)]">Carregando...</p>
                )}
              </div>

              {layout && (
                <div className="flex flex-wrap gap-x-4 gap-y-1 pt-2 text-[12px] text-[var(--zs-dim)]">
                  {layout.zones.map((z, i) => (
                    <span key={i} className="zs-num">
                      <span className="text-[var(--zs-ink)]">{i + 1}</span> ·{" "}
                      {Math.round(z[2] * largMon)} x {Math.round(z[3] * altMon)} px
                    </span>
                  ))}
                </div>
              )}
            </div>

            {cfg && (
              <div className="flex flex-col gap-3">
                <div>
                  <span className="zs-rotulo block pb-1">Layout</span>
                  <div className="flex flex-wrap gap-1.5">
                    {cfg.layouts.map((l, i) => (
                      <button
                        key={l.name}
                        type="button"
                        className="zs-chip"
                        data-ativo={i === cfg.layout}
                        aria-pressed={i === cfg.layout}
                        onClick={() => mexer((c) => ({ ...c, layout: i }))}
                      >
                        {l.name}
                      </button>
                    ))}
                  </div>
                </div>
                <div>
                  <label htmlFor="gap" className="zs-rotulo block pb-1">
                    Espaço entre zonas: {cfg.gap} px
                  </label>
                  <input
                    id="gap"
                    type="range"
                    min={0}
                    max={64}
                    step={2}
                    value={cfg.gap}
                    onChange={(e) => mexer((c) => ({ ...c, gap: numero(e.target.value) }))}
                    className="w-full accent-[var(--zs-accent)]"
                  />
                  <span className="block pt-1 text-[11px] text-[var(--zs-dim)]">
                    metade de cada lado da divisa; vale para os encaixes novos
                  </span>
                </div>

                <dl className="mt-2 grid gap-1 border-t border-[var(--zs-line)] pt-2 text-[11px] text-[var(--zs-dim)]">
                  {[
                    ["Ctrl+Alt+Z", "prende o jogo em foco"],
                    ["Ctrl+Alt+X", "solta o jogo em foco"],
                    ["Ctrl+Alt+B", "joga o jogo em foco na área livre"],
                    ["Ctrl+Alt+S", "pausa ou retoma a vigilância"],
                    ["Win+Alt+L", "próximo layout"],
                    ["Win+Alt+1 a 9", "manda a janela para a zona"],
                  ].map(([tecla, oque]) => (
                    <div key={tecla} className="flex gap-2">
                      <dt className="w-[92px] shrink-0 font-mono text-[var(--zs-ink)]">{tecla}</dt>
                      <dd className="m-0">{oque}</dd>
                    </div>
                  ))}
                </dl>
              </div>
            )}
          </div>
        </section>

        <section className="grid gap-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h2 className="text-[13px] font-semibold text-[var(--zs-ink)]">
              Jogos e janelas
            </h2>
            <p className="text-[12px] text-[var(--zs-dim)]">
              o perfil vale para o processo cujo nome aparece aqui
            </p>
          </div>

          {cfg?.jogos.length === 0 && (
            <p className="rounded-md border border-dashed border-[var(--zs-line)] px-3 py-6 text-center text-[13px] text-[var(--zs-dim)]">
              Nenhum perfil ainda. Adicione o processo de um jogo abaixo.
            </p>
          )}

          <div className="grid gap-3 lg:grid-cols-2">
            {cfg?.jogos.map((jogo, i) => {
              const e = estado?.jogos.find(
                (x) => x.processo.toLowerCase() === jogo.processo.toLowerCase(),
              );
              return (
                <CartaoJogo
                  key={`${jogo.processo}-${i}`}
                  jogo={jogo}
                  aberto={e?.aberto ?? false}
                  preso={e?.preso ?? false}
                  ini={e?.ini ?? null}
                  mensagem={recados[jogo.processo]}
                  aoMudar={(j) => mexerJogo(i, j)}
                  aoRemover={() => mexer((c) => ({ ...c, jogos: c.jogos.filter((_, k) => k !== i) }))}
                  aoPrender={() => executar("prender", jogo.processo)}
                  aoSoltar={() => executar("soltar", jogo.processo)}
                  aoPreparar={() => executar("preparar_civ", jogo.processo)}
                />
              );
            })}
          </div>

          <div className="grid gap-2 rounded-lg border border-[var(--zs-line)] bg-[var(--zs-surface)] p-3 sm:grid-cols-[1fr_1fr_auto] sm:items-end">
            <div>
              <label htmlFor="proc" className="zs-rotulo block pb-1">
                Processo
              </label>
              <input
                id="proc"
                className="zs-campo font-mono"
                placeholder="exe do jogo.exe"
                value={novo.processo}
                onChange={(e) => setNovo({ ...novo, processo: e.target.value })}
              />
            </div>
            <div>
              <label htmlFor="nome" className="zs-rotulo block pb-1">
                Nome do perfil
              </label>
              <input
                id="nome"
                className="zs-campo"
                placeholder="como você quer ver na lista"
                value={novo.nome}
                onChange={(e) => setNovo({ ...novo, nome: e.target.value })}
              />
            </div>
            <Button size="sm" onClick={adicionar} disabled={novo.processo.trim() === ""}>
              Adicionar
            </Button>
          </div>
        </section>

        <footer className="grid gap-1 border-t border-[var(--zs-line)] pt-3 text-[11px] text-[var(--zs-dim)]">
          <p className="zs-num">
            Configuração: {estado?.caminho_config ?? "..."}
            {sujo && <span className="text-[var(--zs-alerta)]"> · alterações não salvas</span>}
          </p>
          <p className="zs-num">Registro: {estado?.caminho_log ?? "..."}</p>
          {estado?.civ_ini && <p className="zs-num">Civ V: {estado.civ_ini}</p>}
          <p>Versão {estado?.versao ?? "0.1.0"}</p>
        </footer>
      </main>
    </div>
  );
}
