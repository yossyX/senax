---
sidebar_position: 4
---
# GraphQL API 定義
SOBO-WEBの業務DBの GraphQL API 定義を記載する。

## 権限一覧
<table>
    <thead>
        <tr>
            <th class="name">内部名</th>
            <th class="name">権限名</th>
        </tr>
    </thead>
    {%- for name, def in api_def.roles %}
    <tr>
        <td class="name">{{name}}</td>
        <td class="name">{{def | get(key="alias", default="")}}</td>
    </tr>
    {%- endfor %}
</table>

## API一覧
{%- for group in api_def.groups %}
{%- if api_def.groups | length > 1 %}

## {{group.group_def.label | default(value=group.name)}}
{%- endif %}
{%- for model in group.models %}

### {{model.name}}{% if model.label %} ({{model.label}}){% endif %}

#### 権限設定
参照：{{ model.readable_roles | join(sep=", ") }}  
登録：{{ model.creatable_roles | join(sep=", ") }}  
インポート：{{ model.importable_roles | join(sep=", ") }}  
更新：{{ model.updatable_roles | join(sep=", ") }}  
削除：{{ model.deletable_roles | join(sep=", ") }}  

{%- if model.readable_filter or model.updatable_filter or model.deletable_filter %}

#### フィルタ設定
{%- if model.readable_filter %}
参照
```
{{model.readable_filter}}
```  
{%- endif %}
{%- if model.updatable_filter %}
更新
```
{{model.updatable_filter}}
```  
{%- endif %}
{%- if model.deletable_filter %}
削除
```
{{model.deletable_filter}}
```  
{%- endif %}
{%- endif %}

#### Query
{%- if model.has_all_query %}
* 全行取得：{ {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { all { <a href="#{{model.gql_name | lower}}">{{model.gql_name}}</a> } } } } }
{%- endif %}
{%- if api_def.use_find_by_pk %}
* 主キー検索：{ {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { findByPk({{model.pk}}) { <a href="#{{model.gql_name | lower}}">{{model.gql_name}}</a> } } } } }
{%- endif %}
* GraphQL ID検索：{ {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { find(_id: ID!) { <a href="#{{model.gql_name | lower}}">{{model.gql_name}}</a> } } } } }
{%- for selector_def in model.selectors %}
* 検索：{ {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { {{selector_def.name | gql_camel}}(filter: <a href="#{{model.gql_name | lower}}query{{selector_def.name | lower}}filter">{{model.gql_name}}Query{{selector_def.name | pascal}}Filter</a>, order: <a href="#{{model.gql_name | lower}}query{{selector_def.name | lower}}order">{{model.gql_name}}Query{{selector_def.name | pascal}}Order</a>, limit: Int, offset: Int) { nodes { <a href="#{{model.gql_name | lower}}">{{model.gql_name}}</a> } } } } } }
* カウント：{ {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { count{{selector_def.name | gql_pascal}}(filter: <a href="#{{model.gql_name | lower}}query{{selector_def.name | lower}}filter">{{model.gql_name}}Query{{selector_def.name | pascal}}Filter</a>) } } } }
{%- endfor %}
{%- if not api_def.disable_mutation %}

#### Mutation
* 登録：mutation { {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { create(data: <a href="#{{model.gql_name | lower}}">{{model.gql_name}}</a>) } } } }
{%- if not def.disable_update %}
{%- if api_def.use_import %}
* インポート：{ {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { import(list: [<a href="#{{model.gql_name | lower}}">{{model.gql_name}}</a>]) } } } }
{%- endif %}
* 更新：mutation { {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { update(data: <a href="#{{model.gql_name | lower}}">{{model.gql_name}}</a>) } } } }
* 削除：mutation { {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { delete(_id: ID!) } } } }
{%- for selector_def in model.selectors %}
{% for js_name, js_def in selector_def.js_updater %}
* 更新：mutation { {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { update{{js_name | gql_pascal}}(filter: <a href="#{{model.gql_name | lower}}query{{selector_def.name | lower}}filter">{{model.gql_name}}Query{{selector_def.name | pascal}}Filter</a>, value: JSON!) } } } }
{%- endfor %}
{%- if selector_def.use_for_update_by_operator %}
* 更新：mutation { {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { updateBy{{selector_def.name | gql_pascal}}(filter: <a href="#{{model.gql_name | lower}}query{{selector_def.name | lower}}filter">{{model.gql_name}}Query{{selector_def.name | pascal}}Filter</a>, operator: JSON!) } } } }
{%- endif %}
{%- if selector_def.use_for_delete %}
* 削除：mutation { {{api_def.cased_db_name}} { {{group.cased_name}} { {{model.cased_name}} { deleteBy{{selector_def.name | gql_pascal}}(filter: <a href="#{{model.gql_name | lower}}query{{selector_def.name | lower}}filter">{{model.gql_name}}Query{{selector_def.name | pascal}}Filter</a>) } } } }
{%- endif %}
{%- endfor %}
{%- endif %}
{%- endif %}

