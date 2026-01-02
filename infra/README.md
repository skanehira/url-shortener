# multipass-k8s

Multipassで作成するすべてのVMに共通の設定を適用できる`cloud-init`ファイルを置くリポジトリです。`cloud-init/base.yaml`を使って起動すれば、kubeadmでクラスタを組む前提となるコンテナランタイムやKubernetesコンポーネントが自動的にセットアップされます。

## バージョン情報

| コンポーネント | バージョン |
|--------------|-----------|
| Kubernetes | v1.32 |
| containerd | v2.1.2 |
| runc | v1.2.6 |
| CNI plugins | v1.6.2 |
| Flannel | v0.26.1 |
| MetalLB | v0.14.9 |

## フォルダ構成
- `cloud-init/base.yaml`: 共通設定の本体。kernelモジュール、sysctl、containerd、runc、CNI plugins、kubelet/kubeadmの導入を自動化します。エラーハンドリングとログ出力機能付き（`/var/log/k8s-setup.log`）。冪等性対応済み。
- `cloud-init/master.yaml`: 制御プレーン専用オーバーレイ。kubectlのインストールとクラスタ初期化スクリプト（`/usr/local/bin/init-cluster.sh`）を配置。
- `cloud-init/worker.yaml`: ワーカ専用オーバーレイ。追加コンポーネントなし。
- `scripts/render-cloud-init.sh`: `yq`でbaseとオーバーレイをマージする補助スクリプト。配列フィールド（`write_files`, `runcmd`, `packages`）は自動的に結合されます。`yq`未インストール時はHomebrewで自動インストールします。
- `scripts/install-metallb.sh`: MetalLBをインストールしてL2モードで設定するスクリプト。引数にIPアドレス範囲を指定します。

## 使い方
1. リポジトリをcloneしたディレクトリでMultipassを起動できることを確認します。
2. 制御プレーン用VMを起動:
   ```bash
   scripts/render-cloud-init.sh cloud-init/base.yaml cloud-init/master.yaml > /tmp/master.yaml
   multipass launch --name k8s-master --cpus 2 --memory 4G --disk 30G \
     --cloud-init /tmp/master.yaml noble
   ```
3. ワーカ用VMも同様:
   ```bash
   scripts/render-cloud-init.sh cloud-init/base.yaml cloud-init/worker.yaml > /tmp/worker1.yaml
   multipass launch --name k8s-worker1 --cpus 2 --memory 4G --disk 30G \
     --cloud-init /tmp/worker1.yaml noble
   ```
4. VM起動後、制御プレーンで初期化スクリプトを実行:
   ```bash
   multipass exec k8s-master -- sudo /usr/local/bin/init-cluster.sh
   ```
   スクリプト完了後、ワーカをjoinするためのコマンドが表示されます。
5. ワーカノードでjoinコマンドを実行:
   ```bash
   multipass exec k8s-worker1 -- sudo kubeadm join <master-ip>:6443 --token <token> --discovery-token-ca-cert-hash <hash>
   ```
6. （オプション）Mac側から直接kubectlを実行できるようにする:
   ```bash
   # kubectlをインストール（未インストールの場合）
   brew install kubectl jq

   # kubeconfigをコピーしてサーバーアドレスを更新
   multipass exec k8s-master -- sudo cat /etc/kubernetes/admin.conf > ~/.kube/config
   MASTER_IP=$(multipass info k8s-master --format json | jq -r '.info["k8s-master"].ipv4[0]')
   sed -i '' "s|https://.*:6443|https://${MASTER_IP}:6443|" ~/.kube/config

   # 確認
   kubectl get nodes
   ```

## カスタマイズ
- バージョンを変更したい場合は`cloud-init/base.yaml`内の`k8s-common.sh`スクリプト冒頭の変数を編集してください:
  ```bash
  CONTAINERD_VERSION="2.1.2"
  RUNC_VERSION="1.2.6"
  CNI_VERSION="1.6.2"
  K8S_VERSION="1.32"
  ```
- Flannelのバージョンを変更したい場合は`cloud-init/master.yaml`内の`init-cluster.sh`スクリプトの変数を編集してください:
  ```bash
  FLANNEL_VERSION="0.26.1"
  ```
- containerdの設定を細かく制御したい場合は、`k8s-common.sh`の`containerd config default | sed ...`行を好きなテンプレートに置き換えられます。
- ノードごとに追加の設定を入れたい場合は、`cloud-init`ディレクトリ内にオーバーレイ用ファイルを作成し、`scripts/render-cloud-init.sh`でマージしてください。

## MetalLB（LoadBalancer）のインストール
ローカル環境でLoadBalancerタイプのServiceを使用するには、MetalLBをインストールします:

```bash
# VMのIPアドレス帯を確認
multipass list

# MetalLBをインストール（IPレンジは環境に合わせて変更）
scripts/install-metallb.sh 192.168.2.200-192.168.2.250
```

インストール後、`type: LoadBalancer`のServiceを作成すると、指定したIP範囲から自動でExternal IPが割り当てられます:

```bash
# LoadBalancer Serviceを作成
kubectl expose deployment nginx --port=80 --type=LoadBalancer --name=nginx-lb

# External IPを確認
kubectl get svc nginx-lb
# NAME       TYPE           CLUSTER-IP    EXTERNAL-IP     PORT(S)
# nginx-lb   LoadBalancer   10.x.x.x      192.168.2.200   80:xxxxx/TCP

# アクセス確認
curl http://192.168.2.200
```

## 動作確認
クラスタが正常に動作しているか確認するため、サンプルのnginx Podをデプロイします:
```bash
# nginx Deploymentを作成
kubectl create deployment nginx --image=nginx

# Podが起動するまで待機
kubectl wait --for=condition=Ready pod -l app=nginx --timeout=60s

# 動作確認
kubectl get pods -o wide

# ポートフォワードでnginxにアクセス（バックグラウンドで実行）
kubectl port-forward deployment/nginx 8080:80 &

# curlで通信確認
curl http://localhost:8080

# ポートフォワードを停止
kill %1

# クリーンアップ
kubectl delete deployment nginx
```

## テストとメンテナンス
- 新しい変更を入れたときは`multipass launch --name ci-test --cloud-init ./cloud-init/base.yaml --timeout 600 noble`で動作確認し、不要になったら`multipass delete ci-test && multipass purge`で後片付けします。
- 失敗時にロールバックしたければ、Multipassのスナップショット（例: `multipass snapshot k8s-master base`)を併用すると便利です。
- セットアップログは`/var/log/k8s-setup.log`、クラスタ初期化ログは`/var/log/k8s-cluster-init.log`で確認できます。
- ログファイルは自動でローテーションされます（7日間保持）。
- ワーカ追加用のjoinコマンドを再表示するには: `kubeadm token create --print-join-command`（トークンは24時間有効）
