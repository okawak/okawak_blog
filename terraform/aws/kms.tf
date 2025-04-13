resource "aws_kms_key" "tf_state" {
  description             = "Terraform state encryption key"
  deletion_window_in_days = 30
  enable_key_rotation     = true
}

resource "aws_kms_alias" "tf_state_alias" {
  name          = "alias/terraform-state-key"
  target_key_id = aws_kms_key.tf_state.key_id
}
