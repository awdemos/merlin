# Spec-Kit Slash Commands

This project uses Specification-Driven Development (SDD) with the following slash commands:

## /specify
Create a new feature specification.

**Usage**: `/specify <feature description>`

**What it does**:
- Creates a new numbered feature directory in `specs/`
- Generates specification document from template
- Auto-branches from main
- Sets up development environment

**Example**:
```bash
/specify Real-time chat system
```

## /plan
Generate implementation plan from specification.

**Usage**: `/plan <technology stack>`

**What it does**:
- Analyzes current specification
- Creates implementation plan with stages
- Defines architecture and approach
- Identifies risks and dependencies

**Example**:
```bash
/plan WebSocket, PostgreSQL, Redis
```

## /tasks
Derive executable task list from plan.

**Usage**: `/tasks`

**What it does**:
- Breaks down plan into individual tasks
- Creates task dependencies and timeline
- Defines acceptance criteria for each task
- Generates quality gates

## /implement
Execute tasks and generate code.

**Usage**: `/implement <task number>`

**What it does**:
- Executes specific task from task list
- Generates code following project patterns
- Creates tests for the implementation
- Updates documentation

## /constitution
Review project constitution and principles.

**Usage**: `/constitution`

**What it does**:
- Displays project constitution
- Shows current development principles
- Provides context for decision making

## /validate
Validate current state against specifications.

**Usage**: `/validate`

**What it does**:
- Checks implementation against specifications
- Validates tests and coverage
- Ensures compliance with constitution
- Generates validation report

## Workflow Example

```bash
/specify Real-time chat system
/plan WebSocket, PostgreSQL, Redis
/tasks
/implement 1
/implement 2
/validate
```

## Best Practices

1. **Always start with `/specify`** - Create specifications before code
2. **Use `/plan` to design architecture** - Think before implementing
3. **Break down with `/tasks`** - Make work manageable
4. **Implement incrementally** - One task at a time
5. **Validate often** - Ensure alignment with specifications

## Integration with AI Agents

These commands are designed to work with AI coding assistants like Claude, Gemini, Copilot, and Cursor. The commands guide the AI through the specification-driven development process.

## Project Structure

After using these commands, your project will have:

```
merlin/
├── memory/
│   └── constitution.md
├── specs/
│   ├── 001-feature-name/
│   │   ├── spec.md
│   │   ├── plan.md
│   │   └── tasks.md
│   └── README.md
├── templates/
│   ├── spec.md
│   ├── plan.md
│   ├── tasks.md
│   └── README.md
└── .claude/
    └── SLASH_COMMANDS.md
```