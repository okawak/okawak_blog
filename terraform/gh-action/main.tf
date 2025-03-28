data "http" "github_actions_openid_configuration" {
  url = "https://token.actions.githubusercontent.com/.well-known/openid-configuration"
}

data "tls_certificate" "github_actions" {
  url = jsondecode(data.http.github_actions_openid_configuration.response_body).jwks_uri
}

resource "aws_iam_openid_connect_provider" "github_actions" {
  url             = "https://token.actions.githubusercontent.com"
  client_id_list  = ["sts.amazonaws.com"]
  thumbprint_list = data.tls_certificate.github_actions.certificates[*].sha1_fingerprint
}

data "aws_iam_policy_document" "role_policy" {
  statement {
    effect  = "Allow"
    actions = ["sts:AssumeRoleWithWebIdentity"]
    principals {
      type        = "Federated"
      identifiers = ["arn:aws:iam::${var.account_id}:oidc-provider/token.actions.githubusercontent.com"]
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

resource "aws_iam_role_policy_attachment" "s3_full" {
  role       = aws_iam_role.gh_role.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonS3FullAccess"
}
