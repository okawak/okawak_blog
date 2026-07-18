variable "name" {
  type = string
}

variable "bucket_arn" {
  type = string
}

variable "ca_certificate_pem" {
  description = "Public CA certificate bundle in PEM format"
  type        = string
}

variable "certificate_subject_cn" {
  type = string
}

variable "session_duration_seconds" {
  type    = number
  default = 3600
}
