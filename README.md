# ZoneSnap

Zonas de encaixe e encaixe de jogos para o Windows, num executável único em Rust,
sem instalador, sem admin e com configuração em JSON legível.

Nasceu de um problema concreto: num ultrawide 3440 x 1440, o Easy Setting Box
parou de mostrar a barra de zonas ao arrastar a janela, e o FancyZones do
PowerToys não fazia o que faltava, que era prender um jogo num tamanho fixo e
manter o resto da tela livre. O ZoneSnap faz as duas coisas: zonas fracionárias
por layout e perfis por processo, com vigilância que reencaixa sozinha.

![Interface do ZoneSnap](docs/interface.png)

## O que ele faz

- **Zonas por layout**: sete layouts prontos pensados para 3440 x 1440, com
  frações de 0 a 1 da área útil do monitor. Você troca de layout pelo menu do
  ícone, pela interface ou por atalho.
- **Prévia ao arrastar**: enquanto você arrasta uma janela com a tecla do
  encaixe apertada, o ZoneSnap desenha as zonas sobre a tela e destaca a zona sob
  o cursor. É a barra que o Easy Setting Box mostrava, só que desenhada pelo
  próprio programa.
- **Encaixe só com a tecla apertada**: segurando `Ctrl` durante o arrasto, as
  zonas aparecem e a janela encaixa ao soltar. Soltando sem a tecla, a janela
  fica exatamente onde você soltou. A tecla é escolhida na seção Layout (Ctrl,
  Shift, Alt ou `Sem tecla`, que volta a encaixar em qualquer arrasto).
- **Prévia avulsa**: `zonesnap.exe --preview` (ou `--preview --seconds 6`) mostra
  as zonas sem precisar arrastar nada.
- **Encaixe por atalho**: `Win+Alt+Shift+1` a `Win+Alt+Shift+9` manda a janela em
  foco para a zona daquele número.
- **Perfis de jogo**: um perfil por processo fixa tamanho, posição e moldura.
  Com vigilância ligada, o ZoneSnap reencaixa a janela quando ela foge (o jogo
  redimensiona ao entrar em uma partida, por exemplo) sem tocar no que você
  estiver fazendo.
- **Caso Civilization V**: modo em que o encaixe é feito pelo cliente da janela,
  com o tamanho nascendo certo no arquivo `.ini` do jogo, mais um comando que
  regrava esse `.ini` com backup. Explicado abaixo.
- **Modo vídeo**: esconde a moldura do navegador (abas, barra de endereço e a
  borda do Windows) para o vídeo ocupar a coluna inteira, sem mexer no tamanho
  nem no lugar da janela. `Ctrl+Alt+V` entra e sai; o mesmo comando está no menu
  do ícone e na seção Jogos da interface. Serve para Opera, Chrome, Edge e Brave.
- **Interface local**: uma janela do próprio programa, com uma página servida em
  `127.0.0.1`, com a planta do monitor em escala, os perfis de jogo, os atalhos
  que o Windows registrou e os caminhos dos arquivos. O que você muda ali só é
  gravado quando você clica em `Salvar`.
- **Sem telemetria**: o ZoneSnap não faz conexão de saída. A interface é servida
  na própria máquina, com token. A janela usa o WebView2 do Windows, que é o
  mesmo componente do Edge: ele carrega só o endereço local, mas o runtime tem
  as conexões próprias dele, fora do controle do ZoneSnap.

## Como usar

1. Baixe `zonesnap.exe` (ou compile, veja o fim deste arquivo).
2. Rode. Ele fica no ícone ao lado do relógio.
3. Clique no ícone para abrir o menu (clique esquerdo também abre).
4. Arraste a janela segurando `Ctrl`: as zonas aparecem e a janela encaixa na
   zona sob o cursor ao soltar. Sem a tecla, ela fica onde você soltou.
   `Win+Alt+Shift+1` a `9` encaixa direto, sem arrastar.

Não precisa instalar, não pede administrador e não escreve nada fora de
`%APPDATA%\ZoneSnap` e da pasta que você mandar ele abrir.

## Menu do ícone

