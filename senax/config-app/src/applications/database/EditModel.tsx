import * as React from "react";
import { Helmet } from "react-helmet-async";
import {
  ScrollRestoration,
  useActionData,
  useNavigate,
  useParams,
  useRouteLoaderData,
  useSubmit,
  useBlocker,
} from "react-router-dom";
import { useForm, useWatch } from "react-hook-form";
import { yupResolver } from "@hookform/resolvers/yup";
import Form from "@cloudscape-design/components/form";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Button from "@cloudscape-design/components/button";
import Container from "@cloudscape-design/components/container";
import { ContentLayout, Header } from "@cloudscape-design/components";
import Toggle from "@cloudscape-design/components/toggle";
import * as pluralize from "pluralize";

import AutoField from "@/components/AutoField";
import { createYupSchema } from "@/components/AutoField/lib";
import Status404 from "@/pages/Status404";

let DETAIL = false;

function EditModel() {
  const params = useParams();
  const [detail, setDetail] = React.useState(DETAIL);
  DETAIL = detail;
  const [db_data, vo_list] = useRouteLoaderData("db") as any;
  const [models, jsonSchema, _model_names] = useRouteLoaderData("group") as any;
  const model_names = {} as any;
  const group_names = [];
  for (const group in _model_names) {
    group_names.push(group);
    model_names[group] = [];
    for (const name of _model_names[group]) {
      model_names[group].push(name);
    }
  }
  let data: any = {};
  if (params.model) {
    data = models.find((v: any) => v.name === params.model);
  }
  const navigate = useNavigate();
  const actionData = useActionData() as any;
  const definitions = jsonSchema.definitions;
  const yupSchema = createYupSchema(jsonSchema, definitions);
  const form = useForm({
    defaultValues: data,
    resolver: yupResolver(yupSchema),
    mode: "all",
  });
  const modelData = form.watch();
  const [dirtyDialog, setDirtyDialog] = React.useState(false);
  const dirty = React.useRef(false);
  dirty.current = form.formState.isDirty || dirtyDialog;
  const [blocked, setBlocked] = React.useState(false);
  useBlocker(() => {
    if (dirty.current) setBlocked(true);
    return dirty.current;
  });
  if (blocked) {
    alert("You have unsaved changes in the form. Save them before exiting");
    setBlocked(false);
  }
  React.useEffect(() => {
    const onBeforeUnload = (e: any) => {
      if (dirty.current) {
        const message =
          "You have unsaved changes in the form. Save them before exiting";
        e.preventDefault();
        e.returnValue = message;
        return message;
      }
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", onBeforeUnload);
    };
  }, []);

  const submit = useSubmit();
  const onSubmit = (data: any) => {
    submit(data, { method: "post", encType: "application/json" });
  };
  const formData = {
    form,
    schema: jsonSchema,
    definitions,
    dirtyDialog,
    setDirtyDialog,
    additionalData: {
      db: params.db,
      group: params.group,
      group_names,
      model_names,
      modelData,
      selfGroup: params.group,
      selfModel: modelData,
      db_data,
      vo_list,
    },
  };
  if (data === undefined) {
    return <Status404 />;
  }
  return (
    <>
      <ScrollRestoration />
      <Helmet>
        <title>Senax Database Configuration</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">Model</Header>}>
        <Container
          header={
            <Header
              variant="h2"
              actions={
                <SpaceBetween direction="horizontal" size="xs">
                  <Toggle
                    onChange={({ detail }) => setDetail(detail.checked)}
                    checked={detail}
                  >
                    Details
                  </Toggle>
                </SpaceBetween>
              }
            ></Header>
          }
        >
          <Form
            actions={
              <SpaceBetween direction="horizontal" size="xs">
                <Button
                  formAction="none"
                  variant="link"
                  onClick={() => {
                    dirty.current = false;
                    navigate(`..`, { replace: true });
                  }}
                >
                  Cancel
                </Button>
                <Button
                  variant="primary"
                  disabled={Object.entries(form.formState.errors).length > 0}
                  onClick={() => {
                    dirty.current = false;
                    form.handleSubmit(onSubmit)();
                  }}
                >
                  Submit
                </Button>
              </SpaceBetween>
            }
            errorText={actionData}
          >
            <SpaceBetween direction="vertical" size="xs">
              <AutoField name="name" {...formData} />
              <AutoField name="label" {...formData} />
              <AutoField name="comment" {...formData} textarea />
              <AutoField name="table_name" {...formData} />
              <AutoField name="skip_ddl" {...formData} />
              <AutoField name="dummy_always_present" {...formData} />
              <AutoField
                name="ignore_foreign_key"
                {...formData}
                hidden={!detail}
              />
              <AutoField name="timestampable" {...formData} hidden={!detail} />
              <AutoField
                name="disable_created_at"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="disable_updated_at"
                {...formData}
                hidden={!detail}
              />
              <AutoField name="soft_delete" {...formData} />
              <AutoField name="versioned" {...formData} hidden={!detail} />
              <AutoField name="counting" {...formData} hidden={!detail} />
              <AutoField name="use_cache" {...formData} hidden={!detail} />
              <AutoField
                name="use_all_rows_cache"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="use_filtered_row_cache"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="use_clear_whole_cache"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="use_auto_replace"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="use_update_notice"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="use_insert_delayed"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="use_save_delayed"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="use_update_delayed"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="use_upsert_delayed"
                {...formData}
                hidden={!detail}
              />
              <AutoField name="disable_update" {...formData} hidden={!detail} />
              <AutoField
                name="disable_insert_cache_propagation"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="use_on_delete_fn"
                {...formData}
                hidden={!detail}
              />
              <AutoField name="abstract" {...formData} hidden={!detail} />
              <AutoField
                name="inheritance"
                {...formData}
                hidden={!detail}
                component={Inheritance}
              />
              <AutoField name="engine" {...formData} hidden={!detail} />
              {/* <AutoField name="character_set" {...formData} hidden={!detail} /> */}
              <AutoField name="collation" {...formData} hidden={!detail} />
              <AutoField
                name="act_as"
                {...formData}
                hidden={!detail}
                component={ActAs}
              />
              <AutoField
                name="hide_er_relations"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="fields"
                {...formData}
                columns={[
                  { field: "name", editable: true },
                  { field: "label", editable: true },
                  { field: "type", editable: true },
                  { field: "primary", width: 100, editable: true },
                  { field: "not_null", editable: true },
                ]}
                dialog={Field}
                resolver={yupResolver(
                  createYupSchema(definitions.FieldJson, definitions),
                )}
              />
              <AutoField
                name="belongs_to"
                {...formData}
                columns={[
                  { field: "name", editable: true },
                  { field: "model" },
                  { field: "group" },
                  { field: "local" },
                  { field: "on_delete", editable: true },
                ]}
                dialog={BelongsTo}
                resolver={yupResolver(
                  createYupSchema(definitions.BelongsToJson, definitions),
                )}
              />
              <AutoField
                name="belongs_to_outer_db"
                {...formData}
                hidden={!detail}
                columns={[
                  { field: "name", editable: true },
                  { field: "db" },
                  { field: "group" },
                  { field: "model" },
                  { field: "local" },
                ]}
                dialog={BelongsToOuterDb}
                resolver={yupResolver(
                  createYupSchema(definitions.BelongsToOuterDbJson, definitions),
                )}
              />
              <AutoField
                name="has_one"
                {...formData}
                columns={[
                  { field: "name", editable: true },
                  { field: "group" },
                  { field: "model" },
                  { field: "foreign" },
                ]}
                dialog={HasOne}
                resolver={yupResolver(
                  createYupSchema(definitions.HasOneJson, definitions),
                )}
              />
              <AutoField
                name="has_many"
                {...formData}
                columns={[
                  { field: "name", editable: true },
                  { field: "group" },
                  { field: "model" },
                  { field: "foreign" },
                ]}
                dialog={HasMany}
                resolver={yupResolver(
                  createYupSchema(definitions.HasManyJson, definitions),
                )}
              />
              <AutoField
                name="indexes"
                {...formData}
                columns={[
                  { field: "name", width: 200, editable: true },
                  { field: "type", width: 200, editable: true },
                ]}
                dialog={Index}
                resolver={yupResolver(
                  createYupSchema(definitions.IndexJson, definitions),
                )}
              />
              <AutoField
                name="selectors"
                {...formData}
                columns={[{ field: "name", width: 200, editable: true }]}
                dialog={Selector}
                resolver={yupResolver(
                  createYupSchema(definitions.SelectorJson, definitions),
                )}
              />
            </SpaceBetween>
          </Form>
        </Container>
      </ContentLayout>
    </>
  );
}
export default EditModel;

