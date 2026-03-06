; convx Windows bootstrapper (Inno Setup)
; Usage:
;   iscc /DAppMsi="C:\path\to\convx_0.1.0_x64_en-US.msi" convx-bootstrapper.iss

#define MyAppName "convx"
#define MyAppPublisher "convx"
#ifndef AppVersion
  #define AppVersion "0.1.0"
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
OutputBaseFilename=convx-setup-bootstrapper-{#AppVersion}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
LicenseFile=..\EULA.txt

[Files]
Source: "{#AppMsi}"; DestDir: "{tmp}"; Flags: deleteafterinstall
Source: "..\bootstrap-windows.ps1"; DestDir: "{tmp}"; Flags: deleteafterinstall

[Run]
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{tmp}\bootstrap-windows.ps1"""; StatusMsg: "Installing prerequisites..."; Flags: runhidden waituntilterminated
Filename: "msiexec.exe"; Parameters: "/i ""{tmp}\{#ExtractFileName(AppMsi)}"" /passive /norestart"; StatusMsg: "Installing convx..."; Flags: waituntilterminated
Filename: "{autopf}\convx\convx.exe"; Description: "Launch convx"; Flags: nowait postinstall skipifsilent
