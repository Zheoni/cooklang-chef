/* Component/references highlight on hover */

document.querySelectorAll("[data-component-kind]").forEach((el) => {
  const kind = el.getAttribute("data-component-kind");
  const group = el.getAttribute("data-component-ref-group");
  const target = el.getAttribute("data-component-ref-target");
  const query = `[data-component-kind="${kind}"][data-component-ref-group="${group}"][data-component-ref-target="${target}"]`;
  let highlight_id = null;
  if (target === "step") {
    const currentSection = el
      .closest("[data-section-index]")
      .getAttribute("data-section-index");
    highlight_id = `step-${currentSection}-${group}`;
  } else if (target === "section") {
    highlight_id = `section-${group}`;
  }
  el.addEventListener("mouseenter", () => {
    document
      .querySelectorAll(query)
      .forEach((el) => el.classList.add("highlight"));
    if (highlight_id) {
      document.getElementById(highlight_id).classList.add("highlight");
    }
  });
  el.addEventListener("mouseleave", () => {
    document
      .querySelectorAll(query)
      .forEach((el) => el.classList.remove("highlight"));
    if (highlight_id) {
      document.getElementById(highlight_id).classList.remove("highlight");
    }
  });
});

document.querySelectorAll("a[highlight-target-on-hover]").forEach((el) => {
  const target = el.getAttribute("href");
  if (!target.startsWith("#")) {
    console.error("link href is not id");
    return;
  }
  el.addEventListener("mouseenter", () => {
    document.querySelector(target).classList.add("highlight");
  });
  el.addEventListener("mouseleave", () => {
    document.querySelector(target).classList.remove("highlight");
  });
});

/* Step ingredients layout */
function setLayout(layout) {
  document.cookie = "igr_layout=" + layout + "; Path=/;max-age=31536000";
  document
    .querySelector("[data-igr-layout]")
    .setAttribute("data-igr-layout", layout);
}

["line", "list", "hidden"].forEach((layout) => {
  const el = document.getElementById(`igr-layout-${layout}`);
  if (el) el.addEventListener("click", () => setLayout(layout));
});

/* Convert popover functionality */
function registerConvertPopover(el) {
  const q = document.getElementById(el.getAttribute("data-popover"));
  if (!q) return;

  function extractQuantity(el) {
    const value = el.querySelector("[data-quantity-value]").textContent;
    const unit = el.querySelector("[data-quantity-unit]").textContent;
    return { value, unit };
  }

  function setQuantity(quantity) {
    q.querySelector("[data-quantity-value]").textContent = quantity.value;
    q.querySelector("[data-quantity-unit]").textContent = quantity.unit;
    el.dispatchEvent(new Event("closepopover"));
  }

  // load original or extract and save if it's first
  let original;
  if (q.hasAttribute("data-original-value")) {
    let value = q.getAttribute("data-original-value");
    let unit = q.getAttribute("data-original-unit");
    original = { value, unit };
  } else {
    original = extractQuantity(q);
    q.setAttribute("data-original-value", original.value);
    q.setAttribute("data-original-unit", original.unit);
  }

  el.querySelectorAll("[data-conversion]").forEach((el) =>
    el.addEventListener("click", () => {
      const c = extractQuantity(el);
      setQuantity(c);
    }),
  );

  el.querySelector("#conv-reset").addEventListener("click", () => {
    setQuantity(original);
  });
}

htmx.onLoad((el) => {
  if (el.hasAttribute("data-convert-popover")) {
    registerConvertPopover(el);
  }
});

document.body.addEventListener("keydown", (ev) => {
  if (ev.key == "Control" && !ev.repeat) {
    document
      .querySelectorAll("[data-fract-error]")
      .forEach((el) => (el.hidden = false));
  }
});
document.body.addEventListener("keyup", (ev) => {
  if (ev.key == "Control") {
    document
      .querySelectorAll("[data-fract-error]")
      .forEach((el) => (el.hidden = true));
  }
});

function strikeThrough(el) {
  el.classList.toggle("strike-through");
}
