/* eslint-disable react-refresh/only-export-components */
import { Suspense, lazy } from "react";
import { RouteObject, redirect } from "react-router";

import SuspenseLoader from "src/components/SuspenseLoader";

const Loader = (Component: any) => (props: any) => (
  <Suspense fallback={<SuspenseLoader />}>
    <Component {...props} />
  </Suspense>
);

const Groups = Loader(lazy(() => import("./Groups")));
const Config = Loader(lazy(() => import("./Config")));
const Models = Loader(lazy(() => import("./Models")));
const EditModel = Loader(lazy(() => import("./EditModel")));

async function fetch_json(url: string) {
  const res = await fetch(url);
  if (res.status === 200) {
    return res.json();
  }
  throw new Response("Not Found", { status: res.status });
}

const routes: RouteObject[] = [
  {
    path: ":db",
    id: "db",
    loader: async ({ params }) => {
      return Promise.all([
        fetch_json(`/api/db/${params.db}`),
        fetch_json(`/api/vo/simple`),
      ]);
    },
    children: [
      {
        path: "",
        element: <Groups />,
      },
      {
        path: "_config",
        element: <Config />,
        loader: async () => {
          return fetch_json(`/api/config_schema`);
        },
        action: async ({ params, request }) => {
          const data = await request.json();
          const res = await fetch(`/api/db/${params.db}`, {
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
        path: ":group",
        id: "group",
        loader: async ({ params }) => {
          return Promise.all([
            fetch_json(`/api/models/${params.db}/${params.group}`),
            fetch_json(`/api/model_schema`),
            fetch_json(`/api/model_names/${params.db}`),
          ]);
        },
        children: [
          {
            path: "",
            element: <Models />,
          },
          {
            path: "_create",
            element: <EditModel />,
            action: async ({ params, request }) => {
              const data = await request.json();
              const res = await fetch(
                `/api/models/${params.db}/${params.group}`,
                {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify(data),
                },
              );
              if (res.ok) {
                return redirect(`..`);
              }
              const response = await res.text();
              return response;
            },
          },
          {
            path: ":model",
            element: <EditModel />,
            action: async ({ params, request }) => {
              const data = await request.json();
              const res = await fetch(
                `/api/models/${params.db}/${params.group}/${params.model}`,
                {
                  method: "PUT",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify(data),
                },
              );
              if (res.ok) {
                return redirect(`..`);
              }
              const response = await res.text();
              return response;
            },
          },
        ],
      },
    ],
  },
];

export default routes;
