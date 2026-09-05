#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ssh_target="${1:-${OKAWAK_BLOG_VPS_SSH_TARGET:-oci}}"
ssh_port="${OKAWAK_BLOG_VPS_SSH_PORT:-}"
pki_dir="${OKAWAK_BLOG_PKI_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/okawak-blog-pki}"
artifact_bucket="${OKAWAK_BLOG_ARTIFACT_BUCKET:-okawak-blog-resources-bucket}"
certificate_days="${OKAWAK_BLOG_CERTIFICATE_DAYS:-90}"
certificate_subject_cn="${OKAWAK_BLOG_CERTIFICATE_SUBJECT_CN-okawak-blog-vps}"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"

ca_certificate="$pki_dir/ca-cert.pem"
ca_private_key="$pki_dir/ca-key.pem"
ca_serial="$pki_dir/ca-cert.srl"
client_private_key="$pki_dir/vps-client-key-$stamp.pem"
client_request="$pki_dir/vps-client-$stamp.csr"
client_certificate="$pki_dir/vps-client-cert-$stamp.pem"
remote_upload_dir="/tmp/okawak-blog-certificate-$stamp"
remote_script="$remote_upload_dir/activate_runtime_certificate.sh"
rotation_lock="$pki_dir/.certificate-rotation.lock"
remote_upload_created=false
rotation_lock_created=false

usage() {
  cat <<'EOF'
Usage: mise run rotate-runtime-certificate [SSH_TARGET]

Rotate the IAM Roles Anywhere client certificate used by the production VPS.
SSH_TARGET defaults to OKAWAK_BLOG_VPS_SSH_TARGET, or to the SSH alias "oci".

Optional environment variables:
  OKAWAK_BLOG_VPS_SSH_PORT       SSH port (1-65535; default: SSH config)
  OKAWAK_BLOG_PKI_DIR             CA files directory
  OKAWAK_BLOG_ARTIFACT_BUCKET     S3 artifact bucket
  OKAWAK_BLOG_CERTIFICATE_DAYS    New certificate validity in days (minimum: 8; default: 90)
  OKAWAK_BLOG_CERTIFICATE_SUBJECT_CN
                                Subject CN matching Terraform's
                                roles_anywhere_certificate_subject_cn
                                (default: okawak-blog-vps)
EOF
}

fail() {
  echo "certificate-rotation: $*" >&2
  exit 1
}

run_ssh() {
  if [[ -n "$ssh_port" ]]; then
    ssh -p "$ssh_port" "$@"
  else
    ssh "$@"
  fi
}

run_scp() {
  if [[ -n "$ssh_port" ]]; then
    scp -P "$ssh_port" "$@"
  else
    scp "$@"
  fi
}

cleanup_remote_upload() {
  local status="$?"

  trap - EXIT
  trap '' HUP INT TERM
  if [[ "$remote_upload_created" == true ]]; then
    # The remote paths are intentionally expanded locally from the validated stamp.
    # shellcheck disable=SC2029
    run_ssh "$ssh_target" \
      "rm -f '$remote_upload_dir/vps-client-cert-$stamp.pem' '$remote_upload_dir/vps-client-key-$stamp.pem' '$remote_script'; rmdir '$remote_upload_dir' 2>/dev/null || true" \
      >/dev/null 2>&1 || true
  fi
  if [[ "$rotation_lock_created" == true ]]; then
    rmdir "$rotation_lock" 2>/dev/null || true
  fi
  exit "$status"
}

if [[ "$ssh_target" == "--help" || "$ssh_target" == "-h" ]]; then
  usage
  exit 0
fi

[[ "$ssh_target" =~ ^[A-Za-z0-9._@:-]+$ ]] \
  || fail "SSH target contains unsupported characters: $ssh_target"
[[ "$ssh_target" != -* ]] || fail "SSH target must not start with '-'"
if [[ -n "$ssh_port" ]]; then
  if [[ ! "$ssh_port" =~ ^[1-9][0-9]{0,4}$ ]] || ((ssh_port > 65535)); then
    fail "OKAWAK_BLOG_VPS_SSH_PORT must be an integer between 1 and 65535"
  fi
