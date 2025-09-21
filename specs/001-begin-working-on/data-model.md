# Feature Numbering System - Data Model

## Entities

### Feature
Represents a distinct capability or improvement to be developed with systematic numbering.

**Fields**:
- `id`: String - Unique identifier combining number and name (e.g., "001-feature-numbering")
- `number`: u32 - Sequential feature number (1, 2, 3, ...)
- `name`: String - Human-readable feature name
- `description`: String - Detailed feature description
- `status`: FeatureStatus - Current development status
- `created_at`: DateTime - Creation timestamp
- `updated_at`: DateTime - Last modification timestamp
- `metadata`: FeatureMetadata - Additional feature information
- `branch_name`: String - Git branch name for this feature

**Validation Rules**:
- Number must be unique and sequential
- Name must be non-empty and alphanumeric with hyphens
- Status transitions follow defined workflow
- Branch name follows project naming conventions

### FeatureNumber
Represents the unique sequential identifier assigned to each feature.

**Fields**:
- `value`: u32 - The actual number value
- `prefix`: String - Optional prefix for display (e.g., "FEATURE-")
- `is_reserved`: bool - Whether this number is reserved for special use
- `assigned_at`: Option<DateTime> - When this number was assigned
- `feature_id`: Option<String> - Associated feature ID if assigned

**Validation Rules**:
- Value must be positive and within configured range
- Prefix must follow naming conventions
- Reserved numbers cannot be assigned to regular features
- Cannot be reassigned once assigned to a feature

### FeatureMetadata
Additional information about a feature.

**Fields**:
- `priority`: FeaturePriority - Development priority level
- `tags`: Vec<String> - Categorization tags
- `estimated_effort`: Option<String> - Effort estimation
- `assignee`: Option<String> - Assigned developer/team
- `dependencies`: Vec<String> - Dependencies on other features
- `related_features`: Vec<String> - Related feature references

### FeatureReference
System for consistently referring to features across documentation.

**Fields**:
- `feature_id`: String - Reference to the feature
- `reference_type`: ReferenceType - Type of reference (dependency, related, etc.)
- `context`: String - Context where this reference appears
- `created_at`: DateTime - When reference was created

## Enums

### FeatureStatus
```rust
enum FeatureStatus {
    Draft,      // Initial specification phase
    Planned,    // Approved and planned
    InProgress, // Development in progress
    Review,     // Under review
    Completed,  // Development complete
    Cancelled,  // Feature cancelled
    OnHold,     // Temporarily paused
}
```

### FeaturePriority
```rust
enum FeaturePriority {
    Low,
    Medium,
    High,
    Critical,
}
```

### ReferenceType
```rust
enum ReferenceType {
    Dependency,  // This feature depends on another
    Related,     // Related but not dependent
    Duplicates,  // Duplicates another feature
    Supersedes,  // Replaces an existing feature
    BlockedBy,   // Blocked by another feature
}
```

## Relationships

### Feature ↔ FeatureNumber (One-to-One)
- Each Feature has exactly one FeatureNumber
- Each FeatureNumber can be assigned to at most one Feature
- Assignment is permanent once made

### Feature → FeatureMetadata (One-to-One)
- Each Feature has one FeatureMetadata record
- Metadata contains optional information about the feature

### Feature → FeatureReference (One-to-Many)
- A Feature can have multiple FeatureReferences
- References can point to other features or be referenced by others
- Creates a dependency graph

### Feature → Feature (Self-referencing for Dependencies)
- Features can have dependencies on other features
- Creates a directed acyclic graph (DAG) of feature relationships
- Circular dependencies must be prevented

## State Transitions

### FeatureStatus Transitions
```
Draft → Planned → InProgress → Review → Completed
        ↓           ↓           ↓
      Cancelled  OnHold     Cancelled
```

**Transition Rules**:
- Draft → Planned: Requires specification approval
- Planned → InProgress: Can start development
- InProgress → Review: Ready for code review
- Review → Completed: Approved and merged
- Review → InProgress: Changes requested
- Any → Cancelled: Feature abandoned
- InProgress/Review → OnHold: Temporarily paused

## Data Storage Schema

### Features JSON Structure
```json
{
  "features": [
    {
      "id": "001-feature-numbering",
      "number": 1,
      "name": "feature-numbering",
      "description": "Systematic feature numbering and tracking system",
      "status": "Completed",
      "created_at": "2025-09-20T10:00:00Z",
      "updated_at": "2025-09-20T15:30:00Z",
      "metadata": {
        "priority": "High",
        "tags": ["infrastructure", "workflow"],
        "estimated_effort": "3 days",
        "assignee": "team-lead",
        "dependencies": [],
        "related_features": []
      },
      "branch_name": "001-feature-numbering"
    }
  ],
  "last_assigned_number": 1,
  "reserved_numbers": [100, 200, 300],
  "configuration": {
    "start_number": 1,
    "prefix": "",
    "auto_increment": true,
    "max_number": 9999
  }
}
```

## Validation Rules

### Number Assignment
- Next number = last_assigned_number + 1
- Skip reserved numbers automatically
- Validate against max_number limit
- Prevent duplicates

### Feature Creation
- All required fields must be provided
- Name must follow naming conventions
- Branch name must be unique
- Dependencies must reference existing features

### State Changes
- Status transitions must follow allowed paths
- Cannot transition from Completed to other states
- OnHold features cannot be worked on

## Performance Considerations

### Indexing Strategy
- In-memory index on feature numbers for O(1) lookup
- Secondary index on feature names for search
- Dependency graph stored in memory for relationship queries

### Caching Strategy
- Cache entire feature set in memory
- Write-through caching for persistence
- Lazy loading for large datasets

### File Operations
- Atomic writes to prevent corruption
- Backup creation before modifications
- Periodic compaction for efficiency

## Security Considerations

### Data Protection
- File permissions restricted to authorized users
- Input validation on all operations
- Path traversal prevention
- Data validation before persistence

### Access Control
- Read access for all team members
- Write access restricted to feature creators/assignees
- Admin access for reserved number management

---
**Data Model Complete**: All entities, relationships, and validation rules defined.