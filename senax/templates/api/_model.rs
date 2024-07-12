#[allow(unused_imports)]
#[rustfmt::skip]
use domain::models::@{ db|snake|to_var_name }@::@{ group|to_var_name }@::@{ mod_name|to_var_name }@::{self as _domain_, @{ pascal_name }@Updater as _, @{ pascal_name }@UpdaterBase as _};

fn query_guard() -> impl async_graphql::Guard {
    @{ query_guard }@
}
@%- if !api_def.disable_mutation %@

fn create_guard() -> impl async_graphql::Guard {
    @{ create_guard }@
}
@%- if !def.disable_update() %@
@%- if api_def.use_import %@

fn import_guard() -> impl async_graphql::Guard {
    @{ import_guard }@
}
@%- endif %@

fn update_guard() -> impl async_graphql::Guard {
    @{ update_guard }@
}

fn delete_guard() -> impl async_graphql::Guard {
    @{ delete_guard }@
}
@%- endif %@
@%- endif %@

#[allow(unused_variables)]
pub fn readable_filter(auth: &AuthInfo) -> anyhow::Result<_domain_::Filter_> {
    Ok(_domain_::filter!(@{ api_def.readable_filter() }@))
}
@%- if !api_def.disable_mutation %@

#[allow(unused_variables)]
pub fn updatable_filter(auth: &AuthInfo) -> anyhow::Result<_domain_::Filter_> {
    Ok(_domain_::filter!(@{ api_def.updatable_filter() }@))
}

#[allow(unused_variables)]
pub fn deletable_filter(auth: &AuthInfo) -> anyhow::Result<_domain_::Filter_> {
    Ok(_domain_::filter!(@{ api_def.deletable_filter() }@))
}
@%- endif %@

#[cfg(test)]
#[test]
fn test() -> anyhow::Result<()> {
    let auth = AuthInfo::default();
    readable_filter(&auth)?;
    @%- if !api_def.disable_mutation %@
    updatable_filter(&auth)?;
    deletable_filter(&auth)?;
    @%- endif %@
    Ok(())
}