fi
[[ "$artifact_bucket" =~ ^[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]$ ]] \
  || fail "invalid S3 bucket name: $artifact_bucket"
# The VPS requires seven full days remaining, so reject shorter terms before issuance.
[[ "$certificate_days" =~ ^([8-9]|[1-9][0-9]+)$ ]] \
  || fail "OKAWAK_BLOG_CERTIFICATE_DAYS must be an integer of at least 8"
[[ -n "$certificate_subject_cn" && ! "$certificate_subject_cn" =~ [[:cntrl:]] ]] \
  || fail "OKAWAK_BLOG_CERTIFICATE_SUBJECT_CN must be non-empty and contain no control characters"
# Escape OpenSSL's subject separators so the configured value remains a single CN.
escaped_subject_cn="${certificate_subject_cn//\\/\\\\}"
escaped_subject_cn="${escaped_subject_cn//\//\\/}"
escaped_subject_cn="${escaped_subject_cn//+/\\+}"
certificate_subject="/O=okawak/CN=$escaped_subject_cn"

for command_name in openssl scp ssh; do
  command -v "$command_name" >/dev/null \
    || fail "required command is missing: $command_name"
done

for required_file in "$ca_certificate" "$ca_private_key" "$ca_serial"; do
  [[ -f "$required_file" ]] || fail "required CA file is missing: $required_file"
done

for output_file in "$client_private_key" "$client_request" "$client_certificate"; do
  [[ ! -e "$output_file" ]] || fail "refusing to overwrite existing file: $output_file"
done

mkdir "$rotation_lock" \
  || fail "another certificate rotation may be running: $rotation_lock"
rotation_lock_created=true
trap cleanup_remote_upload EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
umask 077

echo "certificate-rotation: generating a new $certificate_days-day client certificate"
openssl genpkey \
  -algorithm EC \
  -pkeyopt ec_paramgen_curve:P-256 \
  -out "$client_private_key"

openssl req \
  -new \
  -utf8 \
  -key "$client_private_key" \
  -out "$client_request" \
  -subj "$certificate_subject"

openssl x509 \
  -req \
  -in "$client_request" \
  -CA "$ca_certificate" \
  -CAkey "$ca_private_key" \
  -CAserial "$ca_serial" \
  -out "$client_certificate" \
  -days "$certificate_days" \
  -sha256 \
  -extfile <(printf '%s\n' \
    'basicConstraints=critical,CA:FALSE' \
    'keyUsage=critical,digitalSignature' \
    'extendedKeyUsage=clientAuth' \
    'subjectKeyIdentifier=hash' \
    'authorityKeyIdentifier=keyid,issuer')

openssl verify \
  -purpose sslclient \
  -CAfile "$ca_certificate" \
  "$client_certificate"

private_key_digest="$(
  openssl pkey -in "$client_private_key" -pubout -outform DER 2>/dev/null |
    openssl dgst -sha256
)"
certificate_digest="$(
  openssl x509 -in "$client_certificate" -pubkey -noout |
    openssl pkey -pubin -outform DER 2>/dev/null |
    openssl dgst -sha256
)"
[[ "$private_key_digest" == "$certificate_digest" ]] \
  || fail "generated certificate does not match its private key"

openssl x509 \
  -in "$client_certificate" \
  -noout -subject -issuer -serial -dates

echo "certificate-rotation: staging files on $ssh_target"
# The remote path is intentionally expanded locally from the validated stamp.
# shellcheck disable=SC2029
run_ssh "$ssh_target" "umask 077; mkdir '$remote_upload_dir'"
remote_upload_created=true
run_scp \
  "$client_certificate" \
  "$client_private_key" \
  "$repo_root/scripts/activate_runtime_certificate.sh" \
  "$ssh_target:$remote_upload_dir/"

run_ssh -tt "$ssh_target" \
  "ROTATION_STAMP='$stamp' ARTIFACT_BUCKET='$artifact_bucket' UPLOAD_DIR='$remote_upload_dir' bash '$remote_script'"

remote_upload_created=false
rmdir "$rotation_lock"
rotation_lock_created=false
trap - EXIT HUP INT TERM

echo "certificate-rotation: completed successfully"
echo "certificate-rotation: retained the new certificate and private key in $pki_dir"
