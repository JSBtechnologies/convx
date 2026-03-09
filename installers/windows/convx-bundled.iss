; ConvX Windows bundled installer (Inno Setup)
;
; Usage:
;   iscc /DTauriDir="C:\path\to\tauri-output" /DDepsDir="C:\path\to\deps" convx-bundled.iss

#define MyAppName "ConvX"
#define MyAppPublisher "JSB Technologies"
#define MyAppExeName "convx-app.exe"
#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif
#ifndef TauriDir
  #error TauriDir define is required. Path to directory containing convx-app.exe from Tauri build.
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
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename=ConvX-Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
LicenseFile=..\EULA.txt
DiskSpanning=no

[Files]
; Tauri app binary + resources
Source: "{#TauriDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs

; MCP wrapper
Source: "convx-mcp.cmd"; DestDir: "{app}"; Flags: ignoreversion

; Bundled dependencies
Source: "{#DepsDir}\bin\*"; DestDir: "{app}\deps\bin"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\lib\*"; DestDir: "{app}\deps\lib"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\LibreOffice\*"; DestDir: "{app}\deps\LibreOffice"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\python\*"; DestDir: "{app}\deps\python"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\wheels\*"; DestDir: "{app}\deps\wheels"; Flags: ignoreversion recursesubdirs

[Run]
; Install Python packages from bundled wheels using bundled pip (no venv needed)
Filename: "{app}\deps\python\python.exe"; Parameters: "-m pip install --no-index --find-links ""{app}\deps\wheels"" pandas openpyxl weasyprint pdf2docx PyMuPDF mobi pyarrow numpy h5py"; StatusMsg: "Setting up conversion tools..."; Flags: runhidden waituntilterminated runasoriginaluser
; Launch app
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent

[Registry]
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
