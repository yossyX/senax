import { ReactNode } from "react";
import { AppLayout } from "@cloudscape-design/components";
import { Outlet } from "react-router-dom";

interface Props {
  children?: ReactNode;
}

export default function BaseLayout(props: Props) {
  return (
    <AppLayout
      toolsHide={true}
      navigationHide={true}
      content={<>{props.children || <Outlet />}</>}
    />
  );
}