@{ def.label|label0 -}@
#[derive(async_graphql::SimpleObject, Serialize)]
#[graphql(name = "Res@{ graphql_name }@")]
pub struct ResObj {
    #[graphql(name = "_id")]
    pub _id: async_graphql::ID,
@%- if camel_case %@
@{- def.for_api_response()|fmt_join("
{label_wo_hash}    pub {var}: {res_api_type},", "") }@
@{- def.relations_one_for_api_response()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_response()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Vec<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_belonging_for_api_response()|fmt_rel_join("
    #[graphql(name = \"_{raw_rel_name}_id\")]
    pub _{raw_rel_name}_id: Option<async_graphql::ID>,
{label_wo_hash}    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@%- else %@
@{- def.for_api_response()|fmt_join("
{label_wo_hash}    #[graphql(name = \"{raw_var}\")]
    pub {var}: {res_api_type},", "") }@
@{- def.relations_one_for_api_response()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_response()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Vec<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_belonging_for_api_response()|fmt_rel_join("
    #[graphql(name = \"_{raw_rel_name}_id\")]
    pub _{raw_rel_name}_id: Option<async_graphql::ID>,
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@%- endif %@
    @%- if !api_def.disable_mutation %@
    #[graphql(name = "_updatable")]
    pub _updatable: bool,
    #[graphql(name = "_deletable")]
    pub _deletable: bool,
    @%- endif %@
}

trait TryFrom_<T>: Sized {
    fn try_from_(value: T, _auth: &AuthInfo) -> anyhow::Result<Self>;
}

impl TryFrom_<&dyn _domain_::@{ pascal_name }@> for ResObj {
    fn try_from_(v: &dyn _domain_::@{ pascal_name }@, _auth: &AuthInfo) -> anyhow::Result<Self> {
        @%- if !api_def.disable_mutation %@
        use domain::models::Check_;
        @%- endif %@
        Ok(Self {
            _id: v.into(),
            @{- def.for_api_response()|fmt_join("
            {var}: v.{var}(){to_res_api_type},", "") }@
            @{- def.relations_one_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().map(|v| v.into()),", "") }@
            @{- def.relations_many_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().map(|v| v.into()).collect(),", "") }@
            @{- def.relations_belonging_for_api_response()|fmt_rel_join("
            _{raw_rel_name}_id: v._{raw_rel_name}_id().map(|v| v.into()),
            {rel_name}: v.{rel_name}().map(|v| v.into()),", "") }@
            @%- if !api_def.disable_mutation %@
            _updatable: updatable_filter(_auth)?.check(v),
            _deletable: deletable_filter(_auth)?.check(v),
            @%- endif %@
        })
    }
}
@%- if def.use_all_rows_cache() || def.use_cache() %@

impl TryFrom_<&dyn _domain_::@{ pascal_name }@Cache> for ResObj {
    fn try_from_(v: &dyn _domain_::@{ pascal_name }@Cache, _auth: &AuthInfo) -> anyhow::Result<Self> {
        @%- if !api_def.disable_mutation %@
        use domain::models::Check_;
        @%- endif %@
        Ok(Self {
            _id: v.into(),
            @{- def.for_api_response()|fmt_join("
            {var}: v.{var}(){to_res_api_type},", "") }@
            @{- def.relations_one_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().map(|v| (&*v).into()),", "") }@
            @{- def.relations_many_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().iter().map(|v| (&**v).into()).collect(),", "") }@
            @{- def.relations_belonging_for_api_response()|fmt_rel_join("
            _{raw_rel_name}_id: v._{raw_rel_name}_id().map(|v| v.into()),
            {rel_name}: v.{rel_name}().map(|v| (&*v).into()),", "") }@
            @%- if !api_def.disable_mutation %@
            _updatable: updatable_filter(_auth)?.check(v),
            _deletable: deletable_filter(_auth)?.check(v),
            @%- endif %@
        })
    }
}
@%- endif %@

#[rustfmt::skip]
#[allow(unused_mut)]
#[allow(clippy::needless_update)]
fn joiner(_look_ahead: async_graphql::Lookahead<'_>, _auth: &AuthInfo) -> anyhow::Result<Option<Box<_domain_::Joiner_>>> {
    let mut joiner = Some(Box::new(_domain_::Joiner_ {
        @%- if camel_case %@
        @{- def.relations_one_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{rel_name_camel}\")),", "") }@
        @{- def.relations_many_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{rel_name_camel}\")),", "") }@
        @{- def.relations_belonging_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{rel_name_camel}\")),", "") }@
        @%- else %@
        @{- def.relations_one_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{raw_rel_name}\")),", "") }@
        @{- def.relations_many_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{raw_rel_name}\")),", "") }@
        @{- def.relations_belonging_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{raw_rel_name}\")),", "") }@
        @%- endif %@
        ..Default::default()
    }));
    @%- if !api_def.disable_mutation %@
    if _look_ahead.field("_updatable").exists() {
        joiner = _domain_::Joiner_::merge(joiner, updatable_filter(_auth)?.joiner()); 
    }
    if _look_ahead.field("_deletable").exists() {
        joiner = _domain_::Joiner_::merge(joiner, deletable_filter(_auth)?.joiner()); 
    }
    @%- endif %@
    Ok(joiner)
}
@%- if !api_def.disable_mutation %@

#[allow(unused_mut)]
#[allow(clippy::needless_update)]
fn updater_joiner() -> Option<Box<_domain_::Joiner_>> {
    let joiner = _domain_::Joiner_ {
        @{- def.relations_one_for_api_request()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::updater_joiner(),", "") }@
        @{- def.relations_many_for_api_request()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::updater_joiner(),", "") }@
        ..Default::default()
    };
    Some(Box::new(joiner))
}

@{ def.label|label0 -}@
#[derive(
    Debug,
    async_graphql::InputObject,
    validator::Validate,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[graphql(name = "Req@{ graphql_name }@")]
pub struct ReqObj {
    #[graphql(name = "_id")]
    #[schemars(skip)]
    pub _id: Option<async_graphql::ID>,
@%- if camel_case %@
@{- def.auto_primary()|fmt_join("
{label_wo_hash}{graphql_secret}{api_validate}{api_serde_default}    pub {var}: {req_api_option_type},", "") }@
@{- def.for_api_request()|fmt_join("
{label_wo_hash}{graphql_secret}{api_validate}{api_serde_default}    pub {var}: {req_api_type},", "") }@
@{- def.relations_one_for_api_request()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Option<_{raw_rel_name}::ReqObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_request()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Option<Vec<_{raw_rel_name}::ReqObj{rel_name_pascal}>>,", "") }@
@%- else %@
@{- def.auto_primary()|fmt_join("
{label_wo_hash}    #[graphql(name = \"{raw_var}\")]
{graphql_secret}{api_validate}{api_serde_default}    pub {var}: {req_api_option_type},", "") }@
@{- def.for_api_request()|fmt_join("
{label_wo_hash}    #[graphql(name = \"{raw_var}\")]
{graphql_secret}{api_validate}{api_serde_default}    pub {var}: {req_api_type},", "") }@
@{- def.relations_one_for_api_request()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<_{raw_rel_name}::ReqObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_request()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<Vec<_{raw_rel_name}::ReqObj{rel_name_pascal}>>,", "") }@
@%- endif %@
}

@{- def.fields_with_default()|fmt_join("
fn default_{raw_var}() -> {req_api_type} {
    {api_default}
}", "") }@

#[allow(clippy::useless_conversion)]
#[allow(clippy::redundant_closure_call)]
impl From<&mut dyn _domain_::@{ pascal_name }@Updater> for ReqObj {
    fn from(v: &mut dyn _domain_::@{ pascal_name }@Updater) -> Self {
        Self {
            _id: Some((&*v).into()),
            @{- def.auto_primary()|fmt_join("
            {var}: Some(v.{var}(){to_req_api_type}),", "") }@
            @{- def.for_api_request()|fmt_join("
            {var}: v.{var}(){to_req_api_type},", "") }@
            @{- def.relations_one_for_api_request()|fmt_rel_join("
            {rel_name}: (|| v.{rel_name}().map(|v| v.into()))(),", "") }@
            @{- def.relations_many_for_api_request()|fmt_rel_join("
            {rel_name}: (|| Some(v.{rel_name}().iter_mut().map(|v| v.into()).collect()))(),", "") }@
        }
    }
}

#[rustfmt::skip]
#[allow(clippy::let_and_return)]
#[allow(clippy::needless_if)]
#[allow(unused_mut)]
#[allow(unused_variables)]
fn create_entity(input: ReqObj, repo: &RepositoriesImpl, auth: &AuthInfo) -> Box<dyn _domain_::@{ pascal_name }@Updater> {
    let mut obj = _domain_::@{ pascal_name }@Factory {
@{- def.non_auto_primary_for_factory()|fmt_join("
        {var}: {from_api_type},", "") }@
    }
    .create(repo);
    @{- def.relations_one_for_api_request()|fmt_rel_join("
    if let Some(input) = input.{rel_name} {
        obj.set_{raw_rel_name}(_{raw_rel_name}::create_entity(input, repo, auth));
    }", "") }@
    @{- def.relations_many_for_api_request()|fmt_rel_join("
    if let Some(data_list) = input.{rel_name} {
        obj.replace_{raw_rel_name}(_{raw_rel_name}::create_list(data_list, repo, auth));
    }", "") }@
    obj
}

#[allow(dead_code)]
pub fn create_list(
    data_list: Vec<ReqObj>,
    repo: &RepositoriesImpl,
    auth: &AuthInfo,
) -> Vec<Box<dyn _domain_::@{ pascal_name }@Updater>> {
    data_list
        .into_iter()
        .map(|v| create_entity(v, repo, auth))
        .collect()
}

#[rustfmt::skip]
#[allow(unused_variables)]
fn update_updater(updater: &mut dyn _domain_::@{ pascal_name }@Updater, input: ReqObj, repo: &RepositoriesImpl, auth: &AuthInfo) -> anyhow::Result<()> {
@{- def.for_api_update_updater()|fmt_join("
    updater.set_{raw_var}({from_api_type_for_update});", "") }@
@{- def.relations_one_for_api_request_with_replace_type(true)|fmt_rel_join("
    if let Some(input) = input.{rel_name} {
        updater.set_{raw_rel_name}(_{raw_rel_name}::create_entity(input, repo, auth));
    }", "") }@
@{- def.relations_one_for_api_request_with_replace_type(false)|fmt_rel_join("
    if let Some(input) = input.{rel_name} {
        if let Some(updater) = updater.{rel_name}() {
            _{raw_rel_name}::update_updater(updater, input, repo, auth)?;
        } else {
            updater.set_{raw_rel_name}(_{raw_rel_name}::create_entity(input, repo, auth));
        }
    }", "") }@
@{- def.relations_many_for_api_request()|fmt_rel_join("
    if let Some(data_list) = input.{rel_name} {
        let list = updater.take_{raw_rel_name}().unwrap();
        updater.replace_{raw_rel_name}(_{raw_rel_name}::update_list(list, data_list, repo, auth)?);        
    }", "") }@
    Ok(())
}
@%- endif %@

#[allow(unused_variables)]
pub fn gen_json_schema(dir: &std::path::Path) -> anyhow::Result<()> {
    @%- if !api_def.disable_mutation %@
    let settings = schemars::gen::SchemaSettings::draft07().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<ReqObj>();
    crate::auto_api::write_json_schema(
        &dir.join("@{ model_name }@.tsx"),
        serde_json::to_string_pretty(&schema)?,
    )?;
    @%- endif %@
    Ok(())
}
@{-"\n"}@
