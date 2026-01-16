use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{_to_ident_name, FieldDef, ModelDef};
use crate::common::ToCase as _;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### フィルタタイプ
pub enum FilterType {
    /// 範囲検索（eq, lt, lte, gt, gte, is_null, is_not_null, is_null_or_lt, is_null_or_lte, is_null_or_gt, is_null_or_gte）
    Range,
    /// ID検索（eq, in, is_null, is_not_null）
    Identity,
    /// リレーション検索（外側から先に絞り込み）
    Exists,
    /// リレーション検索（内側から先に絞り込み）
    EqAny,
    /// 全文検索(MySQLのみ)
    FullText,
    /// 数値配列検索
    ArrayInt,
    /// 文字列配列検索
    ArrayString,
    /// JSON検索
    Json,
    /// 地理検索
    Geometry,
    /// SQL記述検索
    RawQuery,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### フィルタ定義
pub struct FilterDef {
    /// ### フィルタタイプ
    #[serde(rename = "type")]
    pub _type: FilterType,
    /// ### フィールド
    /// 複数の場合は、該当する複数カラムインデックスが必要
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub fields: IndexMap<String, Option<()>>,
    /// ### 必須
    #[serde(default, skip_serializing_if = "crate::schema::is_false")]
    pub required: bool,
    /// ### リレーション
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    /// ### リレーションフィールド
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub relation_fields: IndexMap<String, FilterDef>,
    /// ### JSONパス
    /// 省略時はクエリでの指定が必要となる。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_path: Option<String>,
    /// ### SQLクエリー
    /// typeがraw_queryの場合にSQLを記述する。リクエストパラメータを挿入するプレースホルダは"?"を使用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### フィルタ定義
pub struct FilterJson {
    /// ### フィルタ名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### フィルタタイプ
    #[serde(rename = "type")]
    pub _type: FilterType,
    /// ### フィールド
    /// 複数の場合は該当する複数カラムインデックスが必要
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    /// ### 必須
    #[serde(default, skip_serializing_if = "crate::schema::is_false")]
    pub required: bool,
    /// ### リレーション
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    /// ### リレーションフィールド
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relation_fields: Vec<FilterJson>,
    /// ### JSONパス
    /// 省略時はクエリでの指定が必要となる。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_path: Option<String>,
    /// ### SQLクエリー
    /// typeがraw_queryの場合にSQLを記述する。リクエストパラメータを挿入するプレースホルダは"?"を使用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

impl From<FilterDef> for FilterJson {
    fn from(value: FilterDef) -> Self {
        Self {
            name: String::new(),
            _type: value._type,
            fields: if value.fields.is_empty() {
                None
            } else {
                Some(value.fields.into_keys().collect::<Vec<_>>())
            },
            required: value.required,
            relation: value.relation,
            relation_fields: value
                .relation_fields
                .into_iter()
                .map(|(k, v)| {
                    let mut v: FilterJson = v.into();
                    v.name = k;
                    v
                })
                .collect(),
            json_path: value.json_path,
            query: value.query,
        }
    }
}

impl From<FilterJson> for FilterDef {
    fn from(value: FilterJson) -> Self {
        Self {
            _type: value._type,
            fields: value
                .fields
                .unwrap_or_default()
                .into_iter()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .map(|v| (v, None))
                .collect::<IndexMap<_, _>>(),
            required: value.required,
            relation: value.relation,
            relation_fields: value
                .relation_fields
                .into_iter()
                .map(|v| (v.name.clone(), v.into()))
                .collect(),
            json_path: value.json_path,
            query: value.query,
        }
    }
}

impl FilterDef {
    fn fields(&self, model: &ModelDef) -> Vec<(String, FieldDef)> {
        let mut v = Vec::new();
        for (field, _) in &self.fields {
            if self._type != FilterType::Exists && self._type != FilterType::EqAny {
                let t = model.merged_fields.get(field).unwrap_or_else(|| {
                    error_exit!(
                        "The {} column specified for selectors in the {} model does not exist.",
                        field,
                        model.name
                    )
                });
                let mut t = t.clone();
                t.not_null = true;
                v.push((field.to_string(), t));
            }
        }
        v
    }
    fn query_param_num(&self) -> usize {
        if let Some(query) = &self.query {
            let query = remove_quoted_sections(query);
            query.as_bytes().iter().filter(|c| **c == b'?').count()
        } else {
            0
        }
    }
    pub fn has_default(&self) -> bool {
        match self._type {
            FilterType::Exists if self.relation_fields.is_empty() => false,
            FilterType::EqAny if self.relation_fields.is_empty() => false,
            FilterType::FullText => false,
            FilterType::Geometry => false,
            FilterType::RawQuery if self.query_param_num() == 0 => false,
            _ => true,
        }
    }
    pub fn type_str(
        &self,
        filter: &str,
        pascal_name: &str,
        selector: &str,
        nested_name: &str,
    ) -> String {
        let filter_name = filter.to_pascal();
        let ty = match self._type {
            FilterType::Range => format!(
                "{}Query{}Range{}_{}",
                pascal_name,
                selector.to_pascal(),
                nested_name,
                filter_name
            ),
            FilterType::Identity => format!(
                "{}Query{}Identity{}_{}",
                pascal_name,
                selector.to_pascal(),
                nested_name,
                filter_name
            ),
            FilterType::Exists if !self.relation_fields.is_empty() => format!(
                "{}Query{}{}_{}Filter",
                pascal_name,
                selector.to_pascal(),
                nested_name,
                filter_name
            ),
            FilterType::Exists => "bool".to_string(),
            FilterType::EqAny if !self.relation_fields.is_empty() => format!(
                "{}Query{}{}_{}Filter",
                pascal_name,
                selector.to_pascal(),
                nested_name,
                filter_name
            ),
            FilterType::EqAny => "bool".to_string(),
            FilterType::FullText => "String".to_string(),
            FilterType::ArrayInt => "domain::models::ArrayIntFilter".to_string(),
            FilterType::ArrayString => "domain::models::ArrayStringFilter".to_string(),
            FilterType::Json if self.json_path.is_none() => {
                "domain::models::JsonValueWithPathFilter".to_string()
            }
            FilterType::Json => "domain::models::JsonValueFilter".to_string(),
            FilterType::Geometry => "Vec<domain::models::GeometryFilter>".to_string(),
            FilterType::RawQuery if self.query_param_num() == 0 => "bool".to_string(),
            FilterType::RawQuery if self.query_param_num() == 1 => "String".to_string(),
            FilterType::RawQuery => "Vec<String>".to_string(),
        };
        if self.required {
            ty
        } else {
            format!("Option<{ty}>")
        }
    }
    pub fn emu_str(&self, name: &str, model: &ModelDef) -> String {
        let vname = _to_ident_name(name);
        let mut cmp = String::new();
        let mut eq_vec = vec![];
        let mut null_ng_vec = vec!["false".to_string()];
        let mut null_ok_vec = vec!["true".to_string()];
        if self._type != FilterType::Exists
            && self._type != FilterType::EqAny
            && self._type != FilterType::RawQuery
        {
            for (field, _) in &self.fields {
                let t = model.merged_fields.get(field).unwrap_or_else(|| {
                    error_exit!(
                        "The {} column specified for selectors in the {} model does not exist.",
                        field,
                        model.name
                    )
                });
                let unwrap = if !t.not_null {
                    null_ng_vec.push(format!("v.{}().is_none()", _to_ident_name(field)));
                    null_ok_vec.push(format!("v.{}().is_some()", _to_ident_name(field)));
                    ".unwrap()"
                } else {
                    ""
                };
                let unwrap_arc = if t.id_class.is_none()
                    && t.enum_class.is_none()
                    && t.rel.is_none()
                    && t.outer_db_rel.is_none()
                    && t.is_arc()
                {
                    ".as_ref()"
                } else {
                    ""
                };

                let f = _to_ident_name(field);
                let s = if self.fields.len() == 1 {
                    format!("PartialOrdering_::from(v.{}(){}{}.partial_cmp(p))", f, unwrap, unwrap_arc)
                } else {
                    format!("PartialOrdering_::from(v.{}(){}{}.partial_cmp(&p.{}))", f, unwrap, unwrap_arc, f)
                };
                if cmp.is_empty() {
                    cmp.push_str(&s);
                } else {
                    cmp.push_str(".then_with(|| ");
                    cmp.push_str(&s);
                    cmp.push(')');
                }
                if self.fields.len() == 1 {
                    eq_vec.push(format!("v.{}(){}{}.eq(p)", f, unwrap, unwrap_arc))
                } else {
                    eq_vec.push(format!("v.{}(){}{}.eq(&p.{})", f, unwrap, unwrap_arc, f))
                }
            }
        }
        let eq = eq_vec.join(" && ");
        let null_ng = null_ng_vec.join(" || ");
        let null_ok = null_ok_vec.join(" && ");
        let prefix = if self.required {
            format!(
                "{{
        let f = &filter.{vname};"
            )
        } else {
            format!("if let Some(f) = &filter.{vname} {{")
        };
        match self._type {
            FilterType::Range => {
                format!(
                    "
    {prefix}
        if let Some(p) = &f.eq {{
            if {null_ng} || {cmp} != PartialOrdering_::Equal {{
                return false;
            }}
        }}
        if let Some(p) = &f.lt {{
            if {null_ng} || {cmp} != PartialOrdering_::Less {{
                return false;
            }}
        }}
        if let Some(p) = &f.lte {{
            if {null_ng} || {cmp} == PartialOrdering_::Greater {{
                return false;
            }}
        }}
        if let Some(p) = &f.gt {{
            if {null_ng} || {cmp} != PartialOrdering_::Greater {{
                return false;
            }}
        }}
        if let Some(p) = &f.gte {{
            if {null_ng} || {cmp} == PartialOrdering_::Less {{
                return false;
            }}
        }}
        if f.is_null.unwrap_or_default() && {null_ok} {{
            return false;
        }}
        if f.is_not_null.unwrap_or_default() && ({null_ng}) {{
            return false;
        }}
        if let Some(p) = &f.is_null_or_lt {{
            if {null_ok} && {cmp} != PartialOrdering_::Less {{
                return false;
            }}
        }}
        if let Some(p) = &f.is_null_or_lte {{
            if {null_ok} && {cmp} == PartialOrdering_::Greater {{
                return false;
            }}
        }}
        if let Some(p) = &f.is_null_or_gt {{
            if {null_ok} && {cmp} != PartialOrdering_::Greater {{
                return false;
            }}
        }}
        if let Some(p) = &f.is_null_or_gte {{
            if {null_ok} && {cmp} == PartialOrdering_::Less {{
                return false;
            }}
        }}
    }}"
                )
            }
            FilterType::Identity => {
                format!(
                    "
    {prefix}
        if let Some(p) = &f.eq {{
            if {null_ng} || !({eq}) {{
                return false;
            }}
        }}
        if let Some(p) = &f.r#in {{
            if {null_ng} || !p.iter().any(|p| {eq}) {{
                return false;
            }}
        }}
        if f.is_null.unwrap_or_default() && {null_ok} {{
            return false;
        }}
        if f.is_not_null.unwrap_or_default() && ({null_ng}) {{
            return false;
        }}
    }}"
                )
            }
            FilterType::Exists => "\n    // TODO implement FilterType::Exists".to_string(), // TODO implement
            FilterType::EqAny => "\n    // TODO implement FilterType::EqAny".to_string(), // TODO implement
            FilterType::FullText => {
                let mut v = Vec::new();
                for (field, _) in &self.fields {
                    let t = model.merged_fields.get(field).unwrap_or_else(|| {
                        error_exit!(
                            "The {} column specified for selectors in the {} model does not exist.",
                            field,
                            model.name
                        )
                    });
                    if t.not_null {
                        v.push(format!("!v.{}().contains(p)", _to_ident_name(field)));
                    } else {
                        v.push(format!(
                            "!v.{}().map(|v| v.contains(p)).unwrap_or_default()",
                            _to_ident_name(field)
                        ));
                    }
                }
                format!(
                    "
    {prefix}
        for p in f.split(char::is_whitespace) {{
            if {} {{
                return false;
            }}
        }}
    }}",
                    v.join(" && ")
                )
            }
            // TODO implement
            FilterType::Geometry => "".to_string(),
            FilterType::ArrayInt => "".to_string(),
            FilterType::ArrayString => "".to_string(),
            FilterType::Json => "".to_string(),
            FilterType::RawQuery => "".to_string(),
        }
    }
    pub fn db_str(&self, name: &str, model: &ModelDef, suffix: &str) -> String {
        let vname = _to_ident_name(name);
        let mut cols = Vec::new();
        let mut is_null_vec = vec!["(BOOLEAN false)".to_string()];
        let mut not_null_vec = vec!["(BOOLEAN true)".to_string()];
        if self._type != FilterType::Exists
            && self._type != FilterType::EqAny
            && self._type != FilterType::RawQuery
        {
            for (field, _) in &self.fields {
                cols.push(field.clone());
                let t = model.merged_fields.get(field).unwrap_or_else(|| {
                    error_exit!(
                        "The {} column specified for selectors in the {} model does not exist.",
                        field,
                        model.name
                    )
                });
                if !t.not_null {
                    is_null_vec.push(format!("({field} IS NULL)"));
                    not_null_vec.push(format!("({field} IS NOT NULL)"));
                }
            }
        }
        let cols = if cols.len() > 1 {
            format!("({})", &cols.join(", "))
        } else {
            cols.join(", ")
        };
        let is_null = is_null_vec.join(" OR ");
        let not_null = not_null_vec.join(" AND ");
        let prefix = if self.required {
            format!(
                "{{
        let f = &filter.{vname};"
            )
        } else {
            format!("if let Some(f) = &filter.{vname} {{")
        };
        match self._type {
            FilterType::Range => {
                let values = if self.fields.len() == 1 {
                    ""
                } else {
                    ".values()"
                };
                format!(
                    "
    {prefix}
        if let Some(p) = &f.eq {{
            fltr = fltr.and(filter!({cols} = p{values}));
        }}
        if let Some(p) = &f.lt {{
            fltr = fltr.and(filter!({cols} < p{values}));
        }}
        if let Some(p) = &f.lte {{
            fltr = fltr.and(filter!({cols} <= p{values}));
        }}
        if let Some(p) = &f.gt {{
            fltr = fltr.and(filter!({cols} > p{values}));
        }}
        if let Some(p) = &f.gte {{
            fltr = fltr.and(filter!({cols} >= p{values}));
        }}
        if f.is_null.unwrap_or_default() {{
            fltr = fltr.and(filter!({is_null}));
        }}
        if f.is_not_null.unwrap_or_default() {{
            fltr = fltr.and(filter!({not_null}));
        }}
        if let Some(p) = &f.is_null_or_lt {{
            fltr = fltr.and(filter!(({is_null}) OR ({cols} < p{values})));
        }}
        if let Some(p) = &f.is_null_or_lte {{
            fltr = fltr.and(filter!(({is_null}) OR ({cols} <= p{values})));
        }}
        if let Some(p) = &f.is_null_or_gt {{
            fltr = fltr.and(filter!(({is_null}) OR ({cols} > p{values})));
        }}
        if let Some(p) = &f.is_null_or_gte {{
            fltr = fltr.and(filter!(({is_null}) OR ({cols} >= p{values})));
        }}
    }}"
                )
            }
            FilterType::Identity => {
                let values = if self.fields.len() == 1 {
                    ""
                } else {
                    ".values()"
                };
                format!(
                    "
    {prefix}
        if let Some(p) = &f.eq {{
            fltr = fltr.and(filter!({cols} = p{values}));
        }}
        if let Some(p) = &f.r#in {{
            let p: Vec<_> = p.iter().map(|v| v{values}).collect();
            fltr = fltr.and(filter!({cols} IN p));
        }}
        if f.is_null.unwrap_or_default() {{
            fltr = fltr.and(filter!({is_null}));
        }}
        if f.is_not_null.unwrap_or_default() {{
            fltr = fltr.and(filter!({not_null}));
        }}
    }}"
                )
            }
            FilterType::Exists => {
                let relation = self.relation.as_deref().unwrap_or(name);
                let _relation = _to_ident_name(relation);
                let group = _to_ident_name(&model.group_name.to_snake());
                let mod_name = model.name.to_snake();
                if self.relation_fields.is_empty() {
                    format!(
            "
    {prefix}
        fltr = fltr.and(base_repository::repositories::{group}::_base::_{mod_name}::Filter_::Exists(base_repository::repositories::{group}::_base::_{mod_name}::ColRel_::{_relation}(None)));
    }}"
            )
                } else {
                    format!(
            "
    {prefix}
        fltr = fltr.and(base_repository::repositories::{group}::_base::_{mod_name}::Filter_::Exists(base_repository::repositories::{group}::_base::_{mod_name}::ColRel_::{_relation}(Some(Box::new(_filter{suffix}_{name}(f)?)))));
    }}"
            )
                }
            }
            FilterType::EqAny => {
                let relation = self.relation.as_deref().unwrap_or(name);
                let _relation = _to_ident_name(relation);
                let group = _to_ident_name(&model.group_name.to_snake());
                let mod_name = model.name.to_snake();
                if self.relation_fields.is_empty() {
                    format!(
            "
    {prefix}
        fltr = fltr.and(base_repository::repositories::{group}::_base::_{mod_name}::Filter_::EqAny(base_repository::repositories::{group}::_base::_{mod_name}::ColRel_::{_relation}(None)));
    }}"
            )
                } else {
                    format!(
            "
    {prefix}
        fltr = fltr.and(base_repository::repositories::{group}::_base::_{mod_name}::Filter_::EqAny(base_repository::repositories::{group}::_base::_{mod_name}::ColRel_::{_relation}(Some(Box::new(_filter{suffix}_{name}(f)?)))));
    }}"
            )
                }
            }
            FilterType::FullText => {
                let mut v = Vec::new();
                for (field, _) in &self.fields {
                    v.push(field.to_string());
                }
                format!(
                    r#"
    {prefix}
        let f = senax_common::fulltext::parse(f).db_query();
        fltr = fltr.and(filter!(MATCH ({}) AGAINST (f) IN BOOLEAN MODE));
    }}"#,
                    v.join(", ")
                )
            }
            FilterType::Geometry => {
                let (field, _) = self.fields.first().unwrap();
                format!(
                    "
    {prefix}
        for p in f {{
            match &p.r#type {{
                domain::models::GeometryFilterType::Equals => {{
                    fltr = fltr.and(filter!({field} GEO_EQUALS p.area.clone()));
                }}
                domain::models::GeometryFilterType::Within => {{
                    fltr = fltr.and(filter!({field} WITHIN p.area.clone()));
                }}
                domain::models::GeometryFilterType::Intersects => {{
                    fltr = fltr.and(filter!({field} INTERSECTS p.area.clone()));
                }}
                domain::models::GeometryFilterType::Crosses => {{
                    fltr = fltr.and(filter!({field} CROSSES p.area.clone()));
                }}
                domain::models::GeometryFilterType::DWithin => {{
                    let distance = p.distance.context(\"distance is required.\")?;
                    fltr = fltr.and(filter!({field} D_WITHIN p.area.clone(), distance));
                }}
                domain::models::GeometryFilterType::NotEquals => {{
                    fltr = fltr.and(filter!(NOT ({field} GEO_EQUALS p.area.clone())));
                }}
                domain::models::GeometryFilterType::NotWithin => {{
                    fltr = fltr.and(filter!(NOT ({field} WITHIN p.area.clone())));
                }}
                domain::models::GeometryFilterType::NotIntersects => {{
                    fltr = fltr.and(filter!(NOT ({field} INTERSECTS p.area.clone())));
                }}
                domain::models::GeometryFilterType::NotCrosses => {{
                    fltr = fltr.and(filter!(NOT ({field} CROSSES p.area.clone())));
                }}
                domain::models::GeometryFilterType::NotDWithin => {{
                    let distance = p.distance.context(\"distance is required.\")?;
                    fltr = fltr.and(filter!(NOT ({field} D_WITHIN p.area.clone(), distance)));
                }}
            }}
        }}
    }}"
                )
            }
            FilterType::ArrayInt | FilterType::ArrayString => {
                let (field, _) = self.fields.first().unwrap();
                format!(
                    "
    {prefix}
        if let Some(p) = &f.has {{
            fltr = fltr.and(filter!({field} HAS p));
        }}
        if let Some(p) = &f.contains {{
            fltr = fltr.and(filter!({field} CONTAINS p));
        }}
        if let Some(p) = &f.overlaps {{
            fltr = fltr.and(filter!({field} OVERLAPS p));
        }}
    }}"
                )
            }
            FilterType::Json => {
                let (field, _) = self.fields.first().unwrap();
                let json_path = if let Some(json_path) = &self.json_path {
                    format!("{:?}", json_path)
                } else {
                    "&f.path".to_string()
                };
                format!(
                    "
    {prefix}
        let json_path = {};
        if f.exists.unwrap_or_default() {{
            fltr = fltr.and(filter!({field} JSON_CONTAINS_PATH (json_path)));
        }}
        if let Some(p) = &f.eq {{
            fltr = fltr.and(filter!({field} -> (json_path) = p));
        }}
        if f.is_null.unwrap_or_default() {{
            fltr = fltr.and(filter!({field} -> (json_path) IS NULL));
        }}
        if f.is_not_null.unwrap_or_default() {{
            fltr = fltr.and(filter!({field} -> (json_path) IS NOT NULL));
        }}
        if let Some(p) = &f.r#in {{
            fltr = fltr.and(filter!({field} -> (json_path) IN p));
        }}
        if let Some(p) = &f.contains {{
            fltr = fltr.and(filter!({field} -> (json_path) CONTAINS p));
        }}
        if let Some(p) = &f.lt {{
            fltr = fltr.and(filter!({field} -> (json_path) < p));
        }}
        if let Some(p) = &f.lte {{
            fltr = fltr.and(filter!({field} -> (json_path) <= p));
        }}
        if let Some(p) = &f.gt {{
            fltr = fltr.and(filter!({field} -> (json_path) > p));
        }}
        if let Some(p) = &f.gte {{
            fltr = fltr.and(filter!({field} -> (json_path) >= p));
        }}
    }}",
                    json_path
                )
            }
            FilterType::RawQuery => {
                let query = self
                    .query
                    .as_ref()
                    .unwrap_or_else(|| panic!("query is required for {}.", vname));
                let param_num = self.query_param_num();
                if param_num == 1 {
                    format!(
                        "
    {prefix}
        fltr = fltr.and(filter!(RAW {:?}, [f]));
    }}",
                        query
                    )
                } else if param_num > 0 {
                    format!(
                        "
    {prefix}
        anyhow::ensure!(f.len() == {param_num}, \"Illegal number of {vname} parameters.\");
        fltr = fltr.and(filter!(RAW {:?}, f));
    }}",
                        query
                    )
                } else {
                    format!(
                        "
    {prefix}
        if *f {{
            fltr = fltr.and(filter!(RAW {:?}));
        }}
    }}",
                        query
                    )
                }
            }
        }
    }
}
fn remove_quoted_sections(input: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#""([^"\\]*(?:\\.[^"\\]*)*)"|'([^'\\]*(?:\\.[^'\\]*)*)'"#).unwrap()
    });
    RE.replace_all(input, "").to_string()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_quoted_sections() {
        let input = r#"some message "DDD" 'aaa'"#;
        let output = remove_quoted_sections(input);
        assert_eq!(output, "some message  ");

        let input = r#"some message "D\"DD" 'a\'aa'"#;
        let output = remove_quoted_sections(input);
        assert_eq!(output, "some message  ");

        let input = "some message without quotes";
        let output = remove_quoted_sections(input);
        assert_eq!(output, "some message without quotes");
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### ソート順序定義
pub struct OrderDef {
    /// ### フィールド
    /// カーソルを使用する場合は順序を厳密にするため最後に主キーを登録してください
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub fields: IndexMap<String, ()>,
    /// ### ソート方向
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<FilterSortDirection>,
    /// ### SQL直接記述
    /// ASCとDESCの組み合わせや、JOIN が必要なソートは ORDER BY に続くSQLを記述してください。
    /// JOINする場合のメインテーブル名は _t1 です。
    /// カーソルは使用できません。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_sql: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### ソートフィールド定義
pub struct OrderFieldJson {
    /// ### フィールド名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### ソート順序定義
pub struct OrderJson {
    /// ### ソート順序名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### フィールド
    /// カーソルを使用する場合は順序を厳密にするため最後に主キーを登録してください
    pub fields: Vec<OrderFieldJson>,
    /// ### ソート方向
    pub direction: Option<FilterSortDirection>,
    /// ### SQL直接記述
    /// ASCとDESCの組み合わせや、JOIN が必要なソートは ORDER BY に続くSQLを記述してください。
    /// JOINする場合のメインテーブル名は _t1 です。
    /// カーソルは使用できません。
    pub direct_sql: Option<String>,
}

impl From<OrderDef> for OrderJson {
    fn from(value: OrderDef) -> Self {
        Self {
            name: String::new(),
            fields: value
                .fields
                .into_iter()
                .map(|(k, _)| OrderFieldJson { name: k })
                .collect(),
            direction: value.direction,
            direct_sql: value.direct_sql,
        }
    }
}

impl From<OrderJson> for OrderDef {
    fn from(value: OrderJson) -> Self {
        Self {
            fields: value.fields.into_iter().map(|v| (v.name, ())).collect(),
            direction: value.direction,
            direct_sql: value.direct_sql,
        }
    }
}

impl OrderDef {
    pub fn type_str(&self, model: &ModelDef) -> String {
        if let Some(_direct_sql) = &self.direct_sql {
            return "()".to_string();
        }
        let mut v = Vec::new();
        for (field, _) in &self.fields {
            let t = model.merged_fields.get(field).unwrap_or_else(|| {
                error_exit!(
                    "The {} column specified for selectors in the {} model does not exist.",
                    field,
                    model.name
                )
            });
            let mut t = t.clone();
            t.not_null = true;
            v.push(t.get_filter_type(true));
        }
        format!("({})", &v.join(", "))
    }
    pub fn field_tuple(&self, model: &ModelDef) -> String {
        if let Some(_direct_sql) = &self.direct_sql {
            return "()".to_string();
        }
        let mut v = Vec::new();
        for (field, _) in &self.fields {
            let t = model.merged_fields.get(field).unwrap_or_else(|| {
                error_exit!(
                    "The {} column specified for selectors in the {} model does not exist.",
                    field,
                    model.name
                )
            });
            if t.is_copyable() {
                if t.not_null {
                    v.push(format!("_obj.{}()", _to_ident_name(field)));
                } else {
                    v.push(format!("_obj.{}()?", _to_ident_name(field)));
                }
            } else if t.not_null {
                v.push(format!("_obj.{}().clone()", _to_ident_name(field)));
            } else {
                v.push(format!("_obj.{}().clone()?", _to_ident_name(field)));
            }
        }
        format!("({})", &v.join(", "))
    }
    pub fn emu_str(&self, model: &ModelDef) -> String {
        if let Some(_direct_sql) = &self.direct_sql {
            return "
                        _ => {
                            return true;
                        }"
            .to_string();
        }
        let mut cmp = String::new();
        let mut null_chk_vec = Vec::new();
        for (idx, (field, _)) in (&self.fields).into_iter().enumerate() {
            let t = model.merged_fields.get(field).unwrap_or_else(|| {
                error_exit!(
                    "The {} column specified for selectors in the {} model does not exist.",
                    field,
                    model.name
                )
            });
            let unwrap = if !t.not_null {
                null_chk_vec.push(format!("v.{}().is_none()", _to_ident_name(field)));
                ".unwrap()"
            } else {
                ""
            };
            let unwrap_arc = if t.id_class.is_none()
                && t.enum_class.is_none()
                && t.rel.is_none()
                && t.outer_db_rel.is_none()
                && t.is_arc()
            {
                ".as_ref()"
            } else {
                ""
            };

            let f = _to_ident_name(field);
            let s = if self.fields.len() == 1 {
                format!("PartialOrdering_::from(v.{}(){}{}.partial_cmp(f))", f, unwrap, unwrap_arc)
            } else {
                format!("PartialOrdering_::from(v.{}(){}{}.partial_cmp(&f.{}))", f, unwrap, unwrap_arc, idx)
            };
            if cmp.is_empty() {
                cmp.push_str(&s);
            } else {
                cmp.push_str(".then_with(|| ");
                cmp.push_str(&s);
                cmp.push(')');
            }
        }
        let mut n_chk = String::new();
        if !null_chk_vec.is_empty() {
            let c = null_chk_vec.join(" || ");
            n_chk.push_str(&format!(
                "
                            if {c} {{
                                return false;
                            }}"
            ));
        }
        if self.direction == Some(FilterSortDirection::Desc) {
            format!(
                "
                        domain::models::Cursor::After(f) => {{{n_chk}
                            if {cmp} != PartialOrdering_::Less {{
                                return false;
                            }}
                        }}
                        domain::models::Cursor::Before(f) => {{{n_chk}
                            if {cmp} != PartialOrdering_::Greater {{
                                return false;
                            }}
                        }}"
            )
        } else {
            format!(
                "
                        domain::models::Cursor::After(f) => {{{n_chk}
                            if {cmp} != PartialOrdering_::Greater {{
                                return false;
                            }}
                        }}
                        domain::models::Cursor::Before(f) => {{{n_chk}
                            if {cmp} != PartialOrdering_::Less {{
                                return false;
                            }}
                        }}"
            )
        }
    }
    pub fn db_str(&self) -> String {
        if let Some(_direct_sql) = &self.direct_sql {
            return "
                        _ => {}"
                .to_string();
        }
        let mut cols = Vec::new();
        for (field, _) in &self.fields {
            cols.push(field.clone());
        }
        let cols = if cols.len() > 1 {
            format!("({})", &cols.join(", "))
        } else {
            cols.join(", ")
        };
        if self.direction == Some(FilterSortDirection::Desc) {
            format!(
                "
                        domain::models::Cursor::After(f) => {{
                            fltr = fltr.and(filter!({cols} < f));
                        }}
                        domain::models::Cursor::Before(f) => {{
                            fltr = fltr.and(filter!({cols} > f));
                        }}"
            )
        } else {
            format!(
                "
                        domain::models::Cursor::After(f) => {{
                            fltr = fltr.and(filter!({cols} > f));
                        }}
                        domain::models::Cursor::Before(f) => {{
                            fltr = fltr.and(filter!({cols} < f));
                        }}"
            )
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### フィルターソート方向
pub enum FilterSortDirection {
    /// ### 昇順
    Asc,
    /// ### 降順
    Desc,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### セレクタ定義
pub struct SelectorDef {
    /// ### フィルター
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub filters: IndexMap<String, FilterDef>,
    /// ### ソート順序
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub orders: IndexMap<String, OrderDef>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### セレクタ定義
pub struct SelectorJson {
    /// ### セレクタ名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### フィルター
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<FilterJson>,
    /// ### ソート順序
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orders: Vec<OrderJson>,
}

impl From<SelectorDef> for SelectorJson {
    fn from(value: SelectorDef) -> Self {
        Self {
            name: Default::default(),
            filters: value
                .filters
                .into_iter()
                .map(|(k, v)| {
                    let mut v: FilterJson = v.into();
                    v.name = k;
                    v
                })
                .collect(),
            orders: value
                .orders
                .into_iter()
                .map(|(k, v)| {
                    let mut v: OrderJson = v.into();
                    v.name = k;
                    v
                })
                .collect(),
        }
    }
}

impl From<SelectorJson> for SelectorDef {
    fn from(value: SelectorJson) -> Self {
        Self {
            filters: value
                .filters
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: FilterDef = v.into();
                    (name, v)
                })
                .collect(),
            orders: value
                .orders
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: OrderDef = v.into();
                    (name, v)
                })
                .collect(),
        }
    }
}

pub struct FilterMap {
    pub pascal_name: String,
    pub suffix: String,
    pub model: Arc<ModelDef>,
    pub filters: IndexMap<String, FilterDef>,
}

impl FilterMap {
    pub fn db(&self) -> &str {
        &self.model.db
    }
    pub fn model_group(&self) -> &str {
        &self.model.group_name
    }
    pub fn model_name(&self) -> &str {
        &self.model.name
    }
    pub fn ranges(
        &self,
        pascal_name: &str,
        selector: &str,
        nested_name: &str,
    ) -> Vec<(String, String)> {
        let mut vec = Vec::new();
        for (name, filter) in self.filters.iter() {
            if filter._type == FilterType::Range {
                let fields = filter.fields(&self.model);
                if fields.len() == 1 {
                    vec.push((name.to_pascal(), fields[0].1.get_filter_type(true)))
                } else {
                    vec.push((
                        name.to_pascal(),
                        format!(
                            "{}Query{}RangeValues{}_{}",
                            pascal_name,
                            selector.to_pascal(),
                            nested_name,
                            name.to_pascal(),
                        ),
                    ))
                };
            }
        }
        vec
    }
    pub fn range_tuples(&self) -> Vec<(String, Vec<(String, String)>)> {
        let mut vec = Vec::new();
        for (name, filter) in self.filters.iter() {
            if filter._type == FilterType::Range {
                let fields = filter.fields(&self.model);
                if fields.len() > 1 {
                    vec.push((
                        name.to_pascal(),
                        fields
                            .into_iter()
                            .map(|v| (v.0, v.1.get_filter_type(true)))
                            .collect(),
                    ))
                };
            }
        }
        vec
    }
    pub fn identities(
        &self,
        pascal_name: &str,
        selector: &str,
        nested_name: &str,
    ) -> Vec<(String, String)> {
        let mut vec = Vec::new();
        for (name, filter) in self.filters.iter() {
            if filter._type == FilterType::Identity {
                let fields = filter.fields(&self.model);
                if fields.len() == 1 {
                    vec.push((name.to_pascal(), fields[0].1.get_filter_type(true)))
                } else {
                    vec.push((
                        name.to_pascal(),
                        format!(
                            "{}Query{}IdentityValues{}_{}",
                            pascal_name,
                            selector.to_pascal(),
                            nested_name,
                            name.to_pascal(),
                        ),
                    ))
                };
            }
        }
        vec
    }
    pub fn identity_tuples(&self) -> Vec<(String, Vec<(String, String)>)> {
        let mut vec = Vec::new();
        for (name, filter) in self.filters.iter() {
            if filter._type == FilterType::Identity {
                let fields = filter.fields(&self.model);
                if fields.len() > 1 {
                    vec.push((
                        name.to_pascal(),
                        fields
                            .into_iter()
                            .map(|v| (v.0, v.1.get_filter_type(true)))
                            .collect(),
                    ))
                };
            }
        }
        vec
    }
}
impl SelectorDef {
    pub fn nested_filters(&self, selector: &str, model: &ModelDef) -> Vec<FilterMap> {
        let mut vec = Vec::new();
        Self::_nested_filters(
            &mut vec,
            String::new(),
            format!("_{selector}"),
            Arc::new(model.clone()),
            &self.filters,
        );
        vec
    }
    pub fn filter_is_required(&self) -> bool {
        self.filters.iter().any(|v| v.1.required)
    }
    fn _nested_filters(
        vec: &mut Vec<FilterMap>,
        pascal_name: String,
        suffix: String,
        model: Arc<ModelDef>,
        filters: &IndexMap<String, FilterDef>,
    ) {
        let mut map = FilterMap {
            pascal_name: String::new(),
            suffix: String::new(),
            model: model.clone(),
            filters: IndexMap::new(),
        };
        for (name, filter) in filters.iter() {
            if (filter._type == FilterType::Exists || filter._type == FilterType::EqAny)
                && !filter.relation_fields.is_empty()
            {
                let relation_name = filter.relation.as_ref().unwrap_or(name);
                if let Some(relation) = model.merged_relations.get(relation_name) {
                    let foreign = relation.get_foreign_model();
                    Self::_nested_filters(
                        vec,
                        format!("{}_{}", &pascal_name, name.to_pascal()),
                        format!("{}_{}", &suffix, name),
                        foreign,
                        &filter.relation_fields,
                    );
                } else {
                    error_exit!(
                        "The {} relation specified for selectors in the {} model does not exist.",
                        relation_name,
                        model.name
                    );
                }
            }
            let mut filter = filter.clone();
            if filter.fields.is_empty() {
                filter.fields.insert(name.to_string(), None);
            }
            map.filters.insert(name.clone(), filter);
        }
        map.pascal_name = pascal_name;
        map.suffix = suffix;
        vec.push(map);
    }
    pub fn emu_order(&self, name: &str) -> String {
        let order = self.orders.get(name).unwrap();
        if order.direct_sql.is_some() {
            return "{}".to_string();
        }
        let mut result = String::new();
        for (field, _) in &order.fields {
            let f = _to_ident_name(field);
            let s = match order.direction {
                Some(FilterSortDirection::Desc) => format!("b.{}().partial_cmp(&a.{}()).unwrap_or(std::cmp::Ordering::Equal)", f, f),
                _ => format!("a.{}().partial_cmp(&b.{}()).unwrap_or(std::cmp::Ordering::Equal)", f, f),
            };
            if result.is_empty() {
                result.push_str("{ list.sort_by(|a, b| {");
                result.push_str(&s);
            } else {
                result.push_str(".then_with(|| ");
                result.push_str(&s);
                result.push(')');
            }
        }
        result.push_str("}) }");
        result
    }
    pub fn db_order(&self, name: &str, reverse: bool) -> String {
        let order = self.orders.get(name).unwrap();
        if let Some(direct_sql) = &order.direct_sql {
            return format!("raw_order_by({:?})", direct_sql);
        }
        let mut result = Vec::new();
        for (field, _) in &order.fields {
            let f = _to_ident_name(field);
            if !reverse {
                let s = match order.direction {
                    Some(FilterSortDirection::Desc) => format!("{f} DESC"),
                    _ => format!("{f} ASC"),
                };
                result.push(s);
            } else {
                let s = match order.direction {
                    Some(FilterSortDirection::Desc) => format!("{f} ASC"),
                    _ => format!("{f} DESC"),
                };
                result.push(s);
            }
        }
        format!("order_by(order!({}))", result.join(", "))
    }
}
