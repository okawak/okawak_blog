# AWS runtime認証

## 現行構成

productionのS3 artifact readerは、IAM Roles AnywhereのX.509 identityを`aws_signing_helper`の`credential_process`へ渡し、期限付きrole credentialを取得します。long-livedなIAM user access key、Secrets Manager rotation、credential file、更新cronは使用しません。

VPSで維持するfile:

```text
/usr/local/bin/aws_signing_helper
/etc/okawak_blog/aws/config
/etc/okawak_blog/aws/client-cert.pem
/etc/okawak_blog/aws/client-key.pem
```

管理端末ではCA private key、CA certificate、serial fileをrepositoryや同期対象外の暗号化された保管先で維持します。CA private keyはclient certificate更新に必要なので削除しません。

```text
${HOME}/.local/share/okawak-blog-pki/ca-key.pem
${HOME}/.local/share/okawak-blog-pki/ca-cert.pem
${HOME}/.local/share/okawak-blog-pki/ca-cert.srl
```

systemd unitは次だけをAWS SDKへ渡します。

```text
AWS_PROFILE=blog-s3
AWS_REGION=ap-northeast-1
AWS_CONFIG_FILE=/etc/okawak_blog/aws/config
AWS_EC2_METADATA_DISABLED=true
```

`AWS_SHARED_CREDENTIALS_FILE`は指定しません。serviceは`ProtectHome=true`を維持し、home配下のAWS profileへfallbackしません。

## 初期構築

### 1. 管理端末でCAを作成する

CA private keyはrepositoryや同期directoryの外に置き、暗号化backupを作成します。client certificate更新で再利用するため削除しません。

```bash
export PKI_DIR="${HOME}/.local/share/okawak-blog-pki"
install -d -m 0700 "${PKI_DIR}"
cd "${PKI_DIR}"
umask 077

openssl genpkey \
  -algorithm EC \
  -pkeyopt ec_paramgen_curve:P-256 \
  -out ca-key.pem

openssl req \
  -x509 \
  -new \
  -sha256 \
  -key ca-key.pem \
  -out ca-cert.pem \
  -days 3650 \
  -subj '/O=okawak/CN=okawak-blog-runtime-ca' \
  -addext 'basicConstraints=critical,CA:TRUE' \
  -addext 'keyUsage=critical,keyCertSign,cRLSign' \
  -addext 'subjectKeyIdentifier=hash'

openssl x509 \
  -in ca-cert.pem \
  -noout -subject -issuer -serial -dates
```

### 2. 管理端末でAWS Terraformを適用する

`terraform/aws/terraform.tfvars`へAWS region、account ID、bucket名、公開CA certificate pathなどを設定します。CA private keyはTerraformへ渡しません。

```bash
cd '<REPOSITORY>/terraform/aws'
terraform init
terraform validate
terraform fmt -check -recursive
terraform plan -out=plan_deploy
terraform show -no-color plan_deploy
terraform apply plan_deploy
terraform plan
```

次のoutputを安全な作業メモへ記録します。ARNはsecretではありませんが、誤設定防止のためVPS configへ転記後に照合します。

```bash
terraform output roles_anywhere_trust_anchor_arn
terraform output roles_anywhere_profile_arn
terraform output roles_anywhere_role_arn
```

### 3. 管理端末でVPS用certificateを作成する

更新手順と同じ方法で、Subject CNがTerraformの`roles_anywhere_certificate_subject_cn`と一致するend-entity certificateを作成します。初回も日付付きfile名を使い、chain、期限、key pairを検証します。

```bash
export CERT_STAMP="$(date +%Y%m%d)"
cd "${PKI_DIR}"

openssl genpkey \
  -algorithm EC \
  -pkeyopt ec_paramgen_curve:P-256 \
  -out "vps-client-key-${CERT_STAMP}.pem"

openssl req \
  -new \
  -key "vps-client-key-${CERT_STAMP}.pem" \
  -out "vps-client-${CERT_STAMP}.csr" \
  -subj '/O=okawak/CN=okawak-blog-vps'

openssl x509 \
  -req \
  -in "vps-client-${CERT_STAMP}.csr" \
  -CA ca-cert.pem \
  -CAkey ca-key.pem \
  -CAcreateserial \
  -out "vps-client-cert-${CERT_STAMP}.pem" \
  -days 90 \
  -sha256 \
  -extfile <(printf '%s\n' \
    'basicConstraints=critical,CA:FALSE' \
    'keyUsage=critical,digitalSignature' \
    'extendedKeyUsage=clientAuth' \
    'subjectKeyIdentifier=hash' \
    'authorityKeyIdentifier=keyid,issuer')

openssl verify \
  -CAfile ca-cert.pem \
  "vps-client-cert-${CERT_STAMP}.pem"

openssl x509 \
  -in "vps-client-cert-${CERT_STAMP}.pem" \
  -noout -subject -issuer -serial -dates

openssl pkey \
  -in "vps-client-key-${CERT_STAMP}.pem" \
  -pubout -outform DER | \
  openssl dgst -sha256

openssl x509 \
  -in "vps-client-cert-${CERT_STAMP}.pem" \
  -pubkey -noout | \
  openssl pkey -pubin -outform DER | \
  openssl dgst -sha256
```

