# S3バケットの作成
resource "aws_s3_bucket" "myblog_bucket" {
  bucket        = var.bucket_name
  force_destroy = var.force_destroy
}

# バージョニングの有効化
resource "aws_s3_bucket_versioning" "versioning" {
  bucket = aws_s3_bucket.myblog_bucket.id

  versioning_configuration {
    status = "Enabled"
  }
}

# サーバーサイド暗号化の設定
resource "aws_s3_bucket_server_side_encryption_configuration" "encryption" {
  bucket = aws_s3_bucket.myblog_bucket.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

# オーナーシップコントロールの設定
resource "aws_s3_bucket_ownership_controls" "ownership" {
  bucket = aws_s3_bucket.myblog_bucket.id

  rule {
    object_ownership = "BucketOwnerEnforced"
  }
}

# 意図しないパブリックアクセスの防止
resource "aws_s3_bucket_public_access_block" "block" {
  bucket                  = aws_s3_bucket.myblog_bucket.bucket
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# 古いバージョンのものを削除する
resource "aws_s3_bucket_lifecycle_configuration" "lifecycle" {
  bucket = aws_s3_bucket.myblog_bucket.id

  rule {
    id     = "DeleteOldNoncurrentVersions"
    status = "Enabled"

    filter {
      prefix = ""
    }

    noncurrent_version_expiration {
      noncurrent_days = 30
    }
  }
}
