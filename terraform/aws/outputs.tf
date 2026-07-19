output "bucket_name" {
  value = module.s3.bucket_name
}

output "domain_name" {
  value = module.s3.domain_name
}

output "origin_id" {
  value = module.s3.origin_id
}

output "cloudfront_domain_name" {
  value = module.s3_image_uploader.domain_name
}

output "obsidian_uploader_access_key" {
  value       = module.s3_image_uploader.access_key_id
  description = "AccessKeyId for Obsidian S3 Image Uploader"
}

output "image_uploader_access_key" {
  value     = module.s3_image_uploader.secret_access_key
  sensitive = true
}

output "roles_anywhere_trust_anchor_arn" {
  value = module.runtime_identity.trust_anchor_arn
}

output "roles_anywhere_profile_arn" {
  value = module.runtime_identity.profile_arn
}

output "roles_anywhere_role_arn" {
  value = module.runtime_identity.role_arn
}
