<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    type = 'button',
    onclick,
    children
  }: {
    type?: 'button' | 'submit' | 'reset';
    onclick?: (event: MouseEvent) => void;
    children: Snippet;
  } = $props();
</script>

<button {type} {onclick}>{@render children()}</button>

<style>
  button {
    margin-top: auto;
    align-self: flex-start;
    padding: 0.6rem 1.1rem;
    border: 1px solid transparent;
    border-radius: var(--tina-radius-pill);
    background: var(--tina-gradient-button);
    color: var(--tina-text-on-pink);
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
    font-weight: 600;
    box-shadow: var(--tina-shadow-button);
    transition: filter 0.15s ease, transform 0.15s ease;
  }

  button:hover {
    filter: brightness(1.05);
  }

  button:active {
    transform: translateY(1px);
  }

  button:focus-visible {
    outline: 2px solid var(--tina-pink);
    outline-offset: 2px;
  }
</style>
