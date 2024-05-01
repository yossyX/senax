import * as React from "react";
import {
  ScrollRestoration,
  useNavigate,
  useParams,
  useRouteLoaderData,
  Link,
} from "react-router-dom";
import { Helmet } from "react-helmet-async";
import {
  DragDropContext,
  Droppable,
  Draggable,
  DropResult,
} from "react-beautiful-dnd";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Button from "@cloudscape-design/components/button";
import Container from "@cloudscape-design/components/container";
import { ContentLayout, Header } from "@cloudscape-design/components";
import Toggle from "@cloudscape-design/components/toggle";
import Icon from "@cloudscape-design/components/icon";
import Table from "@cloudscape-design/components/table";
import Box from "@cloudscape-design/components/box";

function Models() {
  const navigate = useNavigate();
  const params = useParams();
  const group = params.group;
  const [ini_models, _jsonSchema] = useRouteLoaderData("group") as any;
  const [models, setModels] = React.useState(ini_models);
  const [reordering, setReorder] = React.useState(false);
  const [selectedItems, setSelectedItems] = React.useState([] as any);
  const handleDelete = async () => {
    const msg =
      selectedItems.length == 1
        ? `Are you sure you want to delete ${selectedItems[0].name}?`
        : "Are you sure you want to delete items?";
    if (!confirm(msg)) {
      return;
    }
    for (const item of selectedItems) {
      const res = await fetch(
        `/api/models/${params.db}/${params.group}/${item.name}`,
        {
          method: "DELETE",
        },
      );
      if (!res.ok) {
        const response = await res.text();
        alert(response);
        return;
      }
      setModels(models.filter((v: any) => v.name !== item.name));
    }
    setSelectedItems([]);
  };

  const reorder = (startIndex: number, endIndex: number) => {
    const result = Array.from(models);
    const [removed] = result.splice(startIndex, 1);
    result.splice(endIndex, 0, removed);
    return result;
  };

  const onDragEnd = async (result: DropResult) => {
    const { source, destination } = result;
    if (!destination) {
      return;
    }
    const update = reorder(source.index, destination.index);
    setModels(update);

    const res = await fetch(`/api/models/${params.db}/${params.group}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(update),
    });
    if (!res.ok) {
      const response = await res.text();
      alert(response);
    }
  };

  return (
    <>
      <ScrollRestoration />
      <Helmet>
        <title>Senax Database Models ({group})</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">{group}</Header>}>
        <Container
          header={
            <Header
              variant="h2"
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
                    onClick={() => handleDelete()}
                    disabled={reordering || selectedItems.length == 0}
                  >
                    Delete
                  </Button>
                  <Button variant="primary" onClick={() => navigate(`_create`)}>
                    Create
                  </Button>
                </SpaceBetween>
              }
            >
              Models
            </Header>
          }
        >
          <Box margin={{ left: "l" }}>
            {reordering ? (
              <table style={{ width: "100%" }}>
                <DragDropContext onDragEnd={onDragEnd}>
                  <Droppable droppableId={"dndTableBody"}>
                    {(provided) => (
                      <tbody
                        ref={provided.innerRef}
                        {...provided.droppableProps}
                      >
                        {models.map((model: any, index: number) => (
                          <Draggable
                            draggableId={model.name}
                            index={index}
                            key={model.name}
                          >
                            {(provided, _snapshot) => (
                              <tr
                                key={model.name}
                                ref={provided.innerRef}
                                {...provided.draggableProps}
                                {...provided.dragHandleProps}
                              >
                                <td style={{ padding: "4px" }}>
                                  <Icon name="drag-indicator" />
                                  &nbsp;{model.name}
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
              <Table
                columnDefinitions={[
                  {
                    id: "name",
                    header: "Name",
                    cell: (item: any) => (
                      <Link
                        to={`${item.name}`}
                        style={{
                          textDecoration: "none",
                        }}
                      >
                        {item.name}
                      </Link>
                    ),
                    sortingField: "name",
                    isRowHeader: true,
                  },
                ]}
                items={models}
                sortingDisabled
                onSelectionChange={({ detail }) =>
                  setSelectedItems(detail.selectedItems)
                }
                selectedItems={selectedItems}
                selectionType="multi"
                variant="embedded"
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
            )}
          </Box>
        </Container>
      </ContentLayout>
    </>
  );
}
export default Models;
