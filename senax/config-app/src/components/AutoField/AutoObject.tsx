import * as React from "react";
import Container from "@cloudscape-design/components/container";
import Header from "@cloudscape-design/components/header";

interface Props {
  name: string;
  path: string;
  form: any;
  errors: object;
  label: string;
  definitions: object;
  definition: object;
  component: any;
}

function AutoObject(props: Props) {
  const name = props.name;
  const path = props.path;
  const form = props.form;
  const errors = props.errors as any;
  const label = props.label;
  const definitions = props.definitions;
  const definition = props.definition;
  const component = props.component;
  if (component === undefined) {
    console.error(`${name} requires component.`);
    return <></>;
  }

  const formData = {
    path: `${path}${name}.`,
    form,
    errors: errors[name] || {},
    schema: definition,
    definitions,
  };

  return (
    <>
      <Container header={<Header variant="h3">{label}</Header>}>
        {React.createElement(component, { formData })}
      </Container>
    </>
  );
}

export default AutoObject;
