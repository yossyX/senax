import { Box, Link } from "@cloudscape-design/components";

export default function Status500() {
  return (
    <Box margin="xxl" padding="xxl" textAlign="center">
      <Box variant="h1">500 Error</Box>
      <Box variant="h2">There was an error, please try again later</Box>
      <Box>
        The server encountered an internal error and was not able to complete
        your request
      </Box>
      <Box>
        <Link href="/">homepage</Link>
      </Box>
    </Box>
  );
}
