# .envファイル

改行区切りを使用するために dotenv ではなく、 dotenvy を使用してください。

## DB設定

|パラメータ名|説明|
|---|---|
|{DB名}_DB_URL|更新用DB接続URL（改行区切りでシャーディング設定）|
|{DB名}_REPLICA_DB_URL|参照用DB接続URL（改行区切りでシャーディング設定、カンマ区切りでレプリカ設定）|
|{DB名}_CACHE_DB_URL|キャッシュ用DB接続URL（改行区切りでシャーディング設定、カンマ区切りでレプリカ設定）|
|{DB名}_TEST_DB_URL|テストDB接続URL|
|{DB名}_DB_MAX_CONNECTIONS|更新用コネクション数|
|{DB名}_REPLICA_DB_MAX_CONNECTIONS|参照用コネクション数|
|{DB名}_CACHE_DB_MAX_CONNECTIONS|キャッシュ用コネクション数|

## キャッシュ設定

|パラメータ名|説明|
|---|---|
|{DB名}_FAST_CACHE_INDEX_SIZE|高速キャッシュインデックスメモリサイズ|
|{DB名}_SHORT_CACHE_CAPACITY|ショートキャッシュメモリサイズ|
|{DB名}_SHORT_CACHE_TIME|ショートキャッシュ保持時間(秒)|
|{DB名}_LONG_CACHE_CAPACITY|ロングキャッシュメモリサイズ|
|{DB名}_LONG_CACHE_TIME|ロングキャッシュ保持時間(秒)|
|{DB名}_LONG_CACHE_IDLE_TIME|ロングキャッシュアイドル時間(秒)|
|{DB名}_DISK_CACHE_INDEX_SIZE|ディスクキャッシュインデックスメモリサイズ|
|{DB名}_DISK_CACHE_FILE_NUM|ディスクキャッシュ分割ファイル数|
|{DB名}_DISK_CACHE_FILE_SIZE|ディスクキャッシュファイルサイズ|
|{DB名}_CACHE_TTL|キャッシュ保持時間|
|DISABLE_{DB名}_CACHE|そのサーバへのキャッシュと更新通知の無効化(true or false)|

## リンカー設定

|パラメータ名|説明|
|---|---|
|LINKER_PORT|リンカー接続ポート(TCP or UNIX)|
|LINKER_PASSWORD|リンカーパスポート|