const Inheritance = (props: any) => {
  const formData = props.formData;
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="extends" {...formData} />
        <AutoField name="type" {...formData} />
        <AutoField name="key_field" {...formData} />
        <AutoField name="key_value" {...formData} />
      </SpaceBetween>
    </>
  );
};

const ActAs = (props: any) => {
  const formData = props.formData;
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="job_queue" {...formData} />
      </SpaceBetween>
    </>
  );
};

function Field({ formData, definitions }: any) {
  const type = useWatch({
    control: formData.form.control,
    name: "type",
  });
  const primary = useWatch({
    control: formData.form.control,
    name: "primary",
  });
  const NUMBER = ["tinyint", "smallint", "int", "bigint", "float", "double"];
  return (
    <>
      <Container header={<Header variant="h2"></Header>}>
        <SpaceBetween direction="vertical" size="xs">
          <AutoField name="name" {...formData} />
          <AutoField name="label" {...formData} />
          <AutoField name="comment" {...formData} textarea />
          <AutoField name="type" {...formData} />
          <AutoField
            name="value_object"
            {...formData}
            hidden={type !== "value_object"}
            autocomplete={formData.additionalData.vo_list.map(
              (v: any) => v.name,
            )}
          />
          <AutoField name="signed" {...formData} />
          <AutoField name="not_null" {...formData} />
          <AutoField name="required" {...formData} />
          <AutoField name="primary" {...formData} />
          <AutoField
            name="auto"
            {...formData}
            hidden={!primary || (!NUMBER.includes(type) && type !== "uuid")}
          />
          <AutoField name="main_primary" {...formData} hidden={!primary} />
          <AutoField
            name="length"
            {...formData}
            hidden={
              !["char", "varchar", "text", "varbinary", "binary", "blob"].includes(type)
            }
          />
          <AutoField name="max" {...formData} hidden={!NUMBER.includes(type)} />
          <AutoField name="min" {...formData} hidden={!NUMBER.includes(type)} />
          <AutoField
            name="collation"
            {...formData}
            hidden={!["char", "varchar", "text"].includes(type)}
          />
          <AutoField
            name="precision"
            {...formData}
            hidden={type !== "decimal"}
          />
          <AutoField name="scale" {...formData} hidden={type !== "decimal"} />
          <AutoField
            name="time_zone"
            {...formData}
            hidden={!["datetime", "timestamp"].includes(type)}
          />
          <AutoField
            name="enum_values"
            {...formData}
            columns={[
              { field: "name", editable: true },
              { field: "label", editable: true },
              { field: "value", editable: true },
            ]}
            dialog={EnumValue}
            resolver={yupResolver(
              createYupSchema(definitions.EnumValue, definitions),
            )}
          />
          <AutoField
            name="json_class"
            {...formData}
            hidden={!["json"].includes(type)}
          />
          <AutoField name="exclude_from_cache" {...formData} />
          <AutoField name="skip_factory" {...formData} />
          <AutoField name="column_name" {...formData} />
          <AutoField
            name="srid"
            {...formData}
            hidden={!["geo_point", "geometry"].includes(type)}
          />
          <AutoField name="default" {...formData} />
          <AutoField name="sql_comment" {...formData} />
          <AutoField name="hidden" {...formData} />
          <AutoField name="secret" {...formData} />
        </SpaceBetween>
      </Container>
    </>
  );
}

