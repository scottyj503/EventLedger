variable "prefix" {
  description = "Prefix for resource names"
  type        = string
  default     = "eventledger"
}

variable "dynamodb_table_name" {
  description = "Name of the DynamoDB table"
  type        = string
}

variable "dynamodb_table_arn" {
  description = "ARN of the DynamoDB table"
  type        = string
}

variable "dynamodb_stream_arn" {
  description = "ARN of the DynamoDB stream"
  type        = string
}

variable "admin_zip_path" {
  description = "Path to the admin Lambda zip file"
  type        = string
}

variable "publish_zip_path" {
  description = "Path to the publish Lambda zip file"
  type        = string
}

variable "poll_zip_path" {
  description = "Path to the poll Lambda zip file"
  type        = string
}

variable "compactor_zip_path" {
  description = "Path to the compactor Lambda zip file"
  type        = string
}

variable "lambda_memory_size" {
  description = "Memory size for Lambda functions (MB)"
  type        = number
  default     = 128
}

variable "log_level" {
  description = "Log level for Lambda functions"
  type        = string
  default     = "info"
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 14
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}
