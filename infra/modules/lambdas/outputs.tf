output "admin_function_name" {
  description = "Name of the admin Lambda function"
  value       = aws_lambda_function.admin.function_name
}

output "admin_function_arn" {
  description = "ARN of the admin Lambda function"
  value       = aws_lambda_function.admin.arn
}

output "admin_invoke_arn" {
  description = "Invoke ARN of the admin Lambda function"
  value       = aws_lambda_function.admin.invoke_arn
}

output "publish_function_name" {
  description = "Name of the publish Lambda function"
  value       = aws_lambda_function.publish.function_name
}

output "publish_function_arn" {
  description = "ARN of the publish Lambda function"
  value       = aws_lambda_function.publish.arn
}

output "publish_invoke_arn" {
  description = "Invoke ARN of the publish Lambda function"
  value       = aws_lambda_function.publish.invoke_arn
}

output "poll_function_name" {
  description = "Name of the poll Lambda function"
  value       = aws_lambda_function.poll.function_name
}

output "poll_function_arn" {
  description = "ARN of the poll Lambda function"
  value       = aws_lambda_function.poll.arn
}

output "poll_invoke_arn" {
  description = "Invoke ARN of the poll Lambda function"
  value       = aws_lambda_function.poll.invoke_arn
}

output "compactor_function_name" {
  description = "Name of the compactor Lambda function"
  value       = aws_lambda_function.compactor.function_name
}

output "compactor_function_arn" {
  description = "ARN of the compactor Lambda function"
  value       = aws_lambda_function.compactor.arn
}

output "lambda_role_arn" {
  description = "ARN of the Lambda execution role"
  value       = aws_iam_role.lambda.arn
}
