variable "prefix" {
  description = "Prefix for resource names"
  type        = string
  default     = "eventledger"
}

variable "admin_function_name" {
  description = "Name of the admin Lambda function"
  type        = string
}

variable "admin_invoke_arn" {
  description = "Invoke ARN of the admin Lambda function"
  type        = string
}

variable "publish_function_name" {
  description = "Name of the publish Lambda function"
  type        = string
}

variable "publish_invoke_arn" {
  description = "Invoke ARN of the publish Lambda function"
  type        = string
}

variable "poll_function_name" {
  description = "Name of the poll Lambda function"
  type        = string
}

variable "poll_invoke_arn" {
  description = "Invoke ARN of the poll Lambda function"
  type        = string
}

variable "cors_allow_origins" {
  description = "Allowed origins for CORS"
  type        = list(string)
  default     = ["*"]
}

variable "throttling_burst_limit" {
  description = "API Gateway throttling burst limit"
  type        = number
  default     = 100
}

variable "throttling_rate_limit" {
  description = "API Gateway throttling rate limit"
  type        = number
  default     = 50
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
