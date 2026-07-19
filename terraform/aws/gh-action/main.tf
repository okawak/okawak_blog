resource "aws_iam_openid_connect_provider" "github_actions" {
  url            = "https://token.actions.githubusercontent.com"
  client_id_list = ["sts.amazonaws.com"]
}

data "aws_iam_policy_document" "role_policy" {
  statement {
    effect  = "Allow"
    actions = ["sts:AssumeRoleWithWebIdentity"]
    principals {
      type        = "Federated"
      identifiers = [aws_iam_openid_connect_provider.github_actions.arn]
    }
    condition {
      test     = "StringEquals"
      variable = "token.actions.githubusercontent.com:aud"
      values   = ["sts.amazonaws.com"]
    }

    condition {
      test     = "StringEquals"
      variable = "token.actions.githubusercontent.com:sub"
      values   = ["repo:${var.gh-user}/${var.gh-repo}:ref:refs/heads/${var.gh-branch}"]
    }
  }
}

resource "aws_iam_role" "gh_role" {
  name               = "oidc-gh-role"
  assume_role_policy = data.aws_iam_policy_document.role_policy.json
}

data "aws_iam_policy_document" "publish" {
  statement {
    sid = "ListBlogBucket"

    actions = [
      "s3:GetBucketLocation",
      "s3:ListBucket",
      "s3:ListBucketMultipartUploads",
    ]

    resources = [var.bucket_arn]
  }

  statement {
    sid = "PublishBlogArtifacts"

    actions = [
      "s3:AbortMultipartUpload",
      "s3:GetObject",
      "s3:ListMultipartUploadParts",
      "s3:PutObject",
    ]

    resources = [
      "${var.bucket_arn}/current.json",
      "${var.bucket_arn}/releases/*",
    ]
  }
}

resource "aws_iam_policy" "publish" {
  name        = "okawak-blog-publisher"
  description = "Publish and validate immutable okawak blog artifacts"
  policy      = data.aws_iam_policy_document.publish.json
}

resource "aws_iam_role_policy_attachment" "publish" {
  role       = aws_iam_role.gh_role.name
  policy_arn = aws_iam_policy.publish.arn
}
