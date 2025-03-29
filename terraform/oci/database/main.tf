#resource "oci_database_db_system" "my_db_system" {
#  #Required
#  availability_domain = var.db_system_availability_domain
#  compartment_id      = var.compartment_id
#  db_home {
#    #Required
#    database {
#      #Required
#      admin_password = var.db_system_db_home_database_admin_password
#
#      #Optional
#      backup_id                  = oci_database_backup.test_backup.id
#      backup_tde_password        = var.db_system_db_home_database_backup_tde_password
#      character_set              = var.db_system_db_home_database_character_set
#      database_id                = oci_database_database.test_database.id
#      database_software_image_id = oci_database_database_software_image.test_database_software_image.id
#      db_backup_config {
#
#        #Optional
#        auto_backup_enabled     = var.db_system_db_home_database_db_backup_config_auto_backup_enabled
#        auto_backup_window      = var.db_system_db_home_database_db_backup_config_auto_backup_window
#        auto_full_backup_day    = var.db_system_db_home_database_db_backup_config_auto_full_backup_day
#        auto_full_backup_window = var.db_system_db_home_database_db_backup_config_auto_full_backup_window
#        backup_deletion_policy  = var.db_system_db_home_database_db_backup_config_backup_deletion_policy
#        backup_destination_details {
#
#          #Optional
#          dbrs_policy_id = oci_identity_policy.test_policy.id
#          id             = var.db_system_db_home_database_db_backup_config_backup_destination_details_id
#          type           = var.db_system_db_home_database_db_backup_config_backup_destination_details_type
#        }
#        recovery_window_in_days   = var.db_system_db_home_database_db_backup_config_recovery_window_in_days
#        run_immediate_full_backup = var.db_system_db_home_database_db_backup_config_run_immediate_full_backup
#      }
#      db_domain    = var.db_system_db_home_database_db_domain
#      db_name      = var.db_system_db_home_database_db_name
#      db_workload  = var.db_system_db_home_database_db_workload
#      defined_tags = var.db_system_db_home_database_defined_tags
#      encryption_key_location_details {
#        #Required
#        hsm_password  = var.db_system_db_home_database_encryption_key_location_details_hsm_password
#        provider_type = var.db_system_db_home_database_encryption_key_location_details_provider_type
#      }
#      freeform_tags       = var.db_system_db_home_database_freeform_tags
#      key_store_id        = oci_database_key_store.test_key_store.id
#      kms_key_id          = oci_kms_key.test_key.id
#      kms_key_version_id  = oci_kms_key_version.test_key_version.id
#      ncharacter_set      = var.db_system_db_home_database_ncharacter_set
#      pdb_name            = var.db_system_db_home_database_pdb_name
#      pluggable_databases = var.db_system_db_home_database_pluggable_databases
#      sid_prefix          = var.db_system_db_home_database_sid_prefix
#      source_encryption_key_location_details {
#        #Required
#        hsm_password  = var.db_system_db_home_database_source_encryption_key_location_details_hsm_password
#        provider_type = var.db_system_db_home_database_source_encryption_key_location_details_provider_type
#      }
#      tde_wallet_password                   = var.db_system_db_home_database_tde_wallet_password
#      time_stamp_for_point_in_time_recovery = var.db_system_db_home_database_time_stamp_for_point_in_time_recovery
#      vault_id                              = oci_kms_vault.test_vault.id
#    }
#
#    #Optional
#    database_software_image_id  = oci_database_database_software_image.test_database_software_image.id
#    db_unique_name              = var.db_unique_name
#    db_version                  = var.db_system_db_home_db_version
#    defined_tags                = var.db_system_db_home_defined_tags
#    display_name                = var.db_system_db_home_display_name
#    freeform_tags               = var.db_system_db_home_freeform_tags
#    is_unified_auditing_enabled = var.db_system_db_home_is_unified_auditing_enabled
#  }
#  hostname        = var.db_system_hostname
#  shape           = var.db_system_shape
#  ssh_public_keys = var.db_system_ssh_public_keys
#  subnet_id       = oci_core_subnet.test_subnet.id
#
#  #Optional
#  backup_network_nsg_ids = var.db_system_backup_network_nsg_ids
#  backup_subnet_id       = oci_core_subnet.test_subnet.id
#  cluster_name           = var.db_system_cluster_name
#  cpu_core_count         = var.db_system_cpu_core_count
#  data_collection_options {
#
#    #Optional
#    is_diagnostics_events_enabled = var.db_system_data_collection_options_is_diagnostics_events_enabled
#    is_health_monitoring_enabled  = var.db_system_data_collection_options_is_health_monitoring_enabled
#    is_incident_logs_enabled      = var.db_system_data_collection_options_is_incident_logs_enabled
#  }
#  data_storage_percentage = var.db_system_data_storage_percentage
#  data_storage_size_in_gb = var.db_system_data_storage_size_in_gb
#  database_edition        = var.db_system_database_edition
#  db_system_options {
#
#    #Optional
#    storage_management = var.db_system_db_system_options_storage_management
#  }
#  defined_tags       = var.db_system_defined_tags
#  disk_redundancy    = var.db_system_disk_redundancy
#  display_name       = var.db_system_display_name
#  domain             = var.db_system_domain
#  fault_domains      = var.db_system_fault_domains
#  freeform_tags      = { "Department" = "Finance" }
#  kms_key_id         = oci_kms_key.test_key.id
#  kms_key_version_id = oci_kms_key_version.test_key_version.id
#  license_model      = var.db_system_license_model
#  maintenance_window_details {
#
#    #Optional
#    custom_action_timeout_in_mins = var.db_system_maintenance_window_details_custom_action_timeout_in_mins
#    days_of_week {
#
#      #Optional
#      name = var.db_system_maintenance_window_details_days_of_week_name
#    }
#    hours_of_day                     = var.db_system_maintenance_window_details_hours_of_day
#    is_custom_action_timeout_enabled = var.db_system_maintenance_window_details_is_custom_action_timeout_enabled
#    is_monthly_patching_enabled      = var.db_system_maintenance_window_details_is_monthly_patching_enabled
#    lead_time_in_weeks               = var.db_system_maintenance_window_details_lead_time_in_weeks
#    months {
#
#      #Optional
#      name = var.db_system_maintenance_window_details_months_name
#    }
#    patching_mode  = var.db_system_maintenance_window_details_patching_mode
#    preference     = var.db_system_maintenance_window_details_preference
#    weeks_of_month = var.db_system_maintenance_window_details_weeks_of_month
#  }
#  node_count                      = var.db_system_node_count
#  nsg_ids                         = var.db_system_nsg_ids
#  private_ip                      = var.db_system_private_ip
#  security_attributes             = var.db_system_security_attributes
#  private_ip_v6                   = var.db_system_private_ip_v6
#  source                          = var.db_system_source
#  source_db_system_id             = oci_database_db_system.test_db_system.id
#  sparse_diskgroup                = var.db_system_sparse_diskgroup
#  storage_volume_performance_mode = var.db_system_storage_volume_performance_mode
#  time_zone                       = var.db_system_time_zone
#}
