# Specifications Directory

This directory contains executable specifications for the merlin project.

## Structure

```
specs/
├── 001-feature-name/
│   ├── spec.md          # Main specification
│   ├── plan.md          # Implementation plan
│   └── tasks.md         # Task breakdown
├── 002-feature-name/
│   ├── spec.md
│   ├── plan.md
│   └── tasks.md
└── README.md            # This file
```

## Creating Specifications

Use the `/specify` slash command to create new specifications:

```bash
/specify Feature description
```

This will:
- Create a new numbered directory
- Generate a specification document
- Auto-branch from main
- Set up the development environment

## Specification Format

Each specification follows the SDD (Specification-Driven Development) format:

1. **Executive Summary**: One-paragraph overview
2. **Requirements**: User-facing requirements
3. **Success Criteria**: Testable outcomes
4. **Technical Requirements**: Implementation constraints
5. **Dependencies**: External dependencies
6. **Implementation Plan**: Step-by-step approach

## Process Flow

1. **Specify**: `/specify <feature>`
2. **Plan**: `/plan <technologies>`
3. **Tasks**: `/tasks`
4. **Implement**: Execute tasks in order
5. **Validate**: All tests pass
6. **Merge**: Back to main