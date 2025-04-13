output "bucket_name" {
  value = aws_s3_bucket.myblog_bucket.bucket
}

output "domain_name" {
  value = aws_s3_bucket.myblog_bucket.bucket_regional_domain_name
}

output "origin_id" {
  value = aws_s3_bucket.myblog_bucket.id
}

output "bucket_arn" {
  value = aws_s3_bucket.myblog_bucket.arn
}
