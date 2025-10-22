import { Controller } from "react-hook-form";
import FormField from "@cloudscape-design/components/form-field";
import Input from "@cloudscape-design/components/input";
import Autosuggest from "@cloudscape-design/components/autosuggest";
import Textarea from "@cloudscape-design/components/textarea";

import { getDefinition } from "./lib";
import AutoObjectArray from "./AutoObjectArray";
import AutoCheckbox from "./AutoCheckbox";
import AutoSelect from "./AutoSelect";
import AutoMultiSelect from "./AutoMultiSelect";
import AutoRadio from "./AutoRadio";
import AutoObject from "./AutoObject";
import AutoCodeEditor from "./AutoCodeEditor";
import AutoNullableCheckbox from "./AutoNullableCheckbox";
import AutoMultiInput from "./AutoMultiInput";

interface Props {
  name: string;
  dialogTitle?: string;
  path?: string;
  form: any;
  errors?: any;
  schema: any;
  definitions: any;
  formType?: string;
  disabled?: boolean;
  hidden?: boolean;
  columns?: any[];
  dialog?: ({ formData }: any) => any;
  resolver?: any;
  component?: ({ formData }: any) => any;
  isModal?: boolean;
  textarea?: boolean;
  codeEditor?: boolean;
  autocomplete?: string[];
  options?: string[];
  dirtyDialog: boolean;
  setDirtyDialog: any;
  additionalData?: any;
}

