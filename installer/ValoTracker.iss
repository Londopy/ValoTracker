; ValoTracker Installer Script
; Built with Inno Setup 6 — https://jrsoftware.org/isinfo.php
;
; Usage (local):
;   iscc /DAppVersion=1.0.1 installer\ValoTracker.iss
;
; Usage (CI — version passed automatically):
;   Handled by .github/workflows/release.yml

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#define AppName      "ValoTracker"
#define AppPublisher "Londopy"
#define AppURL       "https://github.com/Londopy/ValoTracker"

[Setup]
; Unique app ID — do not change after first release (used for upgrades/uninstall)
AppId={{A3F1C2D4-8B5E-4F6A-9C2D-1E7B3A4F5D6C}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
AppUpdatesURL={#AppURL}/releases

; Install to Program Files\ValoTracker by default
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
AllowNoIcons=yes

; Require admin rights to write to Program Files
PrivilegesRequired=admin

; License shown on the second wizard page
LicenseFile=LICENSE.txt

; Output
OutputDir=..\dist
OutputBaseFilename=ValoTracker-Setup-{#AppVersion}

; Compression
Compression=lzma2
SolidCompression=yes

; Appearance
WizardStyle=modern
WizardSizePercent=120
SetupIconFile=ValoTracker.ico

; 64-bit Windows only
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible

; Uninstall icon shown in Add/Remove Programs
UninstallDisplayName={#AppName} {#AppVersion}
UninstallDisplayIcon={app}\ValoTracker.exe

; Version info embedded in the setup exe
VersionInfoVersion={#AppVersion}
VersionInfoCompany={#AppPublisher}
VersionInfoDescription={#AppName} Installer
VersionInfoProductName={#AppName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

; ── Components ────────────────────────────────────────────────────────────────
; TUI is required; GUI is optional.

[Components]
Name: "tui"; \
  Description: "ValoTracker (Terminal)  —  lightweight terminal UI"; \
  Types: full compact custom; \
  Flags: fixed

Name: "gui"; \
  Description: "ValoTracker GUI  —  native desktop window (egui)"; \
  Types: full

; ── Tasks (checkboxes on the "Select Additional Tasks" page) ──────────────────

[Tasks]
Name: "desktopicon_tui"; \
  Description: "Create a desktop shortcut for ValoTracker (Terminal)"; \
  GroupDescription: "Desktop shortcuts:"; \
  Components: tui

Name: "desktopicon_gui"; \
  Description: "Create a desktop shortcut for ValoTracker GUI"; \
  GroupDescription: "Desktop shortcuts:"; \
  Components: gui

Name: "startmenu"; \
  Description: "Add entries to the Start Menu"; \
  GroupDescription: "Start Menu:"; \
  Flags: checkedonce

; ── Files ─────────────────────────────────────────────────────────────────────

[Files]
; TUI binary
Source: "..\target\release\valotracker.exe"; \
  DestDir: "{app}"; \
  DestName: "ValoTracker.exe"; \
  Components: tui; \
  Flags: ignoreversion

; GUI binary
Source: "..\target\release\valotracker-gui.exe"; \
  DestDir: "{app}"; \
  DestName: "ValoTracker-GUI.exe"; \
  Components: gui; \
  Flags: ignoreversion

; ── Shortcuts ─────────────────────────────────────────────────────────────────

[Icons]
; Start Menu — TUI
Name: "{group}\ValoTracker (Terminal)"; \
  Filename: "{app}\ValoTracker.exe"; \
  Comment: "Real-time VALORANT match tracker — terminal UI"; \
  Tasks: startmenu; \
  Components: tui

; Start Menu — GUI
Name: "{group}\ValoTracker GUI"; \
  Filename: "{app}\ValoTracker-GUI.exe"; \
  Comment: "Real-time VALORANT match tracker — desktop window"; \
  Tasks: startmenu; \
  Components: gui

; Start Menu — Uninstall
Name: "{group}\Uninstall ValoTracker"; \
  Filename: "{uninstallexe}"; \
  Tasks: startmenu

; Desktop — TUI
Name: "{autodesktop}\ValoTracker"; \
  Filename: "{app}\ValoTracker.exe"; \
  IconFilename: "{app}\ValoTracker-GUI.exe"; \
  Comment: "Real-time VALORANT match tracker"; \
  Tasks: desktopicon_tui; \
  Components: tui

; Desktop — GUI
Name: "{autodesktop}\ValoTracker GUI"; \
  Filename: "{app}\ValoTracker-GUI.exe"; \
  IconFilename: "{app}\ValoTracker-GUI.exe"; \
  Comment: "Real-time VALORANT match tracker — desktop window"; \
  Tasks: desktopicon_gui; \
  Components: gui

; ── Post-install launch option ────────────────────────────────────────────────

[Run]
Filename: "{app}\ValoTracker.exe"; \
  Description: "Launch ValoTracker (Terminal)"; \
  Flags: nowait postinstall skipifsilent unchecked; \
  Components: tui

Filename: "{app}\ValoTracker-GUI.exe"; \
  Description: "Launch ValoTracker GUI"; \
  Flags: nowait postinstall skipifsilent unchecked; \
  Components: gui
