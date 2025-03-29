output "compartment_id" {
  value = oci_identity_compartment.my_compartment.id
}

output "vcn_id" {
  value = oci_core_vcn.my_vcn.id
}

output "subnet_id" {
  value = oci_core_subnet.my_subnet.id
}

output "availability_domain" {
  value = data.oci_identity_availability_domains.ads.availability_domains[0].name
}
