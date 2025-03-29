variable "tenancy_ocid" {
  type        = string
  description = "tenancy OCID"
}

variable "region" {
  type        = string
  description = "OCI region"
}

variable "user_ocid" {
  type        = string
  description = "user OCID"
}

variable "fingerprint" {
  type        = string
  description = "fingerprint"
}

variable "private_key_path" {
  type        = string
  description = "private key path for OCI"
}

variable "ssh_public_key_path" {
  type        = string
  description = "SSH public key path"
}

variable "source_id" {
  type        = string
  description = "OCI Image OCID for the instance"
}

#
# variable "db_admin_password" {
#   type        = string
#   description = "Base Database Serviceの管理者パスワード"
# }
