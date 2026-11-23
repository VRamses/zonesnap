# Componentes de terceiros

O ZoneSnap é MIT (veja `LICENSE`). Estes são os componentes de fora que entram no
produto, com a licença de cada um.

## shadcn/ui

Os arquivos em `ui/src/components/ui/` (`button.tsx`, `switch.tsx`) vieram do
[shadcn/ui](https://github.com/shadcn-ui/ui), sob licença MIT, Copyright (c)
2023 shadcn. O aviso de copyright está no topo de cada arquivo e foi mantido.
Alterações feitas aqui: só o caminho de import do helper `cn` e o nome do pacote
do Radix.

## Radix UI

`@radix-ui/react-slot` e `@radix-ui/react-switch`, MIT, Copyright (c) 2022
WorkOS. Usados como dependência, sem cópia de código.

## Tailwind CSS

`tailwindcss` e `@tailwindcss/vite`, MIT, Copyright (c) Tailwind Labs, Inc. A
folha `ui/src/styles/shadcn.css` carrega os tokens e os keyframes do shadcn e é
importada separada de `app.css`, sem fusão com os tokens do ZoneSnap.

## Outros

| Pacote | Licença |
|---|---|
| `windows-sys` | MIT OR Apache-2.0 |
| `tiny-skia` | BSD-3-Clause |
| `serde`, `serde_json` | MIT OR Apache-2.0 |
| `react`, `react-dom` | MIT |
| `vite`, `vite-plugin-singlefile` | MIT |
| `class-variance-authority`, `clsx`, `tailwind-merge` | MIT |
