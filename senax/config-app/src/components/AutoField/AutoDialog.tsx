import * as React from "react";
import { useForm } from "react-hook-form";
import { CLOSE_DIALOG } from "./AutoObjectArray";
import Modal from "@cloudscape-design/components/modal";
import Box from "@cloudscape-design/components/box";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Button from "@cloudscape-design/components/button";

interface Props {
  path: string;
  index: number;
  setDialogIndex: any;
  schema: any;
  definitions: any;
  form: any;
  errors: object;
  update: any;
  component: any;
  resolver: any;
  header?: string;
  dirtyDialog: boolean;
  setDirtyDialog: any;
  additionalData?: any;
}

function AutoDialog(props: Props) {
  const schema: any = props.schema;
  const definitions = props.definitions;

  const form = useForm({
    defaultValues:
      props.index >= 0
        ? props.form.getValues(`${props.path}.${props.index}`)
        : {},
    resolver: props.resolver,
    mode: "all",
    errors: props.errors,
  });
  if (form.formState.isDirty && !props.dirtyDialog) {
    const setDirtyDialog = props.setDirtyDialog;
    setTimeout(() => setDirtyDialog(true), 0);
  }

  const onSubmit = (data: any) => {
    props.update(props.index, data);
    props.setDialogIndex(CLOSE_DIALOG);
  };
  const formData = {
    form,
    schema,
    definitions,
    isModal: true,
    dirtyDialog: props.dirtyDialog,
    setDirtyDialog: props.setDirtyDialog,
    additionalData: props.additionalData,
  };

  const handleClose = () => {
    props.setDialogIndex(CLOSE_DIALOG);
  };

  return (
    <Modal
      onDismiss={handleClose}
      visible={true}
      size="large"
      header={props.header}
      footer={
        <Box float="right">
          <SpaceBetween direction="horizontal" size="xs">
            <Button variant="link" onClick={handleClose}>
              Cancel
            </Button>
            <Button
              variant="primary"
              disabled={Object.entries(form.formState.errors).length > 0}
              onClick={form.handleSubmit(onSubmit) as any}
            >
              Ok
            </Button>
          </SpaceBetween>
        </Box>
      }
    >
      {React.createElement(props.component, { definitions, formData })}
    </Modal>
  );
}

export default AutoDialog;
