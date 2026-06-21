import { createRouter, createWebHashHistory } from "vue-router";

import Devices from "@/pages/Devices.vue";
import History from "@/pages/History.vue";
import Home from "@/pages/Home.vue";
import Settings from "@/pages/Settings.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: Home },
    { path: "/devices", name: "devices", component: Devices },
    { path: "/history", name: "history", component: History },
    { path: "/settings", name: "settings", component: Settings },
  ],
});

export default router;