| Item | O que faz |
|---|---|
| Desligar zonas / religar | liga e desliga o encaixe ao arrastar |
| Layout | escolhe entre os layouts configurados |
| Abrir interface | abre a interface numa janela do próprio programa |
| Abrir no navegador | abre a mesma interface no navegador padrão |
| Jogos | prender ou soltar cada perfil, pausar a vigilância, soltar todas as janelas presas |
| Esconder a moldura do navegador | modo vídeo: tira abas e barra de endereço do navegador que está em uso |
| Abrir config.json | abre a configuração no editor padrão |
| Recarregar config | relê o arquivo sem reiniciar |
| Sair | fecha o programa e solta as janelas presas |

## Atalhos

Os atalhos valem com a janela que você quer encaixar em foco.

| Atalho | O que faz |
|---|---|
| `Win+Alt+L` | próximo layout |
| `Win+Alt+Z` | liga ou desliga a vigilância |
| `Win+Alt+Shift+1` a `9` | manda a janela em foco para a zona N |
| `Ctrl+Alt+Z` | prende o jogo em foco (usa o perfil do processo) |
| `Ctrl+Alt+X` | solta o jogo em foco e devolve a janela ao estado anterior |
| `Ctrl+Alt+B` | manda o jogo em foco para a área livre, à direita da faixa |
| `Ctrl+Alt+S` | pausa ou retoma a vigilância |
| `Ctrl+Alt+V` | modo vídeo: esconde a moldura do navegador em foco (aperte de novo para voltar) |

Se outro programa já tomou um atalho `Ctrl+Alt+*`, o ZoneSnap registra a versão
com `Shift` e escreve no registro qual foi usada (`Ctrl+Alt+Z ocupada; usei
Ctrl+Alt+Shift+Z`). O log fica em `%APPDATA%\ZoneSnap\zonesnap.log`.

A seção **Atalhos** da interface mostra o que o Windows aceitou de fato neste
computador, não o que o programa pretendia: se um atalho caiu para a variação com
`Shift`, é a variação que aparece na lista. Tecla que nem assim pôde ser
registrada aparece riscada, com o aviso de que outro programa já usa a
combinação.

## Interface

`zonesnap.exe --ui` abre a interface numa janela do próprio programa, já com o
token no endereço. Se o programa já estiver rodando, esse comando pede para a
instância existente mostrar a janela dela, em vez de abrir uma segunda.

A janela é do ZoneSnap, não uma aba do navegador: tem título e ícone na barra de
tarefas, entra no `Alt+Tab`, abre centralizada e no tamanho de 1240x900 (menor
que isso só se o monitor for pequeno). Fechar a janela não fecha o programa: ele
continua no ícone ao lado do relógio, e a próxima vez que você abrir a interface
a janela é criada de novo.

Por dentro dela roda o WebView2, o mesmo componente de navegador que o Windows 11
já traz instalado (Microsoft Edge WebView2 Runtime). Se esse componente não
estiver na máquina, o programa escreve no log e abre a interface no navegador
padrão, para você não ficar sem interface. O `Abrir no navegador`, no menu do
ícone, faz isso de propósito.

A página tem quatro seções, escolhidas na coluna da esquerda:

- **Layout**: a planta do monitor em escala, com a régua em pixels no topo e cada
  zona escrita com o tamanho que vai receber. Abaixo ficam os sete layouts em
  miniatura, com o nome embaixo de cada um, o espaço entre zonas e o painel
  `Arrastar`, com a prévia e a tecla que libera o encaixe.
- **Jogos**: um cartão por perfil, com tamanho, posição, os três interruptores,
  `Prender agora`, `Soltar`, `Remover` e o aviso do `.ini` do jogo, com o botão
  `Corrigir o .ini`. No fim da seção fica o painel do modo vídeo, com o atalho e o
  botão que esconde ou traz de volta a moldura do navegador.
- **Atalhos**: as teclas que o Windows aceitou registrar nesta máquina e o que
  cada uma faz.
- **Programa**: versão, licença, repositório e os caminhos da configuração, do
  registro e do `.ini` do Civilization V, cada um com um botão que copia.

