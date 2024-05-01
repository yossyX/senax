import {
  ScrollRestoration,
  useLoaderData,
  useNavigate,
} from "react-router-dom";
import { Helmet } from "react-helmet-async";
import SpaceBetween from "@cloudscape-design/components/space-between";
import Button from "@cloudscape-design/components/button";
import Container from "@cloudscape-design/components/container";
import { ContentLayout, Header } from "@cloudscape-design/components";
import Box from "@cloudscape-design/components/box";

function Index() {
  const [result] = useLoaderData() as any;
  const navigate = useNavigate();

  const submitBuild = async () => {
    if (confirm("Do you want to run the build?")) {
      const res = await fetch(`/api/build/exec`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(true),
      });
      if (!res.ok) {
        const response = await res.text();
        alert(response);
      } else {
        if (confirm("Reload")) {
          navigate(".", { replace: true });
        }
      }
    }
  };

  return (
    <>
      <ScrollRestoration />
      <Helmet>
        <title>Senax Build</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">Project Build</Header>}>
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
                  <Button variant="primary" onClick={submitBuild}>
                    Build
                  </Button>
                </SpaceBetween>
              }
            ></Header>
          }
        >
          <Box margin={{ left: "l" }}>
            <h2>Build Result</h2>
            <pre>{result.result}</pre>
          </Box>
        </Container>
      </ContentLayout>
    </>
  );
}
export default Index;
