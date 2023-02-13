
#[derive(SimpleObject)]
pub struct @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@ {
    #[graphql(name = "_id")]
    pub _id: ID,
@%- if camel_case %@
@{- def.for_api_response()|fmt_join("
    pub {var}: {api_type},", "") }@
@{- def.relations_one_cache()|fmt_rel_join("
    pub {alias}: Option<--1----2----3--{alias_pascal}>,", "")|replace3(db|pascal, group|pascal, mod_name|pascal) }@
@{- def.relations_many_cache()|fmt_rel_join("
    pub {alias}: Vec<--1----2----3--{alias_pascal}>,", "")|replace3(db|pascal, group|pascal, mod_name|pascal) }@
@{- def.relations_one_only_cache()|fmt_rel_join("
    pub {alias}: Option<--1----2----3--{alias_pascal}>,", "")|replace3(db|pascal, group|pascal, mod_name|pascal) }@
@%- else %@
@{- def.for_api_response()|fmt_join("
    #[graphql(name = \"{raw_var}\")]
    pub {var}: {api_type},", "") }@
@{- def.relations_one_cache()|fmt_rel_join("
    #[graphql(name = \"{raw_alias}\")]
    pub {alias}: Option<--1----2----3--{alias_pascal}>,", "")|replace3(db|pascal, group|pascal, mod_name|pascal) }@
@{- def.relations_many_cache()|fmt_rel_join("
    #[graphql(name = \"{raw_alias}\")]
    pub {alias}: Vec<--1----2----3--{alias_pascal}>,", "")|replace3(db|pascal, group|pascal, mod_name|pascal) }@
@{- def.relations_one_only_cache()|fmt_rel_join("
    #[graphql(name = \"{raw_alias}\")]
    pub {alias}: Option<--1----2----3--{alias_pascal}>,", "")|replace3(db|pascal, group|pascal, mod_name|pascal) }@
@%- endif %@
}

impl From<_@{ pascal_name }@> for @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@ {
    fn from(v: _@{ pascal_name }@) -> Self {
        Self {
            _id: format!("{VER}:@{ db }@:@{ group }@:@{ mod_name }@:@{- def.primaries()|fmt_join("{}", "_") }@", @{ def.primaries()|fmt_join("v.{var}()", ", ") }@).into(),
            @{- def.for_api_response()|fmt_join("
            {var}: v.{var}(){to_api_type},", "") }@
            @{- def.relations_one_cache()|fmt_rel_join("
            {alias}: v.{alias}().map(|v| v.into()),", "") }@
            @{- def.relations_many_cache()|fmt_rel_join("
            {alias}: v.{alias}().iter().map(|v| v.into()).collect(),", "") }@
            @{- def.relations_one_only_cache()|fmt_rel_join("
            {alias}: v.{alias}().map(|v| v.into()),", "") }@
        }
    }
}
@%- if def.use_cache_all() && !def.use_cache_all_with_condition() ||  def.use_cache() %@

impl From<_@{ pascal_name }@Cache> for @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@ {
    fn from(v: _@{ pascal_name }@Cache) -> Self {
        Self {
            _id: format!("{VER}:@{ db }@:@{ group }@:@{ mod_name }@:@{- def.primaries()|fmt_join("{}", "_") }@", @{ def.primaries()|fmt_join("v.{var}()", ", ") }@).into(),
            @{- def.for_api_response()|fmt_join("
            {var}: v.{var}(){to_api_type},", "") }@
            @{- def.relations_one_cache()|fmt_rel_join("
            {alias}: v.{alias}().map(|v| v.into()),", "") }@
            @{- def.relations_many_cache()|fmt_rel_join("
            {alias}: v.{alias}().iter().map(|v| v.into()).collect(),", "") }@
            @{- def.relations_one_only_cache()|fmt_rel_join("
            {alias}: v.{alias}().map(|v| v.into()),", "") }@
        }
    }
}
@%- endif %@

#[allow(dead_code)]
#[allow(unused_variables)]
async fn fetch_obj(
    obj: &mut _@{ pascal_name }@,
    conn: &mut DataConn,
    look_ahead: Lookahead<'_>,
) -> anyhow::Result<()> {
@{- def.relations_one_only_cache()|fmt_rel_join("
    if look_ahead.field(\"{alias_camel}\").exists() {
        obj.fetch_{raw_alias}(conn).await?;
    }", "") }@
    Ok(())
}

#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
#[allow(unused_variables)]
async fn fetch_list(
    list: &mut Vec<_@{ pascal_name }@>,
    conn: &mut DataConn,
    look_ahead: Lookahead<'_>,
) -> anyhow::Result<()> {
@{- def.relations_one_only_cache()|fmt_rel_join("
    if look_ahead.field(\"{alias_camel}\").exists() {
        list.fetch_{raw_alias}(conn).await?;
    }", "") }@
    Ok(())
}
@%- if def.use_cache_all() && !def.use_cache_all_with_condition() ||  def.use_cache() %@

#[allow(dead_code)]
#[allow(unused_variables)]
async fn fetch_cache_obj(
    obj: &mut _@{ pascal_name }@Cache,
    conn: &mut DataConn,
    look_ahead: Lookahead<'_>,
) -> anyhow::Result<()> {
@{- def.relations_one_only_cache()|fmt_rel_join("
    if look_ahead.field(\"{alias_camel}\").exists() {
        obj.fetch_{raw_alias}(conn).await?;
    }", "") }@
    Ok(())
}

#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
#[allow(unused_variables)]
async fn fetch_cache_list(
    list: &mut Vec<_@{ pascal_name }@Cache>,
    conn: &mut DataConn,
    look_ahead: Lookahead<'_>,
) -> anyhow::Result<()> {
@{- def.relations_one_only_cache()|fmt_rel_join("
    if look_ahead.field(\"{alias_camel}\").exists() {
        list.fetch_{raw_alias}(conn).await?;
    }", "") }@
    Ok(())
}
@%- endif %@

#[derive(Debug, InputObject, Validate, Serialize, Deserialize)]
pub struct Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@ {
@%- if camel_case %@
@{- def.for_api_request()|fmt_join("
{api_validate}    pub {var}: {api_type},", "") }@
@{- def.relations_auto_inc_many_cache()|fmt_rel_join("
    pub {alias}: Vec<Req--1----2----3--{alias_pascal}>,", "")|replace3(db|pascal, group|pascal, mod_name|pascal) }@
@%- else %@
@{- def.for_api_request()|fmt_join("
    #[graphql(name = \"{raw_var}\")]
{api_validate}    pub {var}: {api_type},", "") }@
@{- def.relations_auto_inc_many_cache()|fmt_rel_join("
    #[graphql(name = \"{raw_alias}\")]
    pub {alias}: Vec<Req--1----2----3--{alias_pascal}>,", "")|replace3(db|pascal, group|pascal, mod_name|pascal) }@
@%- endif %@
}

#[allow(clippy::let_and_return)]
#[allow(unused_mut)]
fn prepare_create(conn: &mut @{ db|pascal }@Conn, data: Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@) -> _@{ pascal_name }@ForUpdate {
    let mut obj = _@{ pascal_name }@Factory {
@{- def.for_factory()|fmt_join("
        {var}: {from_api_type},", "") }@
    }
    .create(conn);
    @{- def.relations_auto_inc_many_cache()|fmt_rel_join("
    obj.{alias}().replace(
        data.{alias}
            .into_iter()
            .map(|v| {class}Factory::from(v).create(conn))
            .collect(),
    );", "") }@
    obj
}

async fn prepare_update(
    conn: &mut @{ db|pascal }@Conn,
    obj: &mut _@{ pascal_name }@ForUpdate,
    data: Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@,
) -> anyhow::Result<()> {
    let update = _@{ pascal_name }@Factory {
@{- def.for_factory()|fmt_join("
        {var}: {from_api_type},", "") }@
    }
    .create(conn);
    obj._update(update);
    
    @{- def.relations_auto_inc_many_cache()|fmt_rel_join("

    obj.fetch_{raw_alias}(conn).await?;
    let list = obj.{alias}().take().unwrap();
    let mut map: HashMap<_, _> = list
        .into_iter()
        .map(|v| v._set_delete_and_return_self())
        .map(|v| (v.{foreign_pk}().get(), v))
        .collect();
    let mut list = Vec::new();
    for row in data.{alias}.into_iter() {
        if row.{foreign_pk}.is_some() {
            let update = {class}Factory::from(row).create(conn);
            if let Some(mut v) = map.remove(&update.{foreign_pk}().get()) {
                v._cancel_delete();
                v._update(update);
                list.push(v);
            } else {
                anyhow::bail!(\"The {foreign_pk} of {raw_alias} is invalid.\");
            }
        } else {
            let update = {class}Factory::from(row).create(conn);
            list.push(update);
        }
    }
    map.into_iter().for_each(|(_, v)| {
        list.push(v);
    });
    obj.{alias}().replace(list);", "") }@
    Ok(())
}
@{-"\n"}@
