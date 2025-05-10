resource "aws_iam_user" "uploader" {
  name = var.uploader_name
}

data "aws_iam_policy_document" "put_policy" {
  statement {
    actions   = ["s3:PutObject", "s3:PutObjectAcl", "s3:GetObject"]
    resources = ["${aws_s3_bucket.this.arn}/*"]
  }
}

resource "aws_iam_policy" "upload" {
  name   = "${var.uploader_name}-put"
  policy = data.aws_iam_policy_document.put_policy.json
}

resource "aws_iam_user_policy_attachment" "attach" {
  user       = aws_iam_user.uploader.name
  policy_arn = aws_iam_policy.upload.arn
}

resource "aws_iam_access_key" "key" {
  user = aws_iam_user.uploader.name
}
