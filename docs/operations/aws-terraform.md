# AWS Terraform

## 管理範囲

`terraform/aws`は次を管理します。

- blog artifact用S3 bucket
- GitHub Actions用OIDC providerとIAM role
- Obsidian image uploader用S3、CloudFront、IAM user
- IAM Roles Anywhere trust anchor、profile、runtime role
- Terraform state暗号化用KMS keyとalias

Terraform state backendのS3 bucket `okawak-terraform-state`自体はconfigurationに含まれず、管理端末からbootstrapします。DNSとCloudflare TunnelはCloudflare Dashboardで管理します。

## 管理端末の前提

Terraformを実行するAWS管理profileを設定し、対象accountを確認します。

```bash
aws sts get-caller-identity
aws configure get region
```

誤accountでapplyしないよう、返されたaccount IDを`terraform.tfvars`と照合します。

## Backendの初回bootstrap

既存backendがある場合はこの節を実行しません。完全な初回だけ、S3 bucketとKMS keyを先に作ります。

```bash
export AWS_REGION='ap-northeast-1'
export TF_STATE_BUCKET='okawak-terraform-state'

aws s3api create-bucket \
  --bucket "${TF_STATE_BUCKET}" \
  --region "${AWS_REGION}" \
  --create-bucket-configuration \
    "LocationConstraint=${AWS_REGION}"

aws s3api put-public-access-block \
  --bucket "${TF_STATE_BUCKET}" \
  --public-access-block-configuration \
    'BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true'

aws s3api put-bucket-versioning \
  --bucket "${TF_STATE_BUCKET}" \
  --versioning-configuration Status=Enabled

export TF_STATE_KEY_ID="$(aws kms create-key \
  --description 'Terraform state encryption key' \
  --query 'KeyMetadata.KeyId' \
  --output text)"

aws kms enable-key-rotation --key-id "${TF_STATE_KEY_ID}"
aws kms create-alias \
  --alias-name alias/terraform-state-key \
  --target-key-id "${TF_STATE_KEY_ID}"

aws s3api put-bucket-encryption \
  --bucket "${TF_STATE_BUCKET}" \
  --server-side-encryption-configuration \
  "{\"Rules\":[{\"ApplyServerSideEncryptionByDefault\":{\"SSEAlgorithm\":\"aws:kms\",\"KMSMasterKeyID\":\"${TF_STATE_KEY_ID}\"},\"BucketKeyEnabled\":true}]}"
```

backendを初期化したら、手動作成したKMS keyとaliasをconfigurationへimportします。importせずplan/applyすると、同名aliasの作成競合になります。

```bash
cd '<REPOSITORY>/terraform/aws'
terraform init
terraform import aws_kms_key.tf_state "${TF_STATE_KEY_ID}"
terraform import aws_kms_alias.tf_state_alias alias/terraform-state-key
terraform plan
```

backend bucketのversioning、public access block、default encryption、KMS aliasを確認します。

```bash
aws s3api get-bucket-versioning --bucket "${TF_STATE_BUCKET}"
aws s3api get-public-access-block --bucket "${TF_STATE_BUCKET}"
aws s3api get-bucket-encryption --bucket "${TF_STATE_BUCKET}"
aws kms describe-key --key-id alias/terraform-state-key
```

## 初回apply

`terraform/aws/terraform.tfvars`へ次を設定します。

- `aws_region`
- `blog_bucket_name`
- `image_bucket_name`
- `image_uploader_user_name`
- `roles_anywhere_ca_certificate_path`
- `roles_anywhere_certificate_subject_cn`

CA作成は[AWS runtime認証](./aws-runtime-auth.md#初期構築)を参照します。

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

保存済みplanはstate相当の情報を含み得るため、commit、共有、長期保存をしません。apply後の通常planが`No changes`であることを確認します。

## GitHub Actions

artifact upload workflowはGitHub OIDCで`oidc-gh-role`を引き受けます。repositoryのActions secretsへ次を設定します。

- `AWS_REGION`
- `AWS_ACCOUNT_ID`
- `AWS_ROLE_NAME`: `oidc-gh-role`
- `S3_BUCKET`

GitHub App用の`GH_APP_ID`と`GH_APP_PRIVATE_KEY`もprivate Obsidian submoduleのcheckoutに必要です。workflowは`main`から手動実行し、immutable releaseの検証後に`current.json`を切り替えます。

## Image uploader credential

Obsidian image uploaderは専用IAM userのaccess keyを使います。出力をterminal log、shell history、Issue、PRへ残さず、plugin設定とpassword managerへ直接保存します。

```bash
terraform output obsidian_uploader_access_key
terraform output -raw image_uploader_access_key
```

このcredentialはlong-livedな例外です。定期的にlast-usedを確認し、新しいkeyへ切り替えてから古いkeyを無効化・削除します。blog runtimeとGitHub Actionsへ流用しません。

## 定常確認

```bash
terraform plan
terraform state list
aws s3api head-object \
  --bucket '<BLOG_BUCKET>' \
  --key current.json
```

provider更新はlockfile差分、provider changelog、planをreviewし、resource変更と同じapplyへ混ぜません。KMS key、state backend、S3 bucket、IAM roleのdestroyまたはreplaceがあるplanは適用しません。
