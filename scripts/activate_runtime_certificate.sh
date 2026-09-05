#!/usr/bin/env bash

set -Eeuo pipefail

rotation_stamp="${ROTATION_STAMP:?ROTATION_STAMP is required}"
upload_dir="${UPLOAD_DIR:-/tmp/okawak-blog-certificate-$rotation_stamp}"
certificate_dir="${CERTIFICATE_DIR:-/etc/okawak_blog/aws}"
aws_config="${AWS_CONFIG_PATH:-$certificate_dir/config}"
artifact_bucket="${ARTIFACT_BUCKET:-okawak-blog-resources-bucket}"
service_name="${SERVICE_NAME:-okawak_blog}"
service_user="${SERVICE_USER:-okawak}"
service_group="${SERVICE_GROUP:-okawak}"
aws_profile="${AWS_PROFILE_NAME:-blog-s3}"
aws_region="${AWS_REGION_NAME:-ap-northeast-1}"
expected_role_name="${EXPECTED_ROLE_NAME:-okawak-blog-runtime-role}"
local_health_url="${LOCAL_HEALTH_URL:-http://127.0.0.1:8008/api/health}"
local_ready_url="${LOCAL_READY_URL:-http://127.0.0.1:8008/api/ready}"

source_certificate="$upload_dir/vps-client-cert-$rotation_stamp.pem"
source_private_key="$upload_dir/vps-client-key-$rotation_stamp.pem"
candidate_certificate="$certificate_dir/client-cert-$rotation_stamp.pem"
candidate_private_key="$certificate_dir/client-key-$rotation_stamp.pem"
candidate_config="$certificate_dir/config-$rotation_stamp"
active_certificate="$certificate_dir/client-cert.pem"
active_private_key="$certificate_dir/client-key.pem"
rollback_certificate="$certificate_dir/client-cert.pem.rollback-$rotation_stamp"
rollback_private_key="$certificate_dir/client-key.pem.rollback-$rotation_stamp"

service_was_active=false
activation_started=false

fail() {
  echo "certificate-activation: $*" >&2
  return 1
}

cleanup_uploaded_files() {
  rm -f \
    "$source_certificate" \
    "$source_private_key" \
    "$upload_dir/activate_runtime_certificate.sh"
  rmdir "$upload_dir" 2>/dev/null || true
}

cleanup_candidates() {
  sudo rm -f \
    "$candidate_certificate" \
    "$candidate_private_key" \
    "$candidate_config"
}

restore_previous_certificate() {
  local status="$1"
  local line="$2"
  local recovery_succeeded=true

  trap - ERR
  # A second Ctrl-C or SSH hangup must not interrupt the recovery commands.
  trap '' HUP INT TERM
  set +e
  echo "certificate-activation: failed at line $line; restoring the previous certificate" >&2

  if [[ "$activation_started" == true ]]; then
    if sudo systemctl stop "$service_name.service" \
      && sudo install -o root -g "$service_group" -m 0644 \
        "$rollback_certificate" "$active_certificate" \
      && sudo install -o root -g "$service_group" -m 0640 \
        "$rollback_private_key" "$active_private_key" \
      && sudo cmp -s "$rollback_certificate" "$active_certificate" \
      && sudo cmp -s "$rollback_private_key" "$active_private_key"; then
      if [[ "$service_was_active" == true ]] \
        && ! sudo systemctl start "$service_name.service"; then
        recovery_succeeded=false
      fi
    else
      recovery_succeeded=false
    fi
  fi

  if [[ "$recovery_succeeded" == true ]]; then
    sudo rm -f "$rollback_certificate" "$rollback_private_key"
  else
    echo "certificate-activation: automatic recovery failed; rollback files retained for manual recovery:" >&2
    printf '  %s\n' "$rollback_certificate" "$rollback_private_key" >&2
    echo "certificate-activation: restore and verify this pair before starting $service_name.service" >&2
  fi
  cleanup_candidates
  cleanup_uploaded_files
  exit "$status"
}

run_aws_check() {
  local config_path="$1"
  local caller_arn

  caller_arn="$(
    # Let the failed assignment trigger recovery only in the parent shell.
    trap - ERR
    sudo -u "$service_user" env \
      HOME=/nonexistent \
      AWS_PROFILE="$aws_profile" \
      AWS_REGION="$aws_region" \
      AWS_CONFIG_FILE="$config_path" \
      AWS_SHARED_CREDENTIALS_FILE=/dev/null \
      AWS_EC2_METADATA_DISABLED=true \
      AWS_PAGER="" \
      aws sts get-caller-identity \
        --query Arn \
        --output text
  )"
  [[ "$caller_arn" == *":assumed-role/$expected_role_name/"* ]] \
    || fail "unexpected caller identity: $caller_arn"
  echo "$caller_arn"

  sudo -u "$service_user" env \
    HOME=/nonexistent \
    AWS_PROFILE="$aws_profile" \
    AWS_REGION="$aws_region" \
    AWS_CONFIG_FILE="$config_path" \
    AWS_SHARED_CREDENTIALS_FILE=/dev/null \
    AWS_EC2_METADATA_DISABLED=true \
    AWS_PAGER="" \
    aws s3api head-object \
      --bucket "$artifact_bucket" \
      --key current.json \
      --query ContentLength \
      --output text
}

