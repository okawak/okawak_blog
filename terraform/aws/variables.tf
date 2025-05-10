# AWS region
variable "aws_region" {
  description = "AWS region"
  type        = string
}

# AWS account ID
variable "account_id" {
  description = "AWS account ID"
  type        = string
}

variable "blog_bucket_name" {
  default = "okawak-blog-resources-bucket"
  type    = string
}

variable "iam_reader_name" {
  description = "value of IAM user name"
  type        = string
}
variable "secret_name" {
  description = "Secret name for IAM access key"
  type        = string
}

variable "rotation_interval" {
  description = "IAM access key rotation interval (days)"
  type        = number
}

variable "image_bucket_name" {
  default = "Image bucket name for Obsidian S3 Image Uploader"
  type    = string
}

variable "image_uploader_user_name" {
  description = "IAM user name for Obsidian uploading images"
  type        = string
  default     = "obsidian-image-uploader"
}