function EnumValue({ formData }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField name="label" {...formData} />
        <AutoField name="comment" {...formData} textarea />
        <AutoField name="value" {...formData} />
      </SpaceBetween>
    </>
  );
}

function BelongsTo({ formData }: any) {
  const group = useWatch({
    control: formData.form.control,
    name: "group",
  });
  const model = useWatch({
    control: formData.form.control,
    name: "model",
  });
  React.useEffect(() => {
    if (formData.form.getValues("name") === undefined) {
      if (model !== undefined) {
        formData.form.setValue("name", model, {
          shouldDirty: true,
          shouldValidate: true,
        });
      }
    }
  }, [model]);
  const baseGroup = formData.additionalData.group;
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField name="label" {...formData} />
        <AutoField name="comment" {...formData} textarea />
        <AutoField name="group" {...formData} autocomplete={formData.additionalData.group_names} />
        <AutoField
          name="model"
          {...formData}
          autocomplete={formData.additionalData.model_names[group || baseGroup] || []}
        />
        <AutoField
          name="local"
          {...formData}
          autocomplete={formData.additionalData.modelData?.fields?.map(
            (v: any) => v.name,
          )}
        />
        <AutoField name="with_trashed" {...formData} />
        <AutoField name="disable_index" {...formData} />
        <AutoField name="on_delete" {...formData} />
        <AutoField name="on_update" {...formData} />
      </SpaceBetween>
    </>
  );
}

