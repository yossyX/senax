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

import AutoField from "@/components/AutoField";
import { createYupSchema } from "@/components/AutoField/lib";
import Status404 from "@/pages/Status404";

let DETAIL = false;

function EditValue() {
  const [detail, setDetail] = React.useState(DETAIL);
  DETAIL = detail;
  const [voList, jsonSchema] = useRouteLoaderData("index") as [any[], any];
  const params = useParams();
  let data: any = {};
  if (params.vo) {
    data = voList.find((v) => v.name === params.vo);
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

  const type = useWatch({
    control: form.control,
    name: "type",
  });
  const NUMBER = ["tinyint", "smallint", "int", "bigint", "float", "double"];

  if (data === undefined) {
    return <Status404 />;
  }
  return (
    <>
      <ScrollRestoration />
      <Helmet>
        <title>Senax Value Object Configuration ({data.name || 'New'})</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">{data.name || 'New'}</Header>}>
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
              <AutoField name="type" {...formData} />
              <AutoField name="signed" {...formData} />
              <AutoField
                name="length"
                {...formData}
                hidden={!["text", "blob", "varchar"].includes(type)}
              />
              <AutoField
                name="max"
                {...formData}
                hidden={!NUMBER.includes(type)}
              />
              <AutoField
                name="min"
                {...formData}
                hidden={!NUMBER.includes(type)}
              />
              <AutoField
                name="collation"
                {...formData}
                hidden={!["text", "varchar"].includes(type)}
              />
              <AutoField
                name="precision"
                {...formData}
                hidden={type !== "decimal"}
              />
              <AutoField
                name="scale"
                {...formData}
                hidden={type !== "decimal"}
              />
              <AutoField
                name="output_time_zone"
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
              <AutoField
                name="exclude_from_cache"
                {...formData}
                hidden={!detail}
              />
              <AutoField name="skip_factory" {...formData} hidden={!detail} />
              <AutoField name="column_name" {...formData} hidden={!detail} />
              <AutoField name="srid" {...formData} hidden={!detail} />
              <AutoField name="default" {...formData} hidden={!detail} />
              <AutoField name="query" {...formData} hidden={!detail} />
              <AutoField name="stored" {...formData} hidden={!detail} />
              <AutoField name="sql_comment" {...formData} hidden={!detail} />
              <AutoField name="hidden" {...formData} hidden={!detail} />
              <AutoField name="secret" {...formData} hidden={!detail} />
            </SpaceBetween>
          </Form>
        </Container>
      </ContentLayout>
    </>
  );
}
export default EditValue;

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
