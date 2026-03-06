; convx Windows bundled installer (Inno Setup)
; Includes all dependencies (ffmpeg, vips, pandoc, poppler, LibreOffice, Python, wheels)
;
; Usage:
;   iscc /DAppMsi="C:\path\to\convx.msi" /DDepsDir="C:\path\to\deps" convx-bundled.iss

#define MyAppName "ConvX"
#define MyAppPublisher "Whefibo LLC"
#define MyAppExeName "convx.exe"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif
#ifndef AppMsi
  #error AppMsi define is required. Example: /DAppMsi="C:\path\to\convx.msi"
#endif
#ifndef DepsDir
  #error DepsDir define is required. Example: /DDepsDir="C:\path\to\deps"
#endif

[Setup]
AppId={{E11F2AA0-46DB-4E79-BB2A-4F6F6A65A6EA}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\convx
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename=ConvX-Setup-{#AppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
LicenseFile=..\EULA.txt
DiskSpanning=no

[Files]
; Tauri MSI (installed silently during setup)
Source: "{#AppMsi}"; DestDir: "{tmp}"; Flags: deleteafterinstall

; MCP server wrapper (unified binary with --mcp flag)
Source: "convx-mcp.cmd"; DestDir: "{app}"; Flags: ignoreversion

; Bundled dependencies
Source: "{#DepsDir}\bin\*"; DestDir: "{app}\deps\bin"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\lib\*"; DestDir: "{app}\deps\lib"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\LibreOffice\*"; DestDir: "{app}\deps\LibreOffice"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\python\*"; DestDir: "{app}\deps\python"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\wheels\*"; DestDir: "{app}\deps\wheels"; Flags: ignoreversion recursesubdirs

[Run]
; Install the Tauri MSI silently
Filename: "msiexec.exe"; Parameters: "/i ""{tmp}\{#ExtractFileName(AppMsi)}"" INSTALLDIR=""{app}"" /passive /norestart"; StatusMsg: "Installing ConvX application..."; Flags: waituntilterminated

; Create Python venv and install wheels offline
Filename: "{app}\deps\python\python.exe"; Parameters: "-m venv ""{userappdata}\.convx\venv"""; StatusMsg: "Setting up Python environment..."; Flags: runhidden waituntilterminated runasoriginaluser

; Install wheels offline into venv
Filename: "{userappdata}\.convx\venv\Scripts\pip.exe"; Parameters: "install --no-index --find-links ""{app}\deps\wheels"" pandas openpyxl weasyprint pdf2docx mobi pyarrow numpy h5py"; StatusMsg: "Installing Python packages..."; Flags: runhidden waituntilterminated runasoriginaluser

; Launch app
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent

[Registry]
; Add deps\bin and deps\lib to system PATH so convx can find bundled binaries
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}\deps\bin;{app}\deps\lib"; Check: NeedsAddPath('{app}\deps\bin')

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\.convx\venv"
