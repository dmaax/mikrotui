# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.3](https://github.com/dmaax/mikrotui/compare/v0.4.2...v0.4.3) - 2026-08-21

### Added

- authenticate with an SSH private key

## [0.4.2](https://github.com/dmaax/mikrotui/compare/v0.4.1...v0.4.2) - 2026-08-21

### Added

- fetch only the visible tab, and add --refresh

## [0.4.1](https://github.com/dmaax/mikrotui/compare/v0.4.0...v0.4.1) - 2026-08-21

### Fixed

- correct dated log lines, stale rows and two divergent-index paths

## [0.4.0](https://github.com/dmaax/mikrotui/compare/v0.3.1...v0.4.0) - 2026-08-21

### Fixed

- [**breaking**] close two host key and read-only bypasses found in review
- stop the TUI from stranding itself on a hang, a stale ping or a panic

## [0.3.1](https://github.com/dmaax/mikrotui/compare/v0.3.0...v0.3.1) - 2026-08-21

### Added

- make the data tables scrollable

### Fixed

- keep the selection inside the list when a refresh shrinks it

## [0.3.0](https://github.com/dmaax/mikrotui/compare/v0.2.2...v0.3.0) - 2026-08-21

### Added

- [**breaking**] verify SSH host keys and stop persisting passwords by default
- [**breaking**] enforce read-only with a command allowlist

### Fixed

- [**breaking**] upgrade russh to 0.63 to close a remotely reachable DoS
- enable the keychain store so the macOS keyring build compiles
- point at --accept-new-hostkey when an unknown key blocks a script

### Other

- gate pushes and pull requests on tests, lints and advisories
- clear the remaining clippy warnings so CI can gate on them
- apply rustfmt across the tree

## [0.2.2](https://github.com/dmaax/mikrotui/compare/v0.2.1...v0.2.2) - 2026-08-07

### Added

- add Network Neighbors tab (/ip neighbor) for MNDP/CDP/LLDP discovery

## [0.2.1](https://github.com/dmaax/mikrotui/compare/v0.2.0...v0.2.1) - 2026-08-07

### Added

- configure release-plz for automatic semver versioning and release management based on conventional commits
- add smart version check on GitHub Action publish workflow and use dynamic env!(CARGO_PKG_VERSION)

### Fixed

- use double brackets [[package]] syntax in release-plz.toml

### Other

- eliminate all compiler warnings for clean builds
