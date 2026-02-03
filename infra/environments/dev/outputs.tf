output "api_endpoint" {
  description = "API Gateway endpoint URL"
  value       = module.api.api_endpoint
}

output "dynamodb_table_name" {
  description = "DynamoDB table name"
  value       = module.dynamodb.table_name
}

output "dynamodb_table_arn" {
  description = "DynamoDB table ARN"
  value       = module.dynamodb.table_arn
}

output "admin_function_name" {
  description = "Admin Lambda function name"
  value       = module.lambdas.admin_function_name
}

output "publish_function_name" {
  description = "Publish Lambda function name"
  value       = module.lambdas.publish_function_name
}

output "poll_function_name" {
  description = "Poll Lambda function name"
  value       = module.lambdas.poll_function_name
}

output "compactor_function_name" {
  description = "Compactor Lambda function name"
  value       = module.lambdas.compactor_function_name
}
