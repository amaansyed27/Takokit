#define MyAppName "Takokit"
#define MyAppVersion "0.0.1"
#define MyAppPublisher "Dawnlight Labs"

#ifndef SourceRoot
  #error SourceRoot must point to the assembled installed application tree
#endif
#ifndef OutputRoot
  #define OutputRoot "..\..\dist\windows"
#endif

[Setup]
AppId={{C5EC7671-2A42-43A6-9ED4-BC9FE091BC91}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
VersionInfoVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription=Takokit local voice AI runtime
DefaultDirName={localappdata}\Programs\Takokit
DefaultGroupName=Takokit
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputRoot}
OutputBaseFilename=Takokit-v{#MyAppVersion}-windows-x86_64-installer
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\..\assets\favicon\favicon.ico
UninstallDisplayIcon={app}\bin\Takokit.exe
UninstallDisplayName=Takokit {#MyAppVersion}
ChangesEnvironment=yes
CloseApplications=no
RestartApplications=no
UsePreviousAppDir=yes
UsePreviousGroup=yes
DisableWelcomePage=no
AllowNoIcons=yes

[Files]
Source: "{#SourceRoot}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Takokit"; Filename: "{app}\bin\Takokit.exe"; WorkingDir: "{localappdata}\Takokit"; Comment: "Takokit local voice AI runtime"
Name: "{autoprograms}\Takokit (TUI)"; Filename: "{app}\bin\tako.exe"; WorkingDir: "{userprofile}"; Comment: "Open Takokit in a terminal"; Flags: createonlyiffileexists

[Code]
const
  WM_SETTINGCHANGE = $001A;
  HWND_BROADCAST = $FFFF;
  SMTO_ABORTIFHUNG = $0002;

function SendMessageTimeout(hWnd: HWND; Msg: UINT; wParam: WPARAM;
  lParam: string; fuFlags, uTimeout: UINT; var lpdwResult: DWORD): LRESULT;
  external 'SendMessageTimeoutW@user32.dll stdcall';

function TrimTrailingSlash(Value: string): string;
begin
  Result := RemoveBackslashUnlessRoot(Trim(Value));
end;

function PathEntryEquals(Left, Right: string): Boolean;
begin
  Result := CompareText(TrimTrailingSlash(Left), TrimTrailingSlash(Right)) = 0;
end;

function RemoveExactPathEntry(ExistingPath, OwnedEntry: string): string;
var
  Remaining: string;
  Entry: string;
  Separator: Integer;
begin
  Remaining := ExistingPath;
  Result := '';
  while Length(Remaining) > 0 do
  begin
    Separator := Pos(';', Remaining);
    if Separator = 0 then
    begin
      Entry := Remaining;
      Remaining := '';
    end
    else
    begin
      Entry := Copy(Remaining, 1, Separator - 1);
      Delete(Remaining, 1, Separator);
    end;

    Entry := Trim(Entry);
    if (Entry <> '') and (not PathEntryEquals(Entry, OwnedEntry)) then
    begin
      if Result <> '' then
        Result := Result + ';';
      Result := Result + Entry;
    end;
  end;
end;

function PathContainsExact(ExistingPath, OwnedEntry: string): Boolean;
var
  WithoutOwned: string;
begin
  WithoutOwned := RemoveExactPathEntry(ExistingPath, OwnedEntry);
  Result := CompareText(WithoutOwned, ExistingPath) <> 0;
end;

procedure BroadcastEnvironmentChange;
var
  BroadcastResult: DWORD;
begin
  SendMessageTimeout(HWND_BROADCAST, WM_SETTINGCHANGE, 0, 'Environment',
    SMTO_ABORTIFHUNG, 5000, BroadcastResult);
end;

procedure AddOwnedPathEntry;
var
  ExistingPath: string;
  OwnedEntry: string;
  NewPath: string;
begin
  OwnedEntry := ExpandConstant('{app}\bin');
  if not RegQueryStringValue(HKCU, 'Environment', 'Path', ExistingPath) then
    ExistingPath := '';

  { Normalize away duplicate Takokit entries before adding our exact current path. }
  NewPath := RemoveExactPathEntry(ExistingPath, OwnedEntry);
  if NewPath <> '' then
    NewPath := NewPath + ';';
  NewPath := NewPath + OwnedEntry;

  if CompareText(NewPath, ExistingPath) <> 0 then
  begin
    RegWriteExpandStringValue(HKCU, 'Environment', 'Path', NewPath);
    BroadcastEnvironmentChange;
  end;
end;

procedure RemoveOwnedPathEntry;
var
  ExistingPath: string;
  OwnedEntry: string;
  NewPath: string;
begin
  OwnedEntry := ExpandConstant('{app}\bin');
  if not RegQueryStringValue(HKCU, 'Environment', 'Path', ExistingPath) then
    Exit;

  NewPath := RemoveExactPathEntry(ExistingPath, OwnedEntry);
  if CompareText(NewPath, ExistingPath) <> 0 then
  begin
    if NewPath = '' then
      RegDeleteValue(HKCU, 'Environment', 'Path')
    else
      RegWriteExpandStringValue(HKCU, 'Environment', 'Path', NewPath);
    BroadcastEnvironmentChange;
  end;
end;

procedure StopOwnedTakokitDaemon;
var
  TakoExe: string;
  ResultCode: Integer;
begin
  TakoExe := ExpandConstant('{app}\bin\tako.exe');
  if FileExists(TakoExe) then
  begin
    { tako daemon stop verifies the persisted runtime identity, executable,
      storage root and instance id before it requests shutdown. The installer
      never terminates a process by image name. }
    Exec(TakoExe, 'daemon stop', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): string;
begin
  StopOwnedTakokitDaemon;
  Result := '';
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    AddOwnedPathEntry;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    StopOwnedTakokitDaemon;
    RemoveOwnedPathEntry;
  end;
end;
