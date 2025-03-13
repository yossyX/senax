import { Controller } from "react-hook-form";
import FormField from "@cloudscape-design/components/form-field";
import Input from "@cloudscape-design/components/input";
import AttributeEditor from "@cloudscape-design/components/attribute-editor";
import ButtonDropdown from "@cloudscape-design/components/button-dropdown";
import Autosuggest from "@cloudscape-design/components/autosuggest";

interface Props {
  name: string;
  path: string;
  form: any;
  definition: any;
  disabled?: boolean;
  required: boolean;
  errors: object;
  label: string;
  autocomplete?: any[];
  isModal?: boolean;
}

export default (props: Props) => {
  const name = props.name;
  const path = props.path;
  const form = props.form;
  const errors = props.errors as any;
  const label = props.label;
  const fullPath = `${path}${name}`;
  let itemError: any, baseError: any
  if (Array.isArray(errors[name])) {
    itemError = errors[name];
  } else if (typeof errors[name] === "object") {
    baseError = errors[name];
  } 

  return (
    <Controller
      name={fullPath}
      control={form.control}
      render={({ field }) => {
        return (
          <FormField
            label={(field.value || []).length == 0 ? label : null}
            errorText={baseError?.message}
            stretch={props.isModal}
          >
            <AttributeEditor
              additionalInfo={props.definition.description}
              onAddButtonClick={() => {
                const value = field.value || [];
                value.push("");
                form.setValue(
                  fullPath,
                  value,
                  { shouldDirty: true },
                );
              }}
              onRemoveButtonClick={({
                detail: { itemIndex }
              }) => {
                const value = field.value || [];
                value.splice(itemIndex, 1);
                form.setValue(
                  fullPath,
                  value,
                  { shouldDirty: true, shouldValidate: true },
                );
              }}
              items={field.value || []}
              customRowActions={({ itemIndex }) => {
                const onClick = ({ detail: { id } }: any) => {
                  const value = field.value || [];
                  const item = value[itemIndex];
                  switch (id) {
                    case "move-up":
                      value[itemIndex] =
                        value[itemIndex - 1];
                      value[itemIndex - 1] = item;
                      break;
                    case "move-down":
                      value[itemIndex] =
                        value[itemIndex + 1];
                      value[itemIndex + 1] = item;
                      break;
                  }
                  form.setValue(
                    fullPath,
                    value,
                    { shouldDirty: true, shouldValidate: true },
                  );
                };
                return (
                  <ButtonDropdown
                    items={[
                      { text: "Move up", id: "move-up" },
                      { text: "Move down", id: "move-down" }
                    ]}
                    ariaLabel={`Remove item ${itemIndex + 1}`}
                    mainAction={{
                      text: "Remove",
                      onClick: () => {
                        const value = field.value || [];
                        value.splice(itemIndex, 1);
                        form.setValue(
                          fullPath,
                          value,
                          { shouldDirty: true, shouldValidate: true },
                        );
                      }
                    }}
                    onItemClick={onClick}
                  />
                );
              }}
              addButtonText="Add new item"
              definition={[
                {
                  label: label,
                  errorText: (_item, itemIndex) => itemError?.at(itemIndex)?.message,
                  control: (item: string, itemIndex: number) => (
                    props.autocomplete ?
                      <Autosuggest
                        disabled={props.disabled}
                        onChange={({ detail }) => {
                          const value = field.value || [];
                          value[itemIndex] = detail.value;
                          form.setValue(
                            fullPath,
                            value,
                            { shouldDirty: true, shouldValidate: true },
                          )
                        }}
                        onBlur={field.onBlur}
                        value={item}
                        name={name}
                        ref={field.ref}
                        options={(props.autocomplete).map(
                          (v: any) => ({ value: v }),
                        )}
                      /> :
                      <Input
                        value={item}
                        onChange={({ detail }) => {
                          const value = field.value || [];
                          value[itemIndex] = detail.value;
                          form.setValue(
                            fullPath,
                            value,
                            { shouldDirty: true, shouldValidate: true },
                          )
                        }}
                      />
                  )
                }
              ]}
            />
          </FormField>
        );
      }}
    />
  );
}
