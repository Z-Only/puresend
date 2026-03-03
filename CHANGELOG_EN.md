# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-03-03

### Added

- Add MIT open source license
- Add CONTRIBUTING.md contribution guide
- Add CODE_OF_CONDUCT.md code of conduct
- Add GitHub Issue templates (Bug report, Feature request)
- Add GitHub Pull Request template
- Improve package.json metadata
- Add cloud storage transfer (Alibaba Cloud OSS, Alibaba Cloud Drive)
- Add Web upload receive functionality
- Add transfer encryption (AES-256-GCM + P-256 ECDH)
- Add dynamic compression (zstd algorithm)
- Add resume transfer and chunked transfer
- Add transfer history persistence
- Add network adaptive (auto-detect network changes)
- Add multi-IP support
- Add PIN protection for Web download links
- Add tab bar layout configuration and font size adjustment
- Add VitePress documentation site
- Add English documentation support

### Changed

- Comprehensive codebase optimization (performance, readability, maintainability, robustness, security)
- Update tech stack: Vuetify 4, Pinia 3, TypeScript ~5.9, rolldown-vite
- Optimize Rust backend error handling and concurrency safety
- Optimize file transfer performance and memory usage
- Sync documentation with code implementation

## [0.1.0] - 2025-02-18

### Added

- Initial release
- Cross-platform file transfer application based on Tauri 2 + Vue 3 + TypeScript
- Support for macOS, Windows, Linux, and Android platforms
- Core file transfer functionality
- Device discovery feature
- Multi-language support (Chinese, English)
- Settings functionality

---

[Unreleased]: https://github.com/z-only/puresend/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/z-only/puresend/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/z-only/puresend/releases/tag/v0.1.0
