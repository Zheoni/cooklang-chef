const themeStorageKey = "theme";
const darkMediaQuery = "(prefers-color-scheme: dark)";
function getSelectedTheme() {
  return (
    localStorage.getItem(themeStorageKey) ||
    (window.matchMedia(darkMediaQuery).matches ? "dark" : "light")
  );
}
function applyTheme(theme) {
  const root = document.documentElement.classList;
  if (theme === "dark") {
    root.add("dark");
  } else {
    root.remove("dark");
  }
}
applyTheme(getSelectedTheme());
