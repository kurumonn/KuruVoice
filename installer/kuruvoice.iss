#define EnvVersion GetEnv("KURUVOICE_VERSION")
#if EnvVersion == ""
#define MyAppVersion "0.1.0"
#else
#define MyAppVersion EnvVersion
#endif

#define MyAppName "KuruVoice"
#define MyAppPublisher "kurumonn"
#define MyAppExeName "kuruvoice.exe"

[Setup]
AppId={{A3F6C9F0-4A28-4F44-9B6B-54E25D2F13F9}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\KuruVoice
DefaultGroupName=KuruVoice
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=KuruVoiceSetup-v{#MyAppVersion}-windows-x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#MyAppExeName}
PrivilegesRequired=lowest

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "デスクトップにショートカットを作成する"; GroupDescription: "追加オプション"; Flags: unchecked

[Files]
Source: "..\target\x86_64-pc-windows-msvc\release\kuruvoice.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\config.example.toml"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\docs\*"; DestDir: "{app}\docs"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

[Icons]
Name: "{group}\KuruVoice"; Filename: "{app}\{#MyAppExeName}"
Name: "{commondesktop}\KuruVoice"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "KuruVoice を起動する"; Flags: nowait postinstall skipifsilent