function BelongsToOuterDb({ formData }: any) {
  const [dbs, setDbs] = React.useState([]);
  React.useEffect(() => {
    fetch("/api/db")
      .then((res) => res.json())
      .then((json) => setDbs(json))
      .catch(() => alert("error"));
  }, []);
  const db = useWatch({
    control: formData.form.control,
    name: "db",
  });
  const baseGroup = formData.additionalData.group;
  const [groups, setGroups] = React.useState([]);
  React.useEffect(() => {
    if (db) {
      fetch(`/api/db/${db}`)
        .then((res) => res.json())
        .then((json) => setGroups(json.groups))
        .catch(() => alert("error"));
    }
  }, [db]);
  const group = useWatch({
    control: formData.form.control,
    name: "group",
  });
  const [models, setModels] = React.useState([]);
  React.useEffect(() => {
    const g = group || baseGroup;
    if (db && g) {
      fetch(`/api/model_names/${db}`)
        .then((res) => res.json())
        .then((json) => setModels(json[g] || []))
        .catch(() => alert("error"));
    }
  }, [db, group]);
  const model = useWatch({
    control: formData.form.control,
    name: "model",
  });
  if (formData.form.getValues("name") === undefined) {
    if (model !== undefined) {
      formData.form.setValue("name", model, {
        shouldDirty: true,
        shouldValidate: true,
      });
    }
  }
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField name="label" {...formData} />
        <AutoField name="comment" {...formData} textarea />
        <AutoField name="db" {...formData} autocomplete={dbs} />
        <AutoField name="group" {...formData} autocomplete={groups.map(
          (v: any) => v.name,
        )} />
        <AutoField name="model" {...formData} autocomplete={models} />
        <AutoField
          name="local"
          {...formData}
          autocomplete={formData.additionalData.modelData?.fields?.map(
            (v: any) => v.name,
          )}
        />
        <AutoField name="with_trashed" {...formData} />
        <AutoField name="disable_index" {...formData} />
      </SpaceBetween>
    </>
  );
}

function HasOne({ formData }: any) {
  const group = useWatch({
    control: formData.form.control,
    name: "group",
  });
  const model = useWatch({
    control: formData.form.control,
    name: "model",
  });
  React.useEffect(() => {
    if (formData.form.getValues("name") === undefined) {
      if (model !== undefined) {
        formData.form.setValue("name", model, {
          shouldDirty: true,
          shouldValidate: true,
        });
      }
    }
  }, [model]);
  const baseGroup = formData.additionalData.group;
  const [foreign, setForeign] = React.useState(undefined as any);
  const db = formData.additionalData.db;
  const _group = group || formData.additionalData.group;
  React.useEffect(() => {
    fetch(`/api/models/${db}/${_group}`)
      .then((res) => res.json())
      .then((data) => {
        setForeign(data.find((v: any) => v.name === model));
      });
  }, [db, group, model]);
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField name="label" {...formData} />
        <AutoField name="comment" {...formData} textarea />
        <AutoField name="group" {...formData} autocomplete={formData.additionalData.group_names} />
        <AutoField
          name="model"
          {...formData}
          autocomplete={formData.additionalData.model_names[group || baseGroup] || []}
        />
        <AutoField
          name="foreign"
          {...formData}
          autocomplete={foreign?.fields?.map((v: any) => v.name)}
        />
        <AutoField name="disable_cache" {...formData} />
      </SpaceBetween>
    </>
  );
}

