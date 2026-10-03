import { mount } from "svelte";
import App from "./App.svelte";
import { snapshot } from "./lib/api.ts";
import "./app.css";

const target = document.getElementById("app");
if (!target) throw new Error("no #app element");
// The app mounts on its first snapshot, so it never shows or acts on made-up data.
export default mount(App, { target, props: { snapshot: await snapshot() } });
