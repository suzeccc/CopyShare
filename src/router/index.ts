import { createRouter, createWebHashHistory } from "vue-router";

import About from "@/pages/About.vue";
import Clipboard from "@/pages/Clipboard.vue";
import Devices from "@/pages/Devices.vue";
import FloatingClipboardHistory from "@/pages/FloatingClipboardHistory.vue";
import Home from "@/pages/Home.vue";
import Logs from "@/pages/Logs.vue";
import MediaPreview from "@/pages/MediaPreview.vue";
import MobileQr from "@/pages/MobileQr.vue";
import Ocr from "@/pages/Ocr.vue";
import Settings from "@/pages/Settings.vue";
import Translate from "@/pages/Translate.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: Home },
    { path: "/clipboard", name: "clipboard", component: Clipboard },
    { path: "/ocr", name: "ocr", component: Ocr },
    { path: "/translate", name: "translate", component: Translate },
    { path: "/devices", name: "devices", component: Devices },
    { path: "/mobile", name: "mobile", component: MobileQr },
    { path: "/logs", name: "logs", component: Logs },
    { path: "/floating-clipboard", name: "floating-clipboard", component: FloatingClipboardHistory },
    { path: "/media-preview", name: "media-preview", component: MediaPreview },
    { path: "/history", redirect: "/logs" },
    { path: "/settings", name: "settings", component: Settings },
    { path: "/about", name: "about", component: About },
  ],
});

export default router;
