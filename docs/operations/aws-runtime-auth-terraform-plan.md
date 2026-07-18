# AWS runtime認証のTerraform変更計画

## この文書の位置付け

IAM Roles Anywhere移行に必要なTerraform変更をrepository ownerが実装するための設計書です。Codexを含む通常のrepository作業では`terraform/`をread-onlyとして扱い、この文書の作成時点ではTerraform fileもstateも変更していません。

実際の変更、plan確認、applyはrepository ownerが行います。一度に旧認証を削除せず、次の2段階に分けます。

1. Phase TF-A: 現行rotationを停止し、IAM Roles Anywhereを既存認証と並行追加
2. Phase TF-B: application切替と安定観測後、旧IAM user / Secret / Lambdaを撤去

VPS側の作業順序は[AWS runtime認証のIAM Roles Anywhere移行](./aws-runtime-auth-migration.md)を参照してください。

HashiCorp公式資料:

- [aws_rolesanywhere_trust_anchor](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/rolesanywhere_trust_anchor)
- [aws_rolesanywhere_profile](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/rolesanywhere_profile)

## 現在のTerraform構成

現在の依存関係は次の通りです。

```text
module.s3
  -> module.iam_reader
       -> IAM user + S3 read policy
       -> module.secret_rotation
            -> initial IAM access key
            -> Secrets Manager secret
            -> rotation Lambda
            -> 30-day rotation schedule
```

主な対象file:

- `terraform/aws/main.tf`
- `terraform/aws/variables.tf`
- `terraform/aws/terraform.tfvars`
- `terraform/aws/outputs.tf`
- `terraform/aws/iam/`
- `terraform/aws/secret/`
- `terraform/aws/providers.tf`
- `terraform/aws/.terraform.lock.hcl`

lockfileのAWS providerはv5.93.0で、IAM Roles Anywhereのtrust anchorとprofile resourceを利用できます。provider upgradeをこのmigrationへ混ぜる必要はありません。`required_providers`へversion制約を追加する場合も、別の意図として差分を確認します。

## 目標構成

```text
module.s3
  -> module.runtime_identity
       -> IAM Roles Anywhere trust anchor
       -> S3 read-only IAM role
       -> IAM Roles Anywhere profile

external/offline CA
  -> public CA certificate: Terraformでtrust anchorへ登録
  -> CA private key: Terraform/state/repositoryへ入れない
  -> VPS end-entity certificate: Terraform管理外
```

AWS Private CAを採用する場合はCA自体もTerraform管理できますが、費用とCA lifecycleが増えます。このblog基盤では、まずexternal CAのpublic certificate bundleをtrust anchorへ登録し、CA private keyとclient private keyをTerraformへ渡さない構成を推奨します。

public CA certificateは秘密情報ではありません。再現性を優先してrepositoryへcommitするか、ownerの管理pathから`file()`で読むかを選べます。ただしCA private keyとVPS client private keyは絶対にcommitせず、Terraform variableやstateにも入れません。

## Phase TF-A: 並行追加

### A-1. 現行rotation scheduleを停止

`terraform/aws/secret/main.tf`から次のresourceだけを外します。

```hcl
resource "aws_secretsmanager_secret_rotation" "rotation" {
  # existing block
}
```

この時点では次を残します。

- IAM reader user
- 現在のIAM access key
- Secrets Manager secretとcurrent version
- rotation Lambdaと実行role
- secret resource policy

最初のplanで、rotation association以外のlegacy resourceがdestroy / replaceされる場合はapplyしません。既に手動で`cancel-rotate-secret`済みなら、この差分はAWS実体とTerraform desired stateを再同期するための変更です。

### A-2. runtime identity moduleを追加

`terraform/aws/runtime_identity/`の追加を推奨します。

```text
terraform/aws/runtime_identity/
  main.tf
  variables.tf
  outputs.tf
```

moduleの責務:

- external CA certificate bundleをtrust anchorへ登録
- Roles Anywhere専用IAM roleを作成
- roleへS3 GetObjectだけを付与
- role trust policyをtrust anchor ARNとcertificate CNで制限
- 対象roleだけを許可するRoles Anywhere profileを作成
- trust anchor / profile / role ARNをoutputする

moduleへclient certificateやprivate keyの生成責務を持たせません。

### A-3. module variables例

```hcl
variable "name" {
  type = string
}

variable "bucket_arn" {
  type = string
}

variable "ca_certificate_pem" {
  description = "Public CA certificate bundle in PEM format"
  type        = string
}

variable "certificate_subject_cn" {
  type = string
}

variable "session_duration_seconds" {
  type    = number
  default = 3600
}
```

