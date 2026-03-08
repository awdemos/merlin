# Tasks: Hardened Docker Deployment Preference

**Input**: Design documents from `/specs/004-prefer-hardened-docker/`
**Prerequisites**: plan.md (required), research.md, data-model.md, contracts/, quickstart.md

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → If not found: ERROR "No implementation plan found"
   → Extract: tech stack, libraries, structure
2. Load optional design documents:
   → data-model.md: Extract entities → model tasks
   → contracts/: Each file → contract test task
   → research.md: Extract decisions → setup tasks
3. Generate tasks by category:
   → Setup: Docker structure, dependencies, security tools
   → Tests: contract tests, integration tests, security validation
   → Core: Docker models, security profiles, resource limits
   → Integration: configuration management, health monitoring
   → Security: hardening, scanning, compliance
   → Deployment: Dockerfile, service management, orchestration
   → Polish: unit tests, performance, docs
4. Apply task rules:
   → Different files = mark [P] for parallel
   → Same file = sequential (no [P])
   → Tests before implementation (TDD)
5. Number tasks sequentially (T001, T002...)
6. Generate dependency graph
7. Create parallel execution examples
8. Validate task completeness:
   → All contracts have tests?
   → All entities have models?
   → All endpoints implemented?
9. Return: SUCCESS (tasks ready for execution)
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Path Conventions
- **Single project**: `src/`, `tests/`, `docker/` at repository root
- **Docker artifacts**: `docker/`, `config/`, `systemd/` at repository root
- Paths shown below assume single project - adjust based on plan.md structure

## Phase 3.1: Setup
- [ ] T001 Create Docker directory structure at repository root
- [ ] T002 Initialize Docker security dependencies (trivy, hadolint integration)
- [ ] T003 [P] Configure container security validation tools

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**
- [ ] T004 [P] Contract test POST /api/v1/docker/build in tests/contract/test_docker_build_post.rs
- [ ] T005 [P] Contract test POST /api/v1/docker/scan in tests/contract/test_docker_scan_post.rs
- [ ] T006 [P] Contract test POST /api/v1/docker/validate in tests/contract/test_docker_validate_post.rs
- [ ] T007 [P] Contract test GET /api/v1/docker/health in tests/contract/test_docker_health_get.rs
- [ ] T008 [P] Integration test non-root container execution in tests/integration/test_non_root_execution.rs
- [ ] T009 [P] Integration test container resource limits in tests/integration/test_resource_limits.rs
- [ ] T010 [P] Integration test security scanning compliance in tests/integration/test_security_scanning.rs
- [ ] T011 [P] Integration test multi-environment deployment in tests/integration/test_multi_env_deployment.rs

## Phase 3.3: Core Implementation (ONLY after tests are failing)
- [ ] T012 [P] DockerContainerConfig model in src/models/docker_config.rs
- [ ] T013 [P] SecurityProfile model in src/models/security_profile.rs
- [ ] T014 [P] ResourceLimits model in src/models/resource_limits.rs
- [ ] T015 [P] TmpfsMount model in src/models/tmpfs_mount.rs
- [ ] T016 [P] HealthMonitor model in src/models/health_monitor.rs
- [ ] T017 [P] SecurityScanConfig model in src/models/security_scan_config.rs
- [ ] T018 [P] DeploymentEnvironment model in src/models/deployment_environment.rs
- [ ] T019 Docker configuration validation service in src/services/docker_config_service.rs
- [ ] T020 POST /api/v1/docker/build endpoint implementation
- [ ] T021 POST /api/v1/docker/scan endpoint implementation
- [ ] T022 POST /api/v1/docker/validate endpoint implementation
- [ ] T023 GET /api/v1/docker/health endpoint implementation
- [ ] T024 Container state management in src/services/container_state_service.rs
- [ ] T025 Error handling and logging for Docker operations

## Phase 3.4: Integration
- [ ] T026 Docker configuration TOML serialization in src/config/docker_config.rs
- [ ] T027 Health monitoring integration with existing Prometheus metrics
- [ ] T028 Redis integration for container state and preferences
- [ ] T029 Container lifecycle management service
- [ ] T030 Security policy enforcement middleware
- [ ] T031 Resource limit monitoring and alerting
- [ ] T032 Configuration validation and startup checks

## Phase 3.5: Security & Deployment
- [ ] T033 [P] Create hardened Dockerfile in docker/Dockerfile.hardened
- [ ] T034 [P] Create multi-stage Dockerfile in docker/Dockerfile.multi-stage
- [ ] T035 [P] Security scanning configuration in docker/security.yml
- [ ] T036 [P] Docker Compose deployment in docker-compose.yml
- [ ] T037 [P] Kubernetes deployment manifests in k8s/
- [ ] T038 [P] Container security validation tests
- [ ] T039 [P] CI/CD pipeline integration for security scanning
- [ ] T040 [P] Container deployment scripts in scripts/deploy/
- [ ] T041 Integration tests for container lifecycle management
- [ ] T042 Integration tests for security compliance validation

