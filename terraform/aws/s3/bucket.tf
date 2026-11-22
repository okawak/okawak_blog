resource "aws_s3_bucket" "myblog_bucket" {
  bucket        = var.bucket_name
  force_destroy = var.force_destroy

  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_s3_bucket_versioning" "versioning" {
  bucket = aws_s3_bucket.myblog_bucket.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "encryption" {
  bucket = aws_s3_bucket.myblog_bucket.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_ownership_controls" "ownership" {
  bucket = aws_s3_bucket.myblog_bucket.id

  rule {
    object_ownership = "BucketOwnerEnforced"
  }
}

resource "aws_s3_bucket_public_access_block" "block" {
  bucket                  = aws_s3_bucket.myblog_bucket.bucket
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_lifecycle_configuration" "lifecycle" {
  bucket = aws_s3_bucket.myblog_bucket.id

  depends_on = [
    aws_s3_bucket_versioning.versioning,
  ]

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

# Enforce create-only writes for release objects, including multipart completion.
# current.json is deliberately outside this prefix.
data "aws_iam_policy_document" "immutable_releases" {
  statement {
    sid       = "RequireConditionalReleaseCreation"
    effect    = "Deny"
    actions   = ["s3:PutObject"]
    resources = ["${aws_s3_bucket.myblog_bucket.arn}/releases/*"]

    principals {
      type        = "*"
      identifiers = ["*"]
    }

    condition {
      test     = "Null"
      variable = "s3:if-none-match"
      values   = ["true"]
    }

    condition {
      test     = "Bool"
      variable = "s3:ObjectCreationOperation"
      values   = ["true"]
    }
  }
}

resource "aws_s3_bucket_policy" "immutable_releases" {
  bucket = aws_s3_bucket.myblog_bucket.id
  policy = data.aws_iam_policy_document.immutable_releases.json
}
