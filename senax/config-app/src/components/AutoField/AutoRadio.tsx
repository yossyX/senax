import { Controller } from "react-hook-form";
import FormField from "@cloudscape-design/components/form-field";
import RadioGroup from "@cloudscape-design/components/radio-group";

interface Props {
  name: string;
  path: string;
  form: any;
  definition: any;
  errors: object;
  label: string;
  values: any[];
}

function AutoRadio(props: Props) {
  const name = props.name;
  const path = props.path;
  const form = props.form;
  const errors = props.errors as any;
  const label = props.label;
  const values = props.values;

  return (
    <Controller
      name={`${path}${name}`}
      control={form.control}
      render={({ field }) => (
        <FormField
          label={label}
          constraintText={props.definition.description}
          errorText={errors[name]?.message}
        >
          <RadioGroup
            onChange={({ detail }) =>
              form.setValue(
                `${path}${name}`,
                detail.value === "" || detail.value === undefined
                  ? null
                  : detail.value,
                { shouldDirty: true, shouldValidate: true },
              )
            }
            value={
              field.value === undefined || field.value === null
                ? ""
                : field.value
            }
            name={name}
            ref={field.ref}
            items={values.map((val) => {
              if (typeof val === "string") {
                return { value: val, label: val };
              } else {
                return { value: val.const, label: val.title || val.const };
              }
            })}
          />
        </FormField>
      )}
    />
  );
}

export default AutoRadio;
