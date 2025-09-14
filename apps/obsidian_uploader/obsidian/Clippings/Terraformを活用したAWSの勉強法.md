---
title: Terraformを活用したAWSの勉強法
source: https://zenn.dev/katsukiniwa/articles/8aebc891b53370
author:
  - "[[Zenn]]"
published: 2025-05-07
created: 2025-05-10T19:27:44+09:00
description: 
tags: [clippings]
updated: 2025-05-10T23:57:58+09:00
---

31

17[tech](https://zenn.dev/tech-or-idea)

# この記事を書いたきっかけ

先日Xで以下のポストを見かけました。

自分もプライベートでAWSで何かしらのサービスや機能をキャッチアップする時は以下の理由からTerraformを活用しています。

1. コンソール画面経由で作成するとコンソール画面のデザインが変わった時に作成方法が分からなくなる
2. Terraform(IaC)を活用することで自分が前回どのように設定したのか記録が残る
3. AWSとTerraformを同時に勉強することができて一石二鳥

しかし最初から初めて触るAWSのサービスや機能をTerraformで書くのは難しいのではないかと思います。少なくとも自分は自信がありません…。

そこで自分が普段どうやってTerraformを活用しつつAWSを勉強しているかをお伝えできればと思います。

# 前提

この記事はterraform v1.11.0で検証しています。

# 結論

`terraform import` を使いましょう。

これだけだと???だと思うので詳細をご説明します。

## 🚨注意

`terraform import` を実行してtfstateにはあるのにtffileにない状態でapplyするとリソースが削除されます！

チーム開発でimportする場合はご注意ください🙏  
詳細は [Terraform state 概論](https://chroju.dev/blog/terraform_state_introduction) をご覧下さい。

# Terraform Importとは

[Import | Terraform | HashiCorp Developer](https://developer.hashicorp.com/terraform/cli/v1.10.x/import) には

> Terraform can import existing infrastructure resources. This functionality lets you bring existing resources under Terraform management.

とあります。

つまり既存リソースをterraformの管理下に置くためのコマンドです。

コンソールやAPI経由で作成したリソースはterraformの管理対象ではないため、`terraform import` を実行することで対象のリソースをterraformの管理対象にできます。

詳細は以下の記事が参考になるかと思います。

[Terraform import のススメ 〜開発効率化編〜](https://tech.layerx.co.jp/entry/improve-iac-development-with-terraform-import)

# 実際にterraform Importを活用する

なんとなく `terraform import` の使い方がわかったところで、実際に `terraform import` を活用してCloudWatch Logsを.tfに書き起こしてみます。

なぜCloudWatch Logsなのかというと最近自分が勉強しているからです笑。

## ① コンソール画面経由でリソースを作成

CloudWatch Logsには保存されたログを特定の条件でフィルター・集計するメトリクスフィルターという機能があります。

以下はCloudWatch Logsの公式ドキュメントの [Filter pattern syntax for metric filters](https://docs.aws.amazon.com/AmazonCloudWatch/latest/logs/FilterAndPatternSyntaxForMetricFilters.html) からの引用です。

> Metric filters allow you to search and filter log data coming into CloudWatch Logs, extract metric observations from the filtered log data, and transform the data points into a CloudWatch Logs metric. You define the terms and patterns to look for in log data as it is sent to CloudWatch Logs. Metric filters are assigned to log groups, and all of the filters assigned to a log group are applied to their log streams.

実際にコンソール画面からメトリクスフィルターを作成してみるとこんな感じに表示されます。

![cloudwatch-logs-custom-metric-filter](https://res.cloudinary.com/zenn/image/fetch/s--S4muPgxP--/c_limit%2Cf_auto%2Cfl_progressive%2Cq_auto%2Cw_1200/https://storage.googleapis.com/zenn-user-upload/deployed-images/a56c7748dddae1178b9e79ca.png%3Fsha%3D81af2d88eed57c488c982505d5b1faa1b40b95c7)

## ② Terraform Importを実行

作成したリソースをterraformに取り込みます。

取り込みたいリソースは `aws_cloudwatch_log_metric_filter` なのでterraform　providerのドキュメントを確認します。

[aws\_cloudwatch\_log\_metric\_filter](https://registry.terraform.io/providers/hashicorp/aws/5.97.0/docs/resources/cloudwatch_log_metric_filter.html)

ひとまず適当な.tfファイルを用意してとりあえず公式ドキュメントのサンプルコードを持ってきて `log_group_name` の部分だけ変えてあげましょう。

```
resource "aws_cloudwatch_log_metric_filter" "banana_filter" {
  name           = "MyAppAccessCount"
  pattern        = ""
  log_group_name = aws_cloudwatch_log_group.for_ecs.name

  metric_transformation {
    name      = "EventCount"
    namespace = "YourNamespace"
    value     = "1"
  }
}

resource "aws_cloudwatch_log_group" "for_ecs" {
  name = "/ecs/golang-terraform"
}
```

またドキュメントには

> Using terraform import, import CloudWatch Log Metric Filter using the log\_group\_name:name.

とあるので実行すると↓な感じのログが出るかと思います。

```shell
$ terraform import aws_cloudwatch_log_metric_filter.golang-terraform-apple-count /ecs/golang-terraform:golang-terraform-apple-filter 
terraform import aws_cloudwatch_log_metric_filter.golang-terraform-apple-count /ecs/golang-terraform:golang-terraform-apple-filter 
aws_cloudwatch_log_metric_filter.golang-terraform-apple-count: Importing from ID "/ecs/golang-terraform:golang-terraform-apple-filter"...
data.aws_iam_policy_document.ecs: Reading...
data.aws_iam_policy_document.ecs_assume: Reading...
aws_cloudwatch_log_metric_filter.golang-terraform-apple-count: Import prepared!
  Prepared aws_cloudwatch_log_metric_filter for import
aws_cloudwatch_log_metric_filter.golang-terraform-apple-count: Refreshing state... [id=golang-terraform-apple-filter]
data.aws_iam_policy_document.ecs_task_assume: Reading...
data.aws_kms_secrets.secrets: Reading...
data.aws_iam_policy_document.ecs_task_assume: Read complete after 0s [id=12345678901]
data.aws_iam_policy_document.ecs: Read complete after 0s [id=12345678901]
data.aws_iam_policy_document.ecs_assume: Read complete after 0s [id=12345678901]
data.aws_kms_secrets.secrets: Read complete after 1s [id=ap-northeast-1]

Import successful!

The resources that were imported are shown above. These resources are now in
your Terraform state and will henceforth be managed by Terraform.
```

## ③ Terraform planで差分を確認しつつ.tfを編集

今はサンプルコードを持ってきただけなので、現実のリソースとは差分があります。この状態で `terraform plan` を実行すると以下のようにリソースの置き換えが発生する旨が表示されます。

```bash
$ terraform plan
~~省略~~

Terraform used the selected providers to generate the following execution plan. Resource actions are indicated with the following symbols:
-/+ destroy and then create replacement

Terraform will perform the following actions:

  # aws_cloudwatch_log_metric_filter.golang-terraform-apple-count must be replaced
-/+ resource "aws_cloudwatch_log_metric_filter" "golang-terraform-apple-count" {
      ~ id             = "golang-terraform-apple-filter" -> (known after apply)
      ~ name           = "golang-terraform-apple-filter" -> "MyAppAccessCount" # forces replacement
      - pattern        = "{ $.name = \"apple\" }" -> null
        # (1 unchanged attribute hidden)

      ~ metric_transformation {
          - dimensions    = {} -> null
          ~ name          = "golang-terraform-apple-count" -> "EventCount"
          ~ namespace     = "Custom/golang-terraform" -> "YourNamespace"
          + unit          = "None"
            # (2 unchanged attributes hidden)
        }
    }

Plan: 1 to add, 0 to change, 1 to destroy.
```

あとは差分が発生しないように.tfを編集していきましょう。

ゴニョゴニョ編集してもう一度 `terraform plan` を実行します。

```bash
Terraform used the selected providers to generate the following execution plan. Resource actions are indicated with the following symbols:
  ~ update in-place

Terraform will perform the following actions:

  # aws_cloudwatch_log_metric_filter.golang-terraform-apple-count will be updated in-place
  ~ resource "aws_cloudwatch_log_metric_filter" "golang-terraform-apple-count" {
        id             = "golang-terraform-apple-filter"
        name           = "golang-terraform-apple-filter"
        # (2 unchanged attributes hidden)

      ~ metric_transformation {
            name          = "golang-terraform-apple-count"
          + unit          = "None"
            # (4 unchanged attributes hidden)
        }
    }

Plan: 0 to add, 1 to change, 0 to destroy.
```

unitでdiffが発生していますね…。なんで？？？って感じなのですがドキュメントを確認すると

> unit - (Optional) The unit to assign to the metric. If you omit this, the unit is set as None.

とありました、どうやらterraform側で自動で付与されるため差分が発生するのは仕方ないようです。しかし、これ以外は一致させることが出来たのでapplyしましょう。

こんな感じで自分はいつも勉強したAWSリソースをTerraformに書き起こして管理しています。

# その他

今回は手動で `terraform import` を実行しましたが、terraform v1.5からimportブロックが提供されており `terraform plan -generate-config-out=import.tf` と組み合わせることで自動で.tfに書き起こすことが出来ます。

詳細は [Terraformのimportコマンドとimportブロックを試してみた](https://dev.classmethod.jp/articles/terraform-import-command-and-import-block/) をご覧下さい。

# まとめ

手順をまとめると以下の通りです。

1. AWSコンソール画面でリソースを作成する
2. 作成したリソースのterraformのproviderドキュメントを参照する
3. .tfにimportしたいリソースの適当なコードを書く
4. `terraform import` を実行する
5. `terraform plan` で差分が発生しなくなるまで.tfを編集する
6. `terraform apply` を実行する

自分はこの手順を踏むようになってから勉強効率が上がりました。

何かの参考になれば幸いです。

未熟ゆえ間違い等あるかと思いますが、コメントでご指摘頂ければ幸いです。

[GitHubで編集を提案](https://github.com/Katsukiniwa/zenn-content/blob/main/articles/8aebc891b53370.md)

31

17
