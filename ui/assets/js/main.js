/* Twemoji */
function applyTwemoji(el) {
  twemoji.parse(el, { folder: "svg", ext: ".svg" });
}
htmx.onLoad((el) => htmx.findAll(el, "[twemoji]").forEach(applyTwemoji));

/* Preserve open state in details across same session */
function rememberOpen(el) {
  if (el.id.length === 0) {
    console.error("missing 'id' for details element", el);
    return;
  }
  const key = "details-" + el.id;
  el.open = window.sessionStorage.getItem(key) === "true";
  el.addEventListener("toggle", () => {
    window.sessionStorage.setItem(key, el.open);
  });
}
htmx.onLoad((el) =>
  htmx.findAll(el, "details[remember-open]").forEach(rememberOpen),
);

/* Language selector */
// has hx-preserve so it's not lost
htmx.find("#lang").addEventListener("change", (ev) => {
  document.cookie = "language=" + ev.target.value + "; Path=/;max-age=31536000";
  window.location.reload();
});

/* Non critical theme toggle behaviour */
function useViewTransition() {
  return (
    document.startViewTransition &&
    !window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function currentTheme() {
  return document.documentElement.classList.contains("dark") ? "dark" : "light";
}

function blockThemeButtons() {
  document.querySelectorAll("[theme-btn]").forEach((el) => {
    el.disabled = true;
    setTimeout(() => (el.disabled = false), 1000);
  });
}

function themeTransition(newTheme, ev) {
  if (newTheme === currentTheme()) {
    return;
  }
  if (!useViewTransition()) {
    applyTheme(newTheme);
    return;
  }

  blockThemeButtons();
  if (ev) {
    manualThemeTransition(newTheme, ev);
  } else {
    automaticThemeTransition(newTheme);
  }
  document.dispatchEvent(new Event("theme-changed"));
}

function manualThemeTransition(newTheme, ev) {
  const x = ev.x;
  const y = ev.y;

  // Get the distance to the furthest corner
  const endRadius = Math.hypot(
    Math.max(x, innerWidth - x),
    Math.max(y, innerHeight - y),
  );

  document.documentElement.classList.add("fancy-transition");
  setTimeout(() => {
    document.documentElement.classList.remove("fancy-transition");
  }, 1000);
  const transition = document.startViewTransition(() => {
    applyTheme(newTheme);
  });

  // Wait for the pseudo-elements to be created:
  transition.ready.then(() => {
    const clipPathNormal = [
      `circle(0px at ${x}px ${y}px)`,
      `circle(${endRadius}px at ${x}px ${y}px)`,
    ];
    const isDark = newTheme === "dark";

    // Animate the root’s new view
    document.documentElement.animate(
      {
        clipPath: isDark ? [...clipPathNormal].reverse() : clipPathNormal,
      },
      {
        duration: 400,
        easing: isDark ? "ease-out" : "ease-in",
        // Specify which pseudo-element to animate
        pseudoElement: isDark
          ? "::view-transition-old(root)"
          : "::view-transition-new(root)",
      },
    );
  });
}

function automaticThemeTransition(newTheme) {
  document.startViewTransition(() => applyTheme(newTheme));
}

// Set button on click
// has hx-preserve so it's not lost
htmx.find("#theme-toggle-button").addEventListener("click", (ev) => {
  const newTheme = currentTheme() === "dark" ? "light" : "dark";
  localStorage.setItem(themeStorageKey, newTheme);
  themeTransition(newTheme, ev);
});

// Auto update with system changes
window.matchMedia(darkMediaQuery).addEventListener("change", ({ matches }) => {
  const newTheme = matches ? "dark" : "light";
  // only save if already saved
  if (localStorage.getItem(themeStorageKey)) {
    localStorage.setItem(themeStorageKey, newTheme);
  }
  themeTransition(newTheme, null);
});

// Toasts
function registerToast(el) {
  let timeout;
  const remove = () => {
    el.classList.add("remove");
    clearTimeout(timeout);
    setTimeout(() => el.remove(), 1000);
  };
  const startTimeout = () => {
    timeout = setTimeout(remove, 5000);
  };
  const stopTimeout = () => {
    clearTimeout(timeout);
  };

  el.querySelector("button").addEventListener("click", remove);
  el.addEventListener("mouseenter", stopTimeout);
  el.addEventListener("mouseleave", startTimeout);
  startTimeout();
}

document.querySelectorAll("[data-toast]").forEach(registerToast);
document.body.addEventListener("htmx:oobAfterSwap", (ev) => {
  ev.detail.target.querySelectorAll("[data-toast]").forEach(registerToast);
});

// Tooltips

let _lastTTId = 1;
function getTooltipId() {
  return "tooltip" + _lastTTId++;
}

/**
 *
 * @param {HTMLElement} el
 */
function registerTooltip(el) {
  const { computePosition, flip, shift, offset, arrow } = window.FloatingUIDOM;

  function showTooltip() {
    if (el.disabled || el.getAttribute("aria-describedby") != null) return;

    let text;
    if (el.classList.contains("tooltip-alt")) {
      text = el.getAttribute("data-tooltip-alt");
    } else {
      text = el.getAttribute("data-tooltip");
    }

    const id = getTooltipId();

    const ttEl = document.createElement("div");
    ttEl.textContent = text;
    ttEl.classList.add("tooltip");
    ttEl.id = id;
    ttEl.role = "tooltip";
    const arrowEl = document.createElement("div");
    arrowEl.classList.add("floating-arrow");
    ttEl.appendChild(arrowEl);

    el.setAttribute("aria-describedby", id);

    computePosition(el, ttEl, {
      placement: "bottom",
      middleware: [
        offset(6),
        flip(),
        shift({ padding: 5 }),
        arrow({ element: arrowEl }),
      ],
    }).then(({ x, y, placement, middlewareData }) => {
      // Set tooltip position
      Object.assign(ttEl.style, {
        left: `${x}px`,
        top: `${y}px`,
      });

      // Set arrow position
      const { x: arrowX, y: arrowY } = middlewareData.arrow;
      const staticSide = {
        top: "bottom",
        right: "left",
        bottom: "top",
        left: "right",
      }[placement.split("-")[0]];
      Object.assign(arrowEl.style, {
        left: arrowX != null ? `${arrowX}px` : "",
        top: arrowY != null ? `${arrowY}px` : "",
        right: "",
        bottom: "",
        [staticSide]: "-4px",
      });
    });
    document.body.appendChild(ttEl);
  }

  function hideTooltip() {
    const id = el.getAttribute("aria-describedby");
    if (!id) return;

    const ttEl = document.getElementById(id);
    if (!ttEl || ttEl.classList.contains("remove")) return;

    if (ttEl) {
      setTimeout(() => ttEl.remove(), 500);
      el.removeAttribute("aria-describedby");
      ttEl.classList.add("remove");
    }
  }

  [
    ["mouseenter", showTooltip],
    ["mouseleave", hideTooltip],
    ["focus", showTooltip],
    ["blur", hideTooltip],
  ].forEach(([event, listener]) => {
    el.addEventListener(event, listener);
  });
}

htmx.onLoad((el) =>
  htmx.findAll(el, "[data-tooltip]").forEach(registerTooltip),
);

/* Localized formatting */
function formatTime(minutes) {
  let hours = Math.trunc(minutes / 60);
  minutes %= 60;
  const days = Math.trunc(hours / 24);
  hours %= 24;

  const parts = [];
  if (days > 0) {
    parts.push(
      new Intl.NumberFormat(currentLocale, {
        style: "unit",
        unit: "day",
      }).format(days),
    );
  }
  if (hours > 0) {
    parts.push(
      new Intl.NumberFormat(currentLocale, {
        style: "unit",
        unit: "hour",
      }).format(hours),
    );
  }
  if (minutes > 0) {
    parts.push(
      new Intl.NumberFormat(currentLocale, {
        style: "unit",
        unit: "minute",
      }).format(minutes),
    );
  }
  return parts.join(" ");
}

function formatTimestamp(secsFromEpoch) {
  const date = new Date(0);
  date.setUTCSeconds(secsFromEpoch);
  return new Intl.DateTimeFormat(currentLocale, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

function formatNumber(num) {
  return new Intl.NumberFormat(currentLocale, {
    maximumFractionDigits: 3,
  }).format(num);
}

function formatAllElements(rootElement) {
  htmx.findAll(rootElement, "[format-minutes]").forEach((el) => {
    const num = Number(el.textContent);
    if (Number.isFinite(num)) {
      const formatted = formatTime(num);
      el.textContent = formatted;
    }
  });

  htmx.findAll(rootElement, "[format-timestamp]").forEach((el) => {
    const secs = Number(el.textContent);
    if (Number.isFinite(secs)) {
      const formatted = formatTimestamp(secs);
      el.textContent = formatted;
    }
  });

  htmx.findAll(rootElement, "[format-number]").forEach((el) => {
    const num = Number(el.textContent);
    if (Number.isFinite(num)) {
      const formatted = formatNumber(num);
      el.textContent = formatted;
    }
  });
}

// Format on initial page load
document.addEventListener("DOMContentLoaded", function () {
  formatAllElements(document.body);
});

// Format before we swap new element in
document.addEventListener("htmx:beforeSwap", function (event) {
  let content = event.detail.xhr.responseText;

  // Create a temporary container to manipulate the content
  let tempDiv = document.createElement("div");
  tempDiv.innerHTML = content;

  formatAllElements(tempDiv);

  event.detail.serverResponse = tempDiv.innerHTML;
});

/* Autoasign an id */
let _lastAutoId = 0;
htmx.onLoad((el) => {
  htmx
    .findAll(el, "[data-auto-id]")
    .forEach((el) => (el.id = "id" + _lastAutoId++));
});

/* Popover */

/**
 *
 * @param {HTMLElement} el
 * @returns
 */
function registerPopover(el) {
  const { computePosition, flip, shift, offset, autoUpdate } =
    window.FloatingUIDOM;

  const trigger = document.getElementById(el.getAttribute("data-popover"));
  if (!trigger) {
    console.error("Popover trigger not found", trigger);
    return;
  }

  function updatePosition() {
    computePosition(trigger, el, {
      placement: "bottom",
      middleware: [offset(6), flip(), shift({ padding: 5, crossAxis: true })],
    }).then(({ x, y }) => {
      Object.assign(el.style, {
        left: `${x}px`,
        top: `${y}px`,
      });

      el.hidden = false;
    });
  }

  const cleanup = autoUpdate(trigger, el, updatePosition);

  trigger.classList.add("popoveractive");

  const handleClick = (ev) => {
    const clicked = ev.target;
    if (!el.contains(clicked)) {
      el.dispatchEvent(new Event("closepopover"));
    }
  };
  document.addEventListener("click", handleClick, true);

  el.addEventListener("closepopover", () => {
    trigger.classList.remove("popoveractive");
    document.removeEventListener("click", handleClick, true);
    el.classList.add("remove");
    cleanup();
    setTimeout(() => el.remove(), 200);
  });
}

function isPopoverClosed() {
  return document.querySelector("[data-popover]") == null;
}

htmx.onLoad((el) => {
  if (el.hasAttribute("data-popover")) {
    registerPopover(el);
  }
});

document.body.addEventListener("keypress", (ev) => {
  if (ev.key.toLowerCase() === "s") {
    const search = document.getElementById("search");
    if (!search || document.activeElement === search) return;
    ev.preventDefault();
    search.focus();
  }
});
