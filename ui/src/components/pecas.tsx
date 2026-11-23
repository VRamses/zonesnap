// Peças pequenas que se repetem nas três seções: campo numérico com unidade,
// linha de interruptor, tecla de atalho e ponto de estado.

import type { ReactNode } from "react";
import { Switch } from "./ui/switch";

export function Campo({
  id,
  rotulo,
  valor,
  aoMudar,
  sufixo = "px",
  min,
  max,
}: {
  id: string;
  rotulo: string;
  valor: number;
  aoMudar: (n: number) => void;
  sufixo?: string;
  min?: number;
  max?: number;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="zs-rotulo">
        {rotulo}
      </label>
      <div className="relative">
        <input
          id={id}
          className="zs-campo pr-8"
          type="number"
          inputMode="numeric"
          min={min}
          max={max}
          value={valor}
          onChange={(e) => {
            const n = Number(e.target.value.replace(",", "."));
            aoMudar(Number.isFinite(n) ? n : 0);
          }}
        />
        {sufixo && (
          <span
            aria-hidden="true"
            className="zs-num pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-[10px] text-[var(--zs-dim)]"
          >
            {sufixo}
          </span>
        )}
      </div>
    </div>
  );
}

export function Chave({
  id,
  rotulo,
  dica,
  valor,
  aoMudar,
}: {
  id: string;
  rotulo: string;
  dica: string;
  valor: boolean;
  aoMudar: (v: boolean) => void;
}) {
  return (
    <div className="flex w-full items-start justify-between gap-4 py-2">
      <div className="min-w-0">
        <label htmlFor={id} className="block text-[13px] text-[var(--zs-ink)]">
          {rotulo}
        </label>
        <span className="block text-[11px] leading-snug text-[var(--zs-dim)]">{dica}</span>
      </div>
      <Switch id={id} checked={valor} onCheckedChange={aoMudar} className="mt-0.5 shrink-0" />
    </div>
  );
}

export function Kbd({ children, morta }: { children: ReactNode; morta?: boolean }) {
  return (
    <kbd className="zs-kbd" data-morta={morta ? "true" : undefined}>
      {children}
    </kbd>
  );
}

export function Ponto({ cor }: { cor: "ok" | "alerta" | "neutro" }) {
  const mapa = { ok: "var(--zs-ok)", alerta: "var(--zs-alerta)", neutro: "var(--zs-line-forte)" };
  return (
    <span
      aria-hidden="true"
      className="inline-block size-[6px] shrink-0 rounded-full"
      style={{ background: mapa[cor] }}
    />
  );
}

export function Vazio({ children }: { children: ReactNode }) {
  return (
    <p className="rounded-lg border border-dashed border-[var(--zs-line)] px-4 py-6 text-center text-[12px] text-[var(--zs-dim)]">
      {children}
    </p>
  );
}