O que você muda na interface fica pendente até clicar em `Salvar`, no rodapé; aí
o `config.json` é reescrito e o encaixe passa a usar os valores novos.
`Descartar` volta ao que está gravado. A seção aberta também aparece no endereço
(`.../#jogos`), então recarregar a página não devolve você para o começo.

A interface conversa com um servidor HTTP preso em `127.0.0.1`, numa porta entre
47121 e 47160, e exige o cabeçalho `X-ZoneSnap` com o token que está em
`%APPDATA%\ZoneSnap\ui.token`. O token existe para impedir que outra página
aberta no navegador mexa na sua configuração; ele não protege contra outro
programa rodando na mesma conta de usuário, que pode ler o arquivo. Se isso for
relevante para você, feche a interface quando não estiver usando.

## Configuração

O arquivo é criado na primeira execução em `%APPDATA%\ZoneSnap\config.json`.
Editar com o programa aberto funciona: ele relê o arquivo a cada 2 segundos.

```json
{
  "layout": 0,
  "gap": 8,
  "show_overlay": true,
  "tecla": "ctrl",
  "jogo_largura": 2300,
  "layouts": [
    {
      "name": "Duas metades",
      "zones": [[0.0, 0.0, 0.5, 1.0], [0.5, 0.0, 0.5, 1.0]]
    }
  ],
  "jogos": [
    {
      "processo": "CivilizationV_DX11.exe",
      "nome": "Civilization V (DX11)",
      "largura": 2300,
      "altura": 1440,
      "x": 0,
      "y": 0,
      "vigiar": true,
      "borda": false,
      "alinhar_cliente": true,
      "preparar": "civ5"
    }
  ]
}
```

- `zones`: cada zona é `[x, y, largura, altura]` em fração da área útil do
  monitor, de 0 a 1. `[0, 0, 0.5, 1]` é a metade esquerda inteira.
- `gap`: fresta em pixels entre zonas, dividida meio a meio na divisa. Com `0`,
  as zonas se encostam e cada uma avança 1 px sobre a vizinha que começa
  exatamente onde ela termina. Esse 1 px existe porque o Windows desenha uma
  borda translúcida na janela encaixada e ela deixa o papel de parede aparecer:
  avançando, a borda cai sobre o conteúdo da vizinha em vez de virar uma linha
  colorida entre as duas janelas.
- `layout`: índice do layout em uso.
- `tecla`: tecla que libera o encaixe ao arrastar, entre `ctrl`, `shift`, `alt` e
  `nenhuma`. Ela é lida no momento em que você solta a janela, então apertar no
  meio do arrasto vale, e soltar antes de soltar a janela cancela o encaixe.
  `nenhuma` volta ao comportamento antigo, de encaixar em qualquer arrasto. Também
  aceita `control` e `controle` para `ctrl`, `none` e `sempre` para `nenhuma`.
- `show_overlay`: desenha as zonas na tela durante o arrasto.
- `jogo_largura`: largura padrão da faixa do jogo quando o perfil não define uma.
- `processo`: nome do executável, como aparece no Gerenciador de Tarefas.
- `vigiar`: reencaixa a janela quando ela sai do lugar.
- `borda`: mantém a moldura e a barra de título. Desligado, a janela fica sem
  moldura e ocupa a zona inteira.
- `alinhar_cliente`: encaixa pelo cliente da janela em vez de pelo retângulo
  total, e desiste se o cliente não couber exatamente. É o modo para jogo que
  mede o cursor contra o cliente do momento em que foi criado.
- `preparar`: receita de preparação. Hoje só `civ5`.

## Civilization V: por que ele precisa de um modo próprio

O Civilization V mede a posição do mouse contra a área de cliente que existia
quando a janela foi criada no Direct3D. Se você tira a moldura e redimensiona
depois, o jogo estica a imagem e os cliques passam a cair no lugar errado: o
mapa responde a um ponto deslocado do cursor. Isso não é bug do ZoneSnap nem do
AutoHotkey, é como o jogo foi escrito.

A receita que funciona tem três partes, e o ZoneSnap faz as três:

1. O jogo precisa **nascer** com o cliente do tamanho da zona. Para isso o
   `.ini` do jogo tem que estar com `FullScreen = 0`, `WindowResX = 2300` e
   `WindowResY = 1440` **antes** de abrir.
