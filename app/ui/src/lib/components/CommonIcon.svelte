<script lang="ts">
  let { name, size = 16 }: { name: string; size?: number } = $props();
</script>

<span
  class="icon"
  style:--icon-url="url('/icons/{name}.svg')"
  style:--icon-size="{size}px"
  aria-hidden="true"
></span>

<style>
  .icon {
    display: inline-block;
    width: var(--icon-size);
    height: var(--icon-size);
    flex: 0 0 auto;
    background: currentColor;
    -webkit-mask: var(--icon-url) center / contain no-repeat;
    mask: var(--icon-url) center / contain no-repeat;
  }
</style>
