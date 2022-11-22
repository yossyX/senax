
# Schema Definition

* [properties/conf](##/definitions/ConfigDef)
* [properties/enum](##/definitions/EnumDef)
* [properties/model](##/definitions/ModelDef)
* [properties/history](##/definitions/History)

---------------------------------------
<a id="#/definitions/ConfigDef"></a>
## Config Definition

データベース設定

**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**db_no**|integer|内部で使用されるデータベースナンバー。同一conf.ymlファイル内では自動生成の乱数で重複が発生しないが、設定ファイルが分かれる場合は重複を防ぐため指定できる。||
|**db**|[DbType](##/definitions/DbType)|使用するDB。現在のところmysqlのみ対応|Yes|
|**title**|string|仕様書等のためのタイトル||
|**author**|string|仕様書等のための著者||
|**ignore_foreign_key**|boolean|falseの場合は外部キー制約をDDLに出力しない||
|**timestampable**|[Timestampable](##/definitions/Timestampable)|デフォルトのタイムスタンプ設定||
|**time_zone**|[TimeZone](##/definitions/TimeZone)|日時型のデフォルトのタイムゾーン設定||
|**timestamp_time_zone**|[TimeZone](##/definitions/TimeZone)|created_at, updated_at, deleted_atに使用されるタイムゾーン||
|**soft_delete**|[SoftDelete](##/definitions/SoftDelete)|論理削除のデフォルト設定||
|**use_cache**|boolean|キャッシュ使用のデフォルト設定||
|**use_fast_cache**|boolean|高速キャッシュ使用設定（experimental）||
|**use_cache_all**|boolean|全キャッシュ使用のデフォルト設定||
|**tx_isolation**|[Isolation](##/definitions/Isolation)|更新トランザクション分離レベル||
|**read_tx_isolation**|[Isolation](##/definitions/Isolation)|参照トランザクション分離レベル||
|**engine**|string|MySQLのストレージエンジン||
|**character_set**|string|文字セット||
|**collate**|string|文字セット照合順序||
|**preserve_column_order**|boolean|DDL出力時のカラム順序維持設定||
|**groups**|Map<property, [GroupDef](##/definitions/GroupDef)>|モデルグループ|Yes|
---------------------------------------
<a id="#/definitions/DbType"></a>
## DB type




**Allowed values**

* `mysql`

---------------------------------------
<a id="#/definitions/Timestampable"></a>
## Timestampable




**any of the following**

* `none`
* `real_time`(クエリー実行日時)
* `fixed_time`(DbConnの生成日時)

---------------------------------------
<a id="#/definitions/TimeZone"></a>
## TimeZone




**Allowed values**

* `local`
* `utc`

---------------------------------------
<a id="#/definitions/SoftDelete"></a>
## SoftDelete




**any of the following**

* `none`
* `time`
* `flag`
* `unix_time`(ユニーク制約に使用するためのUNIXタイムスタンプ UNIX time for unique index support)

---------------------------------------
<a id="#/definitions/Isolation"></a>
## Isolation




**Allowed values**

* `repeatable_read`
* `read_committed`
* `read_uncommitted`
* `serializable`

---------------------------------------
<a id="#/definitions/GroupDef"></a>
## Group Def



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**title**|string||Yes|
|**type**|[GroupType](##/definitions/GroupType)|||
---------------------------------------
<a id="#/definitions/GroupType"></a>
## Group Type




**any of the following**

* `model`(モデル定義)
* `enum`(列挙型定義のみ)

---------------------------------------
<a id="#/definitions/EnumDef"></a>
## Enum Def



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**title**|string|タイトル||
|**comment**|string|コメント||
|**enum_values**|Array<[EnumValue](##/definitions/EnumValue)>|列挙値|Yes|
|**mod_name**|string|列挙子の名前にマルチバイトを使用した場合のmod名||
---------------------------------------
<a id="#/definitions/EnumValue"></a>
## Enum Value



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**name**|string||Yes|
|**title**|string|||
|**comment**|string|||
|**value**|integer|0～255の値|Yes|
---------------------------------------
<a id="#/definitions/ModelDef"></a>
## Model Def



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**title**|string|仕様書等のためのタイトル||
|**comment**|string|コメント||
|**table_name**|string|テーブル名||
|**ignore_foreign_key**|boolean|falseの場合は外部キー制約をDDLに出力しない||
|**timestampable**|[Timestampable](##/definitions/Timestampable)|タイムスタンプ設定||
|**disable_created_at**|boolean|created_atの無効化||
|**disable_updated_at**|boolean|updated_atの無効化||
|**soft_delete**|[SoftDelete](##/definitions/SoftDelete)|論理削除設定||
|**versioned**|boolean|キャッシュ整合性のためのバージョンを使用するか||
|**counting**|string|save_delayedでカウンターを使用するカラム||
|**use_cache**|boolean|キャッシュを使用するか||
|**use_fast_cache**|boolean|高速キャッシュを使用するか(experimental)||
|**use_cache_all**|boolean|全キャッシュを使用するか||
|**ignore_propagated_insert_cache**|boolean|他サーバでinsertされたデータをキャッシュするか||
|**on_delete_fn**|boolean|物理削除時の_before_deleteと_after_deleteの呼び出しを行うか||
|**abstract**|boolean|抽象化モード||
|**inheritance**|[Inheritance](##/definitions/Inheritance)|継承モード||
|**engine**|string|MySQLのストレージエンジン||
|**character_set**|string|文字セット||
|**collate**|string|文字セット照合順序||
|**mod_name**|string|名前にマルチバイトを使用した場合のmod名||
|**columns**|Map<property, [ColumnTypeOrDef](##/definitions/ColumnTypeOrDef)>|カラム||
|**relations**|Map<property, [RelDef](##/definitions/RelDef)>|リレーション||
|**indexes**|Map<property, [IndexDef](##/definitions/IndexDef)>|インデックス||
---------------------------------------
<a id="#/definitions/Inheritance"></a>
## Inheritance



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**extends**|string|継承元|Yes|
|**type**|[InheritanceType](##/definitions/InheritanceType)|継承タイプ|Yes|
|**key_field**|string|column_aggregationの場合のキーカラム||
|**key_value**|[boolean, number, string, integer]|column_aggregationの場合のキーの値||
---------------------------------------
<a id="#/definitions/InheritanceType"></a>
## Inheritance Type




**any of the following**

* `simple`(単一テーブル継承 子テーブルのカラムも含めたすべてのカラムを親となるテーブルに格納する)
* `concrete`(具象テーブル継承 子クラスごとに共通のカラムとそれぞれのモデルのカラムをすべて含んだ状態で独立したテーブルを作成する)
* `column_aggregation`(カラム集約テーブル継承 単一テーブル継承と似ているが、型を特定するための _type カラムがある)

---------------------------------------
<a id="#/definitions/ColumnTypeOrDef"></a>
## Column Type Or Def




**any of the following**

* [ColumnDef](##/definitions/ColumnDef)
* [ColumnSubsetType](##/definitions/ColumnSubsetType)

---------------------------------------
<a id="#/definitions/ColumnDef"></a>
## Column Def



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**title**|string|||
|**comment**|string|||
|**type**|[ColumnType](##/definitions/ColumnType)||Yes|
|**signed**|boolean|指定がない場合はunsigned||
|**not_null**|boolean|指定がない場合はnullable||
|**primary**|boolean|||
|**auto_increment**|[AutoIncrement](##/definitions/AutoIncrement)|||
|**length**|integer|||
|**collate**|string|||
|**not_serializable**|boolean|serializeに出力しない（パスワード等保護用）||
|**precision**|integer|||
|**scale**|integer|||
|**time_zone**|[TimeZone](##/definitions/TimeZone)|||
|**enum_values**|Array<[EnumValue](##/definitions/EnumValue)>|列挙型の値||
|**db_enum_values**|Array<[DbEnumValue](##/definitions/DbEnumValue)>|DBの列挙型を使用する場合の値||
|**enum_model**|string|スキーマ内で定義された列挙値名　（名前は::区切り）||
|**json_class**|string|Json型で使用する型名||
|**exclude_from_cache**|boolean|キャッシュからの除外設定||
|**skip_factory**|boolean|factoryからの除外設定||
|**rename**|string|カラム名の別名設定||
|**srid**|integer|Point型のSRID||
|**default**|string|||
|**sql_comment**|string|||
---------------------------------------
<a id="#/definitions/ColumnType"></a>
## Column Type




**Allowed values**

* `tinyint`
* `smallint`
* `int`
* `bigint`
* `float`
* `double`
* `varchar`
* `boolean`
* `text`
* `blob`
* `timestamp`
* `datetime`
* `date`
* `time`
* `decimal`
* `array_int`
* `array_string`
* `json`
* `enum`
* `db_enum`
* `db_set`
* `point`

---------------------------------------
<a id="#/definitions/AutoIncrement"></a>
## Auto Increment




**Allowed values**

* `auto`

---------------------------------------
<a id="#/definitions/DbEnumValue"></a>
## DB Enum Value



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**name**|string||Yes|
|**title**|string|||
|**comment**|string|||
---------------------------------------
<a id="#/definitions/ColumnSubsetType"></a>
## Column Subset Type




**Allowed values**

* `tinyint`
* `smallint`
* `int`
* `bigint`
* `float`
* `double`
* `varchar`
* `boolean`
* `text`
* `blob`
* `datetime`
* `date`
* `time`
* `decimal`
* `array_int`
* `array_string`
* `json`
* `tinyint_not_null`
* `smallint_not_null`
* `int_not_null`
* `bigint_not_null`
* `float_not_null`
* `double_not_null`
* `varchar_not_null`
* `boolean_not_null`
* `text_not_null`
* `blob_not_null`
* `datetime_not_null`
* `date_not_null`
* `time_not_null`
* `decimal_not_null`
* `array_int_not_null`
* `array_string_not_null`
* `json_not_null`

---------------------------------------
<a id="#/definitions/RelDef"></a>
## Relation Def



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**title**|string|||
|**comment**|string|||
|**model**|string|結合先のモデル　他のグループは::区切りで指定||
|**type**|[RelationsType](##/definitions/RelationsType)|||
|**local**|string|結合するローカルのカラム名||
|**foreign**|string|結合先のカラム名||
|**in_cache**|boolean|manyあるいはone_to_oneの場合にリレーション先も一緒にキャッシュするか 結合深さは1代のみで子テーブルは親に含んだ状態で更新する必要がある||
|**raw_cond**|string|||
|**order_by**|string|||
|**desc**|boolean|||
|**limit**|integer|||
|**use_cache**|boolean|||
|**use_cache_with_trashed**|boolean|リレーション先が論理削除されていてもキャッシュを取得する||
|**on_delete**|[ReferenceOption](##/definitions/ReferenceOption)|DBの外部キー制約による削除およびソフトウェア側での削除制御||
|**on_update**|[ReferenceOption](##/definitions/ReferenceOption)|DBの外部キー制約による更新||
---------------------------------------
<a id="#/definitions/RelationsType"></a>
## Relations Type




**Allowed values**

* `many`
* `one`
* `one_to_one`

---------------------------------------
<a id="#/definitions/ReferenceOption"></a>
## Reference Option




**Allowed values**

* `restrict`
* `cascade`
* `set_null`
* `set_zero`

---------------------------------------
<a id="#/definitions/IndexDef"></a>
## Index Def



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**fields**|Map<property, [IndexFieldDef](##/definitions/IndexFieldDef)>|||
|**type**|[IndexType](##/definitions/IndexType)|||
|**parser**|[Parser](##/definitions/Parser)|||
---------------------------------------
<a id="#/definitions/IndexFieldDef"></a>
## Index Field Def



**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**sorting**|[SortType](##/definitions/SortType)|||
|**length**|integer|||
---------------------------------------
<a id="#/definitions/SortType"></a>
## Sort Type




**Allowed values**

* `asc`
* `desc`

---------------------------------------
<a id="#/definitions/IndexType"></a>
## Index Type




**Allowed values**

* `index`
* `unique`
* `fulltext`
* `spatial`

---------------------------------------
<a id="#/definitions/Parser"></a>
## Parser




**Allowed values**

* `ngram`
* `mecab`

---------------------------------------
<a id="#/definitions/History"></a>
## History

更新履歴

**Properties**

|   |Type|Description|Required|
|---|---|---|---|
|**date**|string|更新日|Yes|
|**description**|string|変更内容|Yes|
|**author**|string|担当者||
|**version**|string|バージョン||

