import Container from "@cloudscape-design/components/container";
import ContentLayout from "@cloudscape-design/components/content-layout";
import Header from "@cloudscape-design/components/header";
import { Helmet } from "react-helmet-async";

function Index() {
  return (
    <>
      <Helmet>
        <title>Senax</title>
      </Helmet>
      <ContentLayout header={<Header variant="h1">Senax Configuration</Header>}>
        Please select a function from the menu.<br />
        DB and API must be registered with the CLI.
      </ContentLayout>
    </>
  );
}
export default Index;
