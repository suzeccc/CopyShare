<script setup lang="ts">
import { Pause, Play } from "lucide-vue-next";

import Button from "@/components/ui/Button.vue";

defineProps<{
  running: boolean;
  loading?: boolean;
}>();

defineEmits<{
  start: [];
  stop: [];
}>();
</script>

<template>
  <div class="flex flex-wrap items-center gap-3">
    <Button v-if="!running" variant="primary" :disabled="loading" @click="$emit('start')">
      <Play class="h-4 w-4" />
      开始同步
    </Button>
    <Button v-else variant="danger" :disabled="loading" @click="$emit('stop')">
      <Pause class="h-4 w-4" />
      停止同步
    </Button>
  </div>
</template>
