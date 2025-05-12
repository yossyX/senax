import { ReactNode, useEffect, useState } from "react";
import { AppLayout } from "@cloudscape-design/components";
import SideNavigation from "@cloudscape-design/components/side-navigation";
import BreadcrumbGroup from "@cloudscape-design/components/breadcrumb-group";
import { Outlet, useMatches, useNavigate } from "react-router-dom";

interface Props {
  children?: ReactNode;
}

export default function SidebarLayout(props: Props) {
  const [activeHref, setActiveHref] = useState("");
  const [db, setDb] = useState([]);
  const [api, setApi] = useState([]);
  const navigate = useNavigate();
  useEffect(() => {
    fetch("/api/db")
      .then((res) => res.json())
      .then((json) => setDb(json))
      .catch(() => alert("error"));
    fetch("/api/api_server")
      .then((res) => res.json())
      .then((json) => setApi(json))
      .catch(() => alert("error"));
  }, []);

  return (
    <AppLayout
      toolsHide={true}
      navigationHide={false}
      breadcrumbs={<Breadcrumbs />}
      navigation={
        <SideNavigation
          activeHref={activeHref}
          header={{ href: "/", text: "Senax" }}
          onFollow={(event) => {
            if (!event.detail.external) {
              setActiveHref(event.detail.href);
              navigate(event.detail.href);
              event.preventDefault();
            }
          }}
          items={[
            {
              type: "section",
              text: "DB",
              items: db.map((db) => ({
                type: "link",
                text: db,
                href: "/db/" + db,
              })),
            },
            {
              type: "section",
              text: "Value Objects",
              items: [{ type: "link", text: "simple", href: "/vo/simple" }],
            },
            {
              type: "section",
              text: "API",
              items: api.map((api) => ({
                type: "link",
                text: api,
                href: "/api_server/" + api,
              })),
            },
            {
              type: "section",
              text: "Build",
              items: [
                { type: "link", text: "build", href: "/build" },
                { type: "link", text: "git", href: "/git" },
              ],
            },
          ]}
        />
      }
      content={<>{props.children || <Outlet />}</>}
    />
  );
}

function Breadcrumbs() {
  const matches = useMatches();
  const navigate = useNavigate();
  const crumbs = matches
    .filter((match: any) => Boolean(match.handle?.crumb))
    .map((match: any) => match.handle.crumb(match));
  return <BreadcrumbGroup
    items={crumbs}
    ariaLabel="Breadcrumbs"
    onClick={(e) => {
      navigate(e.detail.href);
      e.preventDefault();
    }}
  />;
}
