// Seção Layout: a planta do monitor conduz a tela. Escolher a divisão e o
// espaço muda o desenho na hora, antes de a configuração ser salva.

import { Miniatura, Planta } from "../components/planta";
import { Chave, Vazio } from "../components/pecas";
import type { Config, Estado } from "../lib/api";

type Props = {
  cfg: Config;
  estado: Estado | null;
  mexer: (alteracao: (c: Config) => Config) => void;
};

export function SecaoLayout({ cfg, estado, mexer }: Props) {
  const largura = estado?.monitor.largura ?? 3440;
  const altura = estado?.monitor.altura ?? 1440;
  const layout = cfg.layouts[cfg.layout] ?? cfg.layouts[0];

  if (!layout) return <Vazio>Nenhuma divisão na configuração.</Vazio>;

  return (
    <div className="flex flex-col gap-4">
      <section className="zs-painel p-4">
        <div className="flex items-baseline justify-between pb-3">
          <h2 className="zs-rotulo">Planta do monitor</h2>
          <span className="zs-num text-[11px] text-[var(--zs-dim)]">
            {largura} x {altura} px · {layout.zones.length}{" "}
            {layout.zones.length === 1 ? "zona" : "zonas"} · espaço {cfg.gap} px
          </span>
        </div>
        <Planta
          largura={largura}
          altura={altura}
          zonas={layout.zones}
          gap={cfg.gap}
        />
      </section>

      <section>
        <div className="flex items-baseline justify-between pb-2">
          <h2 className="zs-rotulo">Divisão</h2>
          <span className="text-[11px] text-[var(--zs-dim)]">
            vale para o próximo encaixe
          </span>
        </div>
        <div className="grid grid-cols-[repeat(auto-fit,minmax(112px,1fr))] gap-2">
          {cfg.layouts.map((l, i) => (
            <button
              key={`${l.name}-${i}`}
              type="button"
              className="zs-divisao"
              data-ativo={i === cfg.layout}
              aria-pressed={i === cfg.layout}
              onClick={() => mexer((c) => ({ ...c, layout: i }))}
            >
              <Miniatura zonas={l.zones} />
              <span className="truncate text-[11px] leading-none">{l.name}</span>
            </button>
          ))}
        </div>
      </section>

      <div className="grid gap-3 md:grid-cols-2">
        <section className="zs-painel px-4 pt-3 pb-3">
          <div className="flex items-baseline justify-between">
            <label htmlFor="gap" className="zs-rotulo">
              Espaço entre zonas
            </label>
            <span className="zs-num text-[13px] text-[var(--zs-ink)]">{cfg.gap} px</span>
          </div>
          <input
            id="gap"
            type="range"
            min={0}
            max={64}
            step={2}
            value={cfg.gap}
            onChange={(e) => {
              const n = Number(e.target.value);
              mexer((c) => ({ ...c, gap: Number.isFinite(n) ? n : 0 }));
            }}
            className="zs-slider mt-2"
          />
          <p className="text-[11px] leading-snug text-[var(--zs-dim)]">
            metade de cada lado da divisa. Com 0 px as zonas se encostam e cada
            uma avança 1 px sobre a vizinha, para não sobrar fresta.
          </p>
        </section>

        <section className="zs-painel flex items-center px-4 py-3">
          <Chave
            id="previa-arrastar"
            rotulo="Prévia ao arrastar"
            dica="mostra as zonas na tela enquanto você move a janela"
            valor={cfg.show_overlay}
            aoMudar={(v) => mexer((c) => ({ ...c, show_overlay: v }))}
          />
        </section>
      </div>
    </div>
  );
}
