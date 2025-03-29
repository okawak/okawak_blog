module "network" {
  source       = "./network"
  tenancy_ocid = var.tenancy_ocid
  region       = var.region
}

module "compute" {
  source              = "./compute"
  compartment_id      = module.network.compartment_id
  subnet_id           = module.network.subnet_id
  availability_domain = module.network.availability_domain
  ssh_public_key_path = var.ssh_public_key_path
  source_id           = var.source_id
}

#module "database" {
#  source              = "./database"
#  compartment_id      = module.network.compartment_id
#  subnet_id           = module.network.subnet_id
#  availability_domain = module.network.availability_domain
#  db_admin_password   = var.db_admin_password
#}
