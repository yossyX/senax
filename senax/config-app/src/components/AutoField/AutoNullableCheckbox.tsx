import { Controller } from "react-hook-form";
import FormField from "@cloudscape-design/components/form-field";
import Checkbox from "@cloudscape-design/components/checkbox";

interface Props {
  name: string;
  path: string;
  form: any;
  definition: any;
  disabled?: boolean;
  errors: object;
  label: string;
}

function AutoNullableCheckbox(props: Props) {
  const name = props.name;
  const path = props.path;
  const form = props.form;
  const errors = props.errors as any;
  const label = props.label;

  return (
    <Controller
      name={`${path}${name}`}
      control={form.control}
      render={({ field }) => (
        <FormField errorText={errors[name]?.message}>
          <Checkbox
            name={name}
            onChange={({ detail }) => {
              let value = detail.checked as any;
              if (field.value === undefined || field.value === null) {
                value = true;
              } else if (field.value) {
                value = false;
              } else {
                value = undefined;
              }
              form.setValue(`${path}${name}`, value, {
                shouldDirty: true,
                shouldValidate: true,
              })
            }
            }
            onBlur={field.onBlur}
            checked={!!field.value}
            indeterminate={field.value === undefined || field.value === null}
            disabled={props.disabled}
            description={props.definition.description}
            ref={field.ref}
          >
            {label}
          </Checkbox>
        </FormField>
      )}
    />
  );
}

export default AutoNullableCheckbox;
