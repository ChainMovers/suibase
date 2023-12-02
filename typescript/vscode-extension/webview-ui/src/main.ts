import App from "./App.svelte";
import { StateLoop } from "./lib/states/states_loop";

console.log("Main init");
StateLoop.get_instance();

const app = new App({
  target: document.body,
  props: {},
});

export default app;
