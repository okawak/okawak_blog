# AWS runtime認証のIAM Roles Anywhere移行

## 目的

VPSがS3 artifactを読むための長期IAM access keyを廃止し、IAM Roles Anywhereが発行する期限付きcredentialへ移行します。

現行の`service/update_aws_creds.sh`とsystemd timerは、Secrets Managerに保存済みの値をVPSへ反映するための暫定経路です。IAM access keyをrotationする仕組みではありません。IAM Roles Anywhereへの切替後は、この暫定経路を停止します。

## 安全上の前提

- Codexを含む通常作業では`terraform/`をread-onlyとし、ownerが[変更計画](./aws-runtime-auth-terraform-plan.md)に従って変更する
- 現行rotation Lambdaを手動実行しない
- migration完了まで現在動作しているaccess keyを削除しない
- certificate private key、Secrets Managerの値、一時credentialをrepositoryやlogへ出力しない
- AWS側の操作には`secret-get`ではなく、対象resourceを管理できるadmin identityを使う
- 各phaseの受け入れ条件を満たしてから次へ進む

AWS公式資料:

- [IAM Roles Anywhere credential helper](https://docs.aws.amazon.com/rolesanywhere/latest/userguide/credential-helper.html)
- [IAM Roles Anywhere trust model](https://docs.aws.amazon.com/rolesanywhere/latest/userguide/trust-model.html)
- [Secrets Manager cancel-rotate-secret](https://docs.aws.amazon.com/cli/latest/reference/secretsmanager/cancel-rotate-secret.html)

## 現状と解消する問題

現在は次の経路です。

```text
long-lived secret-get credential
  -> Secrets Manager
  -> long-lived blog-s3 credential file
  -> Rust AWS SDK
  -> S3 GetObject
```

問題:

- cron / timerは同じSecretを再取得するだけで、IAM access keyをrotationしない
- `secret-get`と`blog-s3`が同じIAM userなら、旧key失効後に新keyを取得できない
- 現行LambdaはSecrets Manager標準の4段階rotation契約を実装していない
- TerraformにはrotationとIAM userが宣言されたままで、AWS側だけを変更するとdriftする

移行後は次の経路にします。

```text
VPS X.509 end-entity certificate
  -> aws_signing_helper credential-process
  -> IAM Roles Anywhere CreateSession
  -> temporary role credential
  -> Rust AWS SDK automatic refresh
  -> S3 GetObject
```

## Phase 0: 現行状態の記録とrotation停止

### 0.1 Caller identityの確認

VPSでARNだけを確認します。出力をIssueへ貼る場合はaccount IDを必要に応じて伏せます。

```bash
AWS_PROFILE=secret-get aws sts get-caller-identity
AWS_PROFILE=blog-s3 aws sts get-caller-identity
```

両方が同じIAM userなら、自己rotation構成です。現在のkeyを維持したままmigrationを進めます。

### 0.2 Secret metadataの確認

秘密値を取得せず、rotation metadataとversion stageだけを確認します。

```bash
export ADMIN_PROFILE=your-admin-profile
AWS_PROFILE="${ADMIN_PROFILE}" aws secretsmanager describe-secret \
  --secret-id blog/iam_access_key \
  --query '{RotationEnabled:RotationEnabled,LastRotatedDate:LastRotatedDate,NextRotationDate:NextRotationDate,RotationRules:RotationRules,VersionIdsToStages:VersionIdsToStages}'
```

### 0.3 自動rotationの停止

`RotationEnabled=true`なら、管理端末から停止します。これはTerraform変更ではなくAWS側の緊急安全化なので、Terraform driftとして記録します。

```bash
AWS_PROFILE="${ADMIN_PROFILE}" aws secretsmanager cancel-rotate-secret \
  --secret-id blog/iam_access_key
```

停止後に`describe-secret`を再実行し、`RotationEnabled=false`を確認します。実行結果に`VersionId`が含まれる、または`AWSPENDING`が残る場合は、version stageを推測で変更しません。AWS公式の注意事項に従い、`list-secret-version-ids`で状態を確認してから個別に復旧します。

ownerがTerraform変更を実装するまでは現行HCLを`apply`しません。applyするとrotationが再有効化される可能性があります。Terraform Phase TF-Aでrotation resourceをcodeから外し、Roles Anywhere resourceを並行追加したplanをreviewしてからownerがapplyします。

受け入れ条件:

- 現在のVPSが`/api/health`とhomeへ200を返す
- 新binaryのdeploy後は`/api/ready`も200を返す
- rotationが無効である
- 現在のaccess keyが有効なrollback手段として残っている

## Phase 1: AWS側のIAM Roles Anywhere準備

このphaseのAWS resourceはrepository ownerが[変更計画](./aws-runtime-auth-terraform-plan.md)に従ってTerraformで追加します。危険なrotationの緊急停止だけはTerraform変更に先行して構いませんが、そのdriftはPhase TF-Aでcodeと再同期します。resource ARNはTerraform outputからVPSのroot所有設定へ反映します。

### 1.1 CAとend-entity certificate

AWS Private CAまたは外部CAを選びます。個人VPSでは外部CAを使う場合でも、CA private keyをVPSへ置かずofflineで保管します。

end-entity certificateは次を満たす必要があります。

- X.509 v3
- Basic Constraintsが`CA:FALSE`
- Key Usageに`Digital Signature`
- SHA-256以上の署名algorithm
- Subject CNを`okawak-blog-vps`のようにworkload固有にする
- 有効期限を運用可能な短期間にし、期限前更新を設計する

VPSへ置くのはend-entity certificateと対応するprivate keyだけです。CA private keyは置きません。

### 1.2 Trust anchor

選択したCA certificateをIAM Roles Anywhereのtrust anchorとして登録します。作成後に次を記録します。

```text
TRUST_ANCHOR_ARN=arn:aws:rolesanywhere:<region>:<account>:trust-anchor/<id>
```

### 1.3 S3 reader role

IAM userではなく、IAM Roles Anywhereからassumeする専用roleを作ります。permission policyはruntimeが実際に読むobjectへ限定します。

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": "s3:GetObject",
      "Resource": [
        "arn:aws:s3:::okawak-blog-resources-bucket/current.json",
        "arn:aws:s3:::okawak-blog-resources-bucket/releases/*/site/*"
      ]
    }
  ]
}
```

role trust policyはIAM Roles Anywhere serviceを許可し、trust anchor ARNとcertificate CNで制限します。

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Service": "rolesanywhere.amazonaws.com"
      },
      "Action": [
        "sts:AssumeRole",
        "sts:TagSession",
        "sts:SetSourceIdentity"
      ],
      "Condition": {
        "ArnEquals": {
          "aws:SourceArn": "<TRUST_ANCHOR_ARN>"
        },
        "StringEquals": {
          "aws:PrincipalTag/x509Subject/CN": "okawak-blog-vps"
        }
      }
    }
  ]
}
```

