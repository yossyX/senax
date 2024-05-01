import * as React from "react";
import { Helmet } from "react-helmet-async";
import {
  useLoaderData,
  useNavigate,
  useRouteLoaderData,
  useSubmit,
  useBlocker,
} from "react-router-dom";
import { useForm } from "react-hook-form";
import { yupResolver } from "@hookform/resolvers/yup";
import Form from "@cloudscape-design/components/form";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Button from "@cloudscape-design/components/button";
import Container from "@cloudscape-design/components/container";
import { ContentLayout, Header } from "@cloudscape-design/components";
import Toggle from "@cloudscape-design/components/toggle";

import AutoField from "@/components/AutoField";
import { createYupSchema } from "@/components/AutoField/lib";

let DETAIL = false;

function Config() {
  const [detail, setDetail] = React.useState(DETAIL);
  DETAIL = detail;
  const [data, _vo_list] = useRouteLoaderData("db") as any;
  const jsonSchema: any = useLoaderData();
  const definitions = jsonSchema.definitions;
  const navigate = useNavigate();

  const yupSchema = createYupSchema(jsonSchema, definitions);
  const form = useForm({
    defaultValues: data,
    resolver: yupResolver(yupSchema),
    mode: "all",
  });

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
  };
  return (
    <>
      <Helmet>
        <title>Senax Database Configuration</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">Database Config</Header>}>
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
          >
            <SpaceBetween direction="vertical" size="xs">
              <AutoField name="title" {...formData} />
              <AutoField name="author" {...formData} />
              <AutoField name="db" {...formData} />
              <AutoField name="ignore_foreign_key" {...formData} />
              <AutoField name="disable_relation_index" {...formData} />
              <AutoField name="plural_table_name" {...formData} />
              <AutoField name="soft_delete" {...formData} />
              <AutoField
                name="add_soft_delete_column_to_relation_index"
                {...formData}
              />
              <AutoField name="timestampable" {...formData} />
              <AutoField name="time_zone" {...formData} />
              <AutoField name="timestamp_time_zone" {...formData} />
              <AutoField name="disable_timestamp_cache" {...formData} />
              <AutoField name="use_cache" {...formData} />
              <AutoField name="use_fast_cache" {...formData} />
              <AutoField name="use_storage_cache" {...formData} />
              <AutoField name="use_all_row_cache" {...formData} />
              <AutoField name="force_disable_cache" {...formData} />
              <AutoField name="use_clear_whole_cache" {...formData} />
              <AutoField name="use_update_notice" {...formData} />
              <AutoField name="use_insert_delayed" {...formData} />
              <AutoField name="use_save_delayed" {...formData} />
              <AutoField name="use_update_delayed" {...formData} />
              <AutoField name="use_upsert_delayed" {...formData} />
              <AutoField name="disable_update" {...formData} />
              <AutoField name="use_sequence" {...formData} />
              <AutoField name="tx_isolation" {...formData} />
              <AutoField name="read_tx_isolation" {...formData} />
              <AutoField name="engine" {...formData} />
              {/* <AutoField name="character_set" {...formData} /> */}
              <AutoField name="collation" {...formData} />
              <AutoField name="preserve_column_order" {...formData} />
              <AutoField name="excluded_from_domain" {...formData} />
              <AutoField name="export_db_layer" {...formData} />
              <AutoField name="use_label_as_sql_comment" {...formData} />
              <AutoField
                name="rename_created_at"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="label_of_created_at"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="rename_updated_at"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="label_of_updated_at"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="rename_deleted_at"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="label_of_deleted_at"
                {...formData}
                hidden={!detail}
              />
              <AutoField name="rename_deleted" {...formData} hidden={!detail} />
              <AutoField
                name="label_of_deleted"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="rename_aggregation_type"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="label_of_aggregation_type"
                {...formData}
                hidden={!detail}
              />
              <AutoField name="rename_version" {...formData} hidden={!detail} />
              <AutoField
                name="label_of_version"
                {...formData}
                hidden={!detail}
              />
              <AutoField
                name="groups"
                {...formData}
                columns={[
                  { field: "name", width: 200, editable: true },
                  { field: "label", width: 200, editable: true },
                  {
                    field: "exclude_group_from_table_name",
                    width: 300,
                    editable: true,
                  },
                ]}
                dialog={Groups}
                resolver={yupResolver(
                  createYupSchema(definitions.GroupJson, definitions),
                )}
              />
            </SpaceBetween>
          </Form>
        </Container>
      </ContentLayout>
    </>
  );
}
export default Config;

function Groups({ formData }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField name="label" {...formData} />
        <AutoField name="exclude_group_from_table_name" {...formData} />
      </SpaceBetween>
    </>
  );
}
