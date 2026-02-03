variable "table_name" {
  description = "Name of the DynamoDB table"
  type        = string
  default     = "eventledger"
}

variable "billing_mode" {
  description = "DynamoDB billing mode (PAY_PER_REQUEST or PROVISIONED)"
  type        = string
  default     = "PAY_PER_REQUEST"

  validation {
    condition     = contains(["PAY_PER_REQUEST", "PROVISIONED"], var.billing_mode)
    error_message = "billing_mode must be PAY_PER_REQUEST or PROVISIONED"
  }
}

variable "read_capacity" {
  description = "Read capacity units (only used with PROVISIONED billing)"
  type        = number
  default     = 5
}

variable "write_capacity" {
  description = "Write capacity units (only used with PROVISIONED billing)"
  type        = number
  default     = 5
}

variable "enable_autoscaling" {
  description = "Enable auto-scaling for provisioned capacity"
  type        = bool
  default     = false
}

variable "autoscaling_max_read_capacity" {
  description = "Maximum read capacity for auto-scaling"
  type        = number
  default     = 100
}

variable "autoscaling_max_write_capacity" {
  description = "Maximum write capacity for auto-scaling"
  type        = number
  default     = 100
}

variable "enable_point_in_time_recovery" {
  description = "Enable point-in-time recovery"
  type        = bool
  default     = true
}

variable "enable_ttl" {
  description = "Enable TTL for automatic expiration"
  type        = bool
  default     = false
}

variable "ttl_attribute" {
  description = "Attribute name for TTL"
  type        = string
  default     = "expires_at"
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}
