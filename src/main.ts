import { createApp } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App.vue";
import NotificationToast from "./components/NotificationToast.vue";
import "./styles.css";

if (import.meta.env.PROD) {
  document.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });
}

const component = getCurrentWindow().label === "message-toast" ? NotificationToast : App;

createApp(component).mount("#app");
