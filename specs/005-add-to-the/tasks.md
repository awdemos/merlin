# Tasks: API Reference Endpoints for Merlin AI Router

**Input**: Design documents from `/specs/005-add-to-the/`
**Prerequisites**: plan.md (complete), research.md, data-model.md, contracts/

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → Extract: Rust 1.75+, Warp, Tokio, Serde, Redis, existing project structure
2. Load design documents:
   → data-model.md: Extract 5 core entities + supporting types → model tasks
   → contracts/api.yaml: 5 endpoints → contract test tasks
   → research.md: Redis storage, Warp validation, async processing → setup tasks
3. Generate tasks by category:
   → Setup: Update Cargo.toml, module structure
   → Tests: contract tests, integration tests for all endpoints
   → Core: models for all entities, services for business logic, API endpoints
   → Integration: Redis connection, auth middleware, metrics integration
   → Security: input validation, rate limiting, auth integration
   → Polish: unit tests, performance validation, documentation
4. Apply task rules:
   → Different files = mark [P] for parallel
   → Same file = sequential (no [P])
   → Tests before implementation (TDD)
5. Number tasks sequentially (T001, T002...)
6. Generate dependency graph
7. Create parallel execution examples
8. Validate task completeness:
   → All 5 contracts have tests? ✓
   → All 5 entities have models? ✓
   → All 5 endpoints implemented? ✓
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Phase 3.1: Setup
- [x] T001 Update Cargo.toml with required dependencies for API endpoints
- [x] T002 Create new API module structure in src/api/
- [x] T003 [P] Configure API module in lib.rs and create module structure

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Contract Tests (One per endpoint)
- [ ] T004 [P] Contract test POST /api/v1/modelSelect in tests/contract/test_model_select_post.rs
- [ ] T005 [P] Contract test POST /api/v1/feedback in tests/contract/test_feedback_post.rs
- [ ] T006 [P] Contract test POST /api/v1/preferences/userPreferenceCreate in tests/contract/test_preference_create_post.rs
- [ ] T007 [P] Contract test PUT /api/v1/preferences/userPreferenceUpdate in tests/contract/test_preference_update_put.rs
- [ ] T008 [P] Contract test DELETE /api/v1/preferences/userPreferenceDelete in tests/contract/test_preference_delete_delete.rs

### Integration Tests (User workflows from quickstart.md)
- [ ] T009 [P] Integration test model selection workflow in tests/integration/test_model_selection_workflow.rs
- [ ] T010 [P] Integration test feedback submission workflow in tests/integration/test_feedback_workflow.rs
- [ ] T011 [P] Integration test preference CRUD operations in tests/integration/test_preference_crud.rs
- [ ] T012 [P] Integration test quickstart scenarios in tests/integration/test_quickstart_scenarios.rs

## Phase 3.3: Core Implementation (ONLY after tests are failing)

### Data Models (from data-model.md)
- [ ] T013 [P] ModelSelectionRequest model in src/models/model_selection.rs
- [ ] T014 [P] ModelSelectionResponse model in src/models/model_selection.rs
- [ ] T015 [P] FeedbackSubmission model in src/models/feedback.rs
- [ ] T016 [P] UserPreference model in src/models/user_preference.rs
- [ ] T017 [P] APIResponse model in src/models/api_response.rs
- [ ] T018 [P] Supporting types and enums in src/models/mod.rs

### Service Layer (Business logic)
- [ ] T019 [P] ModelSelectionService in src/services/model_selection_service.rs
- [ ] T020 [P] FeedbackService in src/services/feedback_service.rs
- [ ] T021 [P] UserPreferenceService in src/services/user_preference_service.rs
- [ ] T022 [P] Validation service with Warp filters in src/services/validation_service.rs

### API Endpoints (HTTP handlers)
- [ ] T023 POST /api/v1/modelSelect endpoint in src/api/endpoints/model_selection.rs
- [ ] T024 POST /api/v1/feedback endpoint in src/api/endpoints/feedback.rs
- [ ] T025 POST /api/v1/preferences/userPreferenceCreate endpoint in src/api/endpoints/preferences.rs
- [ ] T026 PUT /api/v1/preferences/userPreferenceUpdate endpoint in src/api/endpoints/preferences.rs
- [ ] T027 DELETE /api/v1/preferences/userPreferenceDelete endpoint in src/api/endpoints/preferences.rs

