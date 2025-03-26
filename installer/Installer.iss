; Script generated by the Inno Setup Script Wizard.
; SEE THE DOCUMENTATION FOR DETAILS ON CREATING INNO SETUP SCRIPT FILES!
#include "CodeDependencies.iss"

#define MyAppName "Azookey"
#define MyAppVersion "0.0.1"
#define MyAppPublisher "fkunn1326"
#define MyAppURL "https://github.com/fkunn1326/azooKey-Windows/"

[Setup]
; NOTE: The value of AppId uniquely identifies this application. Do not use the same AppId value in installers for other applications.
; (To generate a new GUID, click Tools | Generate GUID inside the IDE.)
AppId={{80B746D4-D74D-4345-8F81-47E06BCAB515}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
;AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={userappdata}\{#MyAppName}
; "ArchitecturesAllowed=x64compatible" specifies that Setup cannot run
; on anything but x64 and Windows 11 on Arm.
ArchitecturesAllowed=x64compatible
; "ArchitecturesInstallIn64BitMode=x64compatible" requests that the
; install be done in "64-bit mode" on x64 or Windows 11 on Arm,
; meaning it should use the native 64-bit Program Files directory and
; the 64-bit view of the registry.
ArchitecturesInstallIn64BitMode=x64compatible
DisableProgramGroupPage=yes
; Uncomment the following line to run in non administrative install mode (install for current user only).
;PrivilegesRequired=lowest
OutputDir=../build
OutputBaseFilename=azookey-setup
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Files]
Source: "../build/azookey_windows.dll"; DestDir: "{app}"; DestName: "azookey.dll"; Flags: ignoreversion regserver 64bit
Source: "../build/x86/azookey_windows.dll"; DestDir: "{app}"; DestName: "azookey32.dll"; Flags: ignoreversion regserver 32bit
Source: "../build/*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "../target/release/bundle/nsis/Azookey_0.1.0_x64-setup.exe"; Flags: dontcopy noencryption
; NOTE: Don't use "Flags: ignoreversion" on any shared system files

[Run]
Filename: "schtasks"; \
  Parameters: "/Create /F /RL highest /SC ONLOGON /TN ""Azookey Startup"" /TR ""wscript.exe {app}\launch.vbs"""; \
  Description: "Automatically run on logon"; \
  Flags: runhidden postinstall runascurrentuser
Filename: "schtasks"; \
  Parameters: "/Run /TN ""Azookey Startup"""; \
  Description: "Run now"; \
  Flags: runhidden postinstall runascurrentuser nowait 
Filename: "icacls"; \
  Parameters: "{app}\azookey.dll /grant ""*S-1-15-2-1:(RX)"""; \
  Description: "Grant Permission"; \
  Flags: runhidden postinstall runascurrentuser
Filename: "icacls"; \
  Parameters: "{app}\azookey32.dll /grant ""*S-1-15-2-1:(RX)"""; \
  Description: "Grant Permission"; \
  Flags: runhidden postinstall runascurrentuser

[UninstallRun]
Filename: "schtasks"; \
  Parameters: "/Delete /TN ""Azookey Startup"" /F"; \
  Flags: runhidden runascurrentuser

[Code]
function InitializeSetup: Boolean;
begin
  ExtractTemporaryFile('Azookey_0.1.0_x64-setup.exe');
  Dependency_Add('Azookey_0.1.0_x64-setup.exe',
    '/q',
    'Azookey',
    '', '', True, False);

  Result := True;
end;

function UninstallNeedRestart(): Boolean;
begin
  Result := True;
end;

function CmdLineContains(Param: String): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
  begin
    if CompareText(ParamStr(I), Param) = 0 then
    begin
      Result := True;
      Exit;
    end;
  end;
end;

procedure CreateVbsFile();
var
  VbsFile: string;
  VbsContent: AnsiString;
begin
  VbsFile := ExpandConstant('{app}\launch.vbs');
  VbsContent :=
    'Set objShell = CreateObject("WScript.Shell")' + #13#10 +
    'objShell.Run "' + ExpandConstant('{app}\launcher.exe') + '", 0, False' + #13#10;

  if SaveStringToFile(VbsFile, VbsContent, False) then
  begin
    Log('VBS file created: ' + VbsFile);
  end
  else
  begin
    Log('Failed to create VBS file: ' + VbsFile);
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    CreateVbsFile();
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpFinished then
    WizardForm.RunList.Visible := False;
end;