role ARNを記録します。

```text
ROLE_ARN=arn:aws:iam::<account>:role/<role-name>
```

### 1.4 Roles Anywhere profile

IAM Roles Anywhere profileを作り、上記roleだけを許可します。session durationはまず1時間を推奨します。作成後にprofile ARNを記録します。

```text
PROFILE_ARN=arn:aws:rolesanywhere:<region>:<account>:profile/<id>
```

受け入れ条件:

- trust anchorが有効
- profileが有効で対象roleだけを許可
- roleがtrust anchor ARNとcertificate CNで制限されている
- role permissionにS3 write、delete、bucket管理権限がない

## Phase 2: VPSへcredential helperとcertificateを配置

### 2.1 Credential helper

[AWS公式download表](https://docs.aws.amazon.com/rolesanywhere/latest/userguide/credential-helper.html)からVPS architecture向け`aws_signing_helper`を取得し、同ページのSHA-256と照合します。URLとchecksumは更新されるため、この文書へversionを固定しません。

検証後、root所有で配置します。

```bash
sudo install -o root -g root -m 0755 aws_signing_helper \
  /usr/local/bin/aws_signing_helper
```

### 2.2 Certificateとprivate key

```bash
sudo install -d -o root -g okawak -m 0750 /etc/okawak_blog/aws
sudo install -o root -g okawak -m 0644 client-cert.pem \
  /etc/okawak_blog/aws/client-cert.pem
sudo install -o root -g okawak -m 0640 client-key.pem \
  /etc/okawak_blog/aws/client-key.pem
```

private keyにpassphraseを設定する場合、無人起動で安全に解錠する仕組みが別途必要です。平文key fileを採用する場合は、filesystem permission、VPS disk encryption、backup範囲を確認します。可能ならTPM / PKCS#11を後続改善として検討します。

### 2.3 AWS shared config

`/etc/okawak_blog/aws/config`をroot所有、group `okawak`、mode 0640で作ります。

```ini
[profile blog-s3]
region = ap-northeast-1
credential_process = /usr/local/bin/aws_signing_helper credential-process --certificate /etc/okawak_blog/aws/client-cert.pem --private-key /etc/okawak_blog/aws/client-key.pem --trust-anchor-arn <TRUST_ANCHOR_ARN> --profile-arn <PROFILE_ARN> --role-arn <ROLE_ARN> --session-duration 3600
```

### 2.4 Helper単体検証

credential JSONをterminalやfileへ保存しません。AWS CLIからprofileを利用し、read-onlyな`head-object`で確認します。

```bash
sudo -u okawak env \
  HOME=/nonexistent \
  AWS_PROFILE=blog-s3 \
  AWS_CONFIG_FILE=/etc/okawak_blog/aws/config \
  AWS_SHARED_CREDENTIALS_FILE=/dev/null \
  aws s3api head-object \
    --bucket okawak-blog-resources-bucket \
    --key current.json
```

write/delete操作は検証に使いません。

受け入れ条件:

- helper binaryのchecksum確認済み
- private keyを`okawak`とroot以外が読めない
- `head-object`が成功
- temporary credentialをdiskやlogへ保存していない

## Phase 3: Applicationとsystemdの対応

### 3.1 Rust AWS SDK feature

現在のworkspaceは`aws-config`のdefault featureを無効にしているため、`credential_process`を使うには別PRで`credentials-process` featureを明示的に有効化します。

```toml
aws-config = { version = "1", default-features = false, features = ["credentials-process"] }
```

変更後はworkspace test、clippy、S3 E2Eを実行します。AWS公式credential helperはSDK利用時に期限前のcredential refreshへ対応するため、application独自のcredential timerは追加しません。

### 3.2 systemd environment

別PRで`service/okawak_blog.service`を次の方針へ変更します。

```ini
Environment=AWS_PROFILE=blog-s3
Environment=AWS_REGION=ap-northeast-1
Environment=AWS_CONFIG_FILE=/etc/okawak_blog/aws/config
```

次のstatic credential指定は削除します。

```ini
Environment=AWS_SHARED_CREDENTIALS_FILE=/var/lib/okawak_blog/aws/credentials
```

service userがhelper、config、certificate、private keyを読めることを確認します。現在のsystemd hardeningを維持し、必要なread-only pathだけを追加します。

### 3.3 切替前検証

PR CIに加え、VPSで新binaryをserviceとは別portまたは短いmaintenance windowで起動し、次を確認します。

```bash
curl --fail http://127.0.0.1:8008/api/health
curl --fail http://127.0.0.1:8008/api/ready
```

home、article index、実記事も確認します。`/api/ready`だけでは未cache articleのS3 readを網羅しないためです。

## Phase 4: Production切替

切替前に旧runtime credential fileをrollback用として維持します。

1. IAM Roles Anywhere対応binaryとsystemd unitをinstallする
2. `systemctl daemon-reload`を実行する
3. `okawak_blog`を再起動する
4. `/api/health`と`/api/ready`を確認する
5. home、article index、未cache記事を確認する
6. journalにcredential process errorがないことを確認する
7. 外形監視でHTTPS responseを確認する

最低24時間、または複数回のtemporary credential更新を跨いで観測します。この期間は旧IAM access key、Secrets Manager secret、暫定timerを削除しません。ただし暫定timerはstatic fileを書き換えないよう停止して構いません。

```bash
sudo systemctl disable --now okawak_blog-aws-credentials.timer
```

受け入れ条件:

- 複数回のtemporary credential更新後も配信が継続
- reboot後にserviceがreadinessへ到達
- CloudTrailでIAM Roles Anywhere `CreateSession`を確認
- 旧static credentialへfallbackしていない

## Rollback

旧IAM access keyを削除する前なら、次で戻せます。

1. 旧systemd unitへ戻し、`AWS_SHARED_CREDENTIALS_FILE=/var/lib/okawak_blog/aws/credentials`を復元
2. 旧runtime credential fileがmode 0600で存在することを確認
3. `systemctl daemon-reload`とservice restart
4. `/api/ready`、home、未cache記事を確認
5. IAM Roles Anywhere側の失敗原因を調査

旧keyを非activeにした後でも、削除前ならAWS管理者が一時的に再active化できます。削除後は同じsecret access keyを復元できないため、削除は最後に行います。

## Phase 5: 旧credential経路の撤去

安定観測後、段階的に撤去します。

1. 旧IAM access keyを`Inactive`へ変更
2. serviceを再起動し、readinessと実記事を再確認
3. 監視期間を置く
4. 旧IAM access keyを削除
5. Secrets Manager secretの不要化を確認
6. credential refresh timer / service、`update_aws_creds.sh`、sudoers entryを削除
7. `/var/lib/okawak_blog/aws/credentials`を安全に削除
8. `secret-get` profileから不要credentialを削除

AWS側だけを先に削除せず、ownerがTerraform Phase TF-BでIAM user、initial access key、Secret、rotation Lambdaをcodeから外し、planのdestroy対象をreviewしてapplyします。これにより最終的なAWS実体とTerraform stateのdriftを残しません。

## Certificate更新

IAM Roles AnywhereはAWS temporary credentialを自動更新しますが、X.509 certificate自体の更新は運用責務です。

- certificate expiryを日次監視する
- 少なくとも期限7日前より前に新certificateを発行する
- 新certificateとprivate keyを同一directory内でatomicに置き換える
- helper単体の`head-object`を検証する
- service restartとreadiness確認を行う
- 旧certificateを失効または撤去する

期限確認例:

```bash
openssl x509 -checkend 604800 -noout \
  -in /etc/okawak_blog/aws/client-cert.pem
```

exit statusが0以外なら7日以内に期限切れです。certificate更新失敗はAWS credential更新失敗へ直結するため、journalだけでなく外部通知へ接続します。

## 完了条件

- productionがIAM Roles Anywhereのtemporary credentialだけでS3を読んでいる
- static IAM access keyとSecrets Manager反映timerが停止・撤去済み
- rebootとcredential refreshを跨いでreadinessが継続
- certificate expiry監視と更新runbookが運用されている
- Roles AnywhereがTerraform stateで管理され、legacy認証resourceがcode/state/AWSから撤去されている
