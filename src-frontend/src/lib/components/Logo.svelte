<script lang="ts">
  let { size = '3.75rem', alt = 'Tina' } = $props();
</script>

<span class="logo" style="--logo-size: {size};" style:width={size} style:height={size}>
  <img src="/tina.png" {alt} />
</span>

<style>
  .logo {
    display: grid;
    place-items: center;
    flex: none;
    aspect-ratio: 1;
    border-radius: calc(0.28 * var(--logo-size));
    background: var(--tina-gradient-logo);
    border: 1px solid var(--tina-border-glow);
    box-shadow: var(--tina-shadow-logo);
    overflow: hidden;
  }

  .logo img {
    width: 88%;
    height: 88%;
    object-fit: contain;
    display: block;
  }
</style>
