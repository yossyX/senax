import * as React from "react";
import { Helmet } from "react-helmet-async";
import {
  useLoaderData,
  useNavigate,
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

import AutoField from "@/components/AutoField";
import { createYupSchema } from "@/components/AutoField/lib";

function Config() {
  const [jsonSchema, data] = useLoaderData() as any;
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
        <title>Senax Api Configuration</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">Api Config</Header>}>
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
          >
            <SpaceBetween direction="vertical" size="xs">
              <AutoField name="camel_case" {...formData} />
              <AutoField name="with_label" {...formData} />
              <AutoField name="with_comment" {...formData} />
              <AutoField name="hide_timestamp" {...formData} />
              <AutoField name="selector_limit" {...formData} />
              <AutoField
                name="roles"
                {...formData}
                columns={[
                  { field: "name", width: 200, editable: true },
                  { field: "alias", width: 200, editable: true },
                ]}
                dialog={Roles}
                resolver={yupResolver(
                  createYupSchema(definitions.ApiRoleJson, definitions),
                )}
              />
              <AutoField name="default_role" {...formData} />
            </SpaceBetween>
          </Form>
        </Container>
      </ContentLayout>
    </>
  );
}
export default Config;

function Roles({ formData }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField name="name" {...formData} />
        <AutoField name="alias" {...formData} />
      </SpaceBetween>
    </>
  );
}
