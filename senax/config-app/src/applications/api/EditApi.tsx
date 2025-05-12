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

import AutoField from "@/components/AutoField";
import { createYupSchema } from "@/components/AutoField/lib";
import Status404 from "@/pages/Status404";

function EditApi() {
  const params = useParams();
  const [api_models, jsonSchema, models, apiConfig, apiDbConfig] = useRouteLoaderData(
    "api_models"
  ) as any;
  const roles = apiConfig.roles.map((x: any) => x.name);
  const model_names = [];
  for (const model of models) {
    if (!api_models.find((v: any) => v.name === model.name)) {
      model_names.push(model.name);
    }
  }
  let data: any = {};
  if (params.model) {
    data = api_models.find((v: any) => v.name === params.model);
  }
  const navigate = useNavigate();
  const actionData = useActionData() as string | undefined;
  const definitions = jsonSchema.definitions;
  const yupSchema = createYupSchema(jsonSchema, definitions);
  const form = useForm({
    defaultValues: data,
    resolver: yupResolver(yupSchema),
    mode: "all",
  });
  const [errorMsg, setErrorMsg] = React.useState("");
  React.useEffect(() => {
    if (actionData) {
      if (actionData?.startsWith("{")) {
        const setError = (form: any, name: string, value: any) => {
          const code = value[0]?.code;
          if (code) {
            console.error(name);
            form.setError(name, { type: code, message: code });
          } else {
            for (const key in value) {
              setError(form, `${name}.${key}`, value[key]);
            }
          }
        };
        const errors = JSON.parse(actionData);
        for (const key in errors) {
          setError(form, `${key}`, errors[key]);
        }
      } else {
        setErrorMsg(actionData!);
      }
      alert("Failed to register data.");
    }
  }, [form, actionData]);

  const name = useWatch({
    control: form.control,
    name: "name",
  });
  const model_name = useWatch({
    control: form.control,
    name: "model",
  });
  const disable_mutation = useWatch({
    control: form.control,
    name: "disable_mutation",
  });
  const use_import = useWatch({
    control: form.control,
    name: "use_import",
  });
  const [modelData, setModelData] = React.useState(undefined as any);
  const server = params.server;
  const db_path = params.db;
  const group_path = params.group;
  React.useEffect(() => {
    fetch(`/api/api_server/${server}/${db_path}/${group_path}/_models`)
      .then((res) => res.json())
      .then((data) => {
        setModelData(data.find((v: any) => model_name ? (v.name === model_name) : (v.name === name)));
      });
  }, [db_path, group_path, name, model_name]);

  const db = apiDbConfig.db || params.db;
  let group = apiDbConfig.groups.find((e: any) => e.name === group_path).group || group_path;

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
      db,
      group,
      modelData,
      roles,
    },
  };
  if (data === undefined) {
    return <Status404 />;
  }
  return (
    <>
      <ScrollRestoration />
      <Helmet>
        <title>Senax API Configuration ({data.name || 'New'})</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">{data.name || 'New'}</Header>}>
        <Container header={<Header variant="h2"></Header>}>
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
            errorText={errorMsg}
          >
            <SpaceBetween direction="vertical" size="xs">
              <AutoField
                name="name"
                {...formData}
                disabled={!!params.model}
                autocomplete={model_names}
              />
              <AutoField
                name="model"
                {...formData}
                autocomplete={model_names}
              />
              <AutoField name="disable_auto_fields" {...formData} />
              <AutoField name="use_find_by_pk" {...formData} />
              <AutoField name="use_delete_by_pk" {...formData} />
              <AutoField name="disable_mutation" {...formData} />
              <AutoField name="use_import" {...formData} />
              <AutoField
                name="readable_roles"
                {...formData}
                options={formData.additionalData.roles}
              />
              <AutoField
                name="creatable_roles"
                {...formData}
                options={formData.additionalData.roles}
                hidden={disable_mutation}
              />
              <AutoField
                name="importable_roles"
                {...formData}
                options={formData.additionalData.roles}
                hidden={!use_import || disable_mutation}
              />
              <AutoField
                name="updatable_roles"
                {...formData}
                options={formData.additionalData.roles}
                hidden={disable_mutation}
              />
              <AutoField
                name="deletable_roles"
                {...formData}
                options={formData.additionalData.roles}
                hidden={disable_mutation}
              />
              <AutoField name="readable_filter" {...formData} textarea />
              <AutoField
                name="updatable_filter"
                {...formData}
                textarea
                hidden={disable_mutation}
              />
              <AutoField
                name="deletable_filter"
                {...formData}
                textarea
                hidden={disable_mutation}
              />
              <AutoField
                name="fields"
                {...formData}
                columns={[
                  { field: "name", editable: true },
                  { field: "visibility", editable: true },
                  { field: "required", editable: true },
                ]}
                dialog={Field}
                resolver={yupResolver(
                  createYupSchema(definitions.ApiFieldJson, definitions)
                )}
              />
              <AutoField
                name="relations"
                {...formData}
                columns={[
                  { field: "name", editable: true },
                  { field: "visibility", editable: true },
                  { field: "use_replace", editable: true },
                  { field: "disable_auto_fields", editable: true },
                ]}
                dialog={Relation}
                resolver={yupResolver(
                  createYupSchema(definitions.ApiRelationJson, definitions)
                )}
              />
              <AutoField
                name="selector"
                {...formData}
                columns={[
                  { field: "name", editable: true },
                  { field: "use_for_update_by_operator", editable: true },
                  { field: "use_for_delete", editable: true },
                ]}
                dialog={Selector}
                resolver={yupResolver(
                  createYupSchema(definitions.ApiSelectorJson, definitions)
                )}
              />
            </SpaceBetween>
          </Form>
        </Container>
      </ContentLayout>
    </>
  );
}
export default EditApi;

