# Contributing Guide

**Language**: [中文](CONTRIBUTING.md) | [English](CONTRIBUTING_EN.md)

Thank you for considering contributing to PureSend!

## How to Contribute

### Reporting Bugs

If you find a bug, please submit a report via [GitHub Issues](https://github.com/z-only/puresend/issues). Before submitting:

1. Search existing issues to confirm the problem hasn't been reported
2. Use the bug report template and provide a detailed description
3. Include reproduction steps, expected behavior, and actual behavior

### Suggesting New Features

New feature suggestions are welcome! Please:

1. Describe your feature request through Issues
2. Explain the use case and value of the feature
3. Wait for maintainer feedback before starting implementation

### Submitting Code

#### Development Environment Setup

```bash
# Clone the repository
git clone https://github.com/z-only/puresend.git
cd puresend

# Install dependencies
pnpm install

# Start development server
pnpm tauri dev
```

#### Code Standards

- Use TypeScript for type-safe development
- Follow ESLint and Prettier configurations
- Use PascalCase for components, kebab-case for filenames
- Use Vue 3 Composition API with `<script setup>` syntax

#### Commit Message Convention

Please follow [Conventional Commits](https://www.conventionalcommits.org/) specification:

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation update
- `style:` Code style adjustment
- `refactor:` Code refactoring
- `test:` Test related
- `chore:` Build/tool related

#### Pull Request Process

1. Fork this repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Create a Pull Request

Before submitting a PR, please ensure:

- [ ] Code passes ESLint check (`pnpm lint`)
- [ ] Code format is correct (`pnpm format:check`)
- [ ] Commit messages follow the convention
- [ ] Related documentation is updated

## Code Review

All PRs require review before merging. Suggestions for changes may be provided during the review process, please respond in a timely manner.

## License

By submitting code, you agree that your contributions will be licensed under the [MIT License](LICENSE).
