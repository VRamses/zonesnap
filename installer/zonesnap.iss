; Instalador do ZoneSnap (Inno Setup 6).
;
; Instala por usuário, em %LOCALAPPDATA%\Programs\ZoneSnap, sem pedir
; administrador, igual ao resto do programa. Com /ALLUSERS na linha de comando
; ele vai para Program Files e pede elevação.
;
; O executável vem de target\release, então o build tem que vir antes:
; installer\gerar.ps1 compila e chama o ISCC com a versão lida do Cargo.toml.

#define MinhaApp "ZoneSnap"
#ifndef MinhaVersao
  #define MinhaVersao "0.0.0"
#endif
#define MinhaUrl "https://github.com/VRamses/zonesnap"

[Setup]
; Nunca mudar este GUID: é ele que faz o Windows tratar as versões como o mesmo
; programa, atualizando no lugar em vez de instalar uma cópia ao lado.
AppId={{8F2A6C1E-3B4D-4E5F-9A7B-2C3D4E5F6A7B}
AppName={#MinhaApp}
AppVersion={#MinhaVersao}
AppVerName={#MinhaApp} {#MinhaVersao}
AppPublisher=Victor Ramses
AppPublisherURL={#MinhaUrl}
AppSupportURL={#MinhaUrl}/issues
AppUpdatesURL={#MinhaUrl}/releases
DefaultDirName={autopf}\{#MinhaApp}
DefaultGroupName={#MinhaApp}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=commandline
OutputDir=..\dist
OutputBaseFilename=zonesnap-{#MinhaVersao}-setup
SetupIconFile=..\res\zonesnap.ico
UninstallDisplayIcon={app}\zonesnap.exe
UninstallDisplayName={#MinhaApp}
LicenseFile=..\LICENSE
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Fecha uma instância em execução pelo Restart Manager em vez de mandar o
; usuário fechar na mão; depois não reabre sozinho, quem abre é o [Run].
CloseApplications=yes
RestartApplications=no
VersionInfoVersion={#MinhaVersao}.0
VersionInfoCompany=Victor Ramses
VersionInfoDescription=Instalador do ZoneSnap
VersionInfoProductName={#MinhaApp}
VersionInfoProductVersion={#MinhaVersao}

; Assinatura: só entra no build quando o gerar.ps1 recebe -Thumbprint, que passa
; /DAssinar e define a ferramenta por -s na linha de comando do ISCC. O Inno
; Setup 6 não tem seção [SignTools]: definir a ferramenta só no script dá
; "Value of [Setup] section directive SignTool is invalid".
#ifdef Assinar
SignTool=meu
SignedUninstaller=yes
#endif

[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "Criar um atalho na área de trabalho"; GroupDescription: "Atalhos:"
Name: "autostart"; Description: "Iniciar o ZoneSnap junto com o Windows"; GroupDescription: "Início automático:"; Flags: unchecked

[Files]
Source: "..\target\release\zonesnap.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Atalho direto no Menu Iniciar, sem pasta de grupo: é um programa só.
Name: "{userprograms}\{#MinhaApp}"; Filename: "{app}\zonesnap.exe"; Comment: "Zonas de encaixe e encaixe de jogos"
Name: "{autodesktop}\{#MinhaApp}"; Filename: "{app}\zonesnap.exe"; Comment: "Zonas de encaixe e encaixe de jogos"; Tasks: desktopicon
Name: "{userstartup}\{#MinhaApp}"; Filename: "{app}\zonesnap.exe"; Tasks: autostart

[Run]
Filename: "{app}\zonesnap.exe"; Description: "Abrir o ZoneSnap agora"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; O desinstalador do Inno não usa o Restart Manager, então o ZoneSnap aberto
; seguraria o exe e a pasta sobraria. O app é de bandeja e não tem janela para
; pedir "feche antes": encerra ele aqui, antes de apagar os arquivos.
Filename: "{sys}\taskkill.exe"; Parameters: "/IM zonesnap.exe /F"; Flags: runhidden; RunOnceId: "EncerraZoneSnap"

[Code]
const
  ChaveWebView2 = 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';

function WebView2Instalado(): Boolean;
var
  versao: String;
begin
  Result :=
    RegQueryStringValue(HKLM, 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', versao) or
    RegQueryStringValue(HKLM, ChaveWebView2, 'pv', versao) or
    RegQueryStringValue(HKCU, ChaveWebView2, 'pv', versao);
end;

function InitializeSetup(): Boolean;
begin
  Result := True;
  if not WebView2Instalado() then
  begin
    if MsgBox('O ZoneSnap desenha a interface com o WebView2, e esse componente não foi encontrado neste computador.' + #13#10 + #13#10 +
      'O Windows 11 já o traz, e o Edge também instala. Sem ele o programa abre, mas a janela fica em branco.' + #13#10 + #13#10 +
      'Continuar a instalação mesmo assim?', mbConfirmation, MB_YESNO) = IDNO then
      Result := False;
  end;
end;

// O Inno não apaga, numa atualização, o atalho de uma tarefa que o usuário
// desmarcou agora. Sem isto, desmarcar "iniciar com o Windows" no futuro deixaria
// o atalho antigo na pasta Startup.
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep <> ssPostInstall then
    exit;
  if not WizardIsTaskSelected('desktopicon') then
    DeleteFile(ExpandConstant('{autodesktop}\ZoneSnap.lnk'));
  if not WizardIsTaskSelected('autostart') then
    DeleteFile(ExpandConstant('{userstartup}\ZoneSnap.lnk'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  dados: String;
begin
  if CurUninstallStep <> usPostUninstall then
    exit;
  // Desinstalação silenciosa não pode perguntar nada, e o padrão seguro é
  // manter a configuração de quem vai reinstalar.
  if UninstallSilent then
    exit;
  dados := ExpandConstant('{userappdata}\ZoneSnap');
  if not DirExists(dados) then
    exit;
  if MsgBox('Apagar também a configuração do ZoneSnap?' + #13#10 + #13#10 +
    'Em ' + dados + ' ficam o config.json, o log e o token da interface.' + #13#10 +
    'Responda Não para manter os seus ajustes e apagar só o programa.', mbConfirmation, MB_YESNO) = IDYES then
  begin
    DelTree(dados, True, True, True);
    DelTree(ExpandConstant('{localappdata}\ZoneSnap'), True, True, True);
  end;
end;
