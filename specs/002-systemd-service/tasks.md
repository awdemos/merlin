# Tasks: Systemd Service Configuration for Merlin

**Input**: Merlin project analysis for systemd daemon setup
**Prerequisites**: Merlin CLI and HTTP server already implemented

## Execution Flow (main)
```
1. Analyze current Merlin project structure
   → ✅ Merlin has CLI commands and HTTP server
   → Extract: Rust 1.75+, Tokio, existing feature numbering
2. Identify systemd requirements:
   → Service file for both CLI and HTTP modes
   → Installation script for system deployment
   → Configuration management for environments
3. Generate systemd-specific tasks:
   → Setup: directory structure, user permissions
   → Tests: service installation, startup behavior
   → Core: service file, config, install script
   → Integration: logging, monitoring, security
   → Polish: performance, documentation
4. Apply task rules:
   → Service files [P] with config files
   → Tests before implementation (TDD)
5. Number tasks sequentially (T001, T002...)
6. Generate dependency graph
7. Create parallel execution examples
8. Validate task completeness:
   → Service covers all Merlin functionality?
   → Installation handles system requirements?
9. Return: SUCCESS (tasks ready for execution)
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Path Conventions
- **Repository root**: `systemd/` for service files, `scripts/` for installation
- **Tests**: `tests/systemd/` for systemd-specific tests
- **Documentation**: `docs/systemd/` for setup guides

## Phase 1: Setup (Systemd Infrastructure)
- [ ] T001 Create systemd directory structure at repository root
- [ ] T002 [P] Research systemd best practices for Rust/Tokio services
- [ ] T003 [P] Define user and permission requirements for Merlin daemon

## Phase 2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE Phase 3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**
- [ ] T004 [P] Systemd service file validation test in tests/systemd/test_service_file.rs
- [ ] T005 [P] Installation script test in tests/systemd/test_install_script.rs
- [ ] T006 [P] Service startup integration test in tests/systemd/test_service_startup.rs
- [ ] T007 [P] Configuration management test in tests/systemd/test_config_management.rs

## Phase 3: Core Implementation (ONLY after tests are failing)
- [ ] T008 [P] Systemd service file in systemd/merlin.service
- [ ] T009 [P] Daemon configuration file in systemd/merlin.conf
- [ ] T010 Installation script in scripts/install-systemd.sh
- [ ] T011 Environment configuration in systemd/merlin.env
- [ ] T012 Service lifecycle management (start/stop/restart) in main.rs
- [ ] T013 Logging configuration for systemd integration
- [ ] T014 Error handling and recovery mechanisms

## Phase 4: Integration
- [ ] T015 [P] User creation and permission setup in install script
- [ ] T016 [P] Directory structure creation in install script
- [ ] T017 Service dependency management (Redis if needed, network)
- [ ] T018 Health check endpoint for service monitoring
- [ ] T019 Integration with existing CLI commands
- [ ] T020 HTTP server systemd integration

## Phase 5: Polish
- [ ] T021 [P] Performance optimization for systemd service startup
- [ ] T022 [P] Systemd service documentation in docs/systemd/README.md
- [ ] T023 [P] Update main README.md with systemd instructions
- [ ] T024 Security hardening (drop privileges, restrict permissions)
- [x] T025 Manual testing procedure verification

## Dependencies
- Tests (T004-T007) before implementation (T008-T014)
- T008 blocks T012, T017
- T009 blocks T013, T018
- T010 blocks T015, T016
- Implementation before polish (T021-T025)

## Parallel Example
```
# Launch service and config tasks together:
Task: "Systemd service file in systemd/merlin.service"
Task: "Daemon configuration file in systemd/merlin.conf"
Task: "Installation script in scripts/install-systemd.sh"
Task: "Environment configuration in systemd/merlin.env"

# Launch test suite together:
Task: "Systemd service file validation test in tests/systemd/test_service_file.rs"
Task: "Installation script test in tests/systemd/test_install_script.rs"
Task: "Service startup integration test in tests/systemd/test_service_startup.rs"
Task: "Configuration management test in tests/systemd/test_config_management.rs"
```

## Notes
- [P] tasks = different files, no dependencies
- Verify tests fail before implementing
- Follow Rust systemd service best practices
- Consider security implications of running as daemon
- Ensure service integrates with existing Merlin CLI functionality
- Test both development and production configurations

## Task Generation Rules Applied

1. **From Current Merlin State**:
   - Merlin has working CLI (feature commands)
   - HTTP server with axum framework
   - Existing feature numbering functionality
   - Rust/Tokio codebase with proper error handling

2. **Systemd Requirements**:
   - Service file covering both CLI and HTTP modes
   - Installation script for production deployment
   - Configuration management for different environments
   - Proper logging and monitoring integration

3. **Integration Points**:
   - Must preserve all existing Merlin functionality
   - Should work with existing feature numbering system
   - Needs to handle both development and production use cases

## Validation Checklist
- [ ] Systemd service covers all Merlin modes (CLI, HTTP server)
- [ ] Installation script handles all system requirements
- [ ] Tests validate service lifecycle and configuration
- [ ] Security best practices followed (non-root user, restricted permissions)
- [ ] Documentation covers installation and operation
- [ ] Each task specifies exact file path
- [ ] No task conflicts with existing Merlin functionality

## Success Metrics
- **25 total tasks** (3 setup, 4 tests first, 7 core, 6 integration, 5 polish)
- **10 parallel tasks** marked with [P]
- **Complete systemd integration** for production deployment
- **Backward compatibility** with existing CLI functionality
- **Security-hardened** daemon configuration

## Constitutional Compliance
- ✅ **Test-First Development**: All tests written before implementation
- ✅ **Library-First Architecture**: Systemd wrapper around existing library
- ✅ **Configuration-Driven**: Environment and config file support
- ✅ **Code Quality**: Follows existing project patterns
- ✅ **Error Handling**: Proper systemd service status reporting
- ✅ **Security**: Non-root execution, restricted permissions

---
**Tasks Generated**: 25 comprehensive tasks for systemd service setup
**Estimated Duration**: 1-2 days for full implementation
**Ready for**: `/implement [task_number]` command execution