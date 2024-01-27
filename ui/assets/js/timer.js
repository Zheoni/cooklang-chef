/* Timers */

class RecipeTimer {
  constructor(seconds, name) {
    this.audio = document.getElementById("timer-audio");
    this.timerEl = document.getElementById("timer");
    this.timerNameEl = document.getElementById("timer-name");
    this.timerTextEl = document.getElementById("timer-text");
    this.timerStartBtn = document.getElementById("timer-start-btn");
    this.timerPauseBtn = document.getElementById("timer-pause-btn");

    this.seconds = seconds;
    this.name = name;
    this.timeoutId = null;
    this.intervalId = null;
    this.state = "paused";

    this.updateText();
    if (name) {
      this.timerNameEl.textContent = name;
    }
    this.timerPauseBtn.hidden = false;
    this.timerStartBtn.hidden = true;
    this.timerEl.hidden = false;
    this.timerEl.classList.remove("remove");

    // disable other timer buttons
    document
      .querySelectorAll("[data-timer]")
      .forEach((el) => (el.disabled = true));
  }

  start() {
    if (this.state === "finished") return;
    const end = new Date();
    end.setSeconds(end.getSeconds() + this.seconds);
    this.end = end;

    const secs = Math.round(dateDiffSecs(new Date(), end));
    this.timeoutId = setTimeout(() => this.finish(), secs * 1000);
    this.intervalId = setInterval(() => this.updateText(), 250); // Could be optimized...
    this.state = "running";

    this.timerPauseBtn.style.display = "";
    this.timerStartBtn.style.display = "none";
  }

  pause() {
    if (this.state === "finished") return;
    if (this.timeoutId) clearTimeout(this.timeoutId);
    if (this.intervalId) clearInterval(this.intervalId);
    this.timeoutId = null;

    if (this.end) {
      const secs = Math.round(dateDiffSecs(new Date(), this.end));
      this.seconds = secs;
      this.end = null;
    }
    this.state = "paused";

    this.timerPauseBtn.style.display = "none";
    this.timerStartBtn.style.display = "";
  }

  finish() {
    this.state = "finished";
    this.seconds = 0;
    if (this.timeoutId) clearTimeout(this.timeoutId);
    this.timeoutId = null;
    if (this.intervalId) clearInterval(this.intervalId);
    this.intervalId = null;
    this.audio.play();
    this.timerPauseBtn.style.display = "none";
    this.timerStartBtn.style.display = "none";

    setTimeout(() => this.destroy(), 5000);
  }

  destroy() {
    this.timerEl.classList.add("remove");
    setTimeout(() => (this.timerEl.hidden = true), 1000);
    if (this.timeoutId) clearTimeout(this.timeoutId);
    this.timeoutId = null;
    if (this.intervalId) clearInterval(this.intervalId);
    this.intervalId = null;
    this.end = null;
    this.seconds = 0;

    // reenable timer buttons
    document
      .querySelectorAll("[data-timer]")
      .forEach((el) => (el.disabled = false));
  }

  remainingSeconds() {
    if (this.end) {
      const secs = dateDiffSecs(new Date(), this.end);
      return Math.max(0, Math.round(secs));
    } else {
      return this.seconds;
    }
  }

  timerText() {
    let secs = this.remainingSeconds();

    let minutes = Math.trunc(secs / 60);
    secs %= 60;
    let hours = Math.trunc(minutes / 60);
    minutes %= 60;

    const parts = [];
    if (hours > 0) {
      parts.push(hours.toString());
    }
    parts.push(minutes.toString().padStart(2, "0"));
    parts.push(secs.toString().padStart(2, "0"));
    return parts.join(":");
  }

  updateText() {
    this.timerTextEl.textContent = this.timerText();
  }

  get state() {
    return this._state;
  }

  set state(s) {
    this._state = s;
    this.timerEl.setAttribute("data-state", s);
  }
}

function dateDiffSecs(start, end) {
  const diffMillis = end.getTime() - start.getTime();
  return diffMillis / 1000;
}

let currentTimer = null;
function registerTimerBtn(el) {
  if (currentTimer !== null && currentTimer.state !== "finished") {
    el.disabled = true;
  }

  const seconds = el.getAttribute("data-timer") * 1;
  let name = el.getAttribute("data-timer-name");
  if (name === "false") name = null;
  if (!Number.isFinite(seconds)) return;
  el.addEventListener("click", () => {
    if (currentTimer !== null && currentTimer.state !== "finished") return;
    currentTimer = new RecipeTimer(seconds, name);
    currentTimer.start();
  });
}

document.getElementById("timer-start-btn").addEventListener("click", () => {
  if (!currentTimer || currentTimer.state !== "paused") return;
  currentTimer.start();
});
document.getElementById("timer-pause-btn").addEventListener("click", () => {
  if (!currentTimer || currentTimer.state !== "running") return;
  currentTimer.pause();
});
document.getElementById("timer-remove-btn").addEventListener("click", () => {
  if (!currentTimer) return;
  currentTimer.destroy();
});

htmx.onLoad((el) => {
  htmx.findAll(el, "[data-timer]").forEach(registerTimerBtn);
});
