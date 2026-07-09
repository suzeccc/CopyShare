import { createRouter, createWebHashHistory } from "vue-router";

import About from "@/pages/About.vue";
import Clipboard from "@/pages/Clipboard.vue";
import Devices from "@/pages/Devices.vue";
import Home from "@/pages/Home.vue";
import Logs from "@/pages/Logs.vue";
import MobileQr from "@/pages/MobileQr.vue";
import Settings from "@/pages/Settings.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: Home },
    { path: "/clipboard", name: "clipboard", component: Clipboard },
    { path: "/devices", name: "devices", component: Devices },
    { path: "/mobile", name: "mobile", component: MobileQr },
    { path: "/logs", name: "logs", component: Logs },
    { path: "/history", redirect: "/logs" },
    { path: "/settings", name: "settings", component: Settings },
    { path: "/about", name: "about", component: About },
  ],
});

export default router;
