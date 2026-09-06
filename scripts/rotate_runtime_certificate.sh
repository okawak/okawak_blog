#!/usr/bin/env bash

set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ssh_target="${1:-${OKAWAK_BLOG_VPS_SSH_TARGET:-}}"
pki_dir="${XDG_DATA_HOME:-${HOME}/.local/share}/okawak-blog-pki"
artifact_bucket="okawak-blog-resources-bucket"
certificate_days=90
certificate_subject="/O=okawak/CN=okawak-blog-vps"
certificate_validity_seconds=$((certificate_days * 86400))
issuer_host="${OKAWAK_BLOG_CERTIFICATE_ISSUER_HOST:-}"
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
Run this task only on the registered management host; VPS activation runs over SSH.
SSH_TARGET overrides OKAWAK_BLOG_VPS_SSH_TARGET; one of them must be set.

Required environment variable (configure once in mise.local.toml):
  OKAWAK_BLOG_CERTIFICATE_ISSUER_HOST
                                Literal hostname of the management host
  OKAWAK_BLOG_VPS_SSH_TARGET    SSH target (unless supplied as an argument)

SSH config supplies the port, hostname, user and authentication settings.
CA files: ${XDG_DATA_HOME:-${HOME}/.local/share}/okawak-blog-pki.
Certificate: 90 days, Subject CN okawak-blog-vps.
S3 artifact bucket: okawak-blog-resources-bucket.
EOF
}

fail() {
  echo "certificate-rotation: $*" >&2
  exit 1
}

check_ca_validity() {
  openssl x509 -in "$ca_certificate" -checkend "$certificate_validity_seconds" -noout \
    || fail "CA certificate cannot cover the requested $certificate_days-day validity; renew the CA and AWS Trust Anchor before retrying"
}

cleanup_remote_upload() {
  local status="$?"

  trap - EXIT
  trap '' HUP INT TERM
  if [[ "$remote_upload_created" == true ]]; then
    # The remote paths are intentionally expanded locally from the validated stamp.
    # shellcheck disable=SC2029
    ssh "$ssh_target" \
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

[[ -n "$ssh_target" ]] \
  || fail "set OKAWAK_BLOG_VPS_SSH_TARGET in mise.local.toml [env], or pass an SSH target"
[[ "$ssh_target" =~ ^[A-Za-z0-9._@:-]+$ ]] \
  || fail "SSH target contains unsupported characters: $ssh_target"
[[ "$ssh_target" != -* ]] || fail "SSH target must not start with '-'"
# Fail closed before accessing the CA or contacting the VPS, even if CA files were copied.
[[ "$issuer_host" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] \
  || fail "management-host guard: set OKAWAK_BLOG_CERTIFICATE_ISSUER_HOST to the management host's literal hostname in mise.local.toml"
current_hostname="$(hostname)" \
  || fail "management-host guard: could not determine the current hostname"
[[ "$current_hostname" == "$issuer_host" ]] \
  || fail "management-host guard: run this task on '$issuer_host', not '$current_hostname'; VPS activation is performed automatically over SSH"

for command_name in openssl scp ssh; do
  command -v "$command_name" >/dev/null \
    || fail "required command is missing: $command_name"
done

for required_file in "$ca_certificate" "$ca_private_key" "$ca_serial"; do
  [[ -f "$required_file" ]] || fail "required CA file is missing: $required_file"
done

check_ca_validity
openssl verify -check_ss_sig -CAfile "$ca_certificate" "$ca_certificate" \
  || fail "CA certificate is not currently valid"

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

# Issuance may have waited for a key passphrase; ensure the CA still covers the leaf.
check_ca_validity
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
ssh "$ssh_target" "umask 077; mkdir '$remote_upload_dir'"
remote_upload_created=true
scp \
  "$client_certificate" \
  "$client_private_key" \
  "$repo_root/scripts/activate_runtime_certificate.sh" \
  "$ssh_target:$remote_upload_dir/"

ssh -tt "$ssh_target" \
  "ROTATION_STAMP='$stamp' ARTIFACT_BUCKET='$artifact_bucket' UPLOAD_DIR='$remote_upload_dir' bash '$remote_script'"

remote_upload_created=false
rmdir "$rotation_lock"
rotation_lock_created=false
trap - EXIT HUP INT TERM

echo "certificate-rotation: completed successfully"
echo "certificate-rotation: retained the new certificate and private key in $pki_dir"
