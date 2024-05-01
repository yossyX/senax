import { Controller } from "react-hook-form";
import FormField from "@cloudscape-design/components/form-field";
import Multiselect from "@cloudscape-design/components/multiselect";

interface Props {
  name: string;
  path: string;
  form: any;
  definition: any;
  disabled?: boolean;
  required: boolean;
  errors: object;
  label: string;
  values: any[];
}

function AutoMultiSelect(props: Props) {
  const name = props.name;
  const path = props.path;
  const form = props.form;
  const errors = props.errors as any;
  const label = props.label;
  const options = [] as any[];
  for (const val of props.values) {
    if (typeof val === "string") {
      options.push({ value: val, label: val });
    } else {
      options.push({
        value: val.const,
        label: val.title || val.const,
        description: val.description,
      });
    }
  }

  return (
    <Controller
      name={`${path}${name}`}
      control={form.control}
      render={({ field }) => (
        <FormField
          label={label}
          description={props.definition.description}
          errorText={errors[name]?.message}
        >
          <Multiselect
            ariaRequired={props.required}
            selectedOptions={options.filter((option) =>
              (field.value || []).includes(option.value),
            )}
            onChange={({ detail }) =>
              form.setValue(
                `${path}${name}`,
                detail.selectedOptions.map((x) => x.value),
                { shouldDirty: true, shouldValidate: true },
              )
            }
            onBlur={field.onBlur}
            options={options}
            disabled={props.disabled}
            ref={field.ref}
          />
        </FormField>
      )}
    />
  );
}

export default AutoMultiSelect;
