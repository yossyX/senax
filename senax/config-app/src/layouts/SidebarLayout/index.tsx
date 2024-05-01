import { ReactNode, useEffect, useState } from "react";
import { AppLayout } from "@cloudscape-design/components";
import SideNavigation from "@cloudscape-design/components/side-navigation";
import { Outlet } from "react-router-dom";

interface Props {
  children?: ReactNode;
}

export default function SidebarLayout(props: Props) {
  const [activeHref, setActiveHref] = useState("");
  const [db, setDb] = useState([]);
  const [api, setApi] = useState([]);
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
      navigation={
        <SideNavigation
          activeHref={activeHref}
          header={{ href: "/", text: "Senax" }}
          onFollow={(event) => {
            if (!event.detail.external) {
              setActiveHref(event.detail.href);
            }
          }}
          items={[
            {
              type: "section",
              text: "Models",
              items: db.map((db) => ({
                type: "link",
                text: db,
                href: "/db/" + db,
              })),
            },
            {
              type: "section",
              text: "Value Objects",
              items: [{ type: "link", text: "simple", href: "/simple_vo" }],
            },
            {
              type: "section",
              text: "Api",
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
