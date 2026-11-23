// Seção Jogos: cada jogo é um perfil que o ZoneSnap prende sozinho quando o
// processo aparece. O aviso do .ini fica colado no cartão do jogo, porque é o
// único lugar onde ele faz sentido.

import { Button } from "../components/ui/button";
import { Campo, Chave, Kbd, Ponto, Vazio } from "../components/pecas";
import type { Config, Estado, Ini, Jogo } from "../lib/api";

type Props = {
  cfg: Config;
  estado: Estado | null;
  adicionando: boolean;
  novo: { processo: string; nome: string };
  setNovo: (v: { processo: string; nome: string }) => void;
  adicionar: () => void;
  fecharAdicao: () => void;
  mexerJogo: (indice: number, jogo: Jogo) => void;
  remover: (indice: number) => void;
  prender: (processo: string) => void;
  soltar: (processo: string) => void;
  preparar: (processo: string) => void;
  alternarVideo: () => void;
  recados: Record<string, string>;
};

function PainelVideo({
  abertas,
  aoAlternar,
}: {
  abertas: number;
  aoAlternar: () => void;
}) {
  return (
    <section className="zs-painel flex items-start justify-between gap-4 p-4">
      <div className="min-w-0">
        <p className="zs-rotulo pb-1">Modo vídeo</p>
        <p className="max-w-[68ch] text-[12px] leading-snug text-[var(--zs-dim)]">
          Esconde as abas e a barra de endereço do navegador, para o vídeo ocupar
          a coluna inteira. A janela fica onde está: some só a moldura do alto, e
          um segundo toque devolve tudo. Serve para Opera, Chrome, Edge e Brave.
        </p>
        <p className="flex flex-wrap items-center gap-2 pt-2.5 text-[11px] text-[var(--zs-dim)]">
          <Kbd>Ctrl+Alt+V</Kbd>
          <span>
            {abertas > 0
              ? `${abertas} ${abertas > 1 ? "janelas" : "janela"} sem moldura agora`
              : "nenhuma janela sem moldura"}
          </span>
        </p>
      </div>
      <Button
        size="sm"
        variant="outline"
        className="shrink-0 border-[var(--zs-line-forte)]"
        onClick={aoAlternar}
      >
        {abertas > 0 ? "Trazer a moldura" : "Esconder a moldura"}
      </Button>
    </section>
  );
}

function AvisoIni({
  ini,
  aoPreparar,
}: {
  ini: Ini;
  aoPreparar: () => void;
}) {
  return (
    <div
      className="border-l-2 pl-3"
      style={{
        borderColor: ini.divergente ? "var(--zs-alerta)" : "var(--zs-line)",
      }}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <p className="zs-rotulo pb-1">.ini do jogo</p>
          <p
            className="text-[12px] leading-snug"
            style={{ color: ini.divergente ? "var(--zs-ink)" : "var(--zs-dim)" }}
          >
            {ini.frase}
          </p>
        </div>
        {ini.divergente && (
          <Button
            size="sm"
            variant="outline"
            className="shrink-0 border-[var(--zs-line-forte)] text-[var(--zs-alerta)] hover:text-[var(--zs-alerta)]"
            onClick={aoPreparar}
            disabled={!ini.pode_corrigir}
          >
            Corrigir o .ini
          </Button>
        )}
      </div>
      {ini.caminho && (
        <p className="zs-mono truncate pt-1.5 text-[10px] text-[var(--zs-dim)]">
          {ini.caminho}
        </p>
      )}
    </div>
  );
}

