# 依存関係の更新

依存関係の更新PRはMend-hosted Renovate GitHub Appで作成します。repository内の設定はrootの[renovate.json](../../renovate.json)を正とします。

## 更新対象と方針

| 対象 | Renovate manager | 主なファイル |
| --- | --- | --- |
| Rust crate（Topcoat本体を除く） | `cargo` | root / 各crateの`Cargo.toml`、`Cargo.lock` |
| E2EのBun package | `bun` | `e2e/package.json`、`e2e/bun.lock` |
| Bun本体 | `mise` | `mise.toml`の`[tools].bun`、`mise.lock` |
| GitHub Actions | `github-actions` | `.github/workflows/*.yml` |
| Rust toolchain | `rust-toolchain` | `rust-toolchain.toml` |

- `config:best-practices`を使い、Dependency DashboardのIssueで更新状況を確認します。[presetの内容](https://docs.renovatebot.com/presets-config/#configbest-practices)
- GitHub Actionsをcommit SHAに固定し、対応versionをcommentで追跡します。開発依存のversion固定、npmパッケージ公開後3日の待機、メンテナンス停止候補の検出、Renovate設定のmigration PRも有効になります。
- manifestのversion範囲を変えずにlockfileを更新するメンテナンスPRを、毎週月曜の午前4時まで（日本時間）に作成します。
- 通常の更新PRを作成する時間帯は毎週月曜の終日、日本時間（`Asia/Tokyo`）です。App側の実行周期で処理されるため、時刻を指定した定時実行ではありません。既存PRの更新は月曜以外にも行われます。[スケジュール仕様](https://docs.renovatebot.com/key-concepts/scheduling/)
- PRはCIとreviewを経て手動でmergeします。`automerge`は無効です。
- `platformCommit: enabled`によりGitHub AppのAPI経由で署名付きcommitを作成します。[署名の設定](https://docs.renovatebot.com/configuration-options/#platformcommit)
- `terraform/`とprivate Obsidian submoduleを更新対象から除外します。`git-submodules` managerも有効にしません。
- Topcoat本体とCLIは`ignoreDeps: ["topcoat", "cargo:topcoat-cli"]`でRenovateの更新対象から除外します。`Cargo.toml`の完全固定（`=version`）を維持し、破壊的変更を確認して手動で更新します。Topcoatの脆弱性が通知された場合も手動で対応します。
- Bun本体は`mise` managerで更新を検知し、`mise.toml`と`mise.lock`を更新します。`bun` managerはE2Eのpackage依存を扱うため、Bun本体には別途`mise` managerが必要です。[mise managerの対応範囲](https://docs.renovatebot.com/modules/manager/mise/)
- Topcoat CLI、および`mise.toml`の`[env]`に置くTailwindのversionは共通toolの更新手順で管理します。`mise` managerが検知するのは`[tools]`とtask内の`tools`です。

## GitHubで行う初期設定

repositoryの管理権限があるアカウントで、次を実施します。

1. rootの`renovate.json`の追加と`.github/dependabot.yml`の削除を`main`へ反映します。Dependabotのversion updatesは設定ファイルの削除で停止します。[GitHubの手順](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/secure-your-dependencies/configuring-dependabot-version-updates)
2. [Renovate GitHub App](https://github.com/apps/renovate)を開き、**Install**（導入済みなら**Configure**）を選びます。対象アカウントを選び、**Only select repositories**で`okawak/okawak_blog`を追加します。privateな`okawak/obsidian`へのアクセスは不要です。[Appの導入手順](https://docs.renovatebot.com/getting-started/installing-onboarding/)
3. [repositoryのSettings](https://github.com/okawak/okawak_blog/settings)で**Advanced Security**を開き、以下を設定します。
   - **Dependency graph**: 有効
   - **Dependabot alerts**: 有効
   - **Dependabot security updates**: Renovateの動作確認後に無効化し、修正PRの重複を防ぐ
   - **Dependabot version updates**: 再度有効化して`dependabot.yml`を作り直さない

   RenovateはGitHubのDependabot alertsを入力に脆弱性修正PRの作成を試みます。AppのPermissionsにDependabot alertsのread権限があることも確認します。脆弱性修正PRは通常の週次スケジュールを待たずに作成されます。[Renovateの脆弱性対応](https://docs.renovatebot.com/configuration-options/#vulnerabilityalerts)、[GitHubのsecurity updates設定](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/secure-your-dependencies/configure-security-updates)
4. repositoryの**Issues**を有効にし、Dependency Dashboardが作られることを確認します。設定ファイルが先に`main`にあれば通常はonboarding PRが不要です。先にAppを導入して**Configure Renovate** PRが作られていた場合は、既存の設定と重複しないよう内容を確認します。
5. 初回の更新PRで、Rust CIの`verify` / `browser-e2e`が動くこと、commitが**Verified**になっていること、manifestとlockfileが整合していることを確認します。branch protection / rulesetでPR作成が阻止された場合は、そのルールとApp権限を確認します。
6. Renovateの動作を確認してから、残っているDependabot PRの差分を確認し、重複するものをcloseします。

この構成ではPAT、Renovate用のActions secret、Renovateを起動するworkflowの追加は不要です。既存の`Security audit` workflowによるRust依存の監査も併用します。alertに対応する修正PRがない場合は、Security画面で修正版の有無を確認して手動で対応します。

## 更新PRの確認

- 通常のRust / Bun package更新ではmanifestとlockfileの差分を確認し、Rust CIを通します。Bun依存は[Bun manager](https://docs.renovatebot.com/modules/manager/bun/)が`bun.lock`も更新します。
- Bun本体の更新PRでは`mise.toml`と`mise.lock`が同じversionになっていること、およびlockfile内の既存platformのURL・checksumが更新されていることを確認します。`mise install`後に`mise run versions-check`と`mise run test-e2e`を通します。App側でlockfile更新に失敗した場合は、そのPR上で共通toolの更新手順に従って再生成します。
- Topcoat frameworkを手動更新するときはCLIも同じversionへ揃え、`Cargo.lock`と`mise.lock`を更新します。`mise install`後に`mise run versions-check`と通常のserver / E2E確認を通します。
- `topcoat ui`のcomponentはrepositoryへ配置したsourceであり、Renovateは`components.toml`のhashを解釈しないため、更新有無を直接検出できません。Topcoatまたは互換範囲内の`topcoat-ui-registry`が更新されたら、`topcoat ui list --installed --package server`で保存済みhashとregistry sourceを比較します。`update available`と表示されたcomponentだけ差分を確認し、必要なら`topcoat ui add --overwrite --package server <component>`で再取得してsite固有の変更を反映し直します。
- `topcoat-*` crateはTopcoat本体からversion範囲で参照されているため、週次lockfile更新だけで変わることがあります。`Cargo.lock`の該当差分もreviewします。
- GitHub Actionsの更新追従とSHA固定はRenovateで管理し、PRでは対応versionのcommentと参照先を確認します。Actions参照のmajor・SHA・commentを独自scriptで検証する処理は設けません。`mise run versions-check`はBun・Topcoat・Tailwindの共通build toolの整合と、workflowにtool versionや個別installerを持ち込まないことを確認します。
- Bun本体やTailwindを手動更新する場合も、`mise.toml`と`mise.lock`の関連versionを揃え、`mise install`後に`mise run versions-check`を通します。
- 更新PRを手動修正してcommitする際も、署名設定を確認して署名付きcommitを作成します。

## Renovate設定の検証

Node.js 24.11以降の24系とnpmがある環境で、repository rootから公式validatorを実行します。これは設定を検証するだけで、更新PRを作成しません。

```bash
npm exec --yes --package renovate@44.103.3 -- renovate-config-validator --strict --no-global renovate.json
```

`--strict`は非推奨設定のmigrationも検出し、`--no-global`はrepository用の設定として検証します。[公式validatorの使い方](https://docs.renovatebot.com/config-validation/)

App導入後はDependency Dashboardの警告も確認します。更新が週次スケジュール待ちの場合は、Dashboardのcheckboxから作成を要求できます。
