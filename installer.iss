#define MyAppName "Azookey"
#define MyAppVersion "0.1"
#define MyAppPublisher "fkunn1326"
#define MyAppURL "https://github.com/fkunn1326/azookey-windows"

[Setup]
;; basic info
AppId={{DDEBE3AB-2FAE-426A-9E41-AAAEAE359C72}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultGroupName={#MyAppName}
DefaultDirName={autopf}\{#MyAppName}
;; icon and style
WizardStyle=modern
WizardSizePercent=100
;; allow user to disable start menu shorcuts
AllowNoIcons=yes
;; compile
OutputDir=.\target\release
OutputBaseFilename=azookey-installer
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64

[Languages]
Name: "japanese"; MessagesFile: "compiler:Default.isl"


[Files]
Source: ".\target\release\azookey_windows.dll"; DestDir: "{app}"; DestName: "azookey.dll"; Flags: ignoreversion regserver 64bit
Source: ".\target\i686-pc-windows-msvc\release\azookey_windows.dll"; DestDir: "{app}"; DestName: "azookey32.dll"; Flags: ignoreversion regserver 32bit

[Icons]
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"