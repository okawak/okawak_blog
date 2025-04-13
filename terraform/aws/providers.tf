terraform {
  backend "s3" {
    bucket = "okawak-terraform-state"
    // bucket内のディレクトリを指定
    key     = "prod/blog/terraform.tfstate"
    region  = "ap-northeast-1"
    encrypt = true
    // エイリアス名
    kms_key_id   = "alias/terraform-state-key"
    use_lockfile = true
  }

  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
    archive = {
      source = "hashicorp/archive"
    }
  }
}

provider "aws" {
  region = var.aws_region
  default_tags {
    tags = {
      Terraform = "true"
    }
  }
}

provider "aws" {
  alias  = "us_east_1"
  region = "us-east-1"
  default_tags {
    tags = {
      Terraform = "true"
    }
  }
}
