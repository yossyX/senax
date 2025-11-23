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

function DbConfig() {
  const [jsonSchema, data, apiConfig, dbConfig] = useLoaderData() as any;
  const roles = apiConfig.roles.map((x: any) => x.name);
  const groups = dbConfig.groups.map((x: any) => x.name);
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
    additionalData: {
      groups,
      roles,
    },
  };
  return (
    <>
      <Helmet>
        <title>Senax API DB Configuration</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">API DB Config</Header>}>
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
              <AutoField name="promote_children" {...formData} />
              <AutoField
                name="groups"
                {...formData}
                columns={[{ field: "name", width: 200, editable: true }]}
                dialog={Groups}
                resolver={yupResolver(
                  createYupSchema(definitions.ApiGroupJson, definitions),
                )}
              />
            </SpaceBetween>
          </Form>
        </Container>
      </ContentLayout>
    </>
  );
}
export default DbConfig;

function Groups({ formData }: any) {
  return (
    <>
      <SpaceBetween direction="vertical" size="xs">
        <AutoField
          name="name"
          {...formData}
          autocomplete={formData.additionalData.groups}
        />
        <AutoField name="promote_children" {...formData} />
        <AutoField
          name="group"
          {...formData}
          autocomplete={formData.additionalData.groups}
        />
        <AutoField
          name="readable_roles"
          {...formData}
          options={formData.additionalData.roles}
        />
        <AutoField
          name="creatable_roles"
          {...formData}
          options={formData.additionalData.roles}
        />
        <AutoField
          name="importable_roles"
          {...formData}
          options={formData.additionalData.roles}
        />
        <AutoField
          name="updatable_roles"
          {...formData}
          options={formData.additionalData.roles}
        />
        <AutoField
          name="deletable_roles"
          {...formData}
          options={formData.additionalData.roles}
        />
      </SpaceBetween>
    </>
  );
}
