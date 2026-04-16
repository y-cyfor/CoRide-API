<script setup lang="ts">
defineOptions({
  name: 'GlobalFooter'
});
</script>

<template>
  <DarkModeContainer class="h-full flex-center">
    <span>Copyright &copy; {{ new Date().getFullYear() }} CoRide-API. All Rights Reserved.</span>
  </DarkModeContainer>
</template>

<style scoped></style>
