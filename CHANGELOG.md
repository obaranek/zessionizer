# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-11-01

### Added
- Initial release of Zessionizer
- Project session management with VIM-like keybindings
- Frecency-based project ranking (combines frequency and recency)
- Fuzzy search for filtering projects and sessions
- Two view modes:
  - Sessions view: Shows projects with active Zellij sessions
  - Projects view: Shows projects without active sessions
- Real-time filesystem monitoring for automatic project discovery
- JSON-based persistent storage with frecency scoring
- OpenTelemetry tracing support for debugging
- Active session indicator (`*` prefix)
- Catppuccin theme support (Mocha, Latte, Frappe, Macchiato)
- Custom theme support via TOML files

### Features
- **Keybindings**:
  - `j/k` or `Ctrl+n/p`: Navigate up/down
  - `/`: Enter search mode
  - `Enter`: Select project (create or switch to session)
  - `K`: Kill selected session
  - `n`: Switch to projects view
  - `s`: Switch to sessions view
  - `q`: Quit plugin
  - `Esc`: Exit search/close plugin
- **Search**: Multi-token fuzzy search with highlight visualization
- **Frecency**: Smart ranking based on access patterns
- **Auto-discovery**: Scans for `.git` directories and `.zessionizer` marker files
- **Storage**: Automatic session synchronization and frecency updates

[0.1.0]: https://github.com/obaranek/zessionizer/releases/tag/v0.1.0
