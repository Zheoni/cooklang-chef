(function () {
  const light = htmx.find("#btn-light");
  const dark = htmx.find("#btn-dark");
  const system = htmx.find("#btn-system");
  function updateBorders() {
    light.classList.remove("border-2");
    dark.classList.remove("border-2");
    system.classList.remove("border-2");
    const tm = localStorage.getItem(themeStorageKey);
    if (tm === "light") {
      light.classList.add("border-2");
    } else if (tm === "dark") {
      dark.classList.add("border-2");
    } else {
      system.classList.add("border-2");
    }
  }
  light.addEventListener("click", (ev) => {
    const newTheme = "light";
    localStorage.setItem(themeStorageKey, "light");
    themeTransition(newTheme, ev);
    blockThemeButtons();
    updateBorders();
  });
  dark.addEventListener("click", (ev) => {
    const newTheme = "dark";
    localStorage.setItem(themeStorageKey, "dark");
    themeTransition(newTheme, ev);
    blockThemeButtons();
    updateBorders();
  });
  system.addEventListener("click", (ev) => {
    localStorage.removeItem(themeStorageKey);
    themeTransition(getSelectedTheme(), ev);
    blockThemeButtons();
    updateBorders();
  });
  document.addEventListener("theme-changed", updateBorders);
  updateBorders();
})();

(function () {
  const button = htmx.find("#reset-lang");
  if (!button) return;

  const isSet = document.cookie.includes("language=");
  if (isSet) {
    button.classList.remove("border-2");
  } else {
    button.classList.add("border-2");
  }

  button.addEventListener("click", () => {
    document.cookie =
      "language=; Path=/; Expires=Thu, 01 Jan 1970 00:00:01 GMT;";
    window.location.reload();
  });
})();
