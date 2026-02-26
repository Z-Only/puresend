# 贡献指南

**语言**: [中文](CONTRIBUTING.md) | [English](CONTRIBUTING_EN.md)

感谢您考虑为 PureSend 做出贡献！

## 如何贡献

### 报告 Bug

如果您发现了 Bug，请通过 [GitHub Issues](https://github.com/z-only/puresend/issues) 提交报告。提交前请：

1. 搜索现有 Issues，确认该问题未被报告
2. 使用 Bug 报告模板，提供详细的问题描述
3. 包含复现步骤、预期行为和实际行为

### 提出新功能

欢迎提出新功能建议！请：

1. 通过 Issues 描述您的功能需求
2. 说明功能的使用场景和价值
3. 等待维护者反馈后再开始实现

### 提交代码

#### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/z-only/puresend.git
cd puresend

# 安装依赖
pnpm install

# 启动开发服务器
pnpm tauri dev
```

#### 代码规范

- 使用 TypeScript 进行类型安全开发
- 遵循 ESLint 和 Prettier 配置
- 组件使用 PascalCase 命名，文件使用 kebab-case 命名
- 使用 Vue 3 Composition API 和 `<script setup>` 语法

#### 提交信息规范

请遵循 [约定式提交](https://www.conventionalcommits.org/zh-hans/) 规范：

- `feat:` 新功能
- `fix:` Bug 修复
- `docs:` 文档更新
- `style:` 代码格式调整
- `refactor:` 代码重构
- `test:` 测试相关
- `chore:` 构建/工具相关

#### Pull Request 流程

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

PR 提交前请确保：

- [ ] 代码通过 ESLint 检查 (`pnpm lint`)
- [ ] 代码格式正确 (`pnpm format:check`)
- [ ] 提交信息符合规范
- [ ] 更新相关文档

## 代码审查

所有 PR 都需要经过审查才能合并。审查过程中可能会提出修改建议，请及时响应。

## 许可证

提交代码即表示您同意您的贡献将按照 [MIT 许可证](LICENSE) 授权。
