import { graphql } from '../../../gql';
@%- for (selector, selector_def) in def.selectors %@
@%- for api_selector_def in api_def.selector(selector) %@
// import { @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Filter, @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Order } from '../../../gql/graphql';
@%- endfor %@
@%- endfor %@

export const @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@Fragment = graphql(`fragment @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@ on Res@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@{@{ gql_fields }@}`);

@%- if def.use_all_row_cache() && !def.use_filtered_row_cache() %@

export const All@{ pascal_name }@Query = graphql(`query all_@{ db }@_@{ group }@_@{ mod_name }@{@{ db_case }@{@{ group_case }@{@{ model_case }@{all{...@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@}}}}}`);
@%- endif %@
@%- if api_def.use_find_by_pk %@

export const FindByPk@{ pascal_name }@Query = graphql(`query find_by_pk_@{ db }@_@{ group }@_@{ mod_name }@(@{ def.primaries()|fmt_join("${var}:{gql_type}", ",") }@){@{ db_case }@{@{ group_case }@{@{ model_case }@{findByPk(@{ def.primaries()|fmt_join("{var}: ${var}", ",") }@){...@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@}}}}}`);
@%- endif %@

export const Find@{ pascal_name }@Query = graphql(`query find_@{ db }@_@{ group }@_@{ mod_name }@($_id:ID!){@{ db_case }@{@{ group_case }@{@{ model_case }@{find(_id: $_id){...@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@}}}}}`);
@%- for (selector, selector_def) in def.selectors %@
@%- for api_selector_def in api_def.selector(selector) %@

export const @{ selector|pascal }@@{ pascal_name }@Query = graphql(`query @{ selector }@_@{ db }@_@{ group }@_@{ mod_name }@($filter: @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Filter, $order: @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Order, $limit: Int, $offset: Int){@{ db_case }@{@{ group_case }@{@{ model_case }@{@{ selector|gql_camel }@(filter: $filter, order: $order, first: $limit, offset: $offset){nodes{...@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@}}}}}}`);
export const Count@{ selector|pascal }@@{ pascal_name }@Query = graphql(`query count_@{ selector }@_@{ db }@_@{ group }@_@{ mod_name }@($filter: @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Filter){@{ db_case }@{@{ group_case }@{@{ model_case }@{count@{ selector|gql_pascal }@(filter: $filter)}}}}`);
@%- endfor %@
@%- endfor %@
@%- if !api_def.disable_mutation %@

export const Create@{ pascal_name }@Query = graphql(`mutation create_@{ db }@_@{ group }@_@{ mod_name }@($data:Req@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@!){@{ db_case }@{@{ group_case }@{@{ model_case }@{create(data:$data){...@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@}}}}}`);
@%- if !def.disable_update() %@
@%- if api_def.use_import %@

export const Import@{ pascal_name }@Query = graphql(`mutation import_@{ db }@_@{ group }@_@{ mod_name }@($list:[Req@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@!]!){@{ db_case }@{@{ group_case }@{@{ model_case }@{import(list:$list)}}}}`);
@%- endif %@

export const Update@{ pascal_name }@Query = graphql(`mutation update_@{ db }@_@{ group }@_@{ mod_name }@($data:Req@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@!){@{ db_case }@{@{ group_case }@{@{ model_case }@{update(data:$data){...@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@}}}}}`);

export const Delete@{ pascal_name }@Query = graphql(`mutation delete_@{ db }@_@{ group }@_@{ mod_name }@($_id:ID!){@{ db_case }@{@{ group_case }@{@{ model_case }@{delete(_id:$_id)}}}}`);

@%- for (selector, selector_def) in def.selectors %@
@%- for api_selector_def in api_def.selector(selector) %@
@% for (js_name, js_def) in api_selector_def.js_updater %@
export const Update@{ js_name|pascal }@@{ pascal_name }@Query = graphql(`mutation update_@{ js_name }@_@{ db }@_@{ group }@_@{ mod_name }@($filter: @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Filter!, $value: JSON!){@{ db_case }@{@{ group_case }@{@{ model_case }@{update@{ js_name|gql_pascal }@(filter: $filter, value: $value){...@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@}}}}}`);
@%- endfor %@
@%- if api_selector_def.use_for_update_by_operator %@
export const UpdateBy@{ selector|pascal }@@{ pascal_name }@Query = graphql(`mutation update_by_@{ selector }@_@{ db }@_@{ group }@_@{ mod_name }@($filter: @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Filter!, $operator: JSON!){@{ db_case }@{@{ group_case }@{@{ model_case }@{updateBy@{ selector|gql_pascal }@(filter: $filter, operator: $operator){...@{ db|pascal }@@{ group|pascal }@@{ pascal_name }@}}}}}`);
@%- endif %@
@%- if api_selector_def.use_for_delete %@

export const DeleteBy@{ selector|pascal }@@{ pascal_name }@Query = graphql(`mutation delete_by_@{ selector }@_@{ db }@_@{ group }@_@{ mod_name }@($filter: @{ db|pascal }@@{ group|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Filter!){@{ db_case }@{@{ group_case }@{@{ model_case }@{deleteBy@{ selector|gql_pascal }@(filter: $filter)}}}}`);
@%- endif %@
@%- endfor %@
@%- endfor %@
@%- endif %@
@%- endif %@

// Do not modify below this line. (JsonSchemaStart)
// Do not modify up to this line. (JsonSchemaEnd)
