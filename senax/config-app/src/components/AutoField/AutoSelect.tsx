import { Controller } from "react-hook-form";
import FormField from "@cloudscape-design/components/form-field";
import Select from "@cloudscape-design/components/select";

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

function AutoSelect(props: Props) {
  const name = props.name;
  const path = props.path;
  const form = props.form;
  const errors = props.errors as any;
  const label = props.label;
  const values = [
    { value: "", label: "", description: "Not selected" },
    ...props.values,
  ];
  const options = [] as any[];
  for (const val of values) {
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
          <Select
            ariaRequired={props.required}
            selectedOption={
              options.find((option) => option.value === field.value) ?? null
            }
            onChange={({ detail }) =>
              form.setValue(
                `${path}${name}`,
                detail.selectedOption.value === "" ||
                  detail.selectedOption.value === undefined
                  ? null
                  : detail.selectedOption.value,
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

export default AutoSelect;
