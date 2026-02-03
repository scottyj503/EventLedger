output "api_id" {
  description = "ID of the API Gateway"
  value       = aws_apigatewayv2_api.eventledger.id
}

output "api_endpoint" {
  description = "Endpoint URL of the API Gateway"
  value       = aws_apigatewayv2_api.eventledger.api_endpoint
}

output "api_execution_arn" {
  description = "Execution ARN of the API Gateway"
  value       = aws_apigatewayv2_api.eventledger.execution_arn
}

output "stage_id" {
  description = "ID of the default stage"
  value       = aws_apigatewayv2_stage.default.id
}
