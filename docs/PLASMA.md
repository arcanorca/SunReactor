# Plasma widget

The widget is a Plasma 6 panel item with a popup. It is a QML package plus a small C++ plugin that talks to the daemon's control socket. It does no brightness logic and never touches hardware: it shows what `sunreactord` is doing and sends the same commands as `sunreactorctl`.

The popup has a Brightness page (day curve, one slider per display, automation mode with the next scheduled step) and a Weather page (conditions, forecast, sun times). The panel can show a weather icon, a brightness readout, a small curve or a ring gauge.

The widget follows the terminal UI's theme by default, or you can pin another theme. Many themes have their own layout and weather icons; the rest use a layout chosen by theme family. The pictures below are a selection. Everything is drawn with QML shapes and a few C++ items, no bitmap assets for the vector themes. Internals and the design notes are in [`DESIGN.md`](../plasma/org.sunreactor.plasma/DESIGN.md).

<div align="center">
  <img src="images/widget-themes.png" alt="Plasma widget, brightness page: macOS, PS3, Nothing, Synthwave 84, Hacker Green, Windows XP" width="95%" />
  <br/>
  <img src="images/widget-weather.png" alt="Plasma widget, weather page: macOS, Wii, Casio Digital" width="95%" />
</div>

Top: brightness page in macOS, PS3, Nothing, Synthwave 84, Hacker Green and Windows XP. Bottom: weather page in macOS, Wii and Casio Digital. The full theme list is in [THEMES.md](THEMES.md).

### Requirements
- Plasma 6 with KF6 (Kirigami, KPackage) and Qt 6 (Core, Gui, Qml, Quick, Network)
- CMake 3.22 or newer, `extra-cmake-modules` 6.0 or newer, and a C++20 compiler
- `sunreactord` from this version for themes and the day curve (older daemons still work, see Troubleshooting)

### Install
From a clone of the repository, the installer builds and installs the widget:

```bash
./install.sh                    # installs the widget when plasmashell and kpackagetool6 exist
./install.sh --with-plasma      # try even when those tools are not found
./install.sh --no-plasma        # skip the widget
./install.sh --dry-run          # print what the widget step would do; change nothing
systemctl --user restart plasma-plasmashell.service   # the installer never restarts plasmashell
```

The step builds the plugin with CMake in a temporary directory, copies the plugin and the QML module under `~/.local`, replaces the widget package in one piece, and keeps the previous widget in `~/.local/share/sunreactor-backups/`. A failed widget step is reported and never fails the installation or rolls back the daemon.

By hand, from the repository root:

```bash
cmake -S plasma/org.sunreactor.plasma -B build/plasma -DCMAKE_INSTALL_PREFIX="$HOME/.local"
cmake --build build/plasma -j"$(nproc)"
cmake --install build/plasma
systemctl --user restart plasma-plasmashell.service
```

`cmake --install` places the files here:

| Part | Location |
|---|---|
| C++ plugin | `~/.local/lib/libsunreactorplugin.so` |
| QML module | `~/.local/lib/qt6/qml/org/sunreactor/plasma/` (`qmldir`, `libsunreactorpluginplugin.so`, `libsunreactorplugin.so`, `sunreactorplugin.qmltypes`) |
| Widget package | `~/.local/share/plasma/plasmoids/org.sunreactor.plasma/` |
| Application icon | `~/.local/share/icons/hicolor/scalable/apps/` |

The installer passes `-DKDE_INSTALL_QMLDIR=lib/qt6/qml`, the Qt 6 layout; Plasma must find that directory on its QML import path (add `~/.local/lib/qt6/qml` to `QML2_IMPORT_PATH` in a `~/.config/plasma-workspace/env/*.sh` file if it is not there already).

**KF6 partial-upgrade caveat.** If CMake prints `Plasma not found: the plasmoid package will not be installed`, it could not load Plasma's CMake package. That usually means a half-finished upgrade of the KDE Frameworks and Plasma set. The plugin still builds and installs. Finish the upgrade and run CMake again, or install the package with `kpackagetool6 --type Plasma/Applet --install plasma/org.sunreactor.plasma/package`. The installer does not depend on Plasma's CMake package: it copies the package itself and validates it with `kpackagetool6` when that is present.

### Settings
The configure dialog of the widget holds these settings:

| Section | Keys | Meaning |
|---|---|---|
| General | `showTemperature`, `showSolarElevation` | show the outside temperature and the sun's elevation |
| General | `defaultSuspendMinutes`, `overrideDurationMinutes` | default pause length and how long a manual override lasts (0 = until resumed or never) |
| General | `socketPath` | empty uses the daemon's default socket under `XDG_RUNTIME_DIR` |
| Appearance | `themeMode`, `pinnedTheme` | follow the terminal UI's theme, or pin one (see [THEMES.md](THEMES.md#5-how-the-plasma-widget-uses-themes)) |
| Appearance | `accentOverride`, `densityOverride`, `pixelScale`, `rendererOverride` | override the theme's accent, density, art-pixel size and renderer |
| Appearance | `weatherIconStyle` | `auto`, `modern`, `line`, `solid` or `pixel` weather icons |
| Appearance | `moduleOrder`, `hiddenModules` | popup modules in order, and the ones to hide (`telemetry` starts hidden) |
| Panel | `panelMode`, `panelDisplay` | `sky`, `readout`, `sparkline` or `ring`, and the display it describes (empty = average of all) |

### Popup size
Plasma stores a popup size for each applet as `popupWidth` and `popupHeight` in the applet's `[Configuration]` section of `~/.config/plasma-org.kde.plasma.desktop-appletsrc`. The widget fits its popup window to the page on screen itself, so a stored height no longer keeps the window too tall or too short when the page changes. Plasma still records the size when you resize the window.

### Troubleshooting
- **The popup does not fit, or keeps a wrong height.** Stop plasmashell (`systemctl --user stop plasma-plasmashell.service`). Delete the `popupHeight=` and `popupWidth=` lines in the `[Containments][N][Applets][M][Configuration]` section of the widget's applet (the one whose `plugin=org.sunreactor.plasma`). Start plasmashell again. Do not edit the file while plasmashell runs, because it rewrites the file.
- **Text or art looks wrong after a scale change.** Text and pixel art follow Plasma's display scale and font DPI. Pixel themes size their art to the device pixel ratio while `pixelScale` is 0; set it to 1 to 6 to choose a size. If text clips at a fractional scale, check Plasma's display scale and font settings first.
- **Plasma reports a QML import error for `org.sunreactor.plasma`.** Plasma cannot see the QML module directory. Check `systemctl --user show-environment | grep QML`. The directory from the table above must be on `QML2_IMPORT_PATH` or `QML_IMPORT_PATH`. Add it with a file in `~/.config/environment.d/` and log in again.
- **The daemon is older than the widget.** The widget keeps running. It uses the theme in the daemon's flat fields, hides the light cycle because there is no day curve, and shows a pinned theme as followed, with a note in the settings. To get themes and the day curve, install `sunreactord` from this version and restart it with `systemctl --user restart sunreactord.service`.
- **The widget shows offline.** Check `systemctl --user status sunreactord.service` and `sunreactorctl status`.

Test and screenshot tooling is described in [`DESIGN.md`](../plasma/org.sunreactor.plasma/DESIGN.md) (section 7) and, when present, `plasma/org.sunreactor.plasma/tools/qa.sh`.
