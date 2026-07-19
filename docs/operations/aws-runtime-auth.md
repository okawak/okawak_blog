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
sudo systemctl is-active okawak_blog nginx
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