### A-4. trust anchor例

```hcl
resource "aws_rolesanywhere_trust_anchor" "this" {
  name    = "${var.name}-trust-anchor"
  enabled = true

  source {
    source_data {
      x509_certificate_data = var.ca_certificate_pem
    }
    source_type = "CERTIFICATE_BUNDLE"
  }
}
```

### A-5. S3 permission policy例

runtimeは`current.json`と選択されたrelease artifactを`GetObject`するだけで、`ListBucket`、write、deleteを必要としません。

```hcl
data "aws_iam_policy_document" "s3_read" {
  statement {
    actions = ["s3:GetObject"]
    resources = [
      "${var.bucket_arn}/current.json",
      "${var.bucket_arn}/releases/*/site/*",
    ]
  }
}
```

legacy bucket root fallbackを移行期間にも許可する場合だけ、`articles/*`、`categories/*`、`metadata/*`、`pages/*`を一時追加します。`current.json`が本番に存在することを確認できれば追加しません。

### A-6. role trust policy例

```hcl
data "aws_iam_policy_document" "assume_role" {
  statement {
    actions = [
      "sts:AssumeRole",
      "sts:TagSession",
      "sts:SetSourceIdentity",
    ]

    principals {
      type        = "Service"
      identifiers = ["rolesanywhere.amazonaws.com"]
    }

    condition {
      test     = "ArnEquals"
      variable = "aws:SourceArn"
      values   = [aws_rolesanywhere_trust_anchor.this.arn]
    }

    condition {
      test     = "StringEquals"
      variable = "aws:PrincipalTag/x509Subject/CN"
      values   = [var.certificate_subject_cn]
    }
  }
}

resource "aws_iam_role" "this" {
  name               = "${var.name}-role"
  assume_role_policy = data.aws_iam_policy_document.assume_role.json
}

resource "aws_iam_policy" "s3_read" {
  name   = "${var.name}-s3-read"
  policy = data.aws_iam_policy_document.s3_read.json
}

resource "aws_iam_role_policy_attachment" "s3_read" {
  role       = aws_iam_role.this.name
  policy_arn = aws_iam_policy.s3_read.arn
}
```

必要に応じてissuer CNや`aws:SourceAccount`もconditionへ追加します。trust anchorだけで制限せず、workload certificateを識別できるsubject / issuer条件を維持します。

### A-7. Roles Anywhere profile例

```hcl
resource "aws_rolesanywhere_profile" "this" {
  name             = "${var.name}-profile"
  enabled          = true
  duration_seconds = var.session_duration_seconds
  role_arns        = [aws_iam_role.this.arn]
}
```

profileへ複数roleを登録しません。role側permissionに加えてsession policyでさらに制限することはできますが、最初のmigrationではpolicy境界を二重化しすぎず、role policyを一次情報にします。

### A-8. outputs例

```hcl
output "trust_anchor_arn" {
  value = aws_rolesanywhere_trust_anchor.this.arn
}

output "profile_arn" {
  value = aws_rolesanywhere_profile.this.arn
}

output "role_arn" {
  value = aws_iam_role.this.arn
}
```

これらのARNは秘密情報ではありませんが、VPSのroot所有AWS configへ設定します。

### A-9. root module接続例

rootの`variables.tf`へ追加します。

```hcl
variable "roles_anywhere_ca_certificate_path" {
  description = "Path to the public external CA certificate bundle"
  type        = string
}

variable "roles_anywhere_certificate_subject_cn" {
  type    = string
  default = "okawak-blog-vps"
}
```

`main.tf`へ既存reader moduleと並行して追加します。

```hcl
module "runtime_identity" {
  source = "./runtime_identity"

  name                   = "okawak-blog-runtime"
  bucket_arn             = module.s3.bucket_arn
  ca_certificate_pem     = file(var.roles_anywhere_ca_certificate_path)
  certificate_subject_cn = var.roles_anywhere_certificate_subject_cn
}
```

`outputs.tf`へtrust anchor、profile、role ARNを追加します。private keyやtemporary credentialをoutputしません。

### A-10. Phase TF-A planの期待差分

期待する変更:

- delete: `aws_secretsmanager_secret_rotation.rotation`のみ
- create: trust anchor、runtime IAM role/policy/attachment、Roles Anywhere profile
- no change: S3 bucket、GitHub OIDC role、image uploader
- no change: legacy IAM user/access key/Secret/Lambda本体

S3 bucket、`current.json`、GitHub Actions upload roleのreplace / destroyが含まれたらapplyしません。

