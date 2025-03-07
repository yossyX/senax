import { Controller } from "react-hook-form";
import FormField from "@cloudscape-design/components/form-field";
import Autosuggest from "@cloudscape-design/components/autosuggest";
import TokenGroup from "@cloudscape-design/components/token-group";
import SpaceBetween from "@cloudscape-design/components/space-between";

interface Props {
  name: string;
  path: string;
  form: any;
  definition: any;
  disabled?: boolean;
  required: boolean;
  errors: object;
  label: string;
  autocomplete: any[];
}

function AutoMultiSuggest(props: Props) {
  const name = props.name;
  const path = props.path;
  const form = props.form;
  const errors = props.errors as any;
  const label = props.label;
  return (
    <Controller
      name={`${path}${name}`}
      control={form.control}
      render={({ field }) => {
        return (
          <FormField
            label={label}
            description={props.definition.description}
            errorText={errors[name]?.message}
          >
            <SpaceBetween size="xs">
              <TokenGroup
                onDismiss={({ detail: { itemIndex } }) => {
                  const value = field.value || [];
                  form.setValue(
                    `${path}${name}`,
                    [
                      ...value.slice(0, itemIndex),
                      ...value.slice(itemIndex + 1)
                    ],
                    { shouldDirty: true, shouldValidate: true },
                  )
                }}
                items={(field.value || []).map((v: string) => ({ label: v, dismissLabel: `Remove ${v}` }))}
              />
              <Autosuggest
                disabled={props.disabled}
                onChange={({ detail }) => {
                  const item = detail.value.trim();
                  if (item) {
                    const value = field.value || [];
                    if (!value.find((v: string) => v == item)) {
                      value.push(item);
                      form.setValue(
                        `${path}${name}`,
                        value,
                        { shouldDirty: true, shouldValidate: true },
                      )
                    }
                  }
                }}
                onBlur={field.onBlur}
                value={""}
                name={name}
                ref={field.ref}
                options={(props.autocomplete).map(
                  (v: any) => ({ value: v }),
                )}
              />
            </SpaceBetween>
          </FormField>
        );
      }}
    />
  );
}

export default AutoMultiSuggest;
