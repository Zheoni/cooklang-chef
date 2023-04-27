export function clickOutside(node: Node, omit: Array<Node> = []) {
  const handleClick = (event: MouseEvent) => {
    const clicked = event.target as Node;
    if (!node.contains(clicked)) {
      for (const omitted of omit) {
        if (omitted.contains(clicked)) return;
      }
      node.dispatchEvent(new CustomEvent("outclick"));
    }
  };

  document.addEventListener("click", handleClick, true);

  return {
    destroy() {
      document.removeEventListener("click", handleClick, true);
    },
  };
}
