// Toast notifications with a countdown progress bar — the web equivalent of
// the GTK app's custom AdwToast (6s lifetime, optional action button).

const LIFETIME_MS = 6000;

let container: HTMLElement | null = null;

function ensureContainer(): HTMLElement {
  if (!container) {
    container = document.createElement("div");
    container.id = "rmd-toasts";
    document.body.appendChild(container);
  }
  return container;
}

export function showToast(text: string, action?: { label: string; onClick: () => void }) {
  const host = ensureContainer();
  const toast = document.createElement("div");
  toast.className = "rmd-toast";

  const label = document.createElement("span");
  label.textContent = text;
  toast.appendChild(label);

  if (action) {
    const btn = document.createElement("button");
    btn.textContent = action.label;
    btn.addEventListener("click", () => {
      action.onClick();
      dismiss();
    });
    toast.appendChild(btn);
  }

  const bar = document.createElement("div");
  bar.className = "rmd-toast-bar";
  toast.appendChild(bar);

  host.appendChild(toast);
  // Kick the countdown on the next frame so the transition animates.
  requestAnimationFrame(() => {
    bar.style.transitionDuration = `${LIFETIME_MS}ms`;
    bar.style.transform = "scaleX(0)";
  });

  const timer = setTimeout(dismiss, LIFETIME_MS);
  function dismiss() {
    clearTimeout(timer);
    toast.classList.add("rmd-toast-out");
    setTimeout(() => toast.remove(), 200);
  }
}