wait_for_service() {
  local attempt
  local probe_attempts=15
  local status=1

  # Type=simple does not wait for listen/readiness, including RestartSec delays.
  for ((attempt = 1; attempt <= probe_attempts; attempt += 1)); do
    if curl --fail --silent --show-error --output /dev/null \
      --connect-timeout 2 --max-time 5 "$local_health_url" \
      && curl --fail --silent --show-error --output /dev/null \
        --connect-timeout 2 --max-time 5 "$local_ready_url"; then
      return 0
    else
      status=$?
    fi
    if ((attempt < probe_attempts)); then
      sleep 1
    fi
  done

  echo "certificate-activation: health/readiness checks did not pass after $probe_attempts attempts" >&2
  return "$status"
}

[[ "$rotation_stamp" =~ ^[0-9]{8}T[0-9]{6}Z$ ]] \
  || fail "ROTATION_STAMP has an invalid format"
[[ "$upload_dir" == "/tmp/okawak-blog-certificate-$rotation_stamp" \
  || -n "${ALLOW_CUSTOM_UPLOAD_DIR:-}" ]] \
  || fail "UPLOAD_DIR is outside the expected rotation directory"

for command_name in aws cmp curl openssl sleep sudo systemctl; do
  command -v "$command_name" >/dev/null \
    || fail "required command is missing: $command_name"
done

[[ -f "$source_certificate" ]] \
  || fail "uploaded certificate is missing: $source_certificate"
[[ -f "$source_private_key" ]] \
  || fail "uploaded private key is missing: $source_private_key"
[[ -f "$aws_config" ]] || fail "AWS config is missing: $aws_config"
[[ -f "$active_certificate" ]] \
  || fail "active certificate is missing: $active_certificate"
[[ -f "$active_private_key" ]] \
  || fail "active private key is missing: $active_private_key"

sudo -v
trap 'restore_previous_certificate $? $LINENO' ERR
trap 'restore_previous_certificate 129 $LINENO' HUP
trap 'restore_previous_certificate 130 $LINENO' INT
trap 'restore_previous_certificate 143 $LINENO' TERM

private_key_digest="$(
  # Keep recovery in the parent while pipefail preserves validation failures.
  trap - ERR
  openssl pkey -in "$source_private_key" -pubout -outform DER 2>/dev/null |
    openssl dgst -sha256
)"
certificate_digest="$(
  trap - ERR
  openssl x509 -in "$source_certificate" -pubkey -noout |
    openssl pkey -pubin -outform DER 2>/dev/null |
    openssl dgst -sha256
)"
[[ "$private_key_digest" == "$certificate_digest" ]] \
  || fail "uploaded certificate does not match its private key"
openssl x509 -checkend 604800 -noout -in "$source_certificate" \
  || fail "uploaded certificate is not valid for at least seven more days"

sudo install -o root -g "$service_group" -m 0644 \
  "$source_certificate" "$candidate_certificate"
sudo install -o root -g "$service_group" -m 0640 \
  "$source_private_key" "$candidate_private_key"
sudo cp -p "$aws_config" "$candidate_config"
sudo sed -E -i \
  -e "s#--certificate[[:space:]]+[^[:space:]]+#--certificate $candidate_certificate#" \
  -e "s#--private-key[[:space:]]+[^[:space:]]+#--private-key $candidate_private_key#" \
  "$candidate_config"
sudo chown root:"$service_group" "$candidate_config"
sudo chmod 0640 "$candidate_config"

sudo grep -F -- "--certificate $candidate_certificate" "$candidate_config" >/dev/null \
  || fail "credential_process certificate path was not replaced"
sudo grep -F -- "--private-key $candidate_private_key" "$candidate_config" >/dev/null \
  || fail "credential_process private key path was not replaced"

echo "certificate-activation: validating the staged certificate with IAM Roles Anywhere"
run_aws_check "$candidate_config"

if sudo systemctl is-active --quiet "$service_name.service"; then
  service_was_active=true
fi

sudo cp -p "$active_certificate" "$rollback_certificate"
sudo cp -p "$active_private_key" "$rollback_private_key"
activation_started=true

if [[ "$service_was_active" == true ]]; then
  sudo systemctl stop "$service_name.service"
fi
sudo install -o root -g "$service_group" -m 0644 \
  "$source_certificate" "$active_certificate"
sudo install -o root -g "$service_group" -m 0640 \
  "$source_private_key" "$active_private_key"
if [[ "$service_was_active" == true ]]; then
  sudo systemctl start "$service_name.service"
fi

echo "certificate-activation: validating the active certificate"
run_aws_check "$aws_config"

if [[ "$service_was_active" == true ]]; then
  wait_for_service
fi

trap - ERR HUP INT TERM
sudo rm -f "$rollback_certificate" "$rollback_private_key"
cleanup_candidates
cleanup_uploaded_files

openssl x509 \
  -in "$active_certificate" \
  -noout -subject -issuer -serial -dates
echo "certificate-activation: certificate activated and checks passed"
