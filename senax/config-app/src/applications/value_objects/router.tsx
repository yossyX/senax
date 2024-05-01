/* eslint-disable react-refresh/only-export-components */
import { Suspense, lazy } from "react";
import { RouteObject, redirect } from "react-router";

import SuspenseLoader from "src/components/SuspenseLoader";

const Loader = (Component: any) => (props: any) => (
  <Suspense fallback={<SuspenseLoader />}>
    <Component {...props} />
  </Suspense>
);

const Values = Loader(lazy(() => import("./Values")));
const EditValue = Loader(lazy(() => import("./EditValue")));

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
    id: "index",
    loader: async () => {
      return Promise.all([
        fetch_json(`/api/vo/simple`),
        fetch_json(`/api/vo_schema/simple`),
      ]);
    },
    children: [
      {
        path: "",
        element: <Values />,
      },
      {
        path: "_create",
        element: <EditValue />,
        action: async ({ request }) => {
          const data = await request.json();
          const res = await fetch(`/api/vo/simple`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(data),
          });
          if (res.ok) {
            return redirect(`..`);
          }
          const response = await res.text();
          return response;
        },
      },
      {
        path: ":vo",
        element: <EditValue />,
        action: async ({ params, request }) => {
          const data = await request.json();
          const res = await fetch(`/api/vo/simple/${params.vo}`, {
            method: "PUT",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(data),
          });
          if (res.ok) {
            return redirect(`..`);
          }
          const response = await res.text();
          return response;
        },
      },
    ],
  },
];

export default routes;
