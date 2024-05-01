import { useNavigate, useParams, useRouteLoaderData } from "react-router-dom";
import { Helmet } from "react-helmet-async";
import { Link } from "react-router-dom";
import Container from "@cloudscape-design/components/container";
import { ContentLayout, Header } from "@cloudscape-design/components";
import Table from "@cloudscape-design/components/table";
import Box from "@cloudscape-design/components/box";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Button from "@cloudscape-design/components/button";

function Groups() {
  const navigate = useNavigate();
  const params = useParams();
  const db = params.db;
  const [db_data, _vo_list] = useRouteLoaderData("db") as any;
  return (
    <>
      <Helmet>
        <title>Senax Database Groups ({db})</title>
      </Helmet>
      <ContentLayout
        header={
          <Header
            variant="h1"
            actions={
              <Button variant="primary" onClick={() => navigate(`_config`)}>
                Database Config
              </Button>
            }
          >
            {db}
          </Header>
        }
      >
        <Container>
          <Table
            columnDefinitions={[
              {
                id: "name",
                header: "name",
                cell: (item: any) => (
                  <Link to={item.name}>{item.name || "-"}</Link>
                ),
                isRowHeader: true,
              },
              {
                id: "label",
                header: "label",
                cell: (item: any) => item.label || "-",
              },
            ]}
            items={db_data.groups!}
            sortingDisabled
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
            header={<Header>Groups</Header>}
          />
        </Container>
      </ContentLayout>
    </>
  );
}
export default Groups;
