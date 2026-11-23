// Seção Programa: versão, licença e onde ficam os arquivos. Os caminhos são
// clicáveis para copiar, porque o destino normal é abrir no editor ou no
// Explorer.

import { useState } from "react";
import { Button } from "../components/ui/button";
import type { Estado } from "../lib/api";

function Ficha({
  rotulo,
  valor,
  copiar,
  copiado,
}: {
  rotulo: string;
  valor: string;
  copiar?: () => void;
  copiado?: boolean;
}) {
  return (
    <div className="flex items-center gap-4 border-b border-[var(--zs-line)] py-2.5 last:border-b-0">
      <span className="zs-rotulo w-[104px] shrink-0">{rotulo}</span>
      <span className="zs-mono min-w-0 flex-1 truncate text-[12px] text-[var(--zs-ink)]">
        {valor}
      </span>
      {copiar && (
        <Button
          size="sm"
          variant="ghost"
          className="shrink-0 text-[11px] text-[var(--zs-dim)] hover:text-[var(--zs-ink)]"
          onClick={copiar}
        >
          {copiado ? "copiado" : "copiar"}
        </Button>
      )}
    </div>
  );
}

export function SecaoPrograma({ estado }: { estado: Estado | null }) {
  const [copiado, setCopiado] = useState<string | null>(null);

  async function copiar(chave: string, texto: string) {
    try {
      await navigator.clipboard.writeText(texto);
      setCopiado(chave);
      window.setTimeout(() => setCopiado(null), 1600);
    } catch {
      setCopiado(null);
    }
  }

  const arquivos: { chave: string; rotulo: string; valor: string }[] = [
    { chave: "cfg", rotulo: "Configuração", valor: estado?.caminho_config ?? "..." },
    { chave: "log", rotulo: "Registro", valor: estado?.caminho_log ?? "..." },
  ];
  if (estado?.civ_ini) {
    arquivos.push({ chave: "ini", rotulo: ".ini do Civ V", valor: estado.civ_ini });
  }

  return (
    <div className="flex flex-col gap-3">
      <section className="zs-painel px-4 py-3">
        <h2 className="zs-rotulo pb-2">Programa</h2>
        <p className="max-w-[62ch] text-[12px] leading-relaxed text-[var(--zs-dim)]">
          O ZoneSnap divide a tela em zonas fixas e encaixa a janela arrastada
          na zona onde o cursor soltou. Roda na bandeja, sem instalador e sem
          pedir administrador. A configuração é um arquivo de texto, editável à
          mão.
        </p>
        <div className="pt-3">
          <Ficha rotulo="Versão" valor={estado?.versao ?? "0.0.0"} />
          <Ficha rotulo="Licença" valor="MIT" />
          <Ficha
            rotulo="Repositório"
            valor="github.com/VRamses/zonesnap"
            copiar={() => void copiar("repo", "https://github.com/VRamses/zonesnap")}
            copiado={copiado === "repo"}
          />
        </div>
      </section>

      <section className="zs-painel px-4 py-3">
        <div className="flex items-baseline justify-between pb-1">
          <h2 className="zs-rotulo">Arquivos</h2>
          <span className="text-[11px] text-[var(--zs-dim)]">
            clique em copiar e cole no Explorer ou no editor
          </span>
        </div>
        {arquivos.map((a) => (
          <Ficha
            key={a.chave}
            rotulo={a.rotulo}
            valor={a.valor}
            copiar={() => void copiar(a.chave, a.valor)}
            copiado={copiado === a.chave}
          />
        ))}
      </section>
    </div>
  );
}
