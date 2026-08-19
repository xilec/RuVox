# Delta: windows-installer

## ADDED Requirements

### Requirement: Uninstall data cleanup

The installed app SHALL keep its user data (config, history, audio
cache, downloaded voices) outside the installation directory. When the
user checks "Delete the application data" in the NSIS uninstaller, the
app's data root (`%LOCALAPPDATA%\<bundle identifier>`) SHALL be removed.
When the checkbox is not checked, the data root SHALL be preserved.

#### Scenario: Uninstall with data deletion

- GIVEN an installed RuVox on Windows with synthesized audio and
  downloaded voices
- WHEN the user uninstalls with "Delete the application data" checked
- THEN the install dir AND `%LOCALAPPDATA%\com.ruvox.app` are removed

#### Scenario: Uninstall keeping data

- GIVEN an installed RuVox on Windows with user data
- WHEN the user uninstalls without "Delete the application data"
- THEN the install dir is removed but the data root survives
