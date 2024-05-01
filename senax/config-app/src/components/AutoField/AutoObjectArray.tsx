import * as React from "react";
import AutoDialog from "./AutoDialog";
import yaml from "js-yaml";
import { useFieldArray } from "react-hook-form";
import {
  DragDropContext,
  Droppable,
  Draggable,
  DropResult,
} from "react-beautiful-dnd";
import Table from "@cloudscape-design/components/table";
import Box from "@cloudscape-design/components/box";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Button from "@cloudscape-design/components/button";
import Header from "@cloudscape-design/components/header";
import Icon from "@cloudscape-design/components/icon";
import Input from "@cloudscape-design/components/input";
import Select from "@cloudscape-design/components/select";
import Popover from "@cloudscape-design/components/popover";
import Checkbox from "@cloudscape-design/components/checkbox";
import Toggle from "@cloudscape-design/components/toggle";
import Container from "@cloudscape-design/components/container";
import FormField from "@cloudscape-design/components/form-field";

let ID_APPENDIX = 0;
// eslint-disable-next-line react-refresh/only-export-components
export const CLOSE_DIALOG = -2;

const generateId = () => {
  ID_APPENDIX += 1;
  return (
    Math.random().toString(36).substring(2) +
    Math.random().toString(36).substring(2) +
    ID_APPENDIX
  );
};

interface Props {
  name: string;
  path: string;
  form: any;
  definition: any;
  errors: object;
  label: string;
  dialogTitle?: string;
  definitions: any;
  items: any;
  columns: any[] | undefined;
  dialog: any;
  resolver: any;
  isModal?: boolean;
  dirtyDialog: boolean;
  setDirtyDialog: any;
  additionalData?: any;
}

