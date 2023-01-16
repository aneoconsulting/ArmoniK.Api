const count = ref(0);

export const useMaskId = function () {
  count.value++;
  return `mask-${count.value}`;
};
