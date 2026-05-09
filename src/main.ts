import { createApp } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App.vue";
import NotificationToast from "./components/NotificationToast.vue";
import "./styles.css";

const component = getCurrentWindow().label === "message-toast" ? NotificationToast : App;

createApp(component).mount("#app");
