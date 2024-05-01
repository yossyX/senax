import { Box, Link } from "@cloudscape-design/components";

export default function Status404() {
  return (
    <Box margin="xxl" padding="xxl" textAlign="center">
      <Box variant="h1">404 Error</Box>
      <Box variant="h2">Page not found</Box>
      <Box>Please go back to the homepage.</Box>
      <Box>
        <Link href="/">homepage</Link>
      </Box>
    </Box>
  );
}
