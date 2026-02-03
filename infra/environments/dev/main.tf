# EventLedger - Development Environment

terraform {
  required_version = ">= 1.6.0"

  backend "s3" {
    bucket         = "eventledger-tfstate-dev"
    key            = "dev/terraform.tfstate"
    region         = "us-west-2"
    encrypt        = true
    dynamodb_table = "eventledger-tfstate-lock"
  }

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Environment = "dev"
      Project     = "eventledger"
      ManagedBy   = "terraform"
    }
  }
}

locals {
  prefix = "eventledger-dev"

  lambda_zip_base = "${path.module}/../../../lambdas/target/lambda"

  tags = {
    Environment = "dev"
    Project     = "eventledger"
  }
}

# DynamoDB Table
module "dynamodb" {
  source = "../../modules/dynamodb"

  table_name                    = "${local.prefix}-table"
  billing_mode                  = "PAY_PER_REQUEST"
  enable_point_in_time_recovery = true
  enable_ttl                    = false

  tags = local.tags
}

# Lambda Functions
module "lambdas" {
  source = "../../modules/lambdas"

  prefix              = local.prefix
  dynamodb_table_name = module.dynamodb.table_name
  dynamodb_table_arn  = module.dynamodb.table_arn
  dynamodb_stream_arn = module.dynamodb.stream_arn

  admin_zip_path     = "${local.lambda_zip_base}/eventledger-admin/bootstrap.zip"
  publish_zip_path   = "${local.lambda_zip_base}/eventledger-publish/bootstrap.zip"
  poll_zip_path      = "${local.lambda_zip_base}/eventledger-poll/bootstrap.zip"
  compactor_zip_path = "${local.lambda_zip_base}/eventledger-compactor/bootstrap.zip"

  lambda_memory_size = 128
  log_level          = "debug"
  log_retention_days = 7

  tags = local.tags
}

# API Gateway
module "api" {
  source = "../../modules/api"

  prefix = local.prefix

  admin_function_name   = module.lambdas.admin_function_name
  admin_invoke_arn      = module.lambdas.admin_invoke_arn
  publish_function_name = module.lambdas.publish_function_name
  publish_invoke_arn    = module.lambdas.publish_invoke_arn
  poll_function_name    = module.lambdas.poll_function_name
  poll_invoke_arn       = module.lambdas.poll_invoke_arn

  cors_allow_origins     = ["*"]
  throttling_burst_limit = 100
  throttling_rate_limit  = 50
  log_retention_days     = 7

  tags = local.tags
}
