DNS 設定は手動で行う

```shell
terraform init
terraform plan -out=./plan_deploy
terraform apply ./plan_deploy
```

もし環境を削除したくなったら、

```shell
terraform destroy
```
