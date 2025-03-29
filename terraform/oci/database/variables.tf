variable "compartment_id" {
  type = string
}

variable "subnet_id" {
  type = string
}

variable "availability_domain" {
  type = string
}

variable "db_admin_password" {
  type        = string
  description = "admin password"
}

variable "db_shape" {
  type    = string
  default = "VM.Standard2.1"
}

variable "db_version" {
  type    = string
  default = "19.0.0.0"
}

variable "db_cpu_core_count" {
  type    = number
  default = 1
}

variable "db_data_storage_size_in_gb" {
  type    = number
  default = 50
}
