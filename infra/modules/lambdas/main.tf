# EventLedger Lambda Functions

# IAM Role for Lambda functions
resource "aws_iam_role" "lambda" {
  name = "${var.prefix}-lambda-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      }
    ]
  })

  tags = var.tags
}

# Basic Lambda execution policy
resource "aws_iam_role_policy_attachment" "lambda_basic" {
  role       = aws_iam_role.lambda.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# DynamoDB access policy
resource "aws_iam_role_policy" "dynamodb" {
  name = "${var.prefix}-dynamodb-policy"
  role = aws_iam_role.lambda.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "dynamodb:GetItem",
          "dynamodb:PutItem",
          "dynamodb:UpdateItem",
          "dynamodb:DeleteItem",
          "dynamodb:Query",
          "dynamodb:Scan",
          "dynamodb:BatchGetItem",
          "dynamodb:BatchWriteItem"
        ]
        Resource = [
          var.dynamodb_table_arn,
          "${var.dynamodb_table_arn}/index/*"
        ]
      }
    ]
  })
}

# DynamoDB Streams policy for compactor
resource "aws_iam_role_policy" "dynamodb_streams" {
  name = "${var.prefix}-dynamodb-streams-policy"
  role = aws_iam_role.lambda.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "dynamodb:GetRecords",
          "dynamodb:GetShardIterator",
          "dynamodb:DescribeStream",
          "dynamodb:ListStreams"
        ]
        Resource = [
          var.dynamodb_stream_arn
        ]
      }
    ]
  })
}

# CloudWatch Logs policy
resource "aws_iam_role_policy" "cloudwatch" {
  name = "${var.prefix}-cloudwatch-policy"
  role = aws_iam_role.lambda.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = "arn:aws:logs:*:*:*"
      }
    ]
  })
}

# Admin Lambda
resource "aws_lambda_function" "admin" {
  function_name = "${var.prefix}-admin"
  role          = aws_iam_role.lambda.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["arm64"]
  timeout       = 30
  memory_size   = var.lambda_memory_size

  filename         = var.admin_zip_path
  source_code_hash = filebase64sha256(var.admin_zip_path)

  environment {
    variables = {
      EVENTLEDGER_TABLE = var.dynamodb_table_name
      RUST_LOG          = var.log_level
    }
  }

  tags = var.tags
}

# Publish Lambda
resource "aws_lambda_function" "publish" {
  function_name = "${var.prefix}-publish"
  role          = aws_iam_role.lambda.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["arm64"]
  timeout       = 30
  memory_size   = var.lambda_memory_size

  filename         = var.publish_zip_path
  source_code_hash = filebase64sha256(var.publish_zip_path)

  environment {
    variables = {
      EVENTLEDGER_TABLE = var.dynamodb_table_name
      RUST_LOG          = var.log_level
    }
  }

  tags = var.tags
}

# Poll Lambda
resource "aws_lambda_function" "poll" {
  function_name = "${var.prefix}-poll"
  role          = aws_iam_role.lambda.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["arm64"]
  timeout       = 30
  memory_size   = var.lambda_memory_size

  filename         = var.poll_zip_path
  source_code_hash = filebase64sha256(var.poll_zip_path)

  environment {
    variables = {
      EVENTLEDGER_TABLE = var.dynamodb_table_name
      RUST_LOG          = var.log_level
    }
  }

  tags = var.tags
}

# Compactor Lambda
resource "aws_lambda_function" "compactor" {
  function_name = "${var.prefix}-compactor"
  role          = aws_iam_role.lambda.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["arm64"]
  timeout       = 60
  memory_size   = var.lambda_memory_size

  filename         = var.compactor_zip_path
  source_code_hash = filebase64sha256(var.compactor_zip_path)

  environment {
    variables = {
      EVENTLEDGER_TABLE = var.dynamodb_table_name
      RUST_LOG          = var.log_level
    }
  }

  tags = var.tags
}

# DynamoDB Stream trigger for Compactor
resource "aws_lambda_event_source_mapping" "compactor_stream" {
  event_source_arn  = var.dynamodb_stream_arn
  function_name     = aws_lambda_function.compactor.arn
  starting_position = "LATEST"
  batch_size        = 100

  filter_criteria {
    filter {
      pattern = jsonencode({
        eventName = ["INSERT", "MODIFY"]
      })
    }
  }
}

# CloudWatch Log Groups
resource "aws_cloudwatch_log_group" "admin" {
  name              = "/aws/lambda/${aws_lambda_function.admin.function_name}"
  retention_in_days = var.log_retention_days
  tags              = var.tags
}

resource "aws_cloudwatch_log_group" "publish" {
  name              = "/aws/lambda/${aws_lambda_function.publish.function_name}"
  retention_in_days = var.log_retention_days
  tags              = var.tags
}

resource "aws_cloudwatch_log_group" "poll" {
  name              = "/aws/lambda/${aws_lambda_function.poll.function_name}"
  retention_in_days = var.log_retention_days
  tags              = var.tags
}

resource "aws_cloudwatch_log_group" "compactor" {
  name              = "/aws/lambda/${aws_lambda_function.compactor.function_name}"
  retention_in_days = var.log_retention_days
  tags              = var.tags
}
