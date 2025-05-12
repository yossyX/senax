import { useNavigate, useParams, useRouteLoaderData } from "react-router-dom";
import { Helmet } from "react-helmet-async";
import { Link } from "react-router-dom";
import Container from "@cloudscape-design/components/container";
import { ContentLayout, Header } from "@cloudscape-design/components";
import Table from "@cloudscape-design/components/table";
import Box from "@cloudscape-design/components/box";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Button from "@cloudscape-design/components/button";

function Db() {
  const navigate = useNavigate();
  const params = useParams();
  const server = params.server;
  const db_list = useRouteLoaderData("db_list") as any;
  return (
    <>
      <Helmet>
        <title>Senax API Server ({server})</title>
      </Helmet>
      <ContentLayout
        header={
          <Header
            variant="h1"
            actions={
              <Button variant="primary" onClick={() => navigate(`_config`)}>
                Config
              </Button>
            }
          >
            {server}
          </Header>
        }
      >
        <Container>
          <Table
            columnDefinitions={[
              {
                id: "name",
                header: "name",
                cell: (item: any) => <Link to={item}>{item}</Link>,
                isRowHeader: true,
              },
            ]}
            items={db_list}
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
            header={<Header>DB</Header>}
          />
        </Container>
      </ContentLayout>
    </>
  );
}
export default Db;
