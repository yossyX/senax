import * as React from "react";
import {
  ScrollRestoration,
  useNavigate,
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

function Values() {
  const navigate = useNavigate();
  const [_voList, _jsonSchema] = useRouteLoaderData("index") as [any[], any];
  const [voList, setVoList] = React.useState(_voList);
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
      const res = await fetch(`/api/vo/simple/${item.name}`, {
        method: "DELETE",
      });
      if (!res.ok) {
        const response = await res.text();
        alert(response);
        return;
      }
      setVoList(voList.filter((v: any) => v.name !== item.name));
    }
    setSelectedItems([]);
  };

  const reorder = (startIndex: number, endIndex: number) => {
    const result = Array.from(voList);
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
    setVoList(update);

    const res = await fetch(`/api/vo/simple`, {
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
        <title>Senax Value Objects</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">Simple Value Objects</Header>}>
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
            ></Header>
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
                        {voList.map((obj: any, index: number) => (
                          <Draggable
                            draggableId={obj.name}
                            index={index}
                            key={obj.name}
                          >
                            {(provided, _snapshot) => (
                              <tr
                                key={obj.name}
                                ref={provided.innerRef}
                                {...provided.draggableProps}
                                {...provided.dragHandleProps}
                              >
                                <td style={{ padding: "4px" }}>
                                  <Icon name="drag-indicator" />
                                  &nbsp;{obj.name}
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
                items={voList}
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
export default Values;
