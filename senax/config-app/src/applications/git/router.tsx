/* eslint-disable react-refresh/only-export-components */
import { Suspense, lazy } from "react";
import { RouteObject } from "react-router";

import SuspenseLoader from "src/components/SuspenseLoader";

const Loader = (Component: any) => (props: any) => (
  <Suspense fallback={<SuspenseLoader />}>
    <Component {...props} />
  </Suspense>
);

const Index = Loader(lazy(() => import("./index")));

async function fetch_json(url: string) {
  const res = await fetch(url);
  if (res.status === 200) {
    return res.json();
  }
  throw new Response("Not Found", { status: res.status });
}

const routes: RouteObject[] = [
  {
    path: "",
    element: <Index />,
    loader: async () => {
      return Promise.all([fetch_json(`/api/git/result`)]);
    },
  },
];

export default routes;
