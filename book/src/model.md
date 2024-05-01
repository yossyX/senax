# モデルについて

## メソッド

モデル名で自動生成されるメソッド

|メソッド|説明|
|---|---|
|bulk_insert||
|bulk_upsert||
|clear_cache||
|delete||
|delete_by_ids||
|eq||
|find||
|find_all_from_cache||
|find_by_key||
|find_by_key_from_cache||
|find_for_update||
|find_for_update_with_trashed||
|find_from_cache||
|find_from_cache_with_trashed||
|find_many||
|find_many_for_update||
|find_many_from_cache||
|find_many_from_cache_with_trashed||
|find_many_with_trashed||
|find_optional||
|find_optional_for_update||
|find_optional_from_cache||
|find_optional_from_cache_with_trashed||
|find_optional_with_trashed||
|find_with_trashed||
|updater||
|force_delete||
|force_delete_all||
|force_delete_by_ids||
|force_delete_relations||
|insert_delayed||
|insert_dummy_cache||
|insert_ignore||
|query||
|restore||
|save||
|save_delayed||
|set||
|update_many|指定された複数のIDに対して同一の更新を適用する|
|upsert_delayed||
|_receive_update_notice||

## QueryBuilder

|メソッド|説明|
|---|---|
|bind||
|filter||
|count||
|delete||
|fetch_category||
|fetch_tags||
|force_delete||
|limit||
|offset||
|only_trashed||
|order||
|raw_query||
|select||
|select_for|カラム数を削減したサブセット型で取得を行う|
|select_for_update||
|select_from_cache||
|select_one||
|select_one_for||
|select_stream||
|select_stream_for||
|update||
|with_trashed||

## _{モデル名}Joiner

|メソッド|説明|
|---|---|
|fetch_category||

# CRUD
## Create
```
let obj = _{モデル名}Factory {
    ...
}.create();
_{モデル名}::save(conn, obj).await?;
```

## Read
取得方法は多数ありますので、一例です。
```
let obj = _{モデル名}::find(&mut conn, id).await?
```

## Update
更新時はカラム名のメソッドでアクセッサを呼び出し、set, add, sub, max, min, bit_and, bit_orを実行して更新します。

```
let mut obj = _{モデル名}::find_for_update(&mut conn, id).await?
obj.mut_{カラム名}().set(1);
_{モデル名}::save(conn, obj).await?;
```

## Delete

```
let obj = _{モデル名}::find_for_update(&mut conn, id).await?
_{モデル名}::delete(conn, obj).await?;
```

```
let obj = _{モデル名}::find_for_update(&mut conn, id).await?
_{モデル名}::force_delete(conn, obj).await?;
```
