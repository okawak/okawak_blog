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
