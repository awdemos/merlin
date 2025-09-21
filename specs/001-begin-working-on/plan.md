# Implementation Plan: Feature Numbering System

**Branch**: `001-begin-working-on` | **Date**: 2025-09-20 | **Spec**: `/Users/a/code/merlin/specs/001-begin-working-on/spec.md`
**Input**: Feature specification from `/specs/001-begin-working-on/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → ✅ Feature spec found and loaded
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → ✅ Technical context filled
   → Project Type: single (specification system)
   → Structure Decision: Option 1 (single project)
3. Fill the Constitution Check section based on the content of the constitution document.
   → ✅ Constitution Check section filled
4. Evaluate Constitution Check section below
   → ✅ No violations detected
   → Update Progress Tracking: Initial Constitution Check: PASS
5. Execute Phase 0 → research.md
   → ✅ Phase 0 executed
   → ✅ All NEEDS CLARIFICATION resolved
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, agent-specific template file
   → ✅ Phase 1 executed
   → ✅ All design artifacts generated
7. Re-evaluate Constitution Check section
   → ✅ No new violations detected
   → Update Progress Tracking: Post-Design Constitution Check: PASS
8. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
   → ✅ Phase 2 approach planned
9. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
Systematic feature numbering and tracking system for development workflow organization. The system will automatically assign unique sequential numbers, prevent duplicates, maintain mappings, and support chronological ordering and searching of features.

## Technical Context
**Language/Version**: Rust 1.75+
**Primary Dependencies**: serde (JSON serialization), thiserror (error handling), tokio (async runtime)
**Storage**: File-based configuration and metadata storage
**Testing**: cargo test with mockall for mocking
**Target Platform**: Linux server (CLI tool and library)
**Project Type**: single (specification management system)
**Performance Goals**: <100ms response time for feature number assignment
**Constraints**: Must integrate with existing specification-driven development workflow
**Scale/Scope**: Supports 1000+ features, team of 10+ developers

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ Specification-Driven Development (SDD)
- Feature created with comprehensive specification using `/specify` command
- Specification includes user requirements, success criteria, technical requirements, and acceptance criteria
- No code will be written without approved specification

### ✅ Library-First Architecture
- Core functionality implemented as standalone library with clear public APIs
- Library is self-contained, independently testable, and well-documented
- Binary executable serves only as entry point to library functionality

### ✅ Test-First Development (NON-NEGOTIABLE)
- TDD mandatory: Tests written → User approved → Tests fail → Then implementation
- Red-Green-Refactor cycle strictly enforced
- 100% test coverage required for all critical feature numbering logic

### ✅ Configuration-Driven Design
- All feature numbering policies configured via TOML files
- No hardcoded configuration values permitted
- Configuration validation implemented

### ✅ Code Quality Standards
- Follow Rust API Guidelines (RAG) and idiomatic Rust
- Comprehensive documentation for all public APIs
- Structured logging with tracing crate
- Automatic formatting with rustfmt
- Strict clippy compliance

## Project Structure

### Documentation (this feature)
```
specs/001-begin-working-on/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
# Option 1: Single project (DEFAULT)
src/
├── feature_numbering/
│   ├── mod.rs
│   ├── manager.rs       # Feature number assignment logic
│   ├── storage.rs       # File-based storage backend
│   └── models.rs        # Feature and numbering data models
├── cli/
│   ├── mod.rs
│   └── commands.rs      # CLI commands for feature management
└── lib.rs

tests/
├── contract/
├── integration/
└── unit/
```

**Structure Decision**: Option 1 (Single project) - Feature numbering is a core library component for the Merlin project

## Phase 0: Outline & Research
1. **Extract unknowns from Technical Context** above:
   - File-based storage patterns for Rust applications
   - TOML configuration best practices
   - Feature numbering system integration points

2. **Generate and dispatch research agents**:
   ```
   Research file-based storage patterns for Rust specification systems
   Find best practices for TOML configuration in Rust CLI tools
   Research integration patterns for existing spec-driven workflows
   ```

3. **Consolidate findings** in `research.md` using format:
   - Decision: [what was chosen]
   - Rationale: [why chosen]
   - Alternatives considered: [what else evaluated]

**Output**: research.md with all NEEDS CLARIFICATION resolved

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

1. **Extract entities from feature spec** → `data-model.md`:
   - Entity name, fields, relationships
   - Validation rules from requirements
   - State transitions if applicable

2. **Generate API contracts** from functional requirements:
   - For each user action → endpoint
   - Use standard REST/GraphQL patterns
   - Output OpenAPI/GraphQL schema to `/contracts/`

3. **Generate contract tests** from contracts:
   - One test file per endpoint
   - Assert request/response schemas
   - Tests must fail (no implementation yet)

4. **Extract test scenarios** from user stories:
   - Each story → integration test scenario
   - Quickstart test = story validation steps

5. **Update agent file incrementally** (O(1) operation):
   - Run `.specify/scripts/bash/update-agent-context.sh claude` for your AI assistant
   - If exists: Add only NEW tech from current plan
   - Preserve manual additions between markers
   - Update recent changes (keep last 3)
   - Keep under 150 lines for token efficiency
   - Output to repository root

**Output**: data-model.md, /contracts/*, failing tests, quickstart.md, agent-specific file

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:
- Load `.specify/templates/tasks-template.md` as base
- Generate tasks from Phase 1 design docs (contracts, data model, quickstart)
- Each contract → contract test task [P]
- Each entity → model creation task [P]
- Each user story → integration test task
- Implementation tasks to make tests pass

**Ordering Strategy**:
- TDD order: Tests before implementation
- Dependency order: Models before services before CLI
- Mark [P] for parallel execution (independent files)

**Estimated Output**: 25-30 numbered, ordered tasks in tasks.md

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)
**Phase 4**: Implementation (execute tasks.md following constitutional principles)
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None | All constitutional requirements met | N/A |


## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command)
- [x] Phase 1: Design complete (/plan command)
- [x] Phase 2: Task planning complete (/plan command - describe approach only)
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
- [x] All NEEDS CLARIFICATION resolved
- [x] Complexity deviations documented

**Artifacts Generated**:
- ✅ research.md: Technical research and decisions documented
- ✅ data-model.md: Complete entity definitions and relationships
- ✅ contracts/feature-numbering-api.yaml: OpenAPI specification
- ✅ contracts/test_feature_creation.rs: Failing contract tests
- ✅ quickstart.md: User guide and getting started documentation

---
*Based on Constitution v1.0.0 - See `/memory/constitution.md`*