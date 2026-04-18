; ConvX Windows bootstrapper (Inno Setup)

#define MyAppName "ConvX"
#define MyAppPublisher "JSB Technologies"
#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif
#ifndef AppMsi
  #error AppMsi define is required. Example: /DAppMsi="C:\path\to\convx.msi"
#endif

[Setup]
AppId={{E11F2AA0-46DB-4E79-BB2A-4F6F6A65A6EA}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\convx
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename=ConvX-Bootstrapper
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin

[Files]
Source: "{#AppMsi}"; DestDir: "{tmp}"; Flags: deleteafterinstall
Source: "..\bootstrap-windows.ps1"; DestDir: "{tmp}"; Flags: deleteafterinstall

[Run]
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{tmp}\bootstrap-windows.ps1"""; StatusMsg: "Preparing environment..."; Flags: runhidden waituntilterminated
Filename: "msiexec.exe"; Parameters: "/i ""{tmp}\{#ExtractFileName(AppMsi)}"" /passive /norestart"; StatusMsg: "Installing ConvX..."; Flags: waituntilterminated
Filename: "{autopf}\convx\convx.exe"; Description: "Launch ConvX"; Flags: nowait postinstall skipifsilent
