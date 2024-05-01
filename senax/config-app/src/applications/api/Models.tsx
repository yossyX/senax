import * as React from "react";
import { useNavigate, useParams, useRouteLoaderData } from "react-router-dom";
import { Helmet } from "react-helmet-async";
import { Link } from "react-router-dom";
import Container from "@cloudscape-design/components/container";
import { ContentLayout, Header } from "@cloudscape-design/components";
import Table from "@cloudscape-design/components/table";
import Box from "@cloudscape-design/components/box";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Button from "@cloudscape-design/components/button";

function Models() {
  const navigate = useNavigate();
  const params = useParams();
  const group = params.group;
  const [ini_models] = useRouteLoaderData("api_models") as any;
  const [models, setModels] = React.useState(ini_models);
  const [selectedItems, setSelectedItems] = React.useState([] as any);
  const handleClean = async () => {
    const msg =
      "Are you sure you want to delete the definitions you no longer need and re-sort them?";
    if (!confirm(msg)) {
      return;
    }
    const res = await fetch(
      `/api/clean_api_server/${params.server}/${params.db}/${params.group}`,
    );
    if (!res.ok) {
      const response = await res.text();
      alert(response);
      return;
    }
    setModels(await res.json());
  };
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
        `/api/api_server/${params.server}/${params.db}/${params.group}/${item.name}`,
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

  return (
    <>
      <Helmet>
        <title>Senax Database Api Server ({group})</title>
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
                  <Button onClick={() => handleClean()}>Clean</Button>
                  <Button
                    onClick={() => handleDelete()}
                    disabled={selectedItems.length == 0}
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
          <Table
            columnDefinitions={[
              {
                id: "name",
                header: "name",
                cell: (item: any) => <Link to={item.name}>{item.name}</Link>,
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
        </Container>
      </ContentLayout>
    </>
  );
}
export default Models;