function Field({ formData }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField
          name="name"
          {...formData}
          autocomplete={formData.additionalData.modelData?.merged_fields.map((v: any) => v[0]) || []}
        />
        <AutoField name="visibility" {...formData} />
        <AutoField name="required" {...formData} />
        <AutoField name="disable_update" {...formData} />
        <AutoField name="validator" {...formData} />
        <AutoField name="default" {...formData} />
        <AutoField name="on_insert_formula" {...formData} />
        <AutoField name="on_update_formula" {...formData} />
      </SpaceBetween>
    </>
  );
}

function Relation({ formData, definitions }: any) {
  const relations = formData.additionalData.modelData?.merged_relations || [];
  const relation = useWatch({
    control: formData.form.control,
    name: "name",
  });
  const [foreign, setForeign] = React.useState(undefined as any);
  const relationDef = relations.find((v: any) => v[0] == relation)?.[1];
  const db = (relationDef?.db || formData.additionalData.db);
  const ms = (relationDef?.model || relation || "").split("::") || [];
  const name = ms.pop();
  const group = relationDef?.group || (ms.length > 0 ? ms[0] : formData.additionalData.group);
  React.useEffect(() => {
    fetch(`/api/merged_models/${db}/${group}`)
      .then((res) => res.json())
      .then((data) => {
        setForeign(data.find((v: any) => v.name === name));
      });
  }, [db, group, name]);
  const additionalData = {
    ...formData.additionalData,
    db,
    group,
    modelData: foreign,
  };
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField
          name="name"
          {...formData}
          autocomplete={relations.map((v: any) => v[0]) || []}
        />
        <AutoField name="visibility" {...formData} />
        <AutoField name="use_replace" {...formData} />
        <AutoField name="disable_auto_fields" {...formData} />
        <AutoField
          name="fields"
          {...formData}
          columns={[
            { field: "name", editable: true },
            { field: "visibility", editable: true },
            { field: "required", editable: true },
          ]}
          hidden={!foreign}
          dialogTitle={
            formData.schema.properties["fields"].title +
            " (" +
            formData.additionalData.modelData.name +
            ")"
          }
          dialog={Field}
          resolver={yupResolver(
            createYupSchema(definitions.ApiFieldJson, definitions)
          )}
          additionalData={additionalData}
        />
        <AutoField
          name="relations"
          {...formData}
          columns={[
            { field: "name", editable: true },
            { field: "visibility", editable: true },
            { field: "use_replace", editable: true },
            { field: "disable_auto_fields", editable: true },
          ]}
          hidden={!foreign}
          dialogTitle={
            formData.schema.properties["relations"].title +
            " (" +
            formData.additionalData.modelData.name +
            ")"
          }
          dialog={Relation}
          resolver={yupResolver(
            createYupSchema(definitions.ApiRelationJson, definitions)
          )}
          additionalData={additionalData}
        />
      </SpaceBetween>
    </>
  );
}

function Selector({ formData, definitions }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField
          name="name"
          {...formData}
          autocomplete={formData.additionalData.modelData?.selectors?.map(
            (v: any) => v.name
          )}
        />
        <AutoField
          name="js_updater"
          {...formData}
          columns={[{ field: "name", editable: true }]}
          dialog={JsUpdater}
          resolver={yupResolver(
            createYupSchema(definitions.JsUpdaterJson, definitions)
          )}
        />
        <AutoField name="use_for_update_by_operator" {...formData} />
        <AutoField name="use_for_delete" {...formData} />
        <AutoField name="limit" {...formData} />
        </SpaceBetween>
    </>
  );
}

function JsUpdater({ formData }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField name="script" {...formData} codeEditor />
      </SpaceBetween>
    </>
  );
}