#### {{model.gql_name}}
<table>
    <thead>
        <tr>
            <th class="no narrow">No.</th>
            <th class="name" colspan="100">物理名</th>
            <th class="name">論理名</th>
            <th class="type">データ型</th>
            <th class="null narrow">必須</th>
            <th class="">注記</th>
        </tr>
    </thead>
    <tr>
        <td class="no center">1</td>
        <td class="name" colspan="100">_id</td>
        <td class="name">GraphQL ID</td>
        <td class="type">ID</td>
        <td class="null center"></td>
        <td class=""></td>
    </tr>
    {%- for field in model.all_fields %}
    <tr>
        <td class="no center">{{loop.index + 1}}</td>
        {%- for i in range(end=field.indent) %}
        <td></td>
        {%- endfor %}
        <td class="name" colspan="{{100 - field.indent}}">{{field.cased_name}}{% if field.has_many %}[]{% endif %}</td>
        <td class="name">{{field.label}}</td>
        <td class="type">{{field.gql_type}}</td>
        <td class="null center">{% if field.required %}{% raw %}<div style={{textAlign: 'center'}}>Y</div>{% endraw %}{% endif %}</td>
        <td class="">{% if field.no_update %}参照のみ {% elif field.no_read %}{% if field.disable_update %}登録のみ {%- else %}更新のみ {%- endif %}{%- else %}{% if field.disable_update %}登録後の更新不可 {%- endif %}{% endif %}{% if field.replace %}更新時置換{% endif %} </td>
    </tr>
    {%- endfor %}
</table>
{%- for selector_def in model.selectors %}

#### {{model.gql_name}}Query{{selector_def.name | pascal}}Filter
<table>
    <thead>
        <tr>
            <th class="no narrow">No.</th>
            <th class="name" colspan="100">名前</th>
            <th class="type">タイプ</th>
            <th class="null narrow">必須</th>
            <th class="name">フィールド</th>
            <th class="name">リレーション</th>
        </tr>
    </thead>
    {%- for filter in selector_def.filters %}
    <tr>
        <td class="no center">{{loop.index}}</td>
        {%- for i in range(end=filter.indent) %}
        <td></td>
        {%- endfor %}
        <td class="name" colspan="{{100 - filter.indent}}">{{filter.name}}</td>
        <td class="type">{{filter.type}}</td>
        <td class="null center">{% if filter.required %}{% raw %}<div style={{textAlign: 'center'}}>Y</div>{% endraw %}{% endif %}</td>
        <td class="name">{{filter.fields | join(sep=", ")}}</td>
        <td class="name">{{filter.relation}}</td>
    </tr>
    {%- endfor %}
</table>
{%- endfor %}
{%- for selector_def in model.selectors %}

#### {{model.gql_name}}Query{{selector_def.name | pascal}}Order
<table>
    <thead>
        <tr>
            <th class="no narrow">No.</th>
            <th class="name">名前</th>
            <th class="name">フィールド</th>
            <th class="">方向</th>
        </tr>
    </thead>
    {%- for name, def in selector_def.orders %}
    <tr>
        <td class="no center">{{loop.index}}</td>
        <td class="name">{{name | upper_snake}}</td>
        <td class="name">{{def.fields | join(sep=", ")}}</td>
        <td class="name">{% if def.direction == "asc" %}昇順{% endif %}{% if def.direction == "desc" %}降順{% endif %}</td>
    </tr>
    {%- endfor %}
</table>
{%- endfor %}
{%- endfor %}
{%- endfor %}