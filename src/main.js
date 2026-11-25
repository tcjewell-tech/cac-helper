const { invoke } = window.__TAURI__.core;

const BADGE_GLYPHS = { pass: "✓", warning: "!", fail: "✕" };

// Backend strings (reader names, profile names, paths, tool stderr) are
// untrusted, so the UI is built with DOM nodes and textContent, never HTML strings.
function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text != null) node.textContent = text;
  return node;
}

// Each item is a string, a node, or an array of those (one <li> per item).
function list(items) {
  const ul = el("ul");
  for (const item of items) {
    const li = el("li");
    li.append(...[item].flat());
    ul.append(li);
  }
  return ul;
}

function showResult(statusEl, msg, items) {
  const headline = el("p", "headline");
  if (msg.status) {
    // Decorative: the headline next to it already names the state.
    const badge = el("span", `badge ${msg.status}`, BADGE_GLYPHS[msg.status]);
    badge.setAttribute("aria-hidden", "true");
    headline.append(badge);
  }
  headline.append(msg.headline);
  statusEl.replaceChildren(headline, el("p", "detail", msg.detail));
  if (items.length) statusEl.append(list(items));
}

function showReport(statusEl, okLabel, report) {
  statusEl.replaceChildren();
  if (report.succeeded.length) statusEl.append(el("p", "headline", okLabel), list(report.succeeded));
  if (report.failed.length) {
    statusEl.append(el("p", "headline", "Failed:"), list(report.failed.map(([p, err]) => `${p}: ${err}`)));
  }
}

const MESSAGES = {
  NoReaderFound: {
    status: "fail",
    headline: "No reader detected.",
    detail: "Plug in your smart card reader. If it's already plugged in, the driver may not be installed yet — that's the next step.",
  },
  ReaderReadyNoCard: {
    status: "warning",
    headline: "Reader found, no card inserted.",
    detail: "Insert your CAC. Make sure the gold chip is facing up and fully seated.",
  },
  CardUnreadable: {
    status: "fail",
    headline: "Card detected but unreadable.",
    detail: "This usually means middleware is missing or the card is seated wrong. Try reseating the card, then re-check.",
  },
  // Warning, not pass, until P2.1: a built-in SIM reader also reports a card (VER-025).
  CardPresent: {
    status: "warning",
    headline: "A card is responding.",
    detail: "The reader sees a card, but this check can't yet confirm it's your CAC — some laptops' built-in SIM readers show up here too. Check that your CAC is in the reader listed below. Next: confirm DoD certs are trusted and Firefox can see the card.",
  },
};

async function checkReader() {
  const statusEl = document.getElementById("status");
  statusEl.textContent = "Checking...";

  try {
    const result = await invoke("check_reader");
    const msg = MESSAGES[result.outcome] || { headline: "Unknown state", detail: "" };

    showResult(statusEl, msg, result.readers.map((r) => `${r.name}: ${r.card_present ? "card present" : "no card"}`));
  } catch (e) {
    statusEl.textContent = "Error checking reader: " + e;
  }
}

const MIDDLEWARE_MESSAGES = {
  NotFound: {
    status: "fail",
    headline: "No CAC middleware found.",
    detail: "Your reader may work, but the browser and OS won't be able to use the card without middleware. Install OpenSC (free, open source) or your reader vendor's official driver from the DoD-approved middleware list.",
  },
  FoundButInvalid: {
    status: "fail",
    headline: "Middleware found but not working correctly.",
    detail: "A middleware install was detected but failed a basic validity check — likely a corrupt install or a 32-bit/64-bit mismatch. Try reinstalling the latest version for your system architecture.",
  },
  Valid: {
    status: "pass",
    headline: "Middleware working.",
    detail: "A valid PKCS#11 module was found. Next: confirm DoD certs are trusted, and register the module with Firefox.",
  },
};

let lastMiddlewarePath = null;
let lastMiddlewareName = null;

async function checkMiddleware() {
  const statusEl = document.getElementById("middleware-status");
  statusEl.textContent = "Checking...";

  try {
    const result = await invoke("check_middleware");
    const msg = MIDDLEWARE_MESSAGES[result.outcome] || { headline: "Unknown state", detail: "" };

    lastMiddlewarePath = result.valid_path || null;
    lastMiddlewareName = result.found.length > 0 ? result.found[0].name : null;

    showResult(statusEl, msg, result.found.map((c) => [`${c.name} `, el("span", "path", `(${c.path})`)]));

    // A valid module path is what module 4 needs to check/register against.
    if (lastMiddlewarePath) checkNss();
  } catch (e) {
    statusEl.textContent = "Error checking middleware: " + e;
  }
}

document.getElementById("check-btn").addEventListener("click", checkReader);
document.getElementById("check-middleware-btn").addEventListener("click", checkMiddleware);
window.addEventListener("DOMContentLoaded", () => {
  checkReader();
  checkMiddleware();
  checkTrust();
});

