resource "aws_rolesanywhere_trust_anchor" "this" {
  name    = "${var.name}-trust-anchor"
  enabled = true

  source {
    source_data {
      x509_certificate_data = var.ca_certificate_pem
    }
    source_type = "CERTIFICATE_BUNDLE"
  }
}

data "aws_iam_policy_document" "s3_read" {
  statement {
    actions = ["s3:GetObject"]
    resources = [
      "${var.bucket_arn}/current.json",
      "${var.bucket_arn}/releases/*/site/*",
    ]
  }
}

data "aws_iam_policy_document" "assume_role" {
  statement {
    actions = [
      "sts:AssumeRole",
      "sts:TagSession",
      "sts:SetSourceIdentity",
    ]

    principals {
      type        = "Service"
      identifiers = ["rolesanywhere.amazonaws.com"]
    }

    condition {
      test     = "ArnEquals"
      variable = "aws:SourceArn"
      values   = [aws_rolesanywhere_trust_anchor.this.arn]
    }

    condition {
      test     = "StringEquals"
      variable = "aws:PrincipalTag/x509Subject/CN"
      values   = [var.certificate_subject_cn]
    }
  }
}

resource "aws_iam_role" "this" {
  name               = "${var.name}-role"
  assume_role_policy = data.aws_iam_policy_document.assume_role.json
}

resource "aws_iam_policy" "s3_read" {
  name   = "${var.name}-s3-read"
  policy = data.aws_iam_policy_document.s3_read.json
}

resource "aws_iam_role_policy_attachment" "s3_read" {
  role       = aws_iam_role.this.name
  policy_arn = aws_iam_policy.s3_read.arn
}

resource "aws_rolesanywhere_profile" "this" {
  name             = "${var.name}-profile"
  enabled          = true
  duration_seconds = var.session_duration_seconds
  role_arns        = [aws_iam_role.this.arn]
}
