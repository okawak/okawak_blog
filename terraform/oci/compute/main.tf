# Computeインスタンス作成
resource "oci_core_instance" "oraclelinux_instance" {
  availability_domain = var.availability_domain
  compartment_id      = var.compartment_id
  display_name        = "okawak_blog_server"

  shape = "VM.Standard.E4.Flex"
  shape_config {
    ocpus                     = 2
    memory_in_gbs             = 8
    baseline_ocpu_utilization = "BASELINE_1_8"
  }

  source_details {
    source_id   = var.source_id
    source_type = "image"
  }

  create_vnic_details {
    assign_public_ip = true
    subnet_id        = var.subnet_id
  }

  metadata = {
    ssh_authorized_keys = file(var.ssh_public_key_path)
  }
  preserve_boot_volume = false
}
