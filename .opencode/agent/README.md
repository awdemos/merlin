# Dotfiles Project Agents

This directory contains specialized agents for maintaining the dotfiles repository.

## Primary Agents

Primary agents can be switched between using the **Tab** key during a session.

### @debug
Debugs installation issues and system problems with full diagnostic capabilities.

### @migration  
Handles dotfile migrations, updates, and version transitions with backup/restore.

### @automation
Creates and maintains automation scripts and CI/CD improvements.

## Subagents

Subagents can be invoked manually with `@agent-name` or automatically by primary agents.

### @security-auditor
Performs security audits and identifies vulnerabilities in installation scripts and configurations.

### @cross-platform-tester
Tests cross-platform compatibility across macOS, Linux, and WSL environments.

### @configuration-validator
Validates syntax and correctness of all configuration files.

### @ci-optimizer
Optimizes the Dagger CI/CD pipeline for performance and reliability.

### @documentation-maintainer
Maintains project documentation and keeps it up-to-date.

## Usage

**Primary Agents**: Switch between them using **Tab** key or your configured keybind.

**Subagents**: Invoke by mentioning them with `@`:
- `@security-auditor review the installation scripts`
- `@cross-platform-tester validate macOS compatibility`
- `@configuration-validator check neovim config syntax`
- `@ci-optimizer improve pipeline performance`
- `@documentation-maintainer update installation guide`

## Agent Configuration

Each agent is configured as a markdown file with:
- Description of purpose
- Mode (primary/subagent)
- Available tools and permissions
- Specialized instructions

Agents can also be automatically invoked based on task requirements and descriptions.