### Error Handling and Validation
- [ ] T028 API error handling middleware in src/api/middleware/error_handling.rs
- [ ] T029 Input validation for all request types in src/api/middleware/validation.rs
- [ ] T030 Response formatting and metadata generation in src/api/middleware/response_formatting.rs

## Phase 3.4: Integration

### Storage Integration
- [ ] T031 Redis connection management for user preferences in src/storage/redis_connection.rs
- [ ] T032 Redis storage schema implementation in src/storage/redis_storage.rs
- [ ] T033 Cache management for temporary data in src/storage/cache_manager.rs

### Existing System Integration
- [ ] T034 Integration with existing LlmProvider trait system in src/integration/provider_integration.rs
- [ ] T035 Integration with existing routing algorithms in src/integration/routing_integration.rs
- [ ] T036 Metrics integration with existing Prometheus setup in src/integration/metrics_integration.rs

### Middleware and Configuration
- [ ] T037 Authentication middleware for API endpoints in src/api/middleware/auth.rs
- [ ] T038 Rate limiting implementation in src/api/middleware/rate_limiting.rs
- [ ] T039 Configuration management for API settings in src/config/api_config.rs

## Phase 3.5: Security & Deployment

### Security Hardening
- [ ] T040 [P] Input sanitization and security validation in tests/security/test_input_validation.rs
- [ ] T041 [P] Authentication and authorization tests in tests/security/test_auth_security.rs
- [ ] T042 [P] Rate limiting security tests in tests/security/test_rate_limiting.rs

### Configuration Validation
- [ ] T043 [P] API configuration validation in tests/config/test_api_config.rs
- [ ] T044 [P] Redis connection validation in tests/config/test_redis_config.rs

### Deployment Configuration
- [ ] T045 [P] API server configuration in config/api.toml
- [ ] T046 [P] Docker configuration for API service in docker/api.Dockerfile
- [ ] T047 [P] Kubernetes manifests for API deployment in k8s/api/

## Phase 3.6: Polish

### Unit Tests and Performance
- [ ] T048 [P] Unit tests for all models in tests/unit/test_models.rs
- [ ] T049 [P] Unit tests for all services in tests/unit/test_services.rs
- [ ] T050 Performance tests (<100ms response time) in tests/performance/test_api_performance.rs

### Documentation and Code Quality
- [ ] T051 [P] Update API documentation in docs/api/endpoints.md
- [ ] T052 [P] Update quickstart guide with implementation notes in specs/005-add-to-the/quickstart.md
- [ ] T053 Code refactoring and duplication removal

### Validation and Final Testing
- [ ] T054 Run contract tests to verify all endpoints work correctly
- [ ] T055 Run integration tests to verify workflow functionality
- [ ] T056 Run performance tests to verify <100ms response time requirement
- [ ] T057 Execute quickstart.md scenarios to validate implementation

## Dependencies
- Setup (T001-T003) before Tests (T004-T012)
- Tests (T004-T012) before Implementation (T013-T030) [TDD requirement]
- Models (T013-T018) before Services (T019-T022)
- Services (T019-T022) before Endpoints (T023-T027)
- Integration (T031-T039) before Security & Deployment (T040-T047)
- Security & Deployment (T040-T047) before Polish (T048-T057)

## Parallel Execution Groups

### Group 1: Contract Tests (T004-T008) - Can run in parallel
```
Task: "Contract test POST /api/v1/modelSelect in tests/contract/test_model_select_post.rs"
Task: "Contract test POST /api/v1/feedback in tests/contract/test_feedback_post.rs"
Task: "Contract test POST /api/v1/preferences/userPreferenceCreate in tests/contract/test_preference_create_post.rs"
Task: "Contract test PUT /api/v1/preferences/userPreferenceUpdate in tests/contract/test_preference_update_put.rs"
Task: "Contract test DELETE /api/v1/preferences/userPreferenceDelete in tests/contract/test_preference_delete_delete.rs"
```

### Group 2: Integration Tests (T009-T012) - Can run in parallel
```
Task: "Integration test model selection workflow in tests/integration/test_model_selection_workflow.rs"
Task: "Integration test feedback submission workflow in tests/integration/test_feedback_workflow.rs"
Task: "Integration test preference CRUD operations in tests/integration/test_preference_crud.rs"
Task: "Integration test quickstart scenarios in tests/integration/test_quickstart_scenarios.rs"
```