const TRUST_MESSAGES = {
  NotInstalled: {
    status: "fail",
    headline: "DoD certs not found in trust store.",
    detail: "Your browser won't trust CAC-based logins yet. Download the official cert bundle from DISA and install it below.",
  },
  PartiallyInstalled: {
    status: "warning",
    headline: "Some DoD roots found, but not all.",
    detail: "This can happen with an older or partial install. Re-installing the current bundle from DISA is the safest fix.",
  },
  Installed: {
    status: "pass",
    headline: "DoD root certs are trusted.",
    detail: "The OS trust store has the current DoD PKI roots. Next: register the middleware module with Firefox (module 4).",
  },
};

async function checkTrust() {
  const statusEl = document.getElementById("trust-status");
  statusEl.textContent = "Checking...";

  try {
    const result = await invoke("check_trust");
    const msg = TRUST_MESSAGES[result.outcome] || { headline: "Unknown state", detail: "" };
    showResult(statusEl, msg, result.roots_found);
  } catch (e) {
    statusEl.textContent = "Error checking trust store: " + e;
  }
}

async function openDodPage() {
  try {
    await invoke("open_dod_cert_page");
  } catch (e) {
    alert("Couldn't open the page: " + e);
  }
}

async function installCerts() {
  const raw = document.getElementById("cert-paths").value.trim();
  if (!raw) {
    alert("Enter at least one cert file path first.");
    return;
  }
  const paths = raw.split(",").map((p) => p.trim()).filter(Boolean);

  // Explicit, named confirmation before a security-sensitive, mutating
  // action -- never skip this even though the Rust side also requires
  // the OS's own admin/password prompt.
  const confirmed = confirm(
    `Install ${paths.length} certificate(s) into your system trust store?\n\n${paths.join("\n")}\n\nYou'll see a Windows admin (UAC) prompt or a macOS Touch ID/password prompt — approve it to continue.`
  );
  if (!confirmed) return;

  const statusEl = document.getElementById("trust-status");
  statusEl.textContent = "Installing...";

  try {
    const report = await invoke("install_certs", { paths });
    showReport(statusEl, "Installed:", report);
    checkTrust();
  } catch (e) {
    statusEl.textContent = "Error installing certs: " + e;
  }
}

document.getElementById("check-trust-btn").addEventListener("click", checkTrust);
document.getElementById("open-dod-page-btn").addEventListener("click", openDodPage);
document.getElementById("install-certs-btn").addEventListener("click", installCerts);

const NSS_MESSAGES = {
  NoProfilesFound: {
    status: "fail",
    headline: "No Firefox profiles found.",
    detail: "Firefox may not be installed, or profiles.ini wasn't where we expected.",
  },
  ModuleNotRegisteredAnywhere: {
    status: "fail",
    headline: "Middleware not registered with Firefox.",
    detail: "Firefox has its own certificate database, separate from the OS. Your reader may work in other apps but not in Firefox until this is registered.",
  },
  PartiallyRegistered: {
    status: "warning",
    headline: "Registered in some profiles, not others.",
    detail: "If you use multiple Firefox profiles, register in each one you need the CAC to work in.",
  },
  RegisteredInAllProfiles: {
    status: "pass",
    headline: "Firefox is set up.",
    detail: "The middleware module is registered in every Firefox profile found. That's all four modules — reader, middleware, cert trust, and Firefox — checked.",
  },
};

async function checkNss() {
  const statusEl = document.getElementById("nss-status");

  if (!lastMiddlewarePath) {
    statusEl.textContent = "Run the middleware check above first — no valid module path found yet.";
    return;
  }

  statusEl.textContent = "Checking...";

  try {
    const result = await invoke("check_nss", { modulePath: lastMiddlewarePath });
    const msg = NSS_MESSAGES[result.outcome] || { headline: "Unknown state", detail: "" };
    showResult(statusEl, msg, result.profiles.map((p) => `${p.name}: ${p.module_registered ? "registered" : "not registered"}`));
  } catch (e) {
    statusEl.textContent = "Error checking Firefox profiles: " + e;
  }
}

async function registerNss() {
  if (!lastMiddlewarePath) {
    alert("Run the middleware check first so there's a module to register.");
    return;
  }

  const confirmed = confirm(
    `Register "${lastMiddlewareName || lastMiddlewarePath}" with Firefox?\n\nMake sure Firefox is fully closed first, or this will fail.`
  );
  if (!confirmed) return;

  const statusEl = document.getElementById("nss-status");
  statusEl.textContent = "Registering...";

  try {
    // Re-check profiles fresh so we register into all of them, not a stale list.
    const check = await invoke("check_nss", { modulePath: lastMiddlewarePath });
    const profilePaths = check.profiles.map((p) => p.path);

    const report = await invoke("register_nss", {
      profilePaths,
      moduleName: lastMiddlewareName || "CAC Helper Middleware",
      modulePath: lastMiddlewarePath,
    });

    showReport(statusEl, "Registered:", report);
  } catch (e) {
    statusEl.textContent = "Error registering with Firefox: " + e;
  }
}

document.getElementById("check-nss-btn").addEventListener("click", checkNss);
document.getElementById("register-nss-btn").addEventListener("click", registerNss);
