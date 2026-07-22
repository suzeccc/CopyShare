import { createRouter, createWebHashHistory } from "vue-router";

import Home from "@/pages/Home.vue";

const About = () => import("@/pages/About.vue");
const Clipboard = () => import("@/pages/Clipboard.vue");
const Devices = () => import("@/pages/Devices.vue");
const FloatingClipboardHistory = () => import("@/pages/FloatingClipboardHistory.vue");
const Library = () => import("@/pages/Library.vue");
const Logs = () => import("@/pages/Logs.vue");
const MediaPreview = () => import("@/pages/MediaPreview.vue");
const MobileQr = () => import("@/pages/MobileQr.vue");
const Ocr = () => import("@/pages/Ocr.vue");
const Settings = () => import("@/pages/Settings.vue");
const Translate = () => import("@/pages/Translate.vue");

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: Home },
    { path: "/clipboard", name: "clipboard", component: Clipboard },
    { path: "/library", name: "library", component: Library },
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
