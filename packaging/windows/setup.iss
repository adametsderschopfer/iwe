; Build with the pinned Inno Setup compiler through package-windows.ps1.
#ifndef AppVersion
  #error AppVersion must be provided
#endif
#ifndef FileVersion
  #error FileVersion must be provided
#endif
#ifndef PayloadDirectory
  #error PayloadDirectory must be provided
#endif
#ifndef OutputDirectory
  #error OutputDirectory must be provided
#endif
#ifndef AppArchitecture
  #error AppArchitecture must be provided
#endif

#define AppName "IWE Code"
#define AppId "adametsderschopfer.iwe-code"

[Setup]
AppId={#AppId}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppName}
AppPublisherURL=https://github.com/adametsderschopfer/iwe-code
AppSupportURL=https://github.com/adametsderschopfer/iwe-code/issues
AppUpdatesURL=https://github.com/adametsderschopfer/iwe-code/releases
DefaultDirName={localappdata}\Programs\IWE Code
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
MinVersion=10.0
#if AppArchitecture == "x64"
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#else
ArchitecturesAllowed=x86compatible
#endif
DisableProgramGroupPage=yes
DisableWelcomePage=no
UsePreviousAppDir=yes
UsePreviousTasks=yes
WizardStyle=modern dynamic windows11
WizardSizePercent=120,120
SetupIconFile={#PayloadDirectory}\icon.ico
WizardImageFile={#PayloadDirectory}\welcome.png
WizardImageFileDynamicDark={#PayloadDirectory}\welcome.png
WizardSmallImageFile={#PayloadDirectory}\icon.png
WizardSmallImageFileDynamicDark={#PayloadDirectory}\icon.png
WizardImageBackColor=$181818
WizardImageBackColorDynamicDark=$181818
UninstallDisplayIcon={app}\iwe.exe
UninstallDisplayName={#AppName}
OutputDir={#OutputDirectory}
OutputBaseFilename=iwe-code-{#AppVersion}-windows-{#AppArchitecture}-setup
VersionInfoVersion={#FileVersion}
VersionInfoProductName={#AppName}
VersionInfoDescription={#AppName} Setup
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=no
#ifdef SignedBuild
SignTool=IweCode
SignedUninstaller=yes
#else
SignedUninstaller=no
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

#include PayloadDirectory + "\messages.iss"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#PayloadDirectory}\iwe.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDirectory}\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\{#AppName}"; Filename: "{app}\iwe.exe"; WorkingDir: "{app}"
Name: "{userdesktop}\{#AppName}"; Filename: "{app}\iwe.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\iwe.exe"; WorkingDir: "{app}"; Description: "{cm:LaunchProgram}"; Flags: nowait postinstall skipifsilent