function HasMany({ formData }: any) {
  const group = useWatch({
    control: formData.form.control,
    name: "group",
  });
  const model = useWatch({
    control: formData.form.control,
    name: "model",
  });
  React.useEffect(() => {
    if (formData.form.getValues("name") === undefined) {
      if (model !== undefined) {
        formData.form.setValue("name", pluralize.plural(model), {
          shouldDirty: true,
          shouldValidate: true,
        });
      }
    }
  }, [model]);
  const baseGroup = formData.additionalData.group;
  const [foreign, setForeign] = React.useState(undefined as any);
  const db = formData.additionalData.db;
  const _group = group || formData.additionalData.group;
  React.useEffect(() => {
    fetch(`/api/models/${db}/${_group}`)
      .then((res) => res.json())
      .then((data) => {
        setForeign(data.find((v: any) => v.name === model));
      });
  }, [db, group, model]);
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField name="label" {...formData} />
        <AutoField name="comment" {...formData} textarea />
        <AutoField name="group" {...formData} autocomplete={formData.additionalData.group_names} />
        <AutoField
          name="model"
          {...formData}
          autocomplete={formData.additionalData.model_names[group || baseGroup] || []}
        />
        <AutoField
          name="foreign"
          {...formData}
          autocomplete={foreign?.fields?.map((v: any) => v.name)}
        />
        <AutoField name="disable_cache" {...formData} />
        <AutoField name="additional_filter" {...formData} />
        <AutoField
          name="order_by"
          {...formData}
          autocomplete={foreign?.fields?.map((v: any) => v.name)}
        />
        <AutoField name="desc" {...formData} />
        <AutoField name="limit" {...formData} />
      </SpaceBetween>
    </>
  );
}

function Index({ formData, definitions }: any) {
  const type = useWatch({
    control: formData.form.control,
    name: "type",
  });
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField
          name="fields"
          {...formData}
          columns={[
            { field: "name", editable: true },
            { field: "length", editable: true },
            { field: "query", editable: true },
          ]}
          dialog={IndexField}
          resolver={yupResolver(
            createYupSchema(definitions.IndexFieldJson, definitions),
          )}
        />
        <AutoField name="type" {...formData} />
        <AutoField name="parser" {...formData} hidden={type !== "fulltext"} />
      </SpaceBetween>
    </>
  );
}

function IndexField({ formData }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField
          name="name"
          {...formData}
          autocomplete={getFields(formData.additionalData)}
        />
        <AutoField name="direction" {...formData} />
        <AutoField name="length" {...formData} />
        <AutoField name="query" {...formData} />
      </SpaceBetween>
    </>
  );
}

function Selector({ formData, definitions }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField
          name="filters"
          {...formData}
          columns={[
            { field: "name", editable: true },
            { field: "type", editable: true },
          ]}
          dialog={Filter}
          resolver={yupResolver(
            createYupSchema(definitions.FilterJson, definitions),
          )}
        />
        <AutoField
          name="orders"
          {...formData}
          columns={[{ field: "name", editable: true }]}
          dialog={Order}
          resolver={yupResolver(
            createYupSchema(definitions.OrderJson, definitions),
          )}
        />
      </SpaceBetween>
    </>
  );
}

