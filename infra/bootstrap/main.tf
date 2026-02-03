# Bootstrap: Create S3 bucket and DynamoDB table for Terraform state
# Run this first: cd bootstrap && tofu init && tofu apply

terraform {
  required_version = ">= 1.6.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-west-2"
}

variable "environment" {
  description = "Environment name"
  type        = string
  default     = "dev"
}

locals {
  bucket_name = "eventledger-tfstate-${var.environment}"
  table_name  = "eventledger-tfstate-lock"
}

# S3 bucket for Terraform state
resource "aws_s3_bucket" "tfstate" {
  bucket = local.bucket_name

  tags = {
    Name        = local.bucket_name
    Environment = var.environment
    Purpose     = "terraform-state"
  }
}

resource "aws_s3_bucket_versioning" "tfstate" {
  bucket = aws_s3_bucket.tfstate.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "tfstate" {
  bucket = aws_s3_bucket.tfstate.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "tfstate" {
  bucket = aws_s3_bucket.tfstate.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# DynamoDB table for state locking
resource "aws_dynamodb_table" "tfstate_lock" {
  name         = local.table_name
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "LockID"

  attribute {
    name = "LockID"
    type = "S"
  }

  tags = {
    Name        = local.table_name
    Environment = var.environment
    Purpose     = "terraform-state-lock"
  }
}

output "bucket_name" {
  description = "S3 bucket name for Terraform state"
  value       = aws_s3_bucket.tfstate.id
}

output "dynamodb_table_name" {
  description = "DynamoDB table name for state locking"
  value       = aws_dynamodb_table.tfstate_lock.name
}

output "backend_config" {
  description = "Backend configuration for Terraform"
  value       = <<-EOT
    backend "s3" {
      bucket         = "${aws_s3_bucket.tfstate.id}"
      key            = "${var.environment}/terraform.tfstate"
      region         = "${var.aws_region}"
      encrypt        = true
      dynamodb_table = "${aws_dynamodb_table.tfstate_lock.name}"
    }
  EOT
}
