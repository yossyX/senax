import { RouterProvider, createBrowserRouter } from "react-router-dom";
import router from "./router";
import SuspenseLoader from "./components/SuspenseLoader";
// import { useState } from 'react'
import "./App.css";

function App() {
  const content = createBrowserRouter(router);
  return (
    <RouterProvider router={content} fallbackElement={<SuspenseLoader />} />
  );
}

export default App;