function CartaoJogo({
  indice,
  jogo,
  vivo,
  recado,
  aoMudar,
  aoRemover,
  aoPrender,
  aoSoltar,
  aoPreparar,
}: {
  indice: number;
  jogo: Jogo;
  vivo?: { preso: boolean; aberto: boolean; ini?: Ini | null };
  recado?: string;
  aoMudar: (j: Jogo) => void;
  aoRemover: () => void;
  aoPrender: () => void;
  aoSoltar: () => void;
  aoPreparar: () => void;
}) {
  const preso = vivo?.preso ?? false;
  const aberto = vivo?.aberto ?? false;
  const ini = vivo?.ini ?? null;
  const base = `jogo-${indice}`;

  return (
    <article className="zs-painel flex flex-col gap-4 p-4">
      <header className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="truncate text-[15px] font-semibold leading-tight">
            {jogo.nome || jogo.processo || "sem nome"}
          </h3>
          <p className="zs-mono truncate text-[11px] text-[var(--zs-dim)]">
            {jogo.processo}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-4 text-[11px] text-[var(--zs-dim)]">
          <span className="flex items-center gap-1.5">
            <Ponto cor={preso ? "ok" : "neutro"} />
            {preso ? "encaixado" : "livre"}
          </span>
          <span className="flex items-center gap-1.5">
            <Ponto cor={aberto ? "ok" : "neutro"} />
            {aberto ? "rodando" : "fechado"}
          </span>
        </div>
      </header>

      <div className="grid grid-cols-4 gap-2">
        <Campo
          id={`${base}-largura`}
          rotulo="Largura"
          valor={jogo.largura}
          aoMudar={(n) => aoMudar({ ...jogo, largura: n })}
        />
        <Campo
          id={`${base}-altura`}
          rotulo="Altura"
          valor={jogo.altura}
          aoMudar={(n) => aoMudar({ ...jogo, altura: n })}
        />
        <Campo
          id={`${base}-x`}
          rotulo="X"
          valor={jogo.x}
          aoMudar={(n) => aoMudar({ ...jogo, x: n })}
        />
        <Campo
          id={`${base}-y`}
          rotulo="Y"
          valor={jogo.y}
          aoMudar={(n) => aoMudar({ ...jogo, y: n })}
        />
      </div>

      <div className="grid gap-x-6 border-t border-[var(--zs-line)] pt-1 md:grid-cols-3">
        <Chave
          id={`${base}-vigiar`}
          rotulo="Prender ao abrir"
          dica="encaixa sozinho quando o processo aparece"
          valor={jogo.vigiar}
          aoMudar={(v) => aoMudar({ ...jogo, vigiar: v })}
        />
        <Chave
          id={`${base}-borda`}
          rotulo="Barra de título"
          dica="deixe ligado para manter a moldura"
          valor={jogo.borda}
          aoMudar={(v) => aoMudar({ ...jogo, borda: v })}
        />
        <Chave
          id={`${base}-cliente`}
          rotulo="Cliente exato"
          dica="para jogo que nasce travado num tamanho"
          valor={jogo.alinhar_cliente}
          aoMudar={(v) => aoMudar({ ...jogo, alinhar_cliente: v })}
        />
      </div>

      <div className="flex flex-wrap items-center gap-2 border-t border-[var(--zs-line)] pt-3">
        <Button size="sm" onClick={aoPrender}>
          Prender agora
        </Button>
        <Button size="sm" variant="outline" onClick={aoSoltar}>
          Soltar
        </Button>
        {jogo.preparar.toLowerCase() === "civ5" && !ini && (
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

      {ini && <AvisoIni ini={ini} aoPreparar={aoPreparar} />}

      {recado && (
        <p className="zs-mono border-t border-[var(--zs-line)] pt-2 text-[11px] text-[var(--zs-dim)]">
          {recado}
        </p>
      )}
    </article>
  );
}

export function SecaoJogos({
  cfg,
  estado,
  adicionando,
  novo,
  setNovo,
  adicionar,
  fecharAdicao,
  mexerJogo,
  remover,
  prender,
  soltar,
  preparar,
  alternarVideo,
  recados,
}: Props) {
  return (
    <div className="flex flex-col gap-3">
      {adicionando && (
        <section className="zs-painel p-4">
          <div className="grid items-end gap-2 sm:grid-cols-[1fr_1fr_auto_auto]">
            <div className="flex flex-col gap-1">
              <label htmlFor="novo-proc" className="zs-rotulo">
                Processo
              </label>
              <input
                id="novo-proc"
                className="zs-campo"
                value={novo.processo}
                placeholder="civ5.exe"
                onChange={(e) => setNovo({ ...novo, processo: e.target.value })}
              />
            </div>
            <div className="flex flex-col gap-1">
              <label htmlFor="novo-nome" className="zs-rotulo">
                Nome
              </label>
              <input
                id="novo-nome"
                className="zs-campo"
                value={novo.nome}
                placeholder="Civilization V"
                onChange={(e) => setNovo({ ...novo, nome: e.target.value })}
              />
            </div>
            <Button size="sm" onClick={adicionar} disabled={novo.processo.trim() === ""}>
              Adicionar
            </Button>
            <Button size="sm" variant="ghost" onClick={fecharAdicao}>
              Cancelar
            </Button>
          </div>
          <p className="pt-2 text-[11px] text-[var(--zs-dim)]">
            o processo é o nome do executável, como aparece no Gerenciador de
            Tarefas
          </p>
        </section>
      )}

      {cfg.jogos.length === 0 && !adicionando && (
        <Vazio>
          Nenhum jogo configurado. Use Adicionar jogo para criar o primeiro
          perfil.
        </Vazio>
      )}

      {cfg.jogos.map((jogo, i) => {
        const vivo = estado?.jogos.find(
          (j) => j.processo.toLowerCase() === jogo.processo.toLowerCase(),
        );
        return (
          <CartaoJogo
            key={`${jogo.processo}-${i}`}
            indice={i}
            jogo={jogo}
            vivo={vivo}
            recado={recados[jogo.processo]}
            aoMudar={(j) => mexerJogo(i, j)}
            aoRemover={() => remover(i)}
            aoPrender={() => prender(jogo.processo)}
            aoSoltar={() => soltar(jogo.processo)}
            aoPreparar={() => preparar(jogo.processo)}
          />
        );
      })}

      <PainelVideo abertas={estado?.video ?? 0} aoAlternar={alternarVideo} />
    </div>
  );
}
