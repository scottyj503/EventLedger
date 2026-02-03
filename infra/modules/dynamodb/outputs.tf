output "table_name" {
  description = "Name of the DynamoDB table"
  value       = aws_dynamodb_table.eventledger.name
}

output "table_arn" {
  description = "ARN of the DynamoDB table"
  value       = aws_dynamodb_table.eventledger.arn
}

output "table_id" {
  description = "ID of the DynamoDB table"
  value       = aws_dynamodb_table.eventledger.id
}

output "stream_arn" {
  description = "ARN of the DynamoDB stream"
  value       = aws_dynamodb_table.eventledger.stream_arn
}

output "stream_label" {
  description = "Label of the DynamoDB stream"
  value       = aws_dynamodb_table.eventledger.stream_label
}
