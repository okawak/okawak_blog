output "bucket_name" {
  value = module.s3.bucket_name
}

output "domain_name" {
  value = module.s3.domain_name
}

output "origin_id" {
  value = module.s3.origin_id
}

output "secret_arn" {
  value = module.secret_rotation.secret_arn
}

output "cloudfront_domain_name" {
  value = module.s3_image_uploader.domain_name
}

output "obsidian_uploader_access_key" {
  value       = module.s3_image_uploader.access_key_id
  description = "AccessKeyId for Obsidian S3 Image Uploader"
}

# センシティブ出力 (terraform output -json で取得)
output "iam_reader_access_key_id" {
  value     = module.secret_rotation.initial_key_id
  sensitive = true
}

output "iam_reader_access_key_secret" {
  value     = module.secret_rotation.initial_key_secret
  sensitive = true
}

output "image_uploader_access_key" {
  value     = module.s3_image_uploader.secret_access_key
  sensitive = true
}
