# リンカー

リンカーはキャッシュの更新内容をサーバ間に送信する中継器の役割を持ちます。  
etcdが必要です。

## インストール
```
cargo install senax-linker
```

## 自己認証局ファイル生成
certsディレクトリの下に自己認証局に必要なファイルを生成します。
```
senax-linker --cert
```

## .env設定

|パラメータ名|必須|説明|
|---|---|---|
|KEY|||
|CERT|||
|CA|||
|HOST_NAME||認証局用ホスト名|
|TCP_PORT|||
|UNIX_PORT|||
|LINK_PORT|||
|PASSWORD|Yes|サーバとの接続時に使用されるパスワード|
|ETCD_PORT|||
|ETCD_USER|||
|ETCD_PW|||
|ETCD_DOMAIN_NAME|||
|ETCD_CA_PEM_FILE|||
|ETCD_CERT_PEM_FILE|||
|ETCD_KEY_PEM_FILE|||

