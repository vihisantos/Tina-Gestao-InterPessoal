<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    type = 'button',
    variant = 'primary',
    disabled = false,
    onclick,
    children
  }: {
    type?: 'button' | 'submit' | 'reset';
    variant?: 'primary' | 'secondary';
    disabled?: boolean;
    onclick?: (event: MouseEvent) => void;
    children: Snippet;
  } = $props();
</script>

<button {type} {disabled} class:secondary={variant === 'secondary'} {onclick}>{@render children()}</button>

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

  button.secondary {
    background: var(--tina-surface);
    border-color: var(--tina-border-strong);
    color: var(--tina-text);
    box-shadow: none;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
    transform: none;
  }
</style>
