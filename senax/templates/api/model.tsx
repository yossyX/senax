import { graphql } from '../../../gql';
@%- for (selector, selector_def) in def.selectors %@
@%- for api_selector_def in api_def.selector(selector) %@
// import { @{ pascal_name }@Query@{ selector|pascal }@Filter, @{ pascal_name }@Query@{ selector|pascal }@Order } from '../../../gql/graphql';
@%- endfor %@
@%- endfor %@

export const @{ graphql_name }@Fragment = graphql(`fragment @{ graphql_name }@ on Res@{ graphql_name }@{@{ gql_fields }@}`);

@%- if def.use_all_rows_cache() && !def.use_filtered_row_cache() %@

export const All@{ model_path|pascal }@Query = graphql(`query all_@{ path }@{@{ curly_begin }@{all{...@{ graphql_name }@}}@{ curly_end }@}`);
@%- endif %@
@%- if api_def.use_find_by_pk %@

export const FindByPk@{ model_path|pascal }@Query = graphql(`query find_by_pk_@{ path }@(@{ def.primaries()|fmt_join("${var}:{gql_type}", ",") }@){@{ curly_begin }@{findByPk(@{ def.primaries()|fmt_join("{var}: ${var}", ",") }@){...@{ graphql_name }@}}@{ curly_end }@}`);
@%- endif %@

export const Find@{ model_path|pascal }@Query = graphql(`query find_@{ path }@($_id:ID!){@{ curly_begin }@{find(_id: $_id){...@{ graphql_name }@}}@{ curly_end }@}`);
@%- for (selector, selector_def) in def.selectors %@
@%- for api_selector_def in api_def.selector(selector) %@

export const @{ selector|pascal }@@{ model_path|pascal }@Query = graphql(`query @{ selector }@_@{ path }@($filter: @{ pascal_name }@Query@{ selector|pascal }@Filter@% if selector_def.filter_is_required() %@!@% endif %@, $order: @{ pascal_name }@Query@{ selector|pascal }@Order, $limit: Int, $offset: Int){@{ curly_begin }@{@{ selector|gql_camel }@(filter: $filter, order: $order, first: $limit, offset: $offset){nodes{...@{ graphql_name }@}}}@{ curly_end }@}`);
export const @{ selector|pascal }@WithCursor@{ model_path|pascal }@Query = graphql(`query @{ selector }@_with_cursor_@{ path }@($filter: @{ pascal_name }@Query@{ selector|pascal }@Filter@% if selector_def.filter_is_required() %@!@% endif %@, $order: @{ pascal_name }@Query@{ selector|pascal }@Order, $after: String, $before: String, $first: Int, $last: Int, $offset: Int){@{ curly_begin }@{@{ selector|gql_camel }@(filter: $filter, order: $order, after: $after, before: $before, first: $first, last: $last, offset: $offset){pageInfo{hasPreviousPage,hasNextPage,startCursor,endCursor},nodes{...@{ graphql_name }@}}}@{ curly_end }@}`);
export const Count@{ selector|pascal }@@{ model_path|pascal }@Query = graphql(`query count_@{ selector }@_@{ path }@($filter: @{ pascal_name }@Query@{ selector|pascal }@Filter@% if selector_def.filter_is_required() %@!@% endif %@){@{ curly_begin }@{count@{ selector|gql_pascal }@(filter: $filter)}@{ curly_end }@}`);
@%- endfor %@
@%- endfor %@
@%- if !api_def.disable_mutation %@
@#-
@%- if !def.disable_update() %@
@%- if api_def.use_find_by_pk %@

export const FindForUpdateByPk@{ model_path|pascal }@Query = graphql(`mutation find_for_update_by_pk_@{ path }@(@{ def.primaries()|fmt_join("${var}:{gql_type}", ",") }@){@{ curly_begin }@{findForUpdateByPk(@{ def.primaries()|fmt_join("{var}: ${var}", ",") }@){...@{ graphql_name }@}}@{ curly_end }@}`);
@%- endif %@

export const FindForUpdate@{ model_path|pascal }@Query = graphql(`mutation find_for_update_@{ path }@($_id:ID!){@{ curly_begin }@{findForUpdate(_id: $_id){...@{ graphql_name }@}}@{ curly_end }@}`);
@%- endif %@
#@

export const Create@{ model_path|pascal }@Query = graphql(`mutation create_@{ path }@($data:Req@{ graphql_name }@!){@{ curly_begin }@{create(data:$data){...@{ graphql_name }@}}@{ curly_end }@}`);
@%- if !def.disable_update() %@
@%- if api_def.use_import %@

export const Import@{ model_path|pascal }@Query = graphql(`mutation import_@{ path }@($list:[Req@{ graphql_name }@!]!){@{ curly_begin }@{import(list:$list)}@{ curly_end }@}`);
@%- endif %@

export const Update@{ model_path|pascal }@Query = graphql(`mutation update_@{ path }@($data:Req@{ graphql_name }@!){@{ curly_begin }@{update(data:$data){...@{ graphql_name }@}}@{ curly_end }@}`);

export const Delete@{ model_path|pascal }@Query = graphql(`mutation delete_@{ path }@($_id:ID!){@{ curly_begin }@{delete(_id:$_id)}@{ curly_end }@}`);
@%- if api_def.use_delete_by_pk %@

export const DeleteByPk@{ model_path|pascal }@Query = graphql(`query delete_by_pk_@{ path }@(@{ def.primaries()|fmt_join("${var}:{gql_type}", ",") }@){@{ curly_begin }@{deleteByPk(@{ def.primaries()|fmt_join("{var}: ${var}", ",") }@)}@{ curly_end }@}`);
@%- endif %@
@%- for (selector, selector_def) in def.selectors %@
@%- for api_selector_def in api_def.selector(selector) %@
@% for (js_name, js_def) in api_selector_def.js_updater %@
export const Update@{ js_name|pascal }@@{ model_path|pascal }@Query = graphql(`mutation update_@{ js_name }@_@{ path }@($filter: @{ pascal_name }@Query@{ selector|pascal }@Filter!, $value: JSON!){@{ curly_begin }@{update@{ js_name|gql_pascal }@(filter: $filter, value: $value){...@{ graphql_name }@}}@{ curly_end }@}`);
@%- endfor %@
@%- if api_selector_def.use_for_update_by_operator %@
export const UpdateBy@{ selector|pascal }@@{ model_path|pascal }@Query = graphql(`mutation update_by_@{ selector }@_@{ path }@($filter: @{ pascal_name }@Query@{ selector|pascal }@Filter!, $operator: JSON!){@{ curly_begin }@{updateBy@{ selector|gql_pascal }@(filter: $filter, operator: $operator){...@{ graphql_name }@}}@{ curly_end }@}`);
@%- endif %@
@%- if api_selector_def.use_for_delete %@

export const DeleteBy@{ selector|pascal }@@{ model_path|pascal }@Query = graphql(`mutation delete_by_@{ selector }@_@{ path }@($filter: @{ pascal_name }@Query@{ selector|pascal }@Filter!){@{ curly_begin }@{deleteBy@{ selector|gql_pascal }@(filter: $filter)}@{ curly_end }@}`);
@%- endif %@
@%- endfor %@
@%- endfor %@
@%- endif %@
@%- endif %@

// Do not modify below this line. (JsonSchemaStart)
// Do not modify up to this line. (JsonSchemaEnd)
