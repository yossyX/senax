/* eslint-disable react-refresh/only-export-components */
import { Suspense, lazy } from "react";
import { RouteObject } from "react-router";

import SidebarLayout from "./layouts/SidebarLayout";
import BaseLayout from "./layouts/BaseLayout";

import SuspenseLoader from "./components/SuspenseLoader";
import Index from "./applications/index";
import databaseRouter from "./applications/database/router";
import voRouter from "./applications/value_objects/router";
import apiRouter from "./applications/api/router";
import buildRouter from "./applications/build/router";
import gitRouter from "./applications/git/router";

const Loader = (Component: any) => (props: any) => (
  <Suspense fallback={<SuspenseLoader />}>
    <Component {...props} />
  </Suspense>
);

const Status404 = Loader(lazy(() => import("./pages/Status404")));
const Status500 = Loader(lazy(() => import("./pages/Status500")));

const routes: RouteObject[] = [
  {
    path: "/",
    element: <SidebarLayout />,
    errorElement: <Status500 />,
    children: [
      {
        path: "",
        element: <Index />,
      },
    ],
  },
  {
    path: "",
    element: <BaseLayout />,
    children: [
      {
        path: "*",
        element: <Status404 />,
      },
    ],
  },
  {
    path: "db",
    element: <SidebarLayout />,
    errorElement: <Status500 />,
    children: databaseRouter,
    handle: {
      crumb: () => ({ text: 'DB', href: "#" })
    },
  },
  {
    path: "vo",
    element: <SidebarLayout />,
    errorElement: <Status500 />,
    children: [{
      path: "simple",
      children: voRouter,
    }],
    handle: {
      crumb: () => ({ text: 'Value Objects', href: "#" })
    },
  },
  {
    path: "api_server",
    element: <SidebarLayout />,
    errorElement: <Status500 />,
    children: apiRouter,
    handle: {
      crumb: () => ({ text: 'API', href: "#" })
    },
  },
  {
    path: "build",
    element: <SidebarLayout />,
    errorElement: <Status500 />,
    children: buildRouter,
  },
  {
    path: "git",
    element: <SidebarLayout />,
    errorElement: <Status500 />,
    children: gitRouter,
  },
];

export default routes;
