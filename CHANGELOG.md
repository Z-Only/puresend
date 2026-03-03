# 更新日志

本项目的所有重要更改都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
并且本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [0.2.0] - 2026-03-03

### Added

- 添加 MIT 开源许可证
- 添加 CONTRIBUTING.md 贡献指南
- 添加 CODE_OF_CONDUCT.md 行为准则
- 添加 GitHub Issue 模板（Bug 报告、功能请求）
- 添加 GitHub Pull Request 模板
- 完善 package.json 元数据
- 添加云存储传输功能（阿里云 OSS、阿里云盘）
- 添加 Web 上传接收功能
- 添加传输加密（AES-256-GCM + P-256 ECDH）
- 添加动态压缩（zstd 算法）
- 添加断点续传和分块传输
- 添加传输历史记录持久化
- 添加网络自适应（自动检测网络变化）
- 添加多 IP 支持
- 添加 PIN 码保护 Web 下载链接
- 添加 Tab 栏布局配置和字体大小调节
- 添加 VitePress 文档站点
- 添加英文文档支持

### Changed

- 代码库全面优化（性能、可读性、可维护性、健壮性、安全性）
- 更新技术栈：Vuetify 4、Pinia 3、TypeScript ~5.9、rolldown-vite
- 优化 Rust 后端错误处理和并发安全性
- 优化文件传输性能和内存使用
- 更新文档与代码实现同步

## [0.1.0] - 2025-02-18

### Added

- 初始版本发布
- 基于 Tauri 2 + Vue 3 + TypeScript 的跨平台文件传输应用
- 支持 macOS、Windows、Linux 和 Android 平台
- 文件传输核心功能
- 设备发现功能
- 多语言支持（中文、英文）
- 设置功能

---

[Unreleased]: https://github.com/z-only/puresend/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/z-only/puresend/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/z-only/puresend/releases/tag/v0.1.0
