# Implementation Plan: Systemd Service Configuration for Merlin

**Branch**: `002-systemd-service` | **Date**: 2025-09-21 | **Spec**: New feature
**Input**: User request for systemd service file, daemon configuration, and installation script

## Execution Flow (/plan command scope)
```
1. Load user requirements
   → ✅ Request: Create systemd service file, daemon config, install script
   → Context: Source code repo, not host system
2. Fill Technical Context
   → ✅ Current Merlin: CLI tool + HTTP server in Rust/Tokio
   → ✅ Existing feature numbering system
   → ✅ Project uses Cargo for builds
3. Fill Constitution Check section
   → ✅ No violations detected
4. Execute Phase 0 → research.md
   → ✅ Research systemd best practices for Rust services
5. Execute Phase 1 → contracts, data-model.md, quickstart.md
   → ✅ Systemd service contract
   → ✅ Daemon configuration structure
   → ✅ Installation workflow documented
6. Re-evaluate Constitution Check
   → ✅ No violations detected
7. Plan Phase 2 → Describe task generation approach
   → ✅ Tasks.md created with 25 comprehensive tasks
8. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md (COMPLETE)
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
Create a complete systemd service configuration for the Merlin AI Router, enabling it to run as a system daemon with proper lifecycle management, logging, and monitoring capabilities.

## Technical Context
**Language/Version**: Rust 1.75+ (existing codebase)
**Primary Dependencies**: tokio, axum, serde, tracing (already in project)
**Storage**: Existing file-based JSON storage
**Target Platform**: Linux with systemd (Ubuntu 20.04+, CentOS 8+, Debian 10+)
**Service Type**: Background daemon supporting both CLI commands and HTTP API
**Performance Goals**: <500ms startup time, minimal memory footprint
**Security**: Non-root execution, restricted permissions, secure configuration

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ Specification-Driven Development (SDD)
- User request documented as specification
- Clear requirements for systemd service, daemon config, install script
- Implementation will follow generated tasks

### ✅ Library-First Architecture
- Systemd service will wrap existing library functionality
- No modifications to core business logic required
- Service acts as additional deployment option

### ✅ Test-First Development (NON-NEGOTIABLE)
- TDD approach for all systemd components
- Service installation tests before implementation
- Integration tests for service lifecycle

### ✅ Configuration-Driven Design
- Environment variable support for daemon configuration
- Configuration file override capabilities
- No hardcoded values in service files

### ✅ Code Quality Standards
- Follow existing Rust patterns in codebase
- systemd-specific best practices
- Comprehensive documentation for deployment

## Project Structure

### Documentation (this feature)
```
specs/002-systemd-service/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── tasks.md             # Phase 2 output (/tasks command)
```

### Source Code (repository root)
```
systemd/
├── merlin.service       # Systemd service file
├── merlin.conf          # Daemon configuration
├── merlin.env           # Environment variables
└── README.md            # Setup documentation

scripts/
└── install-systemd.sh   # Installation script

tests/systemd/
├── test_service_file.rs
├── test_install_script.rs
├── test_service_startup.rs
└── test_config_management.rs

docs/systemd/
├── setup.md              # Setup guide
└── troubleshooting.md    # Common issues
```

## Phase 0: Outline & Research
1. **Extract requirements from user request**:
   - Systemd service file for Merlin daemon
   - Daemon configuration management
   - Installation script for deployment
   - Source code integration (not host-specific)

2. **Generate research topics**:
   ```
   Research systemd best practices for Rust/Tokio services
   Find patterns for running CLI tools as systemd services
   Research environment variable management in systemd
   Study security hardening for system daemons
   ```

3. **Consolidate findings** in `research.md`:
   - Decision: [chosen approach]
   - Rationale: [why chosen]
   - Alternatives considered: [evaluated options]

**Output**: research.md with all technical decisions documented

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

1. **Extract service configuration entities** → `data-model.md`:
   - Service configuration structure
   - Environment variable mappings
   - Installation parameters

2. **Generate systemd service contract**:
   - Service lifecycle requirements
   - Dependency management
   - Logging and monitoring integration

3. **Generate installation workflow**:
   - System requirements validation
   - User and permission setup
   - Service activation procedures

4. **Extract test scenarios**:
   - Service installation validation
   - Startup and shutdown testing
   - Configuration management verification

**Output**: data-model.md, service requirements, installation workflow

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do*

**Task Generation Strategy**:
- Load `.specify/templates/tasks-template.md` as base
- Generate tasks from systemd requirements
- Each service component → implementation task [P]
- Each installation step → script task [P]
- Test tasks for all systemd functionality

**Ordering Strategy**:
- TDD order: Tests before implementation
- Dependency order: Infrastructure before services
- Mark [P] for parallel execution (independent files)

**Estimated Output**: 25 numbered, ordered tasks in tasks.md

**IMPORTANT**: This phase is executed by the /tasks command

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)
**Phase 4**: Implementation (execute tasks.md following constitutional principles)
**Phase 5**: Validation (service installation, testing, documentation)

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
- [x] Phase 2: Task planning complete (/tasks command - COMPLETE)
- [ ] Phase 3: Tasks generated (/tasks command - COMPLETE)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
- [x] All requirements clarified
- [x] Complexity deviations documented

**Artifacts Generated**:
- ✅ tasks.md: 25 comprehensive tasks for systemd setup
- ✅ Plan document with technical specifications
- ✅ Ready for implementation execution

---
*Based on Constitution v1.0.0 - See `/memory/constitution.md`*