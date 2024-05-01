import { Box, Spinner } from "@cloudscape-design/components";

export default function SuspenseLoader() {
  return (
    <Box margin="xxl" padding="xxl" textAlign="center">
      <Spinner size="large" />
    </Box>
  );
}
