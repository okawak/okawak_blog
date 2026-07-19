variable "gh-user" {
  description = "GitHub user name"
}

variable "gh-repo" {
  description = "GitHub repository name"
}

variable "gh-branch" {
  description = "GitHub branch name"
}

variable "bucket_arn" {
  description = "Blog artifact bucket ARN"
  type        = string
}
