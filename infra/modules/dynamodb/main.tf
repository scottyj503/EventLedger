# EventLedger DynamoDB Table
# Single-table design with streams enabled for compaction

resource "aws_dynamodb_table" "eventledger" {
  name         = var.table_name
  billing_mode = var.billing_mode
  hash_key     = "PK"
  range_key    = "SK"

  # Provisioned capacity (only used if billing_mode = "PROVISIONED")
  read_capacity  = var.billing_mode == "PROVISIONED" ? var.read_capacity : null
  write_capacity = var.billing_mode == "PROVISIONED" ? var.write_capacity : null

  # Primary key attributes
  attribute {
    name = "PK"
    type = "S"
  }

  attribute {
    name = "SK"
    type = "S"
  }

  # Enable DynamoDB Streams for compaction
  stream_enabled   = true
  stream_view_type = "NEW_IMAGE"

  # Point-in-time recovery
  point_in_time_recovery {
    enabled = var.enable_point_in_time_recovery
  }

  # Server-side encryption
  server_side_encryption {
    enabled = true
  }

  # TTL for automatic event expiration (optional)
  ttl {
    attribute_name = var.ttl_attribute
    enabled        = var.enable_ttl
  }

  tags = merge(var.tags, {
    Name    = var.table_name
    Service = "eventledger"
  })
}

# Auto-scaling for provisioned mode (optional)
resource "aws_appautoscaling_target" "read" {
  count = var.billing_mode == "PROVISIONED" && var.enable_autoscaling ? 1 : 0

  max_capacity       = var.autoscaling_max_read_capacity
  min_capacity       = var.read_capacity
  resource_id        = "table/${aws_dynamodb_table.eventledger.name}"
  scalable_dimension = "dynamodb:table:ReadCapacityUnits"
  service_namespace  = "dynamodb"
}

resource "aws_appautoscaling_policy" "read" {
  count = var.billing_mode == "PROVISIONED" && var.enable_autoscaling ? 1 : 0

  name               = "${var.table_name}-read-autoscaling"
  policy_type        = "TargetTrackingScaling"
  resource_id        = aws_appautoscaling_target.read[0].resource_id
  scalable_dimension = aws_appautoscaling_target.read[0].scalable_dimension
  service_namespace  = aws_appautoscaling_target.read[0].service_namespace

  target_tracking_scaling_policy_configuration {
    predefined_metric_specification {
      predefined_metric_type = "DynamoDBReadCapacityUtilization"
    }
    target_value = 70.0
  }
}

resource "aws_appautoscaling_target" "write" {
  count = var.billing_mode == "PROVISIONED" && var.enable_autoscaling ? 1 : 0

  max_capacity       = var.autoscaling_max_write_capacity
  min_capacity       = var.write_capacity
  resource_id        = "table/${aws_dynamodb_table.eventledger.name}"
  scalable_dimension = "dynamodb:table:WriteCapacityUnits"
  service_namespace  = "dynamodb"
}

resource "aws_appautoscaling_policy" "write" {
  count = var.billing_mode == "PROVISIONED" && var.enable_autoscaling ? 1 : 0

  name               = "${var.table_name}-write-autoscaling"
  policy_type        = "TargetTrackingScaling"
  resource_id        = aws_appautoscaling_target.write[0].resource_id
  scalable_dimension = aws_appautoscaling_target.write[0].scalable_dimension
  service_namespace  = aws_appautoscaling_target.write[0].service_namespace

  target_tracking_scaling_policy_configuration {
    predefined_metric_specification {
      predefined_metric_type = "DynamoDBWriteCapacityUtilization"
    }
    target_value = 70.0
  }
}
