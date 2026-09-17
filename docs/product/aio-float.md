# AIO Float

AIO Float is the independent Windows x64 and macOS Apple Silicon floating
dashboard. It shares the AIO TUI renderer, palette, navigation and provider tests.
The display computer does not need AIO Coding Hub. Windows needs WebView2.

## Connect

1. Upgrade AIO Coding Hub on the computer providing the data.
2. Open Settings, enable LAN observation, and note its LAN IP and port (13799 by
   default). Copy the observation token.
3. Open AIO Float and enter that IP, port and token. Test and save the connection.
4. If the host firewall blocks the connection, allow the chosen TCP port for
   the private network. Neither application edits firewall rules automatically.

This connection is HTTP for a trusted LAN. It is not an Internet-facing service.
Tokens are independent of gateway credentials and the local TUI descriptor.
Disabling LAN observation, changing its port or rotating its token does not
restart or reconfigure the local TUI listener. AIO must remain running.

## Window

Drag the top summary to move the window; drag edges or corners to resize it.
Right-click opens settings, topmost, click-through, hide and exit. The Windows
tray/macOS menu bar always offers Show / Disable click-through. Closing the main
window hides it; `q` exits as in the TUI. A second launch restores the first window.

Ctrl/Cmd plus, minus and zero change/reset font size; Ctrl/Cmd-wheel zooms.
Ordinary wheel movement uses the shared TUI navigation. Font size is 8-32,
line height is 1.2 and cell width is 0.6. Cascadia Mono and its license are embedded;
Chinese characters use system fallback fonts. Background opacity leaves text
opaque. Zero opacity retains the smallest nonzero background alpha so unlocked
transparent regions remain draggable on Windows.

The application remembers geometry, appearance, CLI scope and connection address.
Tokens are stored in Windows Credential Manager or macOS Keychain, not the JSON
settings file. Hidden windows pause polling. A disconnected dashboard retains its
last snapshot; changing server clears it and rejects late responses from the old
connection. Authentication failures require an updated token.

## Build and delivery

All dependency installation, generated bindings, tests and native builds run in
GitHub Actions. `float-build.yml` checks Windows x64 and macOS ARM64, publishes
an EXE or application ZIP and SHA-256 checksums as workflow artifacts. The workflow
also runs on version tags and can be dispatched manually. These are independent
artifacts and are not added to the desktop updater manifest. Companion AIO builds
use the existing `dev-build` workflow for the same source revision.

macOS uses the project's existing ad-hoc signing arrangement; this does not claim
Apple notarization. Native window acceptance must be recorded separately from
cloud compilation, especially on macOS when no local machine is available.
