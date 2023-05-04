/* Code/Component is from solanacookbook.com and has been renamed and *
significantly modified. * See
https://github.com/solana-developers/solana-cookbook/pull/36 for * the original
(and thanks). */

<template>
  <div
    class="code-tab"
    :class="{ 'code-tab__active': active }"
    :aria-selected="active"
  >
    <div :class="{ hidden: preview }">
      <slot name="default" />
    </div>
    <div :class="{ hidden: !preview }">
      <slot name="preview" />
    </div>
  </div>
</template>

<script>
export default {
  name: "SolanaCodeGroupItem",
  props: {
    title: {
      type: String,
      required: true,
    },
    active: {
      type: Boolean,
      required: false,
      default: false,
    },
    preview: {
      type: Boolean,
      required: false,
      default: false,
    },
  },
};
</script>

<style scoped>
.hidden {
  display: none;
}
.block {
  display: block;
}
</style>
