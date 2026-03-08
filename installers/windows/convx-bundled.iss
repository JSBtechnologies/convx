; ConvX Windows bundled installer (Inno Setup)
;
; Usage:
;   iscc /DAppMsi="C:\path\to\convx.msi" /DDepsDir="C:\path\to\deps" convx-bundled.iss

#define MyAppName "ConvX"
#define MyAppPublisher "JSB Technologies"
#define MyAppExeName "convx.exe"
#ifndef AppVersion
  #define AppVersion "1.0.0"
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
OutputBaseFilename=ConvX-Setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
LicenseFile=..\EULA.txt
DiskSpanning=no

[Files]
Source: "{#AppMsi}"; DestDir: "{tmp}"; Flags: deleteafterinstall
Source: "convx-mcp.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#DepsDir}\bin\*"; DestDir: "{app}\deps\bin"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\lib\*"; DestDir: "{app}\deps\lib"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\LibreOffice\*"; DestDir: "{app}\deps\LibreOffice"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\python\*"; DestDir: "{app}\deps\python"; Flags: ignoreversion recursesubdirs
Source: "{#DepsDir}\wheels\*"; DestDir: "{app}\deps\wheels"; Flags: ignoreversion recursesubdirs

[Run]
Filename: "msiexec.exe"; Parameters: "/i ""{tmp}\{#ExtractFileName(AppMsi)}"" INSTALLDIR=""{app}"" /passive /norestart"; StatusMsg: "Installing ConvX..."; Flags: waituntilterminated
Filename: "{app}\deps\python\Scripts\pip.exe"; Parameters: "install --no-index --find-links ""{app}\deps\wheels"" pandas openpyxl weasyprint pdf2docx PyMuPDF mobi pyarrow numpy h5py"; StatusMsg: "Configuring components..."; Flags: runhidden waituntilterminated runasoriginaluser
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
