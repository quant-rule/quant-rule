import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");
if (target === null) {
  throw new Error("the app element is missing from index.html");
}

mount(App, { target });
