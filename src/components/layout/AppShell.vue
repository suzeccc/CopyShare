<script setup lang="ts">
import Sidebar from "@/components/layout/Sidebar.vue";
import TitleBar from "@/components/layout/TitleBar.vue";
</script>

<template>
  <div class="flex h-screen overflow-hidden text-slate-100">
    <Sidebar />
    <main class="flex min-w-0 flex-1 flex-col">
      <TitleBar />
      <div class="min-h-0 flex-1 overflow-auto px-6 py-6">
        <RouterView />
      </div>
    </main>
  </div>
</template>
