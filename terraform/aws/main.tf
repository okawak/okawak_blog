module "s3" {
  source = "./s3"
}

module "gh-action" {
  source     = "./gh-action"
  account_id = var.account_id
  gh-user    = "okawak"
  gh-repo    = "okawak_blog"
  gh-branch  = "main"
}
