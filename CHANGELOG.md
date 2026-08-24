# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.9](https://github.com/dmaax/mikrotui/compare/v0.4.8...v0.4.9) - 2026-08-24

### Fixed

- keyring calls panicked when made from inside the async runtime

## [0.4.8](https://github.com/dmaax/mikrotui/compare/v0.4.7...v0.4.8) - 2026-08-24

### Added

- 238 base16 themes, with a searchable picker

## [0.4.7](https://github.com/dmaax/mikrotui/compare/v0.4.6...v0.4.7) - 2026-08-22

### Added

- drop the least useful columns instead of squeezing all of them

## [0.4.6](https://github.com/dmaax/mikrotui/compare/v0.4.5...v0.4.6) - 2026-08-22

### Added

- give four rows and twenty columns back to the data

## [0.4.5](https://github.com/dmaax/mikrotui/compare/v0.4.4...v0.4.5) - 2026-08-21

### Fixed

- abbreviate counts and stop the status message being pushed off screen

## [0.4.4](https://github.com/dmaax/mikrotui/compare/v0.4.3...v0.4.4) - 2026-08-21

### Other

- record the project's rules in AGENTS.md

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
