module "s3" {
  source        = "./s3"
  bucket_name   = var.blog_bucket_name
  force_destroy = true # terraform destroy 時に中身も削除する
}

module "iam_reader" {
  source     = "./iam"
  name       = var.iam_reader_name
  bucket_arn = module.s3.bucket_arn
}

module "secret_rotation" {
  source            = "./secret"
  iam_user_name     = module.iam_reader.user_name
  iam_user_arn      = module.iam_reader.user_arn
  secret_name       = var.secret_name
  rotation_interval = var.rotation_interval
  depends_on        = [module.iam_reader]
}

module "gh-action" {
  source     = "./gh-action"
  account_id = var.account_id
  gh-user    = "okawak"
  gh-repo    = "okawak_blog"
  gh-branch  = "main"
}
