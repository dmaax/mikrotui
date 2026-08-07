# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
