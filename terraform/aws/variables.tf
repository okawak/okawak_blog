# AWS region
variable "aws_region" {
  description = "AWS region"
  type        = string
}

variable "blog_bucket_name" {
  default = "okawak-blog-resources-bucket"
  type    = string
}

variable "image_bucket_name" {
  description = "S3 bucket name for Obsidian image uploads"
  type        = string
}

variable "image_uploader_user_name" {
  description = "IAM user name for Obsidian uploading images"
  type        = string
  default     = "obsidian-image-uploader"
}

variable "roles_anywhere_ca_certificate_path" {
  description = "Path to the public external CA certificate bundle"
  type        = string
}

variable "roles_anywhere_certificate_subject_cn" {
  type    = string
  default = "okawak-blog-vps"
}
