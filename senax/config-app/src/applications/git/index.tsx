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

  const submitPush = async () => {
    const msg = prompt("Input commit message.");
    if (msg) {
      const res = await fetch(`/api/git/exec/push`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ msg }),
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
  const submitPull = async () => {
    if (confirm("Do you want to run git pull?")) {
      const res = await fetch(`/api/git/exec/pull`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
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
  const submitDiff = async () => {
    const res = await fetch(`/api/git/exec/diff`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    });
    if (!res.ok) {
      const response = await res.text();
      alert(response);
    } else {
      if (confirm("Reload")) {
        navigate(".", { replace: true });
      }
    }
  };

  return (
    <>
      <ScrollRestoration />
      <Helmet>
        <title>Senax Git</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">Git</Header>}>
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
                  <Button variant="primary" onClick={submitPush}>
                    Push
                  </Button>
                  <Button onClick={submitPull}>Pull</Button>
                  <Button onClick={submitDiff}>Diff</Button>
                </SpaceBetween>
              }
            ></Header>
          }
        >
          <Box margin={{ left: "l" }}>
            <h2>Git Result</h2>
            <pre>{result.result}</pre>
          </Box>
        </Container>
      </ContentLayout>
    </>
  );
}
export default Index;