plan fileやstateには既存access key secretが含まれる可能性があります。planを共有・commitせず、不要になったlocal plan fileを安全に削除します。remote backendのstate historyにも過去のsecretが残り得るため、旧keyの失効・削除を必ず完了します。

## Application切替期間

Phase TF-A適用後、次を行います。

1. external CAからVPS用end-entity certificateを発行
2. `aws_signing_helper`、certificate、private key、AWS configをVPSへ配置
3. Rust `aws-config`の`credentials-process` featureを有効化
4. systemdを`AWS_CONFIG_FILE`利用へ変更
5. productionを切替
6. rebootと複数回のtemporary credential refreshを含めて観測

この期間はlegacy IAM user、access key、SecretをTerraformから削除しません。

## Phase TF-B: legacy resource撤去

### B-1. apply前の運用canary

IAM Roles Anywhereだけで安定配信できた後、現在のlegacy access keyをAWS Consoleまたは管理CLIで`Inactive`にします。これは削除前のrollback可能なcanaryです。

次を確認します。

- service restart後も`/api/ready`が成功
- 未cache記事を読める
- reboot後も成功
- CloudTrailでRoles Anywhere `CreateSession`が継続
- 旧cronが停止済みで、新しいstatic credential timerが存在しない

問題があれば旧keyを`Active`へ戻し、applicationをrollbackします。

### B-2. unmanaged access keyの確認

現行Lambdaが過去にkeyを作成していた場合、Terraform state外のaccess keyが残る可能性があります。IAM userのaccess key一覧を確認し、各key IDと状態を記録します。secret access keyは表示しません。

Terraform管理外keyが残ったままだとIAM user削除が失敗する可能性があります。IAM Roles Anywhere切替後であることを再確認してから、ownerが不要keyを削除します。

### B-3. root moduleからlegacy moduleを削除

`terraform/aws/main.tf`から次を削除します。

- `module "secret_rotation"`
- `module "iam_reader"`

不要になったroot variables、tfvars、outputsも削除します。

- `iam_reader_name`
- `secret_name`
- `rotation_interval`
- `iam_reader_access_key_id`
- `iam_reader_access_key_secret`
- `secret_arn`

`terraform/aws/iam/`と`terraform/aws/secret/`は他から参照されていないことを確認して削除します。

### B-4. Phase TF-B planの期待差分

destroy対象:

- legacy IAM access key
- legacy IAM userとS3 read policy attachment
- Secrets Manager secret / version / resource policy
- rotation Lambda、Lambda IAM role/policy、permission

retain対象:

- IAM Roles Anywhere trust anchor/profile/role/policy
- blog S3 bucketとartifact
- GitHub Actions OIDC role
- image upload resources

TerraformがSecrets Manager secretをrecovery window付きで削除する場合、その期限と復旧方法を記録します。IAM access key削除は即時なので、application切替の受け入れ条件を再確認してからapplyします。`archive_file`が作ったlocal zipはAWS resourceではないため、module撤去後にworkspaceへ残っていればownerが別途削除します。

## Terraform実行時の確認

このrepositoryの通常agent作業では実行しません。repository ownerが変更するときは、`terraform/aws`のremote backend、AWS identity、workspaceを確認してから行います。

推奨順序:

1. HCL変更をreview可能なcommitにする
2. format / validateを行う
3. refresh-only差分またはplanで予期しないdriftを確認
4. planを保存する場合は機密fileとして扱う
5. expected resource一覧とplanを照合
6. ownerがapply
7. AWS Console / CLIとapplication probeで結果確認
8. apply結果とresource ARNをIssueへ記録

`-auto-approve`は使いません。target指定による部分applyは依存関係を見落とすため、障害復旧以外では使いません。

## Terraform rollback

### Phase TF-A

Roles Anywhere resource追加に失敗してもlegacy認証は残るため、applicationを切り替えずHCLを戻せます。rotation scheduleだけは危険なため、rollbackで再作成しません。

### Phase TF-B

旧access key削除後は同じsecretを復元できません。Phase TF-Bのrollbackは旧resource再作成ではなく、IAM Roles Anywhere側の修正または新しい緊急role credential経路を使います。そのためPhase TF-B前のinactive canaryと観測期間が必須です。

## 完了条件

- Roles Anywhere resourceがTerraform stateで管理されている
- Terraform codeとAWS実体に意図しないdriftがない
- legacy IAM user、access key、Secret、rotation Lambdaがcode/state/AWSから撤去済み
- CA private key、VPS client private key、temporary credentialがTerraform stateに存在しない
- productionがtemporary role credentialだけでS3を読んでいる
