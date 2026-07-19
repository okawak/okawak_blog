module "s3" {
  source        = "./s3"
  bucket_name   = var.blog_bucket_name
  force_destroy = true # terraform destroy 時に中身も削除する
}

module "gh-action" {
  source     = "./gh-action"
  account_id = var.account_id
  gh-user    = "okawak"
  gh-repo    = "okawak_blog"
  gh-branch  = "main"
}

module "s3_image_uploader" {
  source        = "./s3_image_uploader"
  bucket_name   = var.image_bucket_name
  uploader_name = var.image_uploader_user_name
  force_destroy = true # terraform destroy 時に中身も削除する
}

module "runtime_identity" {
  source                 = "./runtime_identity"
  name                   = "okawak-blog-runtime"
  bucket_arn             = module.s3.bucket_arn
  ca_certificate_pem     = file(var.roles_anywhere_ca_certificate_path)
  certificate_subject_cn = var.roles_anywhere_certificate_subject_cn
}
