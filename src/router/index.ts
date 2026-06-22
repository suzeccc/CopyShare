import { createRouter, createWebHashHistory } from "vue-router";

import Devices from "@/pages/Devices.vue";
import Home from "@/pages/Home.vue";
import Logs from "@/pages/Logs.vue";
import Settings from "@/pages/Settings.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: Home },
    { path: "/devices", name: "devices", component: Devices },
    { path: "/logs", name: "logs", component: Logs },
    { path: "/history", redirect: "/logs" },
    { path: "/settings", name: "settings", component: Settings },
  ],
});

export default router;