2. O perfil precisa estar com `alinhar_cliente: true` e `borda: false`. Nesse
   modo o ZoneSnap não mexe no estilo da janela, mede o cliente e confere se ele
   bate com a zona com tolerância de 1 pixel. Se não bater, ele desfaz e avisa:
   `cliente ficou em 2347x1440 e a zona pede 2300x1440; rode --preparar-civ e
   abra o jogo de novo`.
3. O `.ini` só pode ser gravado com o jogo fechado, porque o jogo lê esse
   arquivo ao abrir e o regrava ao sair. O comando abaixo recusa rodar com o
   jogo aberto e guarda uma cópia com data antes de escrever:

```
zonesnap.exe --preparar-civ
```

### A conferência do `.ini`

O `.ini` é a única peça dessa receita que mora fora do ZoneSnap, então ele é
vigiado. A cada 10 segundos, e de novo a cada atualização da interface, o
programa lê o `GraphicsSettingsDX11.ini`, compara `FullScreen`, `WindowResX` e
`WindowResY` com a largura e a altura do perfil e escreve no registro uma linha
por mudança de conclusão:

```
ini de Civilization V (DX11): ini em 2347x1440, mas o perfil pede 2300x1440.
Com o jogo aberto não dá para corrigir: ele regrava o arquivo ao sair.
```

No cartão do jogo a mesma frase aparece em dois estados. Quando o `.ini` combina
com o perfil, ela sai em tom apagado, com a régua neutra e sem botão. Quando
diverge, vira uma faixa âmbar com o problema em texto claro e o botão
`Corrigir o .ini` ao lado, que só habilita com o jogo fechado; com ele aberto o
botão fica apagado e a própria frase explica por quê (`ele regrava o arquivo ao
sair`). Se o `.ini` ainda não existe, a frase diz que falta abrir o jogo uma vez
em modo janela. A mesma conferência está no menu do ícone, em `Jogos` >
`Conferir o .ini do Civ V`, com o resultado numa caixa de aviso.

Pelo terminal, sem abrir a interface e sem tocar no arquivo:

```
zonesnap.exe --checar-ini
ini existe=true jogo_aberto=false fullscreen=0 janela=2300x1440 alvo=2300x1440 divergente=false pode_corrigir=false
```

O comando não sobe o programa nem briga com a instância que já está rodando,
então serve para diagnosticar com o jogo aberto.

Fluxo completo, do zero:

1. Feche o Civilization V.
2. Rode `zonesnap.exe --preparar-civ`.
3. Abra o jogo pelo Steam, em DirectX 11.
4. Com o jogo aberto, `Ctrl+Alt+Z` (ou o botão `Prender agora` na interface).

Se você mudar a largura da faixa no perfil, rode o passo 2 de novo: o `.ini`
precisa combinar com o perfil, senão o encaixe é recusado de propósito, para não
esticar a imagem.

## Modo vídeo

O navegador não tem um modo de vídeo que respeite uma zona: a tela cheia dele
(`F11`) cobre o monitor inteiro, e em janela comum as abas e a barra de endereço
continuam em cima. O atalho `Ctrl+Alt+V` resolve isso com o navegador em foco: o
ZoneSnap mede a distância entre a janela e a página renderizada dentro dela,
esconde a moldura e empurra a página para começar exatamente no topo da tela. O
vídeo ocupa a coluna da zona do pé ao teto, sem faixa de abas em cima e sem
mostrar a área de trabalho.

O mesmo comando tira de volta: a moldura, a posição e o tamanho voltam ao que
estavam. O menu do ícone tem o item `Esconder a moldura do navegador` e a seção
Jogos da interface tem o painel do modo vídeo, que mostra quantas janelas estão
sem moldura neste momento.

Detalhes que valem saber:

- Funciona com navegadores do Chromium: Opera, Chrome, Edge, Brave e Vivaldi. O
  alvo é o navegador em foco. Quando quem está em foco é a própria interface do
  ZoneSnap, o menu do ícone ou uma janela que não tem moldura para medir (um app
  Electron, por exemplo), o comando pega o navegador logo atrás dessa janela e,
  se não houver, o maior navegador aberto. É isso que faz o botão da interface
  acertar o navegador que você estava usando.
