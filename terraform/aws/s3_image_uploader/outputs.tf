output "domain_name" {
  value = aws_cloudfront_distribution.cdn.domain_name
}

output "access_key_id" {
  value = aws_iam_access_key.key.id
}

output "secret_access_key" {
  value     = aws_iam_access_key.key.secret
  sensitive = true
}
