/* Hot reload */

function hrSetIndicatorState(connected) {
  const hrIndicator = document.getElementById("hot-reload-indicator");
  const tt = document.getElementById("hot-reload-tooltip").classList;
  const cls = hrIndicator.classList;

  if (!cls.contains("shown")) {
    cls.add("show-animation");
    setTimeout(() => {
      cls.add("shown");
      cls.remove("show-animation");
    }, 1000);
  }

  if (connected === true) {
    cls.add("connected");
    tt.remove("tooltip-alt");
  } else if (connected === false) {
    cls.remove("connected");
    tt.add("tooltip-alt");
  }
}

function triggerHotReload() {
  if (!document.getElementById("content")) {
    console.warn("skipping hot reload, no #content element");
    return;
  }

  {
    const cls = document.getElementById("hot-reload-indicator").classList;
    cls.add("hr-animation");
    setTimeout(() => cls.remove("hr-animation"), 1000);
  }

  // hot reload hack to update url
  const el = document.getElementById("hot-reload-target");
  el.setAttribute("hx-get", window.location.href);
  htmx.process(el);

  el.dispatchEvent(new Event("hot-reload", { bubbles: true }));
}

let hrEventSource = null;
function hrConnect() {
  if (hrEventSource !== null) hrEventSource.close();
  hrEventSource = new EventSource("/updates");

  function isIndex() {
    const path = window.location.pathname;
    return path == "/" || path.startsWith("/d/") || path.startsWith("/search");
  }

  function isCurrentRecipe(triggered) {
    let path = window.location.pathname;
    if (!path.startsWith("/r/")) {
      return false;
    }
    let currentRecipe = decodeURI(path.slice(3));
    let triggeredRecipe = triggered.replace(/\.cook$/, "");
    return currentRecipe === triggeredRecipe;
  }

  hrEventSource.addEventListener("open", () => hrSetIndicatorState(true));
  hrEventSource.addEventListener("error", () => hrSetIndicatorState(false));

  hrEventSource.addEventListener("modified", triggerHotReload);
  hrEventSource.addEventListener("deleted", (ev) => {
    if (isIndex()) {
      triggerHotReload();
    } else if (isCurrentRecipe(ev.data)) {
      window.location = "/?deleted=" + ev.data.replace(/\.cook$/, "");
    }
  });
  hrEventSource.addEventListener("added", () => {
    if (isIndex()) {
      triggerHotReload();
    }
  });
  hrEventSource.addEventListener("renamed", (ev) => {
    if (isIndex()) {
      return triggerHotReload();
    }
    let data = JSON.parse(ev.data);
    if (isCurrentRecipe(data.from)) {
      console.log("renamed current recipe");
      let url = "/r/" + data.to.replace(/\.cook$/, "") + window.location.search;
      window.location.replace(url);
    }
  });
}

hrConnect();