- Se nenhum navegador estiver aberto, o atalho não mexe em nada e explica o
  motivo.
- Se a janela já estiver em tela cheia do navegador (`F11`), o ZoneSnap não faz
  nada e pede para você apertar `F11` antes.
- Com o navegador em segundo plano, o comando o traz para a frente para poder
  medir a página; é o comportamento esperado, já que a intenção é ver o vídeo.
- Se o programa fechar de repente, a janela pode ficar sem moldura; abrir o
  ZoneSnap de novo e usar o comando na mesma janela devolve tudo. O encerramento
  normal pelo menu devolve sozinho.

## Iniciar com o Windows

`Win+R`, digite `shell:startup` e ponha um atalho para `zonesnap.exe` na pasta
que abrir. Não precisa de tarefa agendada nem de administrador.

## Onde ficam os arquivos

| O que | Onde |
|---|---|
| Configuração | `%APPDATA%\ZoneSnap\config.json` |
| Registro | `%APPDATA%\ZoneSnap\zonesnap.log` |
| Token e porta da interface | `%APPDATA%\ZoneSnap\ui.token`, `ui.port` |
| Cache do WebView2 da janela | `%LOCALAPPDATA%\ZoneSnap\webview` |
| `.ini` do Civ V | `Documents\My Games\Sid Meier's Civilization 5\GraphicsSettingsDX11.ini` |

A pasta de documentos é resolvida pelo Windows, então funciona com OneDrive e com
o nome traduzido (`Documentos`).

## Limites conhecidos

- Só Windows 10 e 11, x64. Usa API do Windows direto, sem camada de
  compatibilidade.
- Uma instância por vez, garantida por mutex.
- Monitor virtual de streaming (Sunshine, Moonlight, VDD) é ignorado de
  propósito: encaixar janela num monitor que não existe fisicamente faz o jogo
  sumir da tela.
- Janela de jogo que ignora `SetWindowPos` não tem como ser encaixada. O
  ZoneSnap tenta três vezes, desiste e escreve o motivo no log.
- Jogos em tela cheia exclusiva não são afetados: eles não são janelas comuns.
  Use janela sem borda.
- Os atalhos são globais. Se outro programa já usa a combinação, o ZoneSnap cai
  para a variante com `Shift`; se as duas estiverem ocupadas, ele avisa no log e
  segue sem aquele atalho.
- A interface é uma página só, embutida no executável. Não tem autenticação de
  usuário, só o token local descrito acima.

## Comparação com o PowerToys FancyZones

| | ZoneSnap | FancyZones |
|---|---|---|
| Zonas por layout | sim, em JSON | sim, no editor |
| Encaixe ao arrastar | com a tecla apertada, `Ctrl` por padrão | com a tecla apertada, `Shift` por padrão |
| Encaixe por atalho numerado | sim | sim |
| Prender jogo num tamanho fixo com vigilância | sim | não |
| Encaixe pelo cliente da janela (Civ V) | sim | não |
| Regravar o `.ini` do jogo | sim | não |
| Modo vídeo (navegador sem moldura dentro da zona) | sim | não |
| Instalação | nenhuma, um `.exe` | instalador do PowerToys |
| Customização do código | Rust, MIT | C++, MIT |

## Recompilar

Precisa de Rust estável e Node 20 ou mais novo.

```
cd ui
pnpm install
pnpm build
cd ..
cargo build --release
```

O `cargo` embute `ui/dist/index.html` no executável com `include_str!`, então a
interface precisa ser gerada antes. Se o `cargo` reclamar que não achou
`../ui/dist/index.html`, é isso que faltou.

O resultado é `target\release\zonesnap.exe`, um arquivo só, de cerca de 1 MB.

## Licença

MIT. Os componentes de interface vieram do [shadcn/ui](https://github.com/shadcn-ui/ui),
também MIT, e mantêm o aviso de copyright nos arquivos. A lista completa de
componentes de terceiros e suas licenças está em [THIRD-PARTY.md](THIRD-PARTY.md).