function AutoField(props: Props) {
  const name = props.name;
  const path = props.path || "";
  const form = props.form;
  const errors = props.errors || form.formState.errors;
  const schema = props.schema;
  const definitions = props.definitions;
  const required = (schema.required || []).includes(name);
  const property = schema.properties[name];
  if (property === undefined) {
    console.error(`${name} is not in the properties.`);
    return <></>;
  }
  const label = property.title || name;
  const labelWithOptionality = (required ? label : <span>{label} <i>- optional</i></span>);
  const definition = getDefinition(name, property, definitions);
  const type = Array.isArray(definition.type)
    ? definition.type[0]
    : definition.type;
  if (definition.oneOf) {
    if ("enum" in definition.oneOf[0]) {
      const values = [];
      for (const row of definition.oneOf) {
        for (const e of row.enum) {
          values.push({
            const: e,
            title: row.title,
            description: row.description,
          });
        }
      }
      delete definition.oneOf;
      definition.enum = values;
    }
  }

  if (type === "array") {
    const items = definition.items.$ref
      ? definitions[definition.items.$ref.replace("#/definitions/", "")]
      : definition.items;

    if (items.type === "object") {
      if (props.hidden && (form.getValues(name) === undefined || form.getValues(name)?.length == 0)) {
        return <></>;
      }
      return (
        <AutoObjectArray
          name={name}
          path={path}
          form={form}
          definition={definition}
          errors={errors}
          label={label}
          dialogTitle={props.dialogTitle}
          definitions={definitions}
          items={items}
          columns={props.columns}
          dialog={props.dialog}
          resolver={props.resolver}
          isModal={props.isModal}
          dirtyDialog={props.dirtyDialog}
          setDirtyDialog={props.setDirtyDialog}
          additionalData={props.additionalData}
        />
      );
    } else if (props.options) {
      if (props.hidden && (form.getValues(name) === undefined || form.getValues(name)?.length == 0)) {
        return <></>;
      }
      return (
        <AutoMultiSelect
          name={name}
          disabled={props.disabled}
          path={path}
          form={form}
          definition={definition}
          required={required}
          errors={errors}
          label={labelWithOptionality}
          values={props.options}
        />
      );
    } else {
      if (props.hidden && !form.getValues(name)) {
        return <></>;
      }
      return (
        <AutoMultiInput
          name={name}
          path={path}
          form={form}
          definition={definition}
          errors={errors}
          label={labelWithOptionality}
          required={required}
          autocomplete={props.autocomplete || property.autocomplete}
          isModal={props.isModal}
        />
      );
    }
  } else if (type == "object") {
    if (props.hidden && (form.getValues(name) === undefined || JSON.stringify(form.getValues(name)) === "{}")) {
      return <></>;
    }
    return (
      <AutoObject
        name={name}
        path={path}
        form={form}
        errors={errors}
        label={label}
        definitions={definitions}
        definition={definition}
        component={props.component}
      />
    );
  } else if (definition.enum) {
    if (props.formType === "radio") {
      if (props.hidden && !form.getValues(name)) {
        return <></>;
      }
      return (
        <AutoRadio
          name={name}
          path={path}
          form={form}
          definition={definition}
          errors={errors}
          label={labelWithOptionality}
          values={definition.enum}
        />
      );
    } else {
      if (props.hidden && !form.getValues(name)) {
        return <></>;
      }
      return (
        <AutoSelect
          name={name}
          disabled={props.disabled}
          path={path}
          form={form}
          definition={definition}
          required={required}
          errors={errors}
          label={labelWithOptionality}
          values={definition.enum}
        />
      );
    }
  } else if (type == "boolean" && definition.nullable) {
    if (props.hidden && (form.getValues(name) === undefined || form.getValues(name) === null || form.getValues(name) === "")) {
      return <></>;
    }
    return (
      <AutoNullableCheckbox
        name={name}
        path={path}
        form={form}
        definition={definition}
        errors={errors}
        label={label}
      />
    );
  } else if (type == "boolean") {
    if (props.hidden && !form.getValues(name)) {
      return <></>;
    }
    return (
      <AutoCheckbox
        name={name}
        disabled={props.disabled}
        path={path}
        form={form}
        definition={definition}
        errors={errors}
        label={label}
      />
    );
  } else if (type == "integer") {
    if (props.hidden && (form.getValues(name) === undefined || form.getValues(name) === null)) {
      return <></>;
    }
    return (
      <Controller
        name={`${path}${name}`}
        control={form.control}
        render={({ field }) => (
          <FormField
            description={definition.description}
            label={labelWithOptionality}
            errorText={errors[name]?.message}
          >
            <Input
              type="number"
              ariaRequired={required}
              disabled={props.disabled}
              onChange={({ detail }) =>
                form.setValue(
                  `${path}${name}`,
                  detail.value === "" || detail.value === undefined
                    ? null
                    : Number(detail.value),
                  { shouldDirty: true, shouldValidate: true },
                )
              }
              onBlur={field.onBlur}
              value={field.value}
              name={name}
              ref={field.ref}
            />
          </FormField>
        )}
      />
    );
  } else if (props.autocomplete || property.autocomplete) {
    if (props.hidden && !form.getValues(name)) {
      return <></>;
    }
    return (
      <Controller
        name={name}
        control={form.control}
        render={({ field }) => {
          return (
            <FormField
              description={definition.description}
              label={labelWithOptionality}
              errorText={errors[name]?.message}
            >
              <Autosuggest
                ariaRequired={required}
                disabled={props.disabled}
                onChange={({ detail }) =>
                  form.setValue(
                    `${path}${name}`,
                    detail.value === "" || detail.value === undefined
                      ? null
                      : detail.value,
                    { shouldDirty: true, shouldValidate: true },
                  )
                }
                onBlur={field.onBlur}
                value={field.value || ""}
                name={name}
                ref={field.ref}
                options={(props.autocomplete || property.autocomplete).map(
                  (v: any) => ({ value: v }),
                )}
              />
            </FormField>
          );
        }}
      />
    );
  } else if (props.textarea) {
    if (props.hidden && !form.getValues(name)) {
      return <></>;
    }
    return (
      <Controller
        name={`${path}${name}`}
        control={form.control}
        render={({ field }) => (
          <FormField
            description={definition.description}
            label={labelWithOptionality}
            errorText={errors[name]?.message}
          >
            <Textarea
              ariaRequired={required}
              disabled={props.disabled}
              onChange={({ detail }) =>
                form.setValue(
                  `${path}${name}`,
                  detail.value === "" || detail.value === undefined
                    ? null
                    : detail.value,
                  { shouldDirty: true, shouldValidate: true },
                )
              }
              onBlur={field.onBlur}
              value={field.value}
              name={name}
              ref={field.ref}
              rows={Math.max(3, (field.value || "").split("\n").length)}
            />
          </FormField>
        )}
      />
    );
  } else if (props.codeEditor) {
    if (props.hidden && !form.getValues(name)) {
      return <></>;
    }
    return (
      <AutoCodeEditor
        name={name}
        path={path}
        form={form}
        definition={definition}
        errors={errors}
        label={labelWithOptionality}
      />
    );
  }
  if (props.hidden && !form.getValues(name)) {
    return <></>;
  }
  return (
    <Controller
      name={`${path}${name}`}
      control={form.control}
      render={({ field }) => (
        <FormField
          description={definition.description}
          label={labelWithOptionality}
          errorText={errors[name]?.message}
        >
          <Input
            type="text"
            ariaRequired={required}
            disabled={props.disabled}
            onChange={({ detail }) =>
              form.setValue(
                `${path}${name}`,
                detail.value === "" || detail.value === undefined
                  ? null
                  : detail.value,
                { shouldDirty: true, shouldValidate: true },
              )
            }
            onBlur={field.onBlur}
            value={field.value}
            name={name}
            ref={field.ref}
          />
        </FormField>
      )}
    />
  );
}

export default AutoField;
