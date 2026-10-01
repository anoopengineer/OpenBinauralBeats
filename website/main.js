"use strict";

// Download URLs are stable paths; netlify.toml redirects them to the actual release files.
const PLATFORMS = {
  macos: { name: "macOS", url: "/download/macos", detail: "Apple Silicon & Intel · .dmg" },
  windows: { name: "Windows", url: "/download/windows", detail: "Windows 10/11 · 64-bit .exe" },
  linux: { name: "Linux", url: "/download/linux", detail: "x86_64 · AppImage" },
};

const PRESETS = [
  { name: "40 Hz Focus", band: "Gamma", blurb: "Sustained attention, deep work", carrier: 200, beat: 40 },
  { name: "Focus", band: "Beta", blurb: "Alert, active concentration", carrier: 440, beat: 20 },
  { name: "Relax", band: "Alpha", blurb: "Calm, light meditation", carrier: 400, beat: 10 },
  { name: "Meditate", band: "Theta", blurb: "Meditation, creativity", carrier: 300, beat: 6 },
  { name: "Deep Sleep", band: "Delta", blurb: "Sleep, deep rest", carrier: 200, beat: 2 },
];

function detectOS() {
  const ua = navigator.userAgent || "";
  const platform = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || "";

  if (/android/i.test(ua)) return "mobile";
  // iPadOS reports itself as a Mac; touch support gives it away.
  if (/iphone|ipad|ipod/i.test(ua) || (/mac/i.test(platform) && navigator.maxTouchPoints > 1)) return "mobile";
  if (/mac/i.test(platform) || /Mac OS X/.test(ua)) return "macos";
  if (/win/i.test(platform) || /Windows/.test(ua)) return "windows";
  if (/linux|x11|cros/i.test(platform) || /Linux|X11|CrOS/.test(ua)) return "linux";
  return "unknown";
}

function setupDownloads() {
  const os = detectOS();
  const primary = document.getElementById("primary-download");
  if (!primary) return;
  const label = document.getElementById("primary-label");
  const sub = document.getElementById("primary-sub");
  const others = document.getElementById("other-platforms");
  const mobileNote = document.getElementById("mobile-note");


  const p = PLATFORMS[os];
  if (!p) {
    // Mobile or unrecognized: send people to the full list instead of guessing.
    label.textContent = "See download options";
    sub.textContent = "macOS · Windows · Linux";
    primary.href = "#download";
    if (os === "mobile" && mobileNote) mobileNote.hidden = false;
    return;
  }

  label.textContent = `Download for ${p.name}`;
  sub.textContent = p.detail;
  primary.href = p.url;

  const links = Object.entries(PLATFORMS)
    .filter(([key]) => key !== os)
    .map(([, other]) => `<a href="${other.url}">${other.name}</a>`);
  if (others) {
    others.innerHTML = `Also available for ${links.join(" and ")}.`;
    others.hidden = false;
  }

  const card = document.querySelector(`.platform[data-os="${os}"]`);
  if (card) {
    card.classList.add("detected");
    card.parentElement.prepend(card);
  }
}

// --- Interactive app preview ---

let selected = 0;
let audio = null;

function fmt(n) {
  return Number.isInteger(n) ? String(n) : n.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
}

function envelopePath(beat) {
  const w = 300, h = 34, steps = 300;
  let d = "";
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const env = Math.abs(Math.cos(Math.PI * beat * t));
    const x = (t * w).toFixed(1);
    const y = (h - 4 - env * (h - 8)).toFixed(1);
    d += (i === 0 ? "M" : "L") + x + " " + y;
  }
  return d;
}

function renderPreset() {
  const p = PRESETS[selected];
  document.getElementById("mock-title").textContent = p.name;
  document.getElementById("mock-sub").textContent = `${p.band} · ${fmt(p.beat)} Hz beat`;
  document.getElementById("mock-lr").innerHTML = `L ${fmt(p.carrier)} Hz<br>R ${fmt(p.carrier + p.beat)} Hz`;
  document.getElementById("mock-env-path").setAttribute("d", envelopePath(p.beat));
  document.querySelectorAll(".preset").forEach((el, i) => el.classList.toggle("selected", i === selected));
  if (audio) audio.glideTo(p.carrier, p.beat);
}

function setupPresets() {
  const list = document.getElementById("mock-presets");
  if (!list) return;
  PRESETS.forEach((p, i) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "preset";
    b.innerHTML = `<div><b></b><small></small></div><span></span>`;
    b.querySelector("b").textContent = p.name;
    b.querySelector("small").textContent = `${p.band} · ${p.blurb}`;
    b.querySelector("span").textContent = `${fmt(p.beat)} Hz`;
    b.addEventListener("click", () => {
      selected = i;
      renderPreset();
    });
    list.appendChild(b);
  });
  renderPreset();
}

// Two sine oscillators hard-panned left/right: the same thing the app does natively.
function createAudio(carrier, beat) {
  const Ctx = window.AudioContext || window.webkitAudioContext;
  if (!Ctx) return null;
  const ctx = new Ctx();
  const merger = ctx.createChannelMerger(2);
  const gain = ctx.createGain();
  gain.gain.value = 0;
  merger.connect(gain).connect(ctx.destination);

  const left = ctx.createOscillator();
  const right = ctx.createOscillator();
  left.frequency.value = carrier;
  right.frequency.value = carrier + beat;
  left.connect(merger, 0, 0);
  right.connect(merger, 0, 1);
  left.start();
  right.start();

  const now = () => ctx.currentTime;
  gain.gain.setTargetAtTime(0.18, now(), 0.05);

  return {
    glideTo(c, b) {
      for (const [osc, f] of [[left, c], [right, c + b]]) {
        osc.frequency.cancelScheduledValues(now());
        osc.frequency.setValueAtTime(osc.frequency.value, now());
        osc.frequency.linearRampToValueAtTime(f, now() + 2);
      }
    },
    stop() {
      gain.gain.setTargetAtTime(0, now(), 0.04);
      setTimeout(() => ctx.close(), 400);
    },
  };
}

function setupDemo() {
  const btn = document.getElementById("mock-play");
  if (!btn) return;
  const env = document.getElementById("mock-env");
  btn.addEventListener("click", () => {
    if (audio) {
      audio.stop();
      audio = null;
    } else {
      const p = PRESETS[selected];
      audio = createAudio(p.carrier, p.beat);
      if (!audio) {
        btn.textContent = "Audio not supported";
        return;
      }
    }
    btn.textContent = audio ? "Stop" : "Play demo";
    btn.classList.toggle("playing", !!audio);
    env.classList.toggle("playing", !!audio);
  });
}

// "Try it" players on guide pages: <button data-tone data-carrier="200" data-beat="40">.
function setupTonePlayers() {
  const buttons = document.querySelectorAll("button[data-tone]");
  let active = null;
  const reset = (b) => {
    b.textContent = b.dataset.label;
    b.classList.remove("playing");
  };
  buttons.forEach((b) => {
    b.dataset.label = b.textContent;
    b.addEventListener("click", () => {
      if (active) {
        active.audio.stop();
        reset(active.button);
        const same = active.button === b;
        active = null;
        if (same) return;
      }
      const tone = createAudio(Number(b.dataset.carrier), Number(b.dataset.beat));
      if (!tone) {
        b.textContent = "Audio not supported";
        return;
      }
      active = { audio: tone, button: b };
      b.textContent = "Stop";
      b.classList.add("playing");
    });
  });
}

setupDownloads();
setupPresets();
setupDemo();
setupTonePlayers();
