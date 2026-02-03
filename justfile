# EventLedger Development Tasks

set dotenv-load

# Default recipe
default:
    @just --list

# ============================================================================
# Build
# ============================================================================

# Build all Lambda functions
build:
    cd lambdas && cargo build --release

# Build for Lambda (ARM64)
build-lambda:
    cd lambdas && cargo lambda build --release --arm64

# Package Lambda functions as zip files
package: build-lambda
    @echo "Packaging Lambda functions..."
    @mkdir -p lambdas/target/lambda/eventledger-admin
    @mkdir -p lambdas/target/lambda/eventledger-publish
    @mkdir -p lambdas/target/lambda/eventledger-poll
    @mkdir -p lambdas/target/lambda/eventledger-compactor
    @cp lambdas/target/lambda/eventledger-admin/bootstrap lambdas/target/lambda/eventledger-admin/
    @cp lambdas/target/lambda/eventledger-publish/bootstrap lambdas/target/lambda/eventledger-publish/
    @cp lambdas/target/lambda/eventledger-poll/bootstrap lambdas/target/lambda/eventledger-poll/
    @cp lambdas/target/lambda/eventledger-compactor/bootstrap lambdas/target/lambda/eventledger-compactor/
    cd lambdas/target/lambda/eventledger-admin && zip -j bootstrap.zip bootstrap
    cd lambdas/target/lambda/eventledger-publish && zip -j bootstrap.zip bootstrap
    cd lambdas/target/lambda/eventledger-poll && zip -j bootstrap.zip bootstrap
    cd lambdas/target/lambda/eventledger-compactor && zip -j bootstrap.zip bootstrap
    @echo "Lambda packages created!"

# ============================================================================
# Test
# ============================================================================

# Run all tests
test:
    cd lambdas && cargo test

# Run tests with output
test-verbose:
    cd lambdas && cargo test -- --nocapture

# Run integration tests (requires deployed infrastructure)
integration-test:
    @echo "Running integration tests..."
    cd tests/integration && cargo test -- --nocapture

# ============================================================================
# Code Quality
# ============================================================================

# Check code without building
check:
    cd lambdas && cargo check

# Format code
fmt:
    cd lambdas && cargo fmt

# Lint code
lint:
    cd lambdas && cargo clippy -- -D warnings

# Full CI check
ci: fmt lint test
    @echo "CI checks passed!"

# ============================================================================
# Infrastructure - Bootstrap
# ============================================================================

# Bootstrap Terraform state backend (run once)
bootstrap-init:
    cd infra/bootstrap && tofu init

# Create S3 bucket and DynamoDB for state (run once)
bootstrap-apply:
    cd infra/bootstrap && tofu apply

# ============================================================================
# Infrastructure - Dev Environment
# ============================================================================

# Initialize Terraform for dev
tf-init:
    cd infra/environments/dev && tofu init

# Plan Terraform changes for dev
tf-plan: package
    cd infra/environments/dev && tofu plan

# Apply Terraform changes for dev
tf-apply: package
    cd infra/environments/dev && tofu apply

# Destroy Terraform resources for dev
tf-destroy:
    cd infra/environments/dev && tofu destroy

# Show Terraform outputs
tf-output:
    cd infra/environments/dev && tofu output

# ============================================================================
# Deploy
# ============================================================================

# Full deploy: build, package, and apply
deploy: package tf-apply
    @echo "Deployment complete!"
    @just tf-output

# ============================================================================
# Local Development
# ============================================================================

# Start local DynamoDB for testing
dynamodb-local:
    docker run -d --name dynamodb-local -p 8000:8000 amazon/dynamodb-local

# Stop local DynamoDB
dynamodb-local-stop:
    docker stop dynamodb-local && docker rm dynamodb-local

# Create local DynamoDB table
dynamodb-local-create-table:
    aws dynamodb create-table \
        --endpoint-url http://localhost:8000 \
        --table-name eventledger \
        --attribute-definitions \
            AttributeName=PK,AttributeType=S \
            AttributeName=SK,AttributeType=S \
        --key-schema \
            AttributeName=PK,KeyType=HASH \
            AttributeName=SK,KeyType=RANGE \
        --billing-mode PAY_PER_REQUEST

# Watch for changes and rebuild
watch:
    cd lambdas && cargo watch -x check

# ============================================================================
# Utilities
# ============================================================================

# Validate JSON schemas
validate-schemas:
    @echo "Validating JSON schemas..."
    @for f in schemas/*.json; do \
        python3 -c "import json; json.load(open('$$f'))" && echo "✓ $$f" || echo "✗ $$f"; \
    done

# Clean build artifacts
clean:
    cd lambdas && cargo clean
    rm -rf lambdas/target/lambda/*/bootstrap.zip

# Show project structure
tree:
    @find . -type f \( -name "*.rs" -o -name "*.toml" -o -name "*.tf" -o -name "*.json" \) \
        ! -path "*/target/*" ! -path "*/.terraform/*" | sort | head -50
