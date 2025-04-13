resource "aws_iam_user" "this" {
  name = var.name
}

data "aws_iam_policy_document" "read_policy" {
  statement {
    actions   = ["s3:ListBucket"]
    resources = [var.bucket_arn]
  }
  statement {
    actions   = ["s3:GetObject"]
    resources = ["${var.bucket_arn}/*"]
  }
}

resource "aws_iam_policy" "this" {
  name   = "${var.name}-s3-read"
  policy = data.aws_iam_policy_document.read_policy.json
}

resource "aws_iam_user_policy_attachment" "attach" {
  user       = aws_iam_user.this.name
  policy_arn = aws_iam_policy.this.arn
}
