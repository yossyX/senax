use serde_json::Value;
use tera::{Context, Tera};

pub fn gen_schema_md(schema: Value) -> Result<String, anyhow::Error> {
    let tpl = r##"
# {{ title }}
{% for key, value in properties %}
{%- if key == "history" %}
* [properties/{{key}}](##/definitions/History)
{%- else %}
* [properties/{{key}}](#{{value.additionalProperties["$ref"]}})
{%- endif %}
{%- endfor %}
{% for key, value in definitions %}
---------------------------------------
<a id="#/definitions/{{key}}"></a>
## {{value.title}}

{{value.description | default(value="")}}

{% if value.properties -%}
**Properties**

|   |Type|Description|Required|
|---|---|---|---|
{%- for prop_name, prop in value.properties %}
|**{{prop_name}}**|
{%- if prop["$ref"] %}[{{prop["$ref"] | split(pat="/") | last}}](#{{prop["$ref"]}})
{%- elif prop.items %}Array<[{{prop.items["$ref"] | split(pat="/") | last}}](#{{prop.items["$ref"]}})>
{%- elif prop.allOf %}[{{prop.allOf.0["$ref"] | split(pat="/") | last}}](#{{prop.allOf.0["$ref"]}})
{%- elif prop.anyOf %}{% for anyOf in prop.anyOf %}[{{anyOf["$ref"] | split(pat="/") | last}}](#{{anyOf["$ref"]}}){% endfor %}
{%- elif prop.additionalProperties["$ref"] %}Map<property, [{{prop.additionalProperties["$ref"] | split(pat="/") | last}}](#{{prop.additionalProperties["$ref"]}})>
{%- elif prop.additionalProperties.allOf.0["$ref"] %}Map<property, [{{prop.additionalProperties.allOf.0["$ref"] | split(pat="/") | last}}](#{{prop.additionalProperties.allOf.0["$ref"]}})>
{%- else %}{{ prop.type }}{% endif %}|{{prop.description | default(value="")}}|{% if value.required and prop_name in value.required %}Yes{% endif %}|
{%- endfor %}
{%- endif %}
{%- if value.enum %}
**Allowed values**

{% for enum in value.enum -%}
* `{{enum}}`
{% endfor %}
{%- endif %}
{%- if value.anyOf %}
**any of the following**

{% for anyOf in value.anyOf -%}
* [{{anyOf["$ref"] | split(pat="/") | last}}](#{{anyOf["$ref"]}})
{% endfor %}
{%- endif %}
{%- if value.oneOf %}
**any of the following**

{% for oneOf in value.oneOf -%}
{% for enum in oneOf.enum -%}
* `{{enum}}` {%- if oneOf.description %}({{oneOf.description}}){%- endif %}
{% endfor -%}
{% endfor %}
{%- endif %}
{%- endfor %}
"##;
    let context = Context::from_serialize(&schema)?;
    let result = Tera::one_off(tpl, &context, false)?;
    Ok(result)
}
