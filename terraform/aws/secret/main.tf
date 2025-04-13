resource "aws_secretsmanager_secret" "secret" {
  name        = var.secret_name
  description = "Rotating key for ${var.iam_user_name}"
}

resource "aws_iam_access_key" "initial" {
  user = var.iam_user_name
}

resource "aws_secretsmanager_secret_version" "init" {
  secret_id = aws_secretsmanager_secret.secret.id
  secret_string = jsonencode({
    aws_access_key_id     = aws_iam_access_key.initial.id
    aws_secret_access_key = aws_iam_access_key.initial.secret
  })
}

# Lambda zip
data "archive_file" "lambda_zip" {
  type        = "zip"
  source_file = "${path.module}/lambda/rotate_access_key.py"
  output_path = "${path.module}/lambda/rotate_access_key.zip"
}

data "aws_iam_policy_document" "assume" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["lambda.amazonaws.com"]
    }
  }
}
resource "aws_iam_role" "lambda_role" {
  name               = "rotate-key-lambda-role"
  assume_role_policy = data.aws_iam_policy_document.assume.json
}

data "aws_iam_policy_document" "lambda_policy" {
  statement {
    actions   = ["iam:CreateAccessKey", "iam:DeleteAccessKey", "iam:ListAccessKeys"]
    resources = ["arn:aws:iam::*:user/${var.iam_user_name}"]
  }
  statement {
    actions   = ["secretsmanager:PutSecretValue", "secretsmanager:GetSecretValue"]
    resources = [aws_secretsmanager_secret.secret.arn]
  }
  statement {
    actions   = ["logs:*"]
    resources = ["arn:aws:logs:*:*:*"]
  }
}
resource "aws_iam_policy" "lambda_policy" {
  name   = "rotate-key-lambda-policy"
  policy = data.aws_iam_policy_document.lambda_policy.json
}
resource "aws_iam_role_policy_attachment" "attach" {
  role       = aws_iam_role.lambda_role.name
  policy_arn = aws_iam_policy.lambda_policy.arn
}

resource "aws_lambda_function" "rotate" {
  function_name = "rotate_iam_key"
  role          = aws_iam_role.lambda_role.arn
  handler       = "rotate_access_key.lambda_handler"
  runtime       = "python3.12"
  filename      = data.archive_file.lambda_zip.output_path
  timeout       = 30
  environment {
    variables = {
      IAM_USER   = var.iam_user_name
      SECRET_ARN = aws_secretsmanager_secret.secret.arn
    }
  }
}

resource "aws_lambda_permission" "allow_secretsmanager" {
  statement_id  = "AllowSecretsManagerInvoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.rotate.function_name
  principal     = "secretsmanager.amazonaws.com"
  source_arn    = aws_secretsmanager_secret.secret.arn
}

resource "aws_secretsmanager_secret_rotation" "rotation" {
  secret_id           = aws_secretsmanager_secret.secret.id
  rotation_lambda_arn = aws_lambda_function.rotate.arn
  rotation_rules {
    automatically_after_days = var.rotation_interval
  }
  depends_on = [aws_lambda_permission.allow_secretsmanager]
}

data "aws_iam_policy_document" "secret_policy" {
  statement {
    sid = "AllowReaderUser"
    principals {
      type        = "AWS"
      identifiers = [var.iam_user_arn]
    }
    actions   = ["secretsmanager:GetSecretValue"]
    resources = ["*"]
  }
}

resource "aws_secretsmanager_secret_policy" "allow_reader" {
  secret_arn = aws_secretsmanager_secret.secret.arn
  policy     = data.aws_iam_policy_document.secret_policy.json
}
