# network module
output "compartment_id" {
  value = module.network.compartment_id
}

output "vcn_id" {
  value = module.network.vcn_id
}

output "subnet_id" {
  value = module.network.subnet_id
}

output "availability_domain" {
  value = module.network.availability_domain
}

# compute module
output "public-ip-for-compute-instance" {
  value = module.compute.public-ip-for-compute-instance
}

output "instance-name" {
  value = module.compute.instance-name
}

output "instance-OCID" {
  value = module.compute.instance-OCID
}

output "instance-region" {
  value = module.compute.instance-region
}

output "instance-shape" {
  value = module.compute.instance-shape
}

output "instance-state" {
  value = module.compute.instance-state
}

output "instance-OCPUs" {
  value = module.compute.instance-OCPUs
}

output "instance-memory-in-GBs" {
  value = module.compute.instance-memory-in-GBs
}

output "time-created" {
  value = module.compute.time-created
}

#output "db_system_id" {
#  value = module.database.db_system_id
#}