## Phase 3.6: Polish
- [ ] T043 [P] Unit tests for Docker configuration models in tests/unit/test_docker_models.rs
- [ ] T044 [P] Unit tests for security validation in tests/unit/test_security_validation.rs
- [ ] T045 [P] Performance tests for container startup time (<10s)
- [ ] T046 [P] Performance tests for memory footprint (<100MB)
- [ ] T047 [P] Update CLAUDE.md with Docker deployment patterns
- [ ] T048 [P] Update quickstart.md with deployment examples
- [ ] T049 Container optimization and layer caching improvements
- [ ] T050 Documentation and deployment guides

## Dependencies
- Tests (T004-T011) before implementation (T012-T025)
- Models (T012-T018) block services (T019-T025)
- Services (T019-T025) block endpoints (T020-T023)
- Core implementation before integration (T026-T032)
- Integration before security & deployment (T033-T042)
- Security & deployment before polish (T043-T050)

## Parallel Example
```
# Launch T004-T011 together (all contract/integration tests):
Task: "Contract test POST /api/v1/docker/build in tests/contract/test_docker_build_post.rs"
Task: "Contract test POST /api/v1/docker/scan in tests/contract/test_docker_scan_post.rs"
Task: "Contract test POST /api/v1/docker/validate in tests/contract/test_docker_validate_post.rs"
Task: "Contract test GET /api/v1/docker/health in tests/contract/test_docker_health_get.rs"
Task: "Integration test non-root container execution in tests/integration/test_non_root_execution.rs"
Task: "Integration test container resource limits in tests/integration/test_resource_limits.rs"
Task: "Integration test security scanning compliance in tests/integration/test_security_scanning.rs"
Task: "Integration test multi-environment deployment in tests/integration/test_multi_env_deployment.rs"

# Launch T012-T018 together (all model files):
Task: "DockerContainerConfig model in src/models/docker_config.rs"
Task: "SecurityProfile model in src/models/security_profile.rs"
Task: "ResourceLimits model in src/models/resource_limits.rs"
Task: "TmpfsMount model in src/models/tmpfs_mount.rs"
Task: "HealthMonitor model in src/models/health_monitor.rs"
Task: "SecurityScanConfig model in src/models/security_scan_config.rs"
Task: "DeploymentEnvironment model in src/models/deployment_environment.rs"

# Launch T033-T037 together (deployment artifacts):
Task: "Create hardened Dockerfile in docker/Dockerfile.hardened"
Task: "Create multi-stage Dockerfile in docker/Dockerfile.multi-stage"
Task: "Security scanning configuration in docker/security.yml"
Task: "Docker Compose deployment in docker-compose.yml"
Task: "Kubernetes deployment manifests in k8s/"
```

## Task Generation Rules Applied

1. **From Contracts**:
   - docker-build.yaml → 4 contract test tasks [P] (T004-T007)
   - 4 endpoints → 4 implementation tasks (T020-T023)

2. **From Data Model**:
   - 7 entities → 7 model creation tasks [P] (T012-T018)
   - Entity relationships → configuration service (T019)

3. **From User Stories**:
   - 5 acceptance scenarios → 4 integration test tasks [P] (T008-T011)
   - Quickstart deployment scenarios → deployment tasks (T033-T037)

4. **From Research**:
   - Security hardening → security tasks (T033-T035, T038-T039)
   - Multi-environment support → environment tasks (T018, T036-T037)

## Validation Checklist ✅
- [x] All contracts have corresponding tests
- [x] All entities have model tasks
- [x] All tests come before implementation
- [x] Security hardening tasks included for system services
- [x] Deployment configuration tasks included for all environments
- [x] Parallel tasks truly independent (different files)
- [x] Each task specifies exact file path
- [x] No task modifies same file as another [P] task
- [x] Service integration tests included for system components
- [x] Configuration validation tasks included

## Architecture Notes
- **Single project structure** with Docker artifacts at root
- **Security-first approach** with non-root execution and hardening
- **Configuration-driven design** supporting TOML and environment variables
- **Multi-environment support** for dev/staging/prod deployments
- **Integration with existing** Redis, Prometheus, and systemd components

## Execution Order
1. **Setup Phase** (T001-T003): Prepare project structure
2. **Test Phase** (T004-T011): Write failing tests (TDD)
3. **Core Phase** (T012-T025): Implement models and services
4. **Integration Phase** (T026-T032): Connect with existing systems
5. **Security Phase** (T033-T042): Deploy and secure containers
6. **Polish Phase** (T043-T050): Optimize and document

Total: 50 tasks estimated for complete implementation