function AutoObjectArray(props: Props) {
  const name = props.name;
  const path = props.path;
  const form = props.form;
  const errors = props.errors as any;
  const label = props.label;
  const definitions = props.definitions;
  const items = props.items;
  const columns = props.columns;
  const dialog = props.dialog;
  const resolver = props.resolver;

  const err = errors[name];
  const errorMsg = err
    ? Array.isArray(err)
      ? err
          .map((e) =>
            typeof e === "object"
              ? Object.entries(e)
                  .map(([_k, v]) => (v as any).message)
                  .join("\n")
              : e,
          )
          .join("\n")
      : err.message
    : "";

  const { fields, append, remove, move } = useFieldArray({
    name: `${path}${name}`,
    control: form.control,
  });
  const initialData = (fields || []).map(function (e: any) {
    return { ...e, _id_: generateId() };
  });
  const [rows, setRows] = React.useState(initialData);
  const [dialogIndex, setDialogIndex] = React.useState(CLOSE_DIALOG);
  const [reordering, setReorder] = React.useState(false);
  const [selectedItems, setSelectedItems] = React.useState([] as any);

  const handleEditClick = (id: string) => () => {
    const i = rows.findIndex((row) => row._id_ === id);
    setDialogIndex(i);
  };
  const isError = (id: string) => {
    const i = rows.findIndex((row) => row._id_ === id);
    return err && !!err[`${i}`];
  };

  const reorder = (startIndex: number, endIndex: number) => {
    const result = Array.from(rows);
    const [removed] = result.splice(startIndex, 1);
    result.splice(endIndex, 0, removed);
    return result;
  };

  const onDragEnd = async (result: DropResult) => {
    const { source, destination } = result;
    if (!destination) {
      return;
    }
    move(source.index, destination.index);
    const update = reorder(source.index, destination.index);
    setRows(update);
  };

  const handleDelete = () => {
    const msg =
      selectedItems.length == 1
        ? "Are you sure you want to delete an item?"
        : "Are you sure you want to delete items?";
    if (!confirm(msg)) {
      return;
    }
    remove(
      selectedItems.map((item: any) =>
        rows.findIndex((row) => row._id_ === item._id_),
      ),
    );
    setRows(
      rows.filter(
        (row) => !selectedItems.find((item: any) => row._id_ === item._id_),
      ),
    );
    setSelectedItems([]);
  };

  const processRowUpdate = (input: any) => {
    const updatedRow = { ...input };
    const idx = rows.findIndex((row) => row._id_ === input._id_);
    const data = { ...input };
    delete data._id_;
    delete data.id;
    const mergedData = { ...form.getValues(`${path}${name}.${idx}`), ...data };
    form.setValue(`${path}${name}.${idx}`, mergedData, { shouldDirty: true, shouldValidate: true });
    setRows(rows.map((row) => (row._id_ === input._id_ ? updatedRow : row)));
    return updatedRow;
  };

  const updateByDialog = (idx: number, input: any) => {
    const data = { ...input };
    delete data._id_;
    delete data.id;
    if (idx >= 0) {
      const mergedData = {
        ...form.getValues(`${path}${name}.${idx}`),
        ...data,
      };
      form.setValue(`${path}${name}.${idx}`, mergedData, { shouldDirty: true, shouldValidate: true });
      `${path}${name}`
      const tmpRows = [...rows];
      tmpRows[idx] = { ...tmpRows[idx], ...input };
      setRows(tmpRows);
    } else {
      append(data);
      setRows((oldRows) => [...oldRows, { ...data, _id_: generateId() }]);
    }
  };

  const handleCreate = () => {
    setDialogIndex(-1);
  };

  const cleanCopy = (obj: any) => {
    const result = {} as any;
    for (const key of Object.keys(obj)) {
      const val = obj[key];
      if (val === undefined || val === null) {
        /* empty */
      } else if (typeof val === "object") {
        if (Array.isArray(val)) {
          result[key] = val.map((v) => cleanCopy(v));
        } else {
          result[key] = cleanCopy(val);
        }
      } else {
        result[key] = val;
      }
    }
    return result as any;
  };
  const _columns = columns
    ? columns
    : Object.entries(items.properties).map(([k, _v]) => ({ field: k }));

  const columnDefinitions = [
    {
      id: "actions",
      header: "Actions",
      cell: (item: any) => {
        const dump = cleanCopy(item);
        delete dump.id;
        delete dump._id_;
        return (
          <SpaceBetween direction="horizontal" size="xs">
            <Button
              iconName="edit"
              variant="inline-icon"
              onClick={handleEditClick(item._id_)}
            />
            <Popover
              dismissButton={false}
              position="top"
              size="large"
              triggerType="custom"
              content={
                <div style={{ whiteSpace: "pre" }}>{yaml.dump(dump)}</div>
              }
            >
              <Icon name="status-info" variant={isError(item._id_) ? "error" : "normal"}/>
            </Popover>
          </SpaceBetween>
        );
      },
      width: 95,
      minWidth: 95,
    },
    ..._columns.map((e: any) => {
      if (!("header" in e)) {
        e.header = items.properties[e.field]?.title || e.field;
      }
      if (!("width" in e)) {
        e.width = 150;
      }
      let property = items.properties[e.field] || {};
      if (property.$ref) {
        const ref = property.$ref.replace("#/definitions/", "");
        property = definitions[ref];
      } else if (property.allOf) {
        const ref = property.allOf[0].$ref.replace("#/definitions/", "");
        property = definitions[ref];
      }
      const type = property?.type;
      if (!("cell" in e)) {
        if (property.enum) {
          e.cell = (v: any) => v[e.field];
        } else if (type == "boolean") {
          e.cell = (v: any) => (v[e.field] ? <Icon name="check" /> : "");
        } else if (type == "date") {
          e.cell = (v: any) => v[e.field];
        } else if (type == "integer") {
          e.cell = (v: any) => (
            <Box float="right">
              {Number.isFinite(v[e.field])
                ? new Intl.NumberFormat().format(v[e.field])
                : v[e.field]}
            </Box>
          );
        } else {
          e.cell = (v: any) => v[e.field];
        }
      }
      // eslint-disable-next-line no-constant-condition
      if (false && !props.isModal && e.editable && !("editConfig" in e)) {
        if (property.enum) {
          const values = [{ value: "", label: " " }, ...property.enum];
          const options = [] as any[];
          for (const val of values) {
            if (typeof val === "string") {
              options.push({ value: val, label: val });
            } else {
              options.push({ value: val.const, label: val.title || val.const });
            }
          }
          e.editConfig = {
            editingCell: (item: any, { currentValue, setValue }: any) => {
              return (
                <Select
                  ariaRequired={property.required}
                  selectedOption={
                    options.find(
                      (option) =>
                        option.value === (currentValue ?? item[e.field]),
                    ) ?? null
                  }
                  onChange={({ detail }) =>
                    setValue(
                      detail.selectedOption.value === ""
                        ? null
                        : detail.selectedOption.value,
                    )
                  }
                  options={Object.values(options)}
                />
              );
            },
          };
        } else if (type == "boolean") {
          e.editConfig = {
            editingCell: (item: any, { currentValue, setValue }: any) => {
              return (
                <Checkbox
                  onChange={({ detail }) => setValue(detail.checked)}
                  checked={currentValue ?? !!item[e.field]}
                ></Checkbox>
              );
            },
          };
        } else if (type == "date") {
          /* empty */
        } else if (type == "integer") {
          e.editConfig = {
            editingCell: (item: any, { currentValue, setValue }: any) => {
              return (
                <Input
                  type="number"
                  autoFocus={true}
                  ariaRequired={property.required}
                  value={currentValue ?? item[e.field]}
                  onChange={({ detail }) =>
                    setValue(detail.value === "" ? null : Number(detail.value))
                  }
                />
              );
            },
          };
        } else {
          e.editConfig = {
            editingCell: (item: any, { currentValue, setValue }: any) => {
              return (
                <Input
                  autoFocus={true}
                  ariaRequired={property.required}
                  value={currentValue ?? item[e.field]}
                  onChange={({ detail }) =>
                    setValue(detail.value === "" ? null : detail.value)
                  }
                />
              );
            },
          };
        }
      }
      return e;
    }),
  ];

  const firstColumn = _columns.find((_v) => true);

  return (
    <>
      <Box margin={{ top: "xs", bottom: "xs" }}>
        <Container
          header={
            <Header
              variant="h3"
              actions={
                <SpaceBetween
                  direction="horizontal"
                  size="xs"
                  alignItems="center"
                >
                  <Toggle
                    onChange={({ detail }) => setReorder(detail.checked)}
                    checked={reordering}
                  >
                    Reorder
                  </Toggle>
                  <Button
                    onClick={handleDelete}
                    disabled={reordering || selectedItems.length == 0}
                  >
                    Delete
                  </Button>
                  <Button variant="primary" onClick={handleCreate}>
                    Create
                  </Button>
                </SpaceBetween>
              }
            >
              {label}
            </Header>
          }
        >
          {reordering ? (
            <table style={{ width: "100%" }}>
              <DragDropContext onDragEnd={onDragEnd}>
                <Droppable droppableId={"dndTableBody"}>
                  {(provided) => (
                    <tbody ref={provided.innerRef} {...provided.droppableProps}>
                      {rows.map((item: any, index: number) => (
                        <Draggable
                          draggableId={item[firstColumn.field]}
                          index={index}
                          key={item[firstColumn.field]}
                        >
                          {(provided, _snapshot) => (
                            <tr
                              className={props.isModal ? "draggable" : ""}
                              key={item[firstColumn.field]}
                              ref={provided.innerRef}
                              {...provided.draggableProps}
                              {...provided.dragHandleProps}
                            >
                              <td style={{ padding: "4px" }}>
                                <Icon name="drag-indicator" />
                                &nbsp;{item[firstColumn.field]}
                              </td>
                            </tr>
                          )}
                        </Draggable>
                      ))}
                      {provided.placeholder}
                    </tbody>
                  )}
                </Droppable>
              </DragDropContext>
            </table>
          ) : (
            <FormField
              description={props.definition.description}
              errorText={errorMsg && errorMsg.trim()}
              stretch
            >
              <Table
                ariaLabels={{
                  activateEditLabel: (column, item) =>
                    `Edit ${item[firstColumn.field]} ${column.header}`,
                  cancelEditLabel: (column) =>
                    `Cancel editing ${column.header}`,
                  submitEditLabel: (column) =>
                    `Submit editing ${column.header}`,
                  tableLabel: "Table with inline editing",
                }}
                columnDefinitions={columnDefinitions}
                items={rows}
                submitEdit={async (item, column: any, newValue) => {
                  item[column.field] = newValue;
                  processRowUpdate(item);
                }}
                resizableColumns
                onSelectionChange={({ detail }) =>
                  setSelectedItems(detail.selectedItems)
                }
                selectedItems={selectedItems}
                selectionType="multi"
                variant="embedded"
                contentDensity="compact"
                empty={
                  <Box
                    margin={{ vertical: "xs" }}
                    textAlign="center"
                    color="inherit"
                  >
                    <SpaceBetween size="m">
                      <b>No resources</b>
                    </SpaceBetween>
                  </Box>
                }
              />
            </FormField>
          )}
        </Container>
      </Box>
      {dialogIndex > CLOSE_DIALOG ? (
        <AutoDialog
          path={`${path}${name}`}
          index={dialogIndex}
          setDialogIndex={setDialogIndex}
          schema={items}
          definitions={definitions}
          form={form}
          errors={err && err[`${dialogIndex}`]}
          update={updateByDialog}
          component={dialog}
          resolver={resolver}
          header={props.dialogTitle || label}
          dirtyDialog={props.dirtyDialog}
          setDirtyDialog={props.setDirtyDialog}
          additionalData={props.additionalData}
        />
      ) : (
        <></>
      )}
    </>
  );
}

export default AutoObjectArray;
