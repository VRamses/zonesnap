// Seção Atalhos: mostra o que o Windows aceitou registrar neste computador.
// Tecla que outro programa tomou aparece riscada, em vez de sumir da lista.

import { Kbd } from "../components/pecas";
import type { Atalho } from "../lib/api";

// Lista usada só quando a versão do programa em execução ainda não manda os
// atalhos no estado (build antigo).
const PADRAO: Atalho[] = [
  { tecla: "Win+Alt+L", oque: "próximo layout", ok: true },
  { tecla: "Win+Alt+Z", oque: "liga ou desliga a vigilância", ok: true },
  { tecla: "Win+Alt+Shift+1 a 9", oque: "manda a janela em foco para a zona", ok: true },
  { tecla: "Ctrl+Alt+Z", oque: "prende o jogo em foco", ok: true },
  { tecla: "Ctrl+Alt+X", oque: "solta o jogo em foco", ok: true },
  { tecla: "Ctrl+Alt+B", oque: "joga a janela em foco na área livre", ok: true },
  { tecla: "Ctrl+Alt+S", oque: "pausa ou retoma a vigilância", ok: true },
  { tecla: "Ctrl+Alt+V", oque: "esconde a moldura do navegador", ok: true },
];

function Teclas({ tecla, ok }: { tecla: string; ok: boolean }) {
  return (
    <span className="flex flex-wrap items-center gap-1">
      {tecla.split("+").map((parte, i) => (
        <Kbd key={`${parte}-${i}`} morta={!ok}>
          {parte}
        </Kbd>
      ))}
    </span>
  );
}

export function SecaoAtalhos({ atalhos }: { atalhos?: Atalho[] }) {
  const lista = atalhos && atalhos.length > 0 ? atalhos : PADRAO;
  const mortas = lista.filter((a) => !a.ok).length;

  return (
    <div className="flex flex-col gap-3">
      <section className="zs-painel overflow-hidden">
        <div className="flex items-baseline justify-between border-b border-[var(--zs-line)] px-4 py-2.5">
          <h2 className="zs-rotulo">Atalhos globais</h2>
          <span className="text-[11px] text-[var(--zs-dim)]">
            valem com qualquer janela em foco
          </span>
        </div>
        <ul className="divide-y divide-[var(--zs-line)]">
          {lista.map((a, i) => (
            <li
              key={`${a.tecla}-${i}`}
              className="flex items-center gap-4 px-4 py-2.5"
            >
              <div className="w-[190px] shrink-0">
                <Teclas tecla={a.tecla} ok={a.ok} />
              </div>
              <span
                className="min-w-0 flex-1 text-[13px]"
                style={{ color: a.ok ? "var(--zs-ink)" : "var(--zs-dim)" }}
              >
                {a.oque}
              </span>
              {!a.ok && (
                <span className="shrink-0 text-[11px] text-[var(--zs-alerta)]">
                  ocupada por outro programa
                </span>
              )}
            </li>
          ))}
        </ul>
      </section>

      <p className="max-w-[78ch] text-[11px] leading-relaxed text-[var(--zs-dim)]">
        {mortas === 0
          ? "Todas as teclas acima estão registradas. "
          : `${mortas} tecla(s) não puderam ser registradas porque outro programa já usa a combinação. `}
        O Windows não deixa dois programas usarem a mesma combinação, então o
        ZoneSnap tenta a variação com Shift antes de desistir. O registro fica
        no log, na pasta de dados.
      </p>
    </div>
  );
}
