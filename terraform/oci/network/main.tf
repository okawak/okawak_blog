resource "oci_identity_compartment" "my_compartment" {
  name           = "compartment_for_okawak_blog"
  description    = "My compartment for OCI resources"
  compartment_id = var.tenancy_ocid
}

resource "oci_core_vcn" "my_vcn" {
  cidr_block     = "10.0.0.0/16"
  display_name   = "vcn_for_okawak_blog"
  compartment_id = oci_identity_compartment.my_compartment.id
}

resource "oci_core_internet_gateway" "my_internet_gateway" {
  compartment_id = oci_identity_compartment.my_compartment.id
  vcn_id         = oci_core_vcn.my_vcn.id
  display_name   = "okawak_oci_internet_gateway"
}

resource "oci_core_route_table" "my_route_table" {
  compartment_id = oci_identity_compartment.my_compartment.id
  vcn_id         = oci_core_vcn.my_vcn.id
  display_name   = "okawak_oci_route_table"

  route_rules {
    destination       = "0.0.0.0/0"
    network_entity_id = oci_core_internet_gateway.my_internet_gateway.id
  }
}

resource "oci_core_security_list" "my_security_list" {
  compartment_id = oci_identity_compartment.my_compartment.id
  vcn_id         = oci_core_vcn.my_vcn.id
  display_name   = "security_list_for_okawak_server"

  ingress_security_rules {
    # 6: TCP
    protocol = 6
    source   = "0.0.0.0/0"
    tcp_options {
      max = 22
      min = 22
    }
  }

  ingress_security_rules {
    # 6: TCP
    protocol = 6
    source   = "0.0.0.0/0"
    tcp_options {
      max = 60022
      min = 60022
    }
  }


  ingress_security_rules {
    # 6: TCP
    protocol  = 6
    source    = "0.0.0.0/0"
    stateless = false # stateful

    tcp_options {
      min = 80
      max = 80
    }
  }

  ingress_security_rules {
    # 6: TCP
    protocol  = 6
    source    = "0.0.0.0/0"
    stateless = false # stateful

    tcp_options {
      min = 443
      max = 443
    }
  }

  ingress_security_rules {
    # 17: UDP
    protocol = 17
    source   = "0.0.0.0/0"

    udp_options {
      min = 51820
      max = 51820
    }
  }

  ingress_security_rules {
    # 1: ICMP
    protocol  = 1
    source    = "0.0.0.0/0"
    stateless = false # stateful
  }

  egress_security_rules {
    # 6: TCP
    protocol    = 6
    destination = "0.0.0.0/0"
  }
}

resource "oci_core_subnet" "my_subnet" {
  cidr_block        = "10.0.1.0/24"
  vcn_id            = oci_core_vcn.my_vcn.id
  compartment_id    = oci_identity_compartment.my_compartment.id
  security_list_ids = [oci_core_security_list.my_security_list.id]
  route_table_id    = oci_core_route_table.my_route_table.id
}

data "oci_identity_availability_domains" "ads" {
  compartment_id = var.tenancy_ocid
}
