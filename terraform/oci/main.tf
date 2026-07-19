module "network" {
  source       = "./network"
  tenancy_ocid = var.tenancy_ocid
}

module "compute" {
  source              = "./compute"
  compartment_id      = module.network.compartment_id
  subnet_id           = module.network.subnet_id
  availability_domain = module.network.availability_domain
  ssh_public_key_path = var.ssh_public_key_path
  source_id           = var.source_id
}

resource "oci_core_public_ip" "blog" {
  compartment_id = module.network.compartment_id
  display_name   = "okawak_blog_public_ip"
  lifetime       = "RESERVED"
  private_ip_id  = local.blog_primary_private_ip_id

  lifecycle {
    prevent_destroy = true
  }
}

data "oci_core_private_ips" "blog_primary" {
  ip_address = module.compute.private_ip
  subnet_id  = module.network.subnet_id
}

locals {
  blog_primary_private_ip_id = one([
    for private_ip in data.oci_core_private_ips.blog_primary.private_ips :
    private_ip.id
    if private_ip.is_primary
  ])
}
