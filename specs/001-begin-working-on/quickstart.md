# Feature Numbering System - Quick Start Guide

## Overview

The Feature Numbering System provides a systematic approach to numbering and tracking features in the Merlin project. This guide will help you get started with using the system effectively.

## Prerequisites

- Rust 1.75+ installed
- Merlin project repository access
- Basic familiarity with Git and command line

## Installation

### 1. Build the Feature Numbering Library

```bash
# Navigate to project root
cd /Users/a/code/merlin

# Build the project
cargo build --release

# Run tests to verify installation
cargo test feature_numbering
```

### 2. Configuration

Create a configuration file `feature_numbering.toml`:

```toml
[feature_numbering]
start_number = 1
prefix = ""
auto_increment = true
storage_path = "./features.json"

[feature_numbering.validation]
max_number = 9999
reserved_numbers = [100, 200, 300]
allow_gaps = true

[feature_numbering.branches]
auto_create = true
naming_pattern = "{number}-{name}"
```

## Basic Usage

### Creating Your First Feature

```bash
# Using the CLI command
./target/release/merlin feature create \
  --name "user-authentication" \
  --description "Implement user authentication system" \
  --priority "High"

# Expected output:
# Feature created: 001-user-authentication
# Branch created: 001-user-authentication
# Spec file: ./specs/001-user-authentication/spec.md
```

### Listing Features

```bash
# List all features
./target/release/merlin feature list

# List features with specific status
./target/release/merlin feature list --status InProgress

# Search features
./target/release/merlin feature search --query "authentication"
```

### Managing Feature Status

```bash
# Update feature status
./target/release/merlin feature status \
  --feature "001-user-authentication" \
  --status "InProgress"

# View feature details
./target/release/merlin feature show "001-user-authentication"
```

## Integration with Development Workflow

### 1. Starting a New Feature

```bash
# 1. Create feature specification
./target/release/merlin feature create \
  --name "new-feature-name" \
  --description "Feature description" \
  --priority "Medium"

# 2. Switch to the created branch
git checkout 001-new-feature-name

# 3. Follow specification-driven development
# Use /plan, /tasks, /implement commands as usual
```

### 2. Feature Dependencies

```bash
# Add dependency to another feature
./target/release/merlin feature dependency \
  --feature "002-dependent-feature" \
  --add "001-base-feature"

# View dependency graph
./target/release/merlin feature dependencies "002-dependent-feature"
```

### 3. Feature Completion

```bash
# Mark feature as completed
./target/release/merlin feature status \
  --feature "001-completed-feature" \
  --status "Completed"

# Merge feature branch
git checkout main
git merge --no-ff 001-completed-feature
git branch -d 001-completed-feature
```

## API Usage

### Programmatic Access

```rust
use merlin::feature_numbering::FeatureManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the feature manager
    let manager = FeatureManager::new("./feature_numbering.toml").await?;

    // Create a new feature
    let feature = manager.create_feature(
        "api-endpoint",
        "Add new REST API endpoint",
        None
    ).await?;

    println!("Created feature: {}", feature.id);

    // List features
    let features = manager.list_features(None).await?;
    for feature in features {
        println!("{}: {}", feature.number, feature.name);
    }

    Ok(())
}
```

### HTTP API

The system provides a REST API for integration with other tools:

```bash
# Get next available number
curl http://localhost:8080/api/v1/numbers/next

# Create feature via API
curl -X POST http://localhost:8080/api/v1/features \
  -H "Content-Type: application/json" \
  -d '{
    "name": "new-feature",
    "description": "Feature description",
    "metadata": {
      "priority": "High"
    }
  }'

# Search features
curl "http://localhost:8080/api/v1/search?q=authentication&field=name&limit=10"
```

## Configuration Options

### Numbering Configuration

```toml
[feature_numbering]
# Starting number for feature numbering
start_number = 1

# Prefix for feature numbers (e.g., "FEATURE-")
prefix = ""

# Automatically increment numbers
auto_increment = true

# Path to feature storage file
storage_path = "./features.json"
```

### Validation Configuration

```toml
[feature_numbering.validation]
# Maximum allowed feature number
max_number = 9999

# Reserved feature numbers
reserved_numbers = [100, 200, 300, 1000]

# Allow gaps in numbering
allow_gaps = true
```

### Branch Configuration

```toml
[feature_numbering.branches]
# Automatically create branches for features
auto_create = true

# Branch naming pattern
naming_pattern = "{number}-{name}"

# Automatically checkout created branches
auto_checkout = true
```

## Troubleshooting

### Common Issues

**Feature number already exists**
```bash
# Check what features exist
./target/release/merlin feature list

# If duplicate found, check reserved numbers
./target/release/merlin number reserved
```

**Configuration file not found**
```bash
# Create default configuration
./target/release/merlin config init

# Specify configuration path explicitly
./target/release/merlin --config /path/to/config.toml feature create ...
```

**Permission denied**
```bash
# Check file permissions
ls -la features.json

# Fix permissions if needed
chmod 644 features.json
```

### Debug Mode

Enable debug logging for troubleshooting:

```bash
RUST_LOG=debug ./target/release/merlin feature create --name "test-feature"
```

## Best Practices

### 1. Feature Naming
- Use lowercase with hyphens (kebab-case)
- Keep names descriptive but concise
- Avoid special characters and spaces

### 2. Feature Management
- Delete features only in Draft status
- Use appropriate priority levels
- Keep dependencies minimal

### 3. Branch Management
- Keep feature branches focused
- Regularly sync with main branch
- Delete branches after merging

### 4. Documentation
- Update feature descriptions as requirements evolve
- Use tags for categorization
- Document important decisions in feature metadata

## Integration with Existing Tools

### Git Hooks

Create a Git pre-commit hook to validate feature numbers:

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Validate feature numbers in branch names
if [[ $(git branch --show-current) =~ ^[0-9]{3}-.+ ]]; then
    echo "✅ Valid feature branch name"
else
    echo "❌ Invalid feature branch name. Use format: XXX-feature-name"
    exit 1
fi
```

### CI/CD Integration

Add to your CI pipeline:

```yaml
- name: Validate Feature Numbers
  run: |
    ./target/release/merlin validate --all-features
```

## Getting Help

- Check the full documentation at `/docs/feature-numbering/`
- Run `./target/release/merlin --help` for command usage
- Review existing features with `./target/release/merlin feature list`

## Next Steps

1. Create your first feature following the workflow above
2. Explore the API documentation for advanced usage
3. Integrate with your existing development tools
4. Customize configuration for your team's needs

---
**Quick Start Complete**: You now have a working Feature Numbering System ready for use!