# EventLedger API Gateway (HTTP API)

resource "aws_apigatewayv2_api" "eventledger" {
  name          = "${var.prefix}-api"
  protocol_type = "HTTP"
  description   = "EventLedger REST API"

  cors_configuration {
    allow_origins = var.cors_allow_origins
    allow_methods = ["GET", "POST", "DELETE", "OPTIONS"]
    allow_headers = ["Content-Type", "Authorization", "X-Api-Key"]
    max_age       = 300
  }

  tags = var.tags
}

# Default stage with auto-deploy
resource "aws_apigatewayv2_stage" "default" {
  api_id      = aws_apigatewayv2_api.eventledger.id
  name        = "$default"
  auto_deploy = true

  access_log_settings {
    destination_arn = aws_cloudwatch_log_group.api.arn
    format = jsonencode({
      requestId         = "$context.requestId"
      ip                = "$context.identity.sourceIp"
      requestTime       = "$context.requestTime"
      httpMethod        = "$context.httpMethod"
      routeKey          = "$context.routeKey"
      status            = "$context.status"
      protocol          = "$context.protocol"
      responseLength    = "$context.responseLength"
      integrationError  = "$context.integrationErrorMessage"
      integrationLatency = "$context.integrationLatency"
    })
  }

  default_route_settings {
    throttling_burst_limit = var.throttling_burst_limit
    throttling_rate_limit  = var.throttling_rate_limit
  }

  tags = var.tags
}

# CloudWatch Log Group for API Gateway
resource "aws_cloudwatch_log_group" "api" {
  name              = "/aws/apigateway/${var.prefix}-api"
  retention_in_days = var.log_retention_days
  tags              = var.tags
}

# Lambda integrations
resource "aws_apigatewayv2_integration" "admin" {
  api_id                 = aws_apigatewayv2_api.eventledger.id
  integration_type       = "AWS_PROXY"
  integration_uri        = var.admin_invoke_arn
  payload_format_version = "2.0"
}

resource "aws_apigatewayv2_integration" "publish" {
  api_id                 = aws_apigatewayv2_api.eventledger.id
  integration_type       = "AWS_PROXY"
  integration_uri        = var.publish_invoke_arn
  payload_format_version = "2.0"
}

resource "aws_apigatewayv2_integration" "poll" {
  api_id                 = aws_apigatewayv2_api.eventledger.id
  integration_type       = "AWS_PROXY"
  integration_uri        = var.poll_invoke_arn
  payload_format_version = "2.0"
}

# Routes - Admin (streams management)
resource "aws_apigatewayv2_route" "create_stream" {
  api_id    = aws_apigatewayv2_api.eventledger.id
  route_key = "POST /streams"
  target    = "integrations/${aws_apigatewayv2_integration.admin.id}"
}

resource "aws_apigatewayv2_route" "list_streams" {
  api_id    = aws_apigatewayv2_api.eventledger.id
  route_key = "GET /streams"
  target    = "integrations/${aws_apigatewayv2_integration.admin.id}"
}

resource "aws_apigatewayv2_route" "get_stream" {
  api_id    = aws_apigatewayv2_api.eventledger.id
  route_key = "GET /streams/{stream_id}"
  target    = "integrations/${aws_apigatewayv2_integration.admin.id}"
}

resource "aws_apigatewayv2_route" "delete_stream" {
  api_id    = aws_apigatewayv2_api.eventledger.id
  route_key = "DELETE /streams/{stream_id}"
  target    = "integrations/${aws_apigatewayv2_integration.admin.id}"
}

# Routes - Subscriptions
resource "aws_apigatewayv2_route" "create_subscription" {
  api_id    = aws_apigatewayv2_api.eventledger.id
  route_key = "POST /streams/{stream_id}/subscriptions"
  target    = "integrations/${aws_apigatewayv2_integration.admin.id}"
}

resource "aws_apigatewayv2_route" "delete_subscription" {
  api_id    = aws_apigatewayv2_api.eventledger.id
  route_key = "DELETE /streams/{stream_id}/subscriptions/{subscription_id}"
  target    = "integrations/${aws_apigatewayv2_integration.admin.id}"
}

# Routes - Publish
resource "aws_apigatewayv2_route" "publish_events" {
  api_id    = aws_apigatewayv2_api.eventledger.id
  route_key = "POST /streams/{stream_id}/events"
  target    = "integrations/${aws_apigatewayv2_integration.publish.id}"
}

# Routes - Poll and Commit
resource "aws_apigatewayv2_route" "poll" {
  api_id    = aws_apigatewayv2_api.eventledger.id
  route_key = "GET /streams/{stream_id}/subscriptions/{subscription_id}/poll"
  target    = "integrations/${aws_apigatewayv2_integration.poll.id}"
}

resource "aws_apigatewayv2_route" "commit" {
  api_id    = aws_apigatewayv2_api.eventledger.id
  route_key = "POST /streams/{stream_id}/subscriptions/{subscription_id}/commit"
  target    = "integrations/${aws_apigatewayv2_integration.poll.id}"
}

# Lambda permissions for API Gateway
resource "aws_lambda_permission" "admin" {
  statement_id  = "AllowAPIGatewayInvoke"
  action        = "lambda:InvokeFunction"
  function_name = var.admin_function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.eventledger.execution_arn}/*/*"
}

resource "aws_lambda_permission" "publish" {
  statement_id  = "AllowAPIGatewayInvoke"
  action        = "lambda:InvokeFunction"
  function_name = var.publish_function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.eventledger.execution_arn}/*/*"
}

resource "aws_lambda_permission" "poll" {
  statement_id  = "AllowAPIGatewayInvoke"
  action        = "lambda:InvokeFunction"
  function_name = var.poll_function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.eventledger.execution_arn}/*/*"
}