function Filter({ formData, definitions }: any) {
  const relationList: any[] = [].concat(
    formData.additionalData.modelData.belongs_to || [],
    formData.additionalData.modelData.has_one || [],
    formData.additionalData.modelData.has_many || [],
  );

  const type = useWatch({
    control: formData.form.control,
    name: "type",
  });
  const fields = useWatch({
    control: formData.form.control,
    name: "fields",
  });
  const relation = useWatch({
    control: formData.form.control,
    name: "relation",
  });
  if (formData.form.getValues("name") === undefined) {
    if (fields !== undefined) {
      formData.form.setValue(
        "name",
        Array.isArray(fields) ? fields[0] : fields,
        { shouldDirty: true, shouldValidate: true },
      );
    } else if (relation !== undefined) {
      formData.form.setValue("name", relation, {
        shouldDirty: true,
        shouldValidate: true,
      });
    }
  }
  const [foreign, setForeign] = React.useState(undefined as any);
  const db = formData.additionalData.db;
  const relationDef = relationList.find((v: any) => v.name === relation);
  const name = relationDef?.model || relation || "";
  const group = relationDef?.group || formData.additionalData.group;
  React.useEffect(() => {
    fetch(`/api/models/${db}/${group}`)
      .then((res) => res.json())
      .then((data) => {
        setForeign(data.find((v: any) => v.name === name));
      });
  }, [db, group, name]);
  const fieldsAutocomplete = React.useMemo(() => {
    // TODO ValueObject
    let types: string[] = [];
    if (type === "range") {
      types = [
        "char",
        "varchar",
        "uuid",
        "binary_uuid",
        "tinyint",
        "smallint",
        "int",
        "bigint",
        "float",
        "double",
        "decimal",
        "date",
        "time",
        "datetime",
        "timestamp",
      ];
    } else if (type === "identity") {
      types = [
        "char",
        "varchar",
        "uuid",
        "binary_uuid",
        "tinyint",
        "smallint",
        "int",
        "bigint",
        "date",
        "boolean",
        "db_enum",
        "db_set",
        "auto_fk",
      ];
    } else if (type === "full_text") {
      types = ["text"];
    } else if (type === "array_int") {
      types = ["array_int"];
    } else if (type === "array_string") {
      types = ["array_string"];
    } else if (type === "json") {
      types = ["json", "array_int", "array_string"];
    } else if (type === "geometry") {
      types = ["point", "geo_point", "geometry"];
    }
    return getFields(formData.additionalData, types);
  }, [formData.additionalData, type]);
  const relationAutocomplete = React.useMemo(
    () => relationList.map((v: any) => v.name),
    [relationList],
  );
  const additionalData = {
    ...formData.additionalData,
    group,
    modelData:
      formData.additionalData.selfGroup == group &&
        formData.additionalData.selfModel.name == name
        ? formData.additionalData.selfModel
        : foreign,
  };

  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField name="type" {...formData} />
        <AutoField name="required" {...formData} />
        <AutoField
          name="fields"
          {...formData}
          autocomplete={fieldsAutocomplete}
          hidden={type == "exists" || type == "any" || type == "raw_query"}
        />
        <AutoField
          name="relation"
          {...formData}
          autocomplete={relationAutocomplete}
          hidden={type !== "exists" && type !== "any"}
        />
        <AutoField
          name="relation_fields"
          {...formData}
          columns={[
            { field: "name", editable: true },
            { field: "type", editable: true },
          ]}
          dialog={Filter}
          resolver={yupResolver(
            createYupSchema(definitions.FilterJson, definitions),
          )}
          additionalData={additionalData}
          hidden={type !== "exists" && type !== "any"}
        />
        <AutoField name="json_path" {...formData} hidden={type !== "json"} />
        <AutoField
          name="query"
          {...formData}
          textarea
          hidden={type !== "raw_query"}
        />
      </SpaceBetween>
    </>
  );
}

function Order({ formData, definitions }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField
          name="fields"
          {...formData}
          columns={[{ field: "name", editable: true }]}
          dialog={OrderList}
          resolver={yupResolver(
            createYupSchema(definitions.OrderFieldJson, definitions),
          )}
        />
        <AutoField name="direction" {...formData} />
        <AutoField name="direct_sql" {...formData} textarea />
      </SpaceBetween>
    </>
  );
}

function OrderList({ formData }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField
          name="name"
          {...formData}
          autocomplete={getFields(formData.additionalData)}
        />
      </SpaceBetween>
    </>
  );
}

function getFields(additionalData: any, types?: string[]) {
  let fields = additionalData.modelData?.fields || [];
  fields = [...fields];
  const timestampable =
    additionalData.modelData?.timestampable ||
    additionalData.db_data?.timestampable;
  if (timestampable && timestampable !== "none") {
    if (!additionalData.modelData?.disable_created_at) {
      fields.push({
        name: additionalData.db_data?.rename_created_at || "created_at",
        type: "datetime",
      });
    }
    if (!additionalData.modelData?.disable_updated_at) {
      fields.push({
        name: additionalData.db_data?.rename_updated_at || "updated_at",
        type: "datetime",
      });
    }
  }
  const soft_delete =
    additionalData.modelData?.soft_delete ||
    additionalData.db_data?.soft_delete;
  if (soft_delete == "time") {
    fields.push({
      name: additionalData.db_data?.rename_deleted_at || "deleted_at",
      type: "datetime",
    });
  } else if (soft_delete == "flag") {
    fields.push({
      name: additionalData.db_data?.rename_deleted || "deleted",
      type: "boolean",
    });
  } else if (soft_delete == "unix_time") {
    fields.push({
      name: additionalData.db_data?.rename_deleted || "deleted",
      type: "int",
    });
  }
  return fields
    .filter((v: any) => types === undefined || types.includes(v.type))
    .map((v: any) => v.name);
}
