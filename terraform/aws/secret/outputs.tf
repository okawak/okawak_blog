output "secret_arn" {
  value = aws_secretsmanager_secret.secret.arn
}

output "initial_key_id" {
  value     = aws_iam_access_key.initial.id
  sensitive = true
}

output "initial_key_secret" {
  value     = aws_iam_access_key.initial.secret
  sensitive = true
}
