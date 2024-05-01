/* eslint-disable react-refresh/only-export-components */
import { Suspense, lazy } from "react";
import { RouteObject, redirect } from "react-router";

import SuspenseLoader from "src/components/SuspenseLoader";

const Loader = (Component: any) => (props: any) => (
  <Suspense fallback={<SuspenseLoader />}>
    <Component {...props} />
  </Suspense>
);

const Config = Loader(lazy(() => import("./Config")));
const Db = Loader(lazy(() => import("./Db")));
const DbConfig = Loader(lazy(() => import("./DbConfig")));
const Groups = Loader(lazy(() => import("./Groups")));
const Models = Loader(lazy(() => import("./Models")));
const EditApi = Loader(lazy(() => import("./EditApi")));

async function fetch_json(url: string) {
  const res = await fetch(url);
  if (res.status === 200) {
    return res.json();
  }
  throw new Response("Not Found", { status: res.status });
}

const routes: RouteObject[] = [
  {
    path: ":server",
    id: "db_list",
    loader: async ({ params }) => {
      return fetch_json(`/api/api_server/${params.server}/_db`);
    },
    children: [
      {
        path: "",
        element: <Db />,
      },
      {
        path: "_config",
        element: <Config />,
        loader: async ({ params }) => {
          return Promise.all([
            fetch_json(`/api/api_config_schema`),
            fetch_json(`/api/api_server/${params.server}/_config`),
          ]);
        },
        action: async ({ params, request }) => {
          const data = await request.json();
          const res = await fetch(`/api/api_server/${params.server}/_config`, {
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
        path: ":db",
        id: "api_groups",
        loader: async ({ params }) => {
          return fetch_json(
            `/api/api_server/${params.server}/${params.db}/_config`,
          );
        },
        children: [
          {
            path: "",
            element: <Groups />,
          },
          {
            path: "_config",
            element: <DbConfig />,
            loader: async ({ params }) => {
              return Promise.all([
                fetch_json(`/api/api_db_schema`),
                fetch_json(
                  `/api/api_server/${params.server}/${params.db}/_config`,
                ),
                fetch_json(`/api/api_server/${params.server}/_config`),
                fetch_json(`/api/db/${params.db}`),
              ]);
            },
            action: async ({ params, request }) => {
              const data = await request.json();
              const res = await fetch(
                `/api/api_server/${params.server}/${params.db}/_config`,
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
            path: ":group",
            id: "api_models",
            loader: async ({ params }) => {
              return Promise.all([
                fetch_json(
                  `/api/api_server/${params.server}/${params.db}/${params.group}`,
                ),
                fetch_json(`/api/api_schema`),
                fetch_json(`/api/models/${params.db}/${params.group}`),
                fetch_json(`/api/api_server/${params.server}/_config`),
              ]);
            },
            children: [
              {
                path: "",
                element: <Models />,
              },
              {
                path: "_create",
                element: <EditApi />,
                action: async ({ params, request }) => {
                  const data = await request.json();
                  const res = await fetch(
                    `/api/api_server/${params.server}/${params.db}/${params.group}`,
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
                element: <EditApi />,
                action: async ({ params, request }) => {
                  const data = await request.json();
                  const res = await fetch(
                    `/api/api_server/${params.server}/${params.db}/${params.group}/${params.model}`,
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
    ],
  },
];

export default routes;
