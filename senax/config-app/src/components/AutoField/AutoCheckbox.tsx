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

function AutoCheckbox(props: Props) {
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
            onChange={({ detail }) =>
              form.setValue(`${path}${name}`, detail.checked, {
                shouldDirty: true,
                shouldValidate: true,
              })
            }
            onBlur={field.onBlur}
            checked={!!field.value}
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

export default AutoCheckbox;