最後の2つのdigestが一致することを確認します。

### 4. VPSへcredential helperを配置する

管理端末で[AWS公式のcredential helper配布表](https://docs.aws.amazon.com/rolesanywhere/latest/userguide/credential-helper.html)からLinux x86-64用URLとSHA-256を取得します。download後にchecksumを検証してからVPSへ転送し、root所有で配置します。

```bash
export AWS_SIGNING_HELPER_URL='<OFFICIAL_LINUX_X86_64_URL>'
export AWS_SIGNING_HELPER_SHA256='<OFFICIAL_SHA256>'

curl --fail --location \
  --output /tmp/aws_signing_helper \
  "${AWS_SIGNING_HELPER_URL}"
printf '%s  %s\n' \
  "${AWS_SIGNING_HELPER_SHA256}" \
  /tmp/aws_signing_helper | shasum -a 256 -c -

scp -P 60022 \
  /tmp/aws_signing_helper \
  "${PKI_DIR}/vps-client-cert-${CERT_STAMP}.pem" \
  "${PKI_DIR}/vps-client-key-${CERT_STAMP}.pem" \
  '<USER>@<RESERVED_PUBLIC_IP>:/tmp/'
```

VPSでroot管理pathへ配置します。

```bash
export CERT_STAMP='<作成時のYYYYMMDD>'

sudo install -o root -g root -m 0755 \
  /tmp/aws_signing_helper \
  /usr/local/bin/aws_signing_helper

sudo install -d -o root -g okawak -m 0750 \
  /etc/okawak_blog/aws

sudo install -o root -g okawak -m 0644 \
  "/tmp/vps-client-cert-${CERT_STAMP}.pem" \
  /etc/okawak_blog/aws/client-cert.pem

sudo install -o root -g okawak -m 0640 \
  "/tmp/vps-client-key-${CERT_STAMP}.pem" \
  /etc/okawak_blog/aws/client-key.pem
```

`sudoedit /etc/okawak_blog/aws/config`で次を作成し、Terraform outputのARNを設定します。

```ini
[profile blog-s3]
region = ap-northeast-1
credential_process = /usr/local/bin/aws_signing_helper credential-process --certificate /etc/okawak_blog/aws/client-cert.pem --private-key /etc/okawak_blog/aws/client-key.pem --trust-anchor-arn <TRUST_ANCHOR_ARN> --profile-arn <PROFILE_ARN> --role-arn <ROLE_ARN>
```

```bash
sudo chown root:okawak /etc/okawak_blog/aws/config
sudo chmod 0640 /etc/okawak_blog/aws/config
sudo -u okawak test -r /etc/okawak_blog/aws/config
sudo -u okawak test -r /etc/okawak_blog/aws/client-cert.pem
sudo -u okawak test -r /etc/okawak_blog/aws/client-key.pem
```

helper単体のcredential JSONは画面やfileへ保存せず、次の「即時検証」でAWS CLIを通して確認します。成功後、VPSの`/tmp`に転送したhelper、certificate、private keyを削除します。

## 即時検証

credential JSONやsecretをterminalまたはfileへ保存せず、service userとしてcaller identityとread-onlyなS3 accessを確認します。

```bash
sudo -u okawak env \
  HOME=/nonexistent \
  AWS_PROFILE=blog-s3 \
  AWS_CONFIG_FILE=/etc/okawak_blog/aws/config \
  AWS_SHARED_CREDENTIALS_FILE=/dev/null \
  AWS_EC2_METADATA_DISABLED=true \
  aws sts get-caller-identity \
    --query Arn \
    --output text
```

返るARNは`assumed-role/okawak-blog-runtime-role/`を含む必要があります。

```bash
sudo -u okawak env \
  HOME=/nonexistent \
  AWS_PROFILE=blog-s3 \
  AWS_CONFIG_FILE=/etc/okawak_blog/aws/config \
  AWS_SHARED_CREDENTIALS_FILE=/dev/null \
  AWS_EC2_METADATA_DISABLED=true \
  aws s3api head-object \
    --bucket okawak-blog-resources-bucket \
    --key current.json
```

```bash
sudo systemctl is-active okawak_blog cloudflared
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
curl --fail -H 'Cache-Control: no-cache' \
  "https://www.okawak.net/api/ready?auth-check=$(date +%s)"
```

`/api/ready`だけでは未cache articleのS3 readを網羅しないため、home、category、未cacheの記事も確認します。

## Certificate期限確認

client certificateは期限切れ前に更新します。少なくとも7日先まで有効か確認します。

```bash
openssl x509 -checkend 604800 -noout \
  -in /etc/okawak_blog/aws/client-cert.pem
```

exit statusが0以外なら7日以内に期限切れです。日次監視と外部通知へ接続し、journal確認だけに依存しません。

```bash
openssl x509 \
  -in /etc/okawak_blog/aws/client-cert.pem \
  -noout -subject -issuer -serial -dates
```

## Client certificate更新

管理端末で既存CAを使用し、更新ごとに新しいclient private keyとcertificateを別名で作成します。Subject CNはTerraformの`roles_anywhere_certificate_subject_cn`と一致させます。

```bash
export PKI_DIR="${HOME}/.local/share/okawak-blog-pki"
export CERT_STAMP="$(date +%Y%m%d)"
cd "${PKI_DIR}"
umask 077

openssl genpkey \
  -algorithm EC \
  -pkeyopt ec_paramgen_curve:P-256 \
  -out "vps-client-key-${CERT_STAMP}.pem"

openssl req \
  -new \
  -key "vps-client-key-${CERT_STAMP}.pem" \
  -out "vps-client-${CERT_STAMP}.csr" \
  -subj '/O=okawak/CN=okawak-blog-vps'

openssl x509 \
  -req \
  -in "vps-client-${CERT_STAMP}.csr" \
  -CA ca-cert.pem \
  -CAkey ca-key.pem \
  -CAserial ca-cert.srl \
  -out "vps-client-cert-${CERT_STAMP}.pem" \
  -days 90 \
  -sha256 \
  -extfile <(printf '%s\n' \
    'basicConstraints=critical,CA:FALSE' \
    'keyUsage=critical,digitalSignature' \
    'extendedKeyUsage=clientAuth' \
    'subjectKeyIdentifier=hash' \
    'authorityKeyIdentifier=keyid,issuer')
```

chain、Subject、期限、key pairを検証します。2つのdigestは一致する必要があります。

```bash
openssl verify \
  -CAfile ca-cert.pem \
  "vps-client-cert-${CERT_STAMP}.pem"

openssl x509 \
  -in "vps-client-cert-${CERT_STAMP}.pem" \
  -noout -subject -issuer -serial -dates

openssl pkey \
  -in "vps-client-key-${CERT_STAMP}.pem" \
  -pubout -outform DER | \
  openssl dgst -sha256

openssl x509 \
  -in "vps-client-cert-${CERT_STAMP}.pem" \
  -pubkey -noout | \
  openssl pkey -pubin -outform DER | \
  openssl dgst -sha256
```

VPSへ一時名で転送し、root管理pathへ配置します。private keyの転送先、command history、backup先を第三者が読めないようにします。既存fileは更新確認が終わるまでrootだけが読める場所へ退避します。

配置後はhelper単体検証、service restart、readiness、未cache記事を確認します。成功後に旧client certificateとprivate keyを安全に撤去します。CA private key、CA certificate、serial fileは次回更新のため維持します。

## 障害切り分け

1. certificate期限、Subject CN、private keyとの対応を確認する
2. `aws_signing_helper`をservice userが実行できることを確認する
3. `/etc/okawak_blog/aws/config`のtrust anchor、profile、role ARNを確認する
4. Roles Anywhereのtrust anchorとprofileがenabledであることを確認する
5. caller identity、S3 `head-object`、application readinessの順で境界を切り分ける
6. CloudTrailのRoles Anywhere `CreateSession`とservice journalを確認する

static access keyやcredential fileを緊急fallbackとして再導入しません。復旧できない場合はIAM Roles Anywhere、certificate、S3 policyの原因を修正します。