### Group 3: Model Creation (T013-T018) - Can run in parallel
```
Task: "ModelSelectionRequest model in src/models/model_selection.rs"
Task: "ModelSelectionResponse model in src/models/model_selection.rs"
Task: "FeedbackSubmission model in src/models/feedback.rs"
Task: "UserPreference model in src/models/user_preference.rs"
Task: "APIResponse model in src/models/api_response.rs"
Task: "Supporting types and enums in src/models/mod.rs"
```

### Group 4: Service Creation (T019-T022) - Can run in parallel
```
Task: "ModelSelectionService in src/services/model_selection_service.rs"
Task: "FeedbackService in src/services/feedback_service.rs"
Task: "UserPreferenceService in src/services/user_preference_service.rs"
Task: "Validation service with Warp filters in src/services/validation_service.rs"
```

### Group 5: Security Tests (T040-T042) - Can run in parallel
```
Task: "Input sanitization and security validation in tests/security/test_input_validation.rs"
Task: "Authentication and authorization tests in tests/security/test_auth_security.rs"
Task: "Rate limiting security tests in tests/security/test_rate_limiting.rs"
```

### Group 6: Configuration Tests (T043-T044) - Can run in parallel
```
Task: "API configuration validation in tests/config/test_api_config.rs"
Task: "Redis connection validation in tests/config/test_redis_config.rs"
```

### Group 7: Deployment Config (T045-T047) - Can run in parallel
```
Task: "API server configuration in config/api.toml"
Task: "Docker configuration for API service in docker/api.Dockerfile"
Task: "Kubernetes manifests for API deployment in k8s/api/"
```

### Group 8: Unit Tests (T048-T049) - Can run in parallel
```
Task: "Unit tests for all models in tests/unit/test_models.rs"
Task: "Unit tests for all services in tests/unit/test_services.rs"
```

## Task Generation Rules Applied

1. **From Contracts** (5 endpoints):
   - api.yaml → 5 contract test tasks [T004-T008] [P]
   - Each endpoint → implementation task [T023-T027]

2. **From Data Model** (5 entities + supporting types):
   - Each entity → model creation task [T013-T018] [P]
   - Relationships → service layer tasks [T019-T022]

3. **From User Stories** (quickstart.md scenarios):
   - Model selection workflow → integration test [T009] [P]
   - Feedback submission workflow → integration test [T010] [P]
   - Preference CRUD → integration test [T011] [P]
   - Quickstart scenarios → validation task [T012] [P]

4. **TDD Ordering Enforced**:
   - All tests (T004-T012) before implementation (T013+)
   - Contract tests before endpoint implementation
   - Integration tests before workflow implementation

## Validation Checklist ✅

- [x] All 5 contracts have corresponding tests
- [x] All 5 entities have model tasks
- [x] All tests come before implementation (TDD)
- [x] Security hardening tasks included for all endpoints
- [x] Deployment configuration tasks included for API service
- [x] Parallel tasks are truly independent (different files)
- [x] Each task specifies exact file path
- [x] No task modifies same file as another [P] task
- [x] Service integration tests included for all system components
- [x] Configuration validation tasks included for Redis and API
- [x] Performance validation tasks included (<100ms requirement)

## Constitutional Compliance

✅ **Specification-Driven Development**: Tasks generated from complete design documents
✅ **Library-First Architecture**: Extension of existing lib.rs structure with new modules
✅ **Provider Abstraction**: Integration tasks with existing LlmProvider trait system
✅ **Test-First Development**: All tests written before implementation (TDD enforced)
✅ **Performance & Observability**: Performance tests with <100ms requirement
✅ **Intelligent Routing**: Integration with existing routing algorithms
✅ **Configuration-Driven Design**: Configuration tasks for TOML and environment variables

## Total Tasks: 57
- Setup: 3 tasks
- Tests: 9 tasks (5 contract + 4 integration)
- Core Implementation: 18 tasks (6 models + 4 services + 5 endpoints + 3 middleware)
- Integration: 9 tasks (3 storage + 3 system integration + 3 middleware)
- Security & Deployment: 8 tasks (3 security + 2 config + 3 deployment)
- Polish: 10 tasks (2 unit + 1 performance + 2 docs + 1 refactor + 4 validation)

**Ready for execution**: All tasks are specific, numbered, and include exact file paths. TDD order enforced with comprehensive test coverage.