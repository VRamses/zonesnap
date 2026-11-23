// A planta do monitor: o desenho que conduz a tela. Sai em escala real do
// monitor (3440 x 1440 no caso do Ramses), com a régua marcando exatamente onde
// cada zona começa, e cada zona escrita com o tamanho que vai receber.

import type { Zona } from "../lib/api";

type Props = {
  largura: number;
  altura: number;
  zonas: Zona[];
  gap: number;
};

const mono = { fontFamily: "var(--zs-mono)" } as const;

// Régua em HTML, acima do desenho: em SVG o rótulo encolhia junto com a planta
// e ficava ilegível.
function Regua({ marcas, largura }: { marcas: number[]; largura: number }) {
  return (
    <div className="relative mb-1 h-[26px]" aria-hidden="true">
      {marcas.map((x, i) => {
        const interna = i > 0 && i < marcas.length - 1;
        return (
          <span
            key={`traco-${x}-${i}`}
            className="absolute bottom-0 h-[7px] w-px"
            style={{
              left: `${(x / largura) * 100}%`,
              background: interna ? "var(--zs-accent)" : "var(--zs-line-forte)",
            }}
          />
        );
      })}
      {marcas.map((x, i) => {
        const primeira = i === 0;
        const ultima = i === marcas.length - 1;
        return (
          <span
            key={`numero-${x}-${i}`}
            className="zs-num absolute top-0 text-[10px] leading-none"
            style={{
              left: `${(x / largura) * 100}%`,
              transform: `translateX(${primeira ? "0" : ultima ? "-100%" : "-50%"})`,
              color: primeira || ultima ? "var(--zs-dim)" : "var(--zs-ink)",
            }}
          >
            {x}
          </span>
        );
      })}
    </div>
  );
}

export function Planta({ largura, altura, zonas, gap }: Props) {
  const metade = gap / 2;
  const divisas = zonas
    .map((z) => Math.round(z[0] * largura))
    .filter((x, i, lista) => i > 0 && x > lista[i - 1]);
  const marcas = [0, ...divisas, largura];

  return (
    <div>
      <Regua marcas={marcas} largura={largura} />
      <svg
        viewBox={`0 0 ${largura} ${altura}`}
        className="block w-full"
        style={{ aspectRatio: `${largura} / ${altura}` }}
        role="img"
        aria-label={`Planta do monitor com ${zonas.length} zonas de ${largura} por ${altura} pixels`}
      >
        {/* o vidro do monitor fica mais escuro que as zonas, para as áreas
            lerem como áreas e não como um retângulo vazio */}
        <rect
          x={2}
          y={2}
          width={largura - 4}
          height={altura - 4}
          strokeWidth={4}
          style={{ fill: "var(--zs-bg)", stroke: "var(--zs-line-forte)" }}
        />

        {zonas.map((z, i) => {
          const x = z[0] * largura + metade;
          const y = z[1] * altura + metade;
          const w = Math.max(0, z[2] * largura - metade * 2);
          const h = Math.max(0, z[3] * altura - metade * 2);
          const cabeNumero = w > 300 && h > 300;
          const cabeTamanho = w > 860 && h > 460;
          const cabeOrigem = w > 620 && h > 460;

          return (
            <g key={i}>
              <rect
                x={x}
                y={y}
                width={w}
                height={h}
                strokeWidth={4}
                style={{ fill: "var(--zs-surface-2)", stroke: "var(--zs-line-forte)" }}
              />
              {cabeNumero && (
                <text
                  x={x + 52}
                  y={y + 104}
                  fontSize={84}
                  fontWeight={600}
                  style={{ ...mono, fill: "var(--zs-ink)" }}
                >
                  {i + 1}
                </text>
              )}
              {cabeTamanho && (
                <text
                  x={x + 56}
                  y={y + 172}
                  fontSize={46}
                  style={{ ...mono, fill: "var(--zs-dim)" }}
                >
                  {Math.round(z[2] * largura)} x {Math.round(z[3] * altura)} px
                </text>
              )}
              {cabeOrigem && (
                <text
                  x={x + 56}
                  y={y + 228}
                  fontSize={38}
                  style={{ ...mono, fill: "var(--zs-dim)", opacity: 0.6 }}
                >
                  em {Math.round(z[0] * largura)}, {Math.round(z[1] * altura)}
                </text>
              )}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

// Versão de 32 px de altura usada nos botões de divisão: o desenho das zonas
// substitui o nome do layout como rótulo principal.
export function Miniatura({ zonas }: { zonas: Zona[] }) {
  return (
    <svg viewBox="0 0 3440 1440" className="block h-8 w-full" aria-hidden="true">
      {zonas.map((z, i) => (
        <rect
          key={i}
          x={z[0] * 3440 + 8}
          y={z[1] * 1440 + 8}
          width={Math.max(0, z[2] * 3440 - 16)}
          height={Math.max(0, z[3] * 1440 - 16)}
          fill="currentColor"
          fillOpacity={0.55}
          strokeWidth={1}
          vectorEffect="non-scaling-stroke"
          style={{ stroke: "var(--zs-surface)" }}
        />
      ))}
    </svg>
  );
}
