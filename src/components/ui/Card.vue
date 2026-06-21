<script setup lang="ts">
withDefaults(
  defineProps<{
    compact?: boolean;
  }>(),
  {
    compact: false,
  },
);
</script>

<template>
  <section
    class="glass-panel rounded-lg"
    :class="compact ? 'p-4' : 'p-5'"
  >
    <slot />
  </section>
</